#![cfg(windows)]

use std::{
    env,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::{json, Value};
use url::Url;

#[test]
fn real_workspace_hover_definition_and_inlay_hint_are_visible() {
    let root = Path::new(r"D:\Twy59sGthb\new_notes");
    let temp_workspace = tempfile::Builder::new()
        .prefix("windows-repro-")
        .tempdir_in(root)
        .expect("create temp workspace");
    let temp_root = temp_workspace.path().to_path_buf();
    let nested_dir = temp_root.join("nested");
    let source_doc = temp_root.join("source.md");
    let target_doc = nested_dir.join("target.md");
    let source_text = "计划 🧭 [[nested/target]]";
    let target_text = "# target\n\nrepro";

    assert!(root.exists(), "missing test workspace: {root:?}");
    std::fs::create_dir_all(&nested_dir).expect("create nested dir");
    std::fs::write(&source_doc, source_text).expect("write source doc");
    std::fs::write(&target_doc, target_text).expect("write target doc");

    let (line, character) = link_position(source_text, "[[nested/target]]");
    let doc_uri = Url::from_file_path(&source_doc)
        .expect("document uri")
        .to_string();
    let target_uri = Url::from_file_path(&target_doc)
        .expect("target uri")
        .to_string();
    let root_uri = Url::from_directory_path(root)
        .expect("workspace uri")
        .to_string();

    let binary = binary_path();
    let mut process = Command::new(&binary)
        .current_dir(root)
        .env("IWE_DEBUG", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn {binary:?}: {error}"));

    let mut stdin = process.stdin.take().expect("child stdin");
    let stdout = process.stdout.take().expect("child stdout");
    let mut stdout = BufReader::new(stdout);

    let mut next_id = 1_i32;
    let initialize_id = next_request_id(&mut next_id);
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": initialize_id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "workspaceFolders": [{"uri": root_uri, "name": "new_notes"}],
                "capabilities": {},
                "clientInfo": {"name": "windows-repro-test", "version": "1.0"}
            }
        }),
    );
    let initialize_resp = wait_for_response(&mut stdout, initialize_id);
    assert!(
        initialize_resp.get("error").is_none(),
        "initialize failed: {initialize_resp}"
    );

    send(
        &mut stdin,
        &json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
    );
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": doc_uri,
                    "languageId": "markdown",
                    "version": 1,
                    "text": source_text,
                }
            }
        }),
    );

    let hover_id = next_request_id(&mut next_id);
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": hover_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": doc_uri},
                "position": {"line": line, "character": character}
            }
        }),
    );

    let definition_id = next_request_id(&mut next_id);
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": definition_id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": doc_uri},
                "position": {"line": line, "character": character}
            }
        }),
    );

    let inlay_id = next_request_id(&mut next_id);
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": inlay_id,
            "method": "textDocument/inlayHint",
            "params": {
                "textDocument": {"uri": target_uri},
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": target_text.lines().count(), "character": 0}
                }
            }
        }),
    );

    let hover_resp = wait_for_response(&mut stdout, hover_id);
    let definition_resp = wait_for_response(&mut stdout, definition_id);
    let inlay_resp = wait_for_response(&mut stdout, inlay_id);

    let hover_result = hover_resp.get("result").expect("hover result missing");
    assert!(!hover_result.is_null(), "hover returned null: {hover_resp}");

    let definition_result = definition_resp
        .get("result")
        .expect("definition result missing");
    assert!(
        match definition_result {
            Value::Object(_) => true,
            Value::Array(items) => !items.is_empty(),
            _ => false,
        },
        "definition returned empty: {definition_resp}"
    );

    let inlay_result = inlay_resp
        .get("result")
        .and_then(Value::as_array)
        .expect("inlay hint result missing");
    assert!(
        !inlay_result.is_empty(),
        "inlay hints returned empty: {inlay_resp}"
    );

    let shutdown_id = next_request_id(&mut next_id);
    send(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": shutdown_id,
            "method": "shutdown",
            "params": {}
        }),
    );
    let _ = wait_for_response(&mut stdout, shutdown_id);
    send(
        &mut stdin,
        &json!({"jsonrpc": "2.0", "method": "exit", "params": {}}),
    );

    let _ = process.wait();
}

fn next_request_id(next_id: &mut i32) -> i32 {
    let id = *next_id;
    *next_id += 1;
    id
}

fn send(stdin: &mut ChildStdin, message: &Value) {
    let payload = serde_json::to_vec(message).expect("serialize json-rpc message");
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes()).expect("write header");
    stdin.write_all(&payload).expect("write body");
    stdin.flush().expect("flush message");
}

fn wait_for_response(stdout: &mut BufReader<ChildStdout>, id: i32) -> Value {
    loop {
        let message = read_message(stdout);
        if message.get("id").and_then(Value::as_i64) == Some(id as i64) {
            return message;
        }
    }
}

fn read_message(stdout: &mut BufReader<ChildStdout>) -> Value {
    let mut content_length = None;
    let mut line = String::new();

    loop {
        line.clear();
        stdout.read_line(&mut line).expect("read header line");
        if line.is_empty() {
            panic!("unexpected EOF while reading LSP headers");
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().expect("parse content length"));
        }
    }

    let mut body = vec![0_u8; content_length.expect("missing Content-Length")];
    stdout.read_exact(&mut body).expect("read LSP body");
    serde_json::from_slice(&body).expect("parse LSP message")
}

fn link_position(text: &str, needle: &str) -> (u32, u32) {
    for (line_index, line) in text.lines().enumerate() {
        if let Some(byte_offset) = line.find(needle) {
            let character = line[..byte_offset]
                .chars()
                .map(|ch| ch.len_utf16() as u32)
                .sum::<u32>()
                + 2;
            return (line_index as u32, character);
        }
    }

    panic!("needle not found: {needle}");
}

fn binary_path() -> PathBuf {
    if let Ok(path) = env::var("CARGO_BIN_EXE_iwes") {
        return PathBuf::from(path);
    }

    let current_exe = env::current_exe().expect("current exe");
    let target_dir = current_exe
        .parent()
        .and_then(Path::parent)
        .expect("test exe parent")
        .to_path_buf();
    target_dir.join("iwes.exe")
}
