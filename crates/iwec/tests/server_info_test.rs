use iwec::IweServer;
use rmcp::model::{CallToolRequestParams, ContentBlock, ServerPeerInfo};
use rmcp::transport::TokioChildProcess;
use rmcp::ServiceExt;
use std::path::Path;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::process::Command;

use crate::fixture::Fixture;

const DEFAULT_INSTRUCTIONS: &str = "IWE knowledge graph server. Tools: iwe_find, iwe_retrieve, iwe_tree, iwe_stats, iwe_squash, iwe_create, iwe_update, iwe_delete, iwe_query, iwe_rename, iwe_extract, iwe_inline, iwe_normalize, iwe_attach. Prompts: explore, review, refactor. Resources: iwe://documents/{key}, iwe://tree, iwe://stats, iwe://config.";

fn identity(info: &ServerPeerInfo) -> (Option<String>, Option<String>) {
    (
        info.server_info.as_ref().map(|server| server.name.clone()),
        info.instructions.clone(),
    )
}

#[tokio::test]
async fn default_server_identity() {
    let fixture = Fixture::with_documents(vec![]).await;

    assert_eq!(
        identity(&fixture.server_info()),
        (
            Some("iwe".to_string()),
            Some(DEFAULT_INSTRUCTIONS.to_string())
        )
    );
}

#[tokio::test]
async fn custom_instructions_precede_default() {
    let server =
        IweServer::from_documents(vec![]).with_instructions("Personal notes.\n".to_string());
    let fixture = Fixture::with_server(server).await;

    assert_eq!(
        identity(&fixture.server_info()),
        (
            Some("iwe".to_string()),
            Some(format!("Personal notes.\n\n{}", DEFAULT_INSTRUCTIONS))
        )
    );
}

#[tokio::test]
async fn blank_instructions_keep_default() {
    let server = IweServer::from_documents(vec![]).with_instructions("  \n".to_string());
    let fixture = Fixture::with_server(server).await;

    assert_eq!(
        identity(&fixture.server_info()),
        (
            Some("iwe".to_string()),
            Some(DEFAULT_INSTRUCTIONS.to_string())
        )
    );
}

async fn connect(args: &[&str], current_dir: &Path) -> (ServerPeerInfo, serde_json::Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_iwec"));
    command.args(args).current_dir(current_dir);
    let (transport, _) = TokioChildProcess::builder(command)
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn iwec");
    let client = ().serve(transport).await.expect("client to connect");

    let info = client
        .peer_info()
        .expect("server info after handshake")
        .as_ref()
        .clone();
    let result = client
        .call_tool(CallToolRequestParams::new("iwe_find"))
        .await
        .expect("tool call to succeed");
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<String>();
    let json: serde_json::Value = serde_json::from_str(&text).expect("result to be valid JSON");

    client.cancel().await.expect("client to disconnect");

    (info, json)
}

fn workspace_with_document() -> TempDir {
    let workspace = tempfile::tempdir().expect("workspace dir");
    std::fs::write(workspace.path().join("welcome.md"), "# Welcome\n").expect("write document");
    workspace
}

fn welcome_document() -> serde_json::Value {
    serde_json::json!([
        {
            "key": "welcome",
            "title": "Welcome",
            "references": [],
            "includes": [],
            "referencedBy": [],
            "includedBy": []
        }
    ])
}

#[tokio::test]
async fn instructions_file_flag_with_workspace_flag() {
    let workspace = workspace_with_document();
    let other = tempfile::tempdir().expect("other dir");
    std::fs::write(other.path().join("instructions.md"), "Work notes.\n")
        .expect("write instructions");
    let workspace_path = workspace.path().to_string_lossy().to_string();

    let (info, json) = connect(
        &[
            "--instructions-file",
            "instructions.md",
            "--workspace",
            &workspace_path,
        ],
        other.path(),
    )
    .await;

    assert_eq!(
        identity(&info),
        (
            Some("iwe".to_string()),
            Some(format!("Work notes.\n\n{}", DEFAULT_INSTRUCTIONS))
        )
    );
    assert_eq!(json, welcome_document());
}

#[tokio::test]
async fn instructions_text_flag() {
    let workspace = workspace_with_document();

    let (info, json) = connect(&["--instructions", "Work notes."], workspace.path()).await;

    assert_eq!(
        identity(&info),
        (
            Some("iwe".to_string()),
            Some(format!("Work notes.\n\n{}", DEFAULT_INSTRUCTIONS))
        )
    );
    assert_eq!(json, welcome_document());
}

#[test]
fn instructions_text_and_file_flags_conflict() {
    let workspace = workspace_with_document();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_iwec"))
        .args([
            "--instructions",
            "Work notes.",
            "--instructions-file",
            "instructions.md",
        ])
        .current_dir(workspace.path())
        .output()
        .expect("run iwec");

    assert_eq!(
        (output.status.code(), String::from_utf8_lossy(&output.stderr).to_string()),
        (
            Some(2),
            "error: the argument '--instructions <TEXT>' cannot be used with '--instructions-file <PATH>'\n\nUsage: iwec --instructions <TEXT>\n\nFor more information, try '--help'.\n".to_string()
        )
    );
}
