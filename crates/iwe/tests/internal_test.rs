use std::fs::{create_dir_all, read_to_string, write};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use indoc::indoc;
use tempfile::TempDir;

fn run_digest(lines: &[&str], extra: &[&str]) -> Output {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let path = temp_dir.path().join("transcript.jsonl");
    let mut body = lines.join("\n");
    body.push('\n');
    write(&path, body).expect("Failed to write transcript");

    Command::new(crate::common::get_iwe_binary_path())
        .arg("internal")
        .arg("claude")
        .arg("digest")
        .arg("--path")
        .arg(&path)
        .args(extra)
        .output()
        .expect("Failed to execute iwe internal claude digest")
}

fn digest(lines: &[&str], extra: &[&str]) -> String {
    let output = run_digest(lines, extra);
    assert_eq!(output.status.code(), Some(0));
    String::from_utf8(output.stdout).expect("Valid UTF-8 output")
}

#[test]
fn budget_not_reached_covers_every_line() {
    let stdout = digest(
        &[
            r#"{"type":"user","message":{"content":"one"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"two"}]}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [user]
            one

            [assistant]
            two
        "}
    );
}

#[test]
fn overflowing_chunk_is_left_for_the_next_sweep() {
    let stdout = digest(
        &[
            r#"{"type":"user","message":{"content":"one"}}"#,
            r#"{"type":"user","message":{"content":"two"}}"#,
            r#"{"type":"user","message":{"content":"three"}}"#,
        ],
        &["--max-chars", "22"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [user]
            one

            [user]
            two
        "}
    );
}

#[test]
fn oversized_first_chunk_is_truncated_and_covered() {
    let stdout = digest(
        &[r#"{"type":"user","message":{"content":"abcdefghij"}}"#],
        &["--max-chars", "9"],
    );

    assert_eq!(
        stdout,
        "1\n[user]\nab\n[truncated at 9 chars; the line rendered 17]\n"
    );
}

#[test]
fn truncation_lands_on_character_boundaries() {
    let stdout = digest(
        &[r#"{"type":"user","message":{"content":"日本語です"}}"#],
        &["--max-chars", "9"],
    );

    assert_eq!(
        stdout,
        "1\n[user]\n日本\n[truncated at 9 chars; the line rendered 12]\n"
    );
}

#[test]
fn truncation_covers_the_same_lines_as_the_ascii_equivalent() {
    let stdout = digest(
        &[r#"{"type":"user","message":{"content":"abcde"}}"#],
        &["--max-chars", "9"],
    );

    assert_eq!(
        stdout,
        "1\n[user]\nab\n[truncated at 9 chars; the line rendered 12]\n"
    );
}

#[test]
fn unrenderable_lines_advance_covered_without_adding_text() {
    let stdout = digest(
        &[
            "not json at all",
            r#"{"type":"summary","summary":"one"}"#,
            r#"{"type":"user","message":{"content":"kept"}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            3
            [user]
            kept
        "}
    );
}

#[test]
fn blank_and_unparseable_lines_alone_still_advance_covered() {
    let stdout = digest(&["not json at all", ""], &["--max-chars", "100"]);

    assert_eq!(stdout, "2\n\n");
}

#[test]
fn from_skips_leading_lines_and_covered_is_relative() {
    let stdout = digest(
        &[
            r#"{"type":"user","message":{"content":"one"}}"#,
            r#"{"type":"user","message":{"content":"two"}}"#,
            r#"{"type":"user","message":{"content":"three"}}"#,
        ],
        &["--from", "2", "--max-chars", "100"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            1
            [user]
            three
        "}
    );
}

#[test]
fn meta_users_and_unlisted_top_level_types_are_skipped() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"meta"}}"#,
            r#"{"type":"summary","summary":"one"}"#,
            r#"{"type":"system","content":"one"}"#,
            r#"{"type":"file-history-snapshot","messageId":"one"}"#,
            r#"{"message":{"content":"no type"}}"#,
            r#"{"type":"user","message":{"content":"kept"}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            6
            [user]
            kept
        "}
    );
}

#[test]
fn stop_hook_turns_are_skipped_whole() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nsweep now"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-one","name":"Agent","input":{"subagent_type":"plugin:distill","prompt":"work the jobs"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"tool_use_id":"tool-one","type":"tool_result","content":"agent launched"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"launched in the background"}]}}"#,
            r#"{"type":"user","origin":{"kind":"task-notification"},"message":{"content":"agent finished"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"the queue is drained"}]}}"#,
            r#"{"type":"user","message":{"content":"kept"}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            7
            [user]
            kept
        "}
    );
}

#[test]
fn a_span_of_stop_hook_turns_alone_renders_nothing() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nsweep now"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-one","name":"Agent","input":{"subagent_type":"distill","prompt":"work the jobs"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"tool_use_id":"tool-one","type":"tool_result","content":"agent launched"}]}}"#,
            r#"{"type":"user","origin":{"kind":"task-notification"},"message":{"content":"agent finished"}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(stdout, "4\n\n");
}

#[test]
fn a_stop_hook_span_ends_at_the_next_tool_use() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nsweep now"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"first"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-one","name":"Bash","input":{"command":"ls"}}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"second"}]}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            4
            [tool: Bash] ls

            [assistant]
            second
        "}
    );
}

#[test]
fn an_agent_call_outside_the_sweep_is_kept() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nsweep now"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-one","name":"Agent","input":{"subagent_type":"reviewer","prompt":"review the branch"}}]}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [tool: Agent] review the branch
        "}
    );
}

#[test]
fn a_meta_turn_that_is_not_stop_hook_feedback_opens_nothing() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"a different notice"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"kept"}]}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [assistant]
            kept
        "}
    );
}

#[test]
fn a_queued_human_message_is_captured() {
    let stdout = digest(
        &[
            r#"{"type":"attachment","attachment":{"type":"queued_command","prompt":"one more thing","origin":{"kind":"human"}}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            1
            [user]
            one more thing
        "}
    );
}

#[test]
fn a_queued_message_survives_a_stop_hook_span() {
    let stdout = digest(
        &[
            r#"{"type":"user","isMeta":true,"message":{"content":"Stop hook feedback:\nsweep now"}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tool-one","name":"Agent","input":{"subagent_type":"plugin:distill","prompt":"work the jobs"}}]}}"#,
            r#"{"type":"attachment","attachment":{"type":"queued_command","prompt":"one more thing","origin":{"kind":"human"}}}"#,
            r#"{"type":"user","message":{"content":[{"tool_use_id":"tool-one","type":"tool_result","content":"agent launched"}]}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            4
            [user]
            one more thing
        "}
    );
}

#[test]
fn a_queued_command_from_another_source_is_skipped() {
    let stdout = digest(
        &[
            r#"{"type":"attachment","attachment":{"type":"queued_command","prompt":"machine issued","origin":{"kind":"task-notification"}}}"#,
            r#"{"type":"attachment","attachment":{"type":"total_tokens_reminder","tokens":100}}"#,
            r#"{"type":"user","message":{"content":"kept"}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            3
            [user]
            kept
        "}
    );
}

#[test]
fn thinking_renders_its_text_and_never_its_signature() {
    let stdout = digest(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"","signature":"c2lnbmF0dXJl"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"a thought","signature":"c2lnbmF0dXJl"}]}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [thinking]
            a thought
        "}
    );
}

#[test]
fn unknown_content_block_degrades_to_a_placeholder() {
    let stdout = digest(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"widget","body":"content one","tail":"content two"}]}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(stdout, "1\n[block: widget] content one content two\n");
}

#[test]
fn unknown_content_block_without_leaves_renders_the_head_alone() {
    let stdout = digest(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"widget","count":1}]}}"#,
            r#"{"type":"user","message":{"content":[{"count":1},"raw element"]}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            2
            [block: widget]

            [block: unknown]

            [block: unknown] raw element
        "}
    );
}

#[test]
fn tool_use_prefers_the_earlier_input_field() {
    let stdout = digest(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Runner","input":{"url":"http://example.com","command":"one two"}}]}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(stdout, "1\n[tool: Runner] one two\n");
}

#[test]
fn tool_use_falls_back_to_the_serialized_input() {
    let stdout = digest(
        &[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Runner","input":{"other":"one"}}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Runner","input":{}}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","input":{"command":"   "}}]}}"#,
        ],
        &["--max-chars", "200"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            3
            [tool: Runner] {\"other\":\"one\"}

            [tool: Runner]

            [tool: unknown] {\"command\":\" \"}
        "}
    );
}

#[test]
fn tool_result_uses_the_shorter_limit() {
    let content = "a".repeat(601);
    let line = format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","content":"{}"}}]}}}}"#,
        content
    );
    let stdout = digest(&[&line], &["--max-chars", "1000"]);

    assert_eq!(
        stdout,
        format!("1\n[tool result] {}… (+361 chars)\n", "a".repeat(240))
    );
}

#[test]
fn tool_result_error_uses_the_longer_limit() {
    let content = "a".repeat(601);
    let line = format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","is_error":true,"content":"{}"}}]}}}}"#,
        content
    );
    let stdout = digest(&[&line], &["--max-chars", "1000"]);

    assert_eq!(
        stdout,
        format!("1\n[tool error] {}… (+1 chars)\n", "a".repeat(600))
    );
}

#[test]
fn excerpt_overflow_is_reported_in_characters() {
    let content = "日".repeat(601);
    let line = format!(
        r#"{{"type":"user","message":{{"content":[{{"type":"tool_result","is_error":true,"content":"{}"}}]}}}}"#,
        content
    );
    let stdout = digest(&[&line], &["--max-chars", "1000"]);

    assert_eq!(
        stdout,
        format!("1\n[tool error] {}… (+1 chars)\n", "日".repeat(600))
    );
}

#[test]
fn tool_result_joins_content_parts_and_collapses_whitespace() {
    let stdout = digest(
        &[
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":[{"text":"one  two"},{"other":1},{"text":"three"}]}]}}"#,
        ],
        &["--max-chars", "100"],
    );

    assert_eq!(stdout, "1\n[tool result] one two three\n");
}

#[test]
fn missing_path_fails_without_writing_stdout() {
    let output = Command::new(crate::common::get_iwe_binary_path())
        .arg("internal")
        .arg("claude")
        .arg("digest")
        .arg("--max-chars")
        .arg("100")
        .output()
        .expect("Failed to execute iwe internal claude digest");

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn missing_max_chars_fails_without_writing_stdout() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let path = temp_dir.path().join("transcript.jsonl");
    write(
        &path,
        "{\"type\":\"user\",\"message\":{\"content\":\"one\"}}\n",
    )
    .expect("Failed to write transcript");

    let output = Command::new(crate::common::get_iwe_binary_path())
        .arg("internal")
        .arg("claude")
        .arg("digest")
        .arg("--path")
        .arg(&path)
        .output()
        .expect("Failed to execute iwe internal claude digest");

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn unreadable_path_fails_without_writing_stdout() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let path = temp_dir.path().join("absent.jsonl");
    let output = Command::new(crate::common::get_iwe_binary_path())
        .arg("internal")
        .arg("claude")
        .arg("digest")
        .arg("--path")
        .arg(&path)
        .arg("--max-chars")
        .arg("100")
        .output()
        .expect("Failed to execute iwe internal claude digest");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn digest_renders_every_branch() {
    let stdout = digest(
        &[
            r#"{"type":"user","message":{"content":"  one  "}}"#,
            r#"{"type":"user","isMeta":true,"message":{"content":"meta"}}"#,
            r#"{"type":"summary","summary":"one"}"#,
            "not json at all",
            "",
            r#"{"type":"user","message":{"content":[{"type":"text","text":"  two  "},{"type":"text","text":"   "}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"three"},{"type":"tool_use","name":"Runner","input":{"command":"four"}}]}}"#,
            r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"five"},{"type":"tool_result","is_error":true,"content":"six"}]}}"#,
            r#"{"type":"assistant","message":{"content":[{"type":"widget","body":"seven"}]}}"#,
            r#"{"type":"assistant","message":{"content":"not an array"}}"#,
            r#"{"type":"user","message":{"content":42}}"#,
        ],
        &["--max-chars", "1000"],
    );

    assert_eq!(
        stdout,
        indoc! {"
            11
            [user]
            one

            [user]
            two

            [assistant]
            three

            [tool: Runner] four

            [tool result] five

            [tool error] six

            [block: widget] seven
        "}
    );
}

struct HookFixture {
    root: TempDir,
}

impl HookFixture {
    fn new(policy: Option<&str>) -> Self {
        let root = TempDir::new().expect("Failed to create temp directory");
        create_dir_all(root.path().join("store/.iwe")).expect("Failed to create store");
        create_dir_all(root.path().join("transcripts")).expect("Failed to create transcripts");
        let fixture = HookFixture { root };
        if let Some(policy) = policy {
            fixture.write("MEMORY.md", policy);
        }
        fixture
    }

    fn store(&self) -> PathBuf {
        self.root.path().join("store")
    }

    fn transcripts(&self) -> PathBuf {
        self.root.path().join("transcripts")
    }

    fn write(&self, relative: &str, content: &str) {
        let path = self.store().join(relative);
        if let Some(parent) = path.parent() {
            create_dir_all(parent).expect("Failed to create parent");
        }
        write(&path, content).expect("Failed to write document");
    }

    fn transcript_with_timestamps(&self, name: &str, lines: usize) {
        let body: String = (0..lines)
            .map(|index| {
                format!(
                    "{{\"type\":\"user\",\"timestamp\":\"2026-08-19T07:{:02}:00.000Z\",\"message\":{{\"content\":\"line {}\"}}}}\n",
                    index, index
                )
            })
            .collect();
        write(self.transcripts().join(format!("{}.jsonl", name)), body)
            .expect("Failed to write transcript");
    }

    fn transcript_live(&self, name: &str, lines: usize) {
        let stamp = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let body: String = (0..lines)
            .map(|index| {
                format!(
                    "{{\"type\":\"user\",\"timestamp\":\"{}\",\"message\":{{\"content\":\"line {}\"}}}}\n",
                    stamp, index
                )
            })
            .collect();
        write(self.transcripts().join(format!("{}.jsonl", name)), body)
            .expect("Failed to write transcript");
    }

    fn append(&self, name: &str, lines: &[&str]) {
        let path = self.transcripts().join(format!("{}.jsonl", name));
        let mut existing = std::fs::read_to_string(&path).expect("Failed to read the transcript");
        for line in lines {
            existing.push_str(line);
            existing.push('\n');
        }
        write(&path, existing).expect("Failed to append to the transcript");
    }

    fn transcript(&self, name: &str, lines: usize) {
        let body: String = (0..lines)
            .map(|index| {
                format!(
                    "{{\"type\":\"user\",\"message\":{{\"content\":\"line {}\"}}}}\n",
                    index
                )
            })
            .collect();
        write(self.transcripts().join(format!("{}.jsonl", name)), body)
            .expect("Failed to write transcript");
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.store().join(relative)).expect("Failed to read document")
    }

    fn exists(&self, relative: &str) -> bool {
        self.store().join(relative).exists()
    }

    fn tree(&self) -> Vec<(String, String)> {
        let mut entries = Vec::new();
        collect_documents(&self.store(), &self.store(), &mut entries);
        entries.sort();
        entries
    }

    fn age(&self, name: &str, minutes: u64) {
        let path = self.transcripts().join(format!("{}.jsonl", name));
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(minutes * 60);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .expect("Failed to open the transcript")
            .set_modified(when)
            .expect("Failed to backdate the transcript");
    }

    fn run(&self, args: &[&str], payload: Option<&str>) -> Output {
        self.run_tz(args, payload, None)
    }

    fn run_tz(&self, args: &[&str], payload: Option<&str>, tz: Option<&str>) -> Output {
        match tz {
            Some(tz) => self.run_env(args, payload, &[("TZ", tz)]),
            None => self.run_env(args, payload, &[]),
        }
    }

    fn run_env(&self, args: &[&str], payload: Option<&str>, env: &[(&str, &str)]) -> Output {
        let mut command = Command::new(crate::common::get_iwe_binary_path());
        command
            .arg("internal")
            .arg("claude")
            .arg("hook")
            .args(args)
            .current_dir(self.store())
            .env_remove("IWE_MEMORY_TRANSCRIPTS")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("CLAUDE_PLUGIN_ROOT")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        without_knob_variables(&mut command);
        for (name, value) in env {
            command.env(name, value);
        }

        let mut child = command
            .spawn()
            .expect("Failed to spawn iwe internal claude hook");
        if let Some(payload) = payload {
            child
                .stdin
                .as_mut()
                .expect("stdin is piped")
                .write_all(payload.as_bytes())
                .expect("Failed to write payload");
        }
        drop(child.stdin.take());
        child.wait_with_output().expect("Failed to run hook")
    }

    fn session(&self, args: &[&str]) -> Output {
        self.session_env(args, &[])
    }

    fn session_env(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let transcripts = self.transcripts();
        let mut command = Command::new(crate::common::get_iwe_binary_path());
        command
            .arg("internal")
            .arg("claude")
            .arg("session")
            .args(args)
            .arg("--transcripts")
            .arg(transcripts.to_str().expect("path"))
            .current_dir(self.store())
            .env_remove("IWE_MEMORY_TRANSCRIPTS")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        without_knob_variables(&mut command);
        for (name, value) in env {
            command.env(name, value);
        }
        command
            .output()
            .expect("Failed to execute iwe internal claude session")
    }

    fn iwe(&self, args: &[&str]) -> Output {
        let mut command = Command::new(crate::common::get_iwe_binary_path());
        command
            .args(args)
            .current_dir(self.store())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = without_knob_variables(&mut command)
            .output()
            .expect("Failed to execute iwe");
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn iwe_ok(&self, args: &[&str]) -> String {
        String::from_utf8(self.iwe(args).stdout).expect("Valid UTF-8 output")
    }

    fn legacy_records(&self) -> PathBuf {
        self.store()
            .canonicalize()
            .expect("the store resolves")
            .join("sessions")
    }

    fn migrate(&self) -> String {
        self.iwe_ok(&["internal", "claude", "session", "migrate"])
    }

    fn session_ok(&self, args: &[&str]) -> String {
        self.session_ok_env(args, &[])
    }

    fn session_ok_env(&self, args: &[&str], env: &[(&str, &str)]) -> String {
        let output = self.session_env(args, env);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn session_err(&self, args: &[&str]) -> String {
        let output = self.session(args);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        String::from_utf8(output.stderr).expect("Valid UTF-8 output")
    }

    fn brief(&self) -> String {
        let mut command = Command::new(crate::common::get_iwe_binary_path());
        command
            .args(["internal", "claude", "session", "brief"])
            .current_dir(self.store())
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let output = without_knob_variables(&mut command)
            .output()
            .expect("Failed to run session brief");
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn session_start(&self, payload: Option<&str>) -> String {
        self.session_start_with(&[], payload)
    }

    fn session_start_env(&self, env: &[(&str, &str)]) -> String {
        let output = self.run_env(&["session-start"], None, env);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn session_start_with(&self, extra: &[&str], payload: Option<&str>) -> String {
        let mut args = vec!["session-start"];
        args.extend_from_slice(extra);
        let output = self.run(&args, payload);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn bind_schema(&self, name: &str, pattern: &str, schema: &str) {
        let schemas = self.store().join(".iwe/schemas");
        create_dir_all(&schemas).expect("Failed to create schemas");
        write(schemas.join(format!("{}.yaml", name)), schema).expect("Failed to write schema");

        let config = self.store().join(".iwe/config.toml");
        let existing = read_to_string(&config).unwrap_or_default();
        write(
            &config,
            format!(
                "{}\n[schemas.{}]\nmatch = \"{}\"\n",
                existing.trim_end(),
                name,
                pattern
            ),
        )
        .expect("Failed to bind schema");
    }

    fn post_tool(&self, payload: &str) -> Output {
        self.run(&["post-tool"], Some(payload))
    }

    fn post_tool_quiet(&self, payload: &str) {
        let output = self.post_tool(payload);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, Vec::<u8>::new());
    }

    fn post_tool_report(&self, payload: &str) -> String {
        let output = self.post_tool(payload);
        assert_eq!(output.status.code(), Some(0));
        let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
        let body: serde_json::Value =
            serde_json::from_str(&stdout).expect("The hook emits one JSON object");
        assert_eq!(body["hookSpecificOutput"]["hookEventName"], "PostToolUse");
        body["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .expect("additionalContext is a string")
            .to_string()
    }

    fn post_tool_notice(&self, payload: &str) -> String {
        let output = self.post_tool(payload);
        assert_eq!(output.status.code(), Some(0));
        let body: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("The hook emits one JSON object");
        body["systemMessage"]
            .as_str()
            .expect("systemMessage is a string")
            .to_string()
    }

    fn editor_write(&self, relative: &str) -> String {
        let path = self.store().join(relative);
        format!(
            r#"{{"cwd":{},"tool_name":"Write","tool_input":{{"file_path":{}}}}}"#,
            json_string(&self.store().to_string_lossy()),
            json_string(&path.to_string_lossy())
        )
    }

    fn bash_write(&self, command: &str, stdout: &str) -> String {
        format!(
            r#"{{"cwd":{},"tool_name":"Bash","tool_input":{{"command":{}}},"tool_response":{{"stdout":{}}}}}"#,
            json_string(&self.store().to_string_lossy()),
            json_string(command),
            json_string(stdout)
        )
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("Failed to encode JSON string")
}

fn collect_documents(root: &PathBuf, directory: &PathBuf, entries: &mut Vec<(String, String)>) {
    let listing = match std::fs::read_dir(directory) {
        Ok(listing) => listing,
        Err(_) => return,
    };
    for entry in listing.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_documents(root, &path, entries);
        } else if path.extension().map(|ext| ext == "md").unwrap_or(false) {
            let relative = path
                .strip_prefix(root)
                .expect("under root")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            entries.push((
                relative,
                std::fs::read_to_string(&path).expect("Failed to read"),
            ));
        }
    }
}

const MEMORY_POLICY: &str = indoc! {"
    ---
    chunk_chars: 30
    max_proposals_per_read: 2
    ---

    # Memory policy

    How this store is written.
"};

const KNOB_VARIABLES: [&str; 6] = [
    "IWE_CHUNK_CHARS",
    "IWE_MAX_PROPOSALS_PER_READ",
    "IWE_REMIND_EVERY_DAYS",
    "IWE_INJECTION_MAX_TOKENS",
    "IWE_KNOWLEDGE_FILTER",
    "IWE_RECENCY_FIELD",
];

fn without_knob_variables(command: &mut Command) -> &mut Command {
    for name in KNOB_VARIABLES {
        command.env_remove(name);
    }
    command
}

#[test]
fn hook_without_the_memory_document_stays_silent() {
    let fixture = HookFixture::new(None);
    fixture.transcript("session-one", 20);

    assert_eq!(fixture.session_start(None), "");
    assert_eq!(fixture.tree(), Vec::new());
}

#[test]
fn the_sweep_hook_is_gone_and_the_two_that_remain_are_named() {
    let output = Command::new(crate::common::get_iwe_binary_path())
        .args(["internal", "claude", "hook", "--help"])
        .output()
        .expect("Failed to run hook --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("session-start"), "{}", stdout);
    assert!(stdout.contains("post-tool"), "{}", stdout);
    assert!(!stdout.contains("\n  stop"), "{}", stdout);

    let stopped = Command::new(crate::common::get_iwe_binary_path())
        .args(["internal", "claude", "hook", "stop"])
        .output()
        .expect("Failed to run the retired hook");
    assert_eq!(stopped.status.code(), Some(2));
}

#[test]
fn session_commands_outside_a_memory_store_fail_loudly() {
    let fixture = HookFixture::new(None);
    fixture.transcript("session-one", 20);

    for args in [
        vec!["list"],
        vec!["read", "session-one"],
        vec!["complete", "session-one"],
        vec!["adopt"],
    ] {
        let output = fixture.session(&args);
        assert_eq!(output.status.code(), Some(1), "{:?}", args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("not a memory-enabled"), "{}", stderr);
        assert!(stderr.contains("/iwe:init"), "{}", stderr);
    }
}

#[test]
fn listing_without_a_transcript_directory_fails_loudly() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));

    let output = Command::new(crate::common::get_iwe_binary_path())
        .args([
            "internal",
            "claude",
            "session",
            "list",
            "--transcripts",
            "/nonexistent/transcripts/for/this/test",
        ])
        .current_dir(fixture.store())
        .output()
        .expect("Failed to run session list");

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no transcript directory"));
}

#[test]
fn hook_with_a_missing_cwd_stays_silent() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.transcript("session-one", 20);

    let payload = "{\"cwd\":\"/nonexistent/directory/for/this/test\"}";
    assert_eq!(fixture.session_start(Some(payload)), "");
    assert_eq!(fixture.tree().len(), 1);
}

#[test]
fn session_start_indexes_dated_documents_newest_first() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write(
        "beta.md",
        "---\ncreated: \"2026-08-05 11:00\"\n---\n\n# Beta Note\n\nBody two.\n",
    );

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Beta Note](beta) · created: 2026-08-05 11:00
            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_drops_the_oldest_entries_over_the_token_budget() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write(
        "beta.md",
        "---\ncreated: \"2026-08-05 11:00\"\n---\n\n# Beta Note\n\nBody two.\n",
    );
    fixture.write(
        "gamma.md",
        "---\ncreated: \"2026-08-09 12:00\"\n---\n\n# Gamma Note\n\nBody three.\n",
    );

    assert_eq!(
        fixture.session_start_env(&[("IWE_INJECTION_MAX_TOKENS", "20")]),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Gamma Note](gamma) · created: 2026-08-09 12:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_names_the_query_cookbook_when_present() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write("queries.md", "# Queries\n\nCookbook.\n");

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            The `queries` document is this store's query cookbook.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_falls_back_to_undated_documents() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write("alpha.md", "# Alpha Note\n\nBody one.\n");

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Alpha Note](alpha)

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_falls_back_on_an_unusable_token_budget() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        injection_max_tokens: \"lots\"
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_is_silent_when_only_policy_documents_exist() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write("queries.md", "# Queries\n\nCookbook.\n");

    assert_eq!(fixture.session_start(None), "");
}

#[test]
fn session_start_injects_the_policy_at_session_start_section() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        chunk_chars: 30
        ---

        # Memory policy

        ## How to write it

        Flat slugs.

        ## At session start

        Offer once per turn: \"Worth remembering: <title>\".

        Run `example one` to record a note.

        ## Curation

        Merge only.
    "}));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Offer once per turn: \"Worth remembering: <title>\".

            Run `example one` to record a note.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_lists_what_the_knowledge_filter_selects_by_the_recency_field() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        recency_field: date
        knowledge_filter:
          type: note
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.write(
        "alpha.md",
        "---\ntype: note\ndate: \"2026-08-01\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write(
        "beta.md",
        "---\ntype: note\ndate: \"2026-08-05\"\n---\n\n# Beta Note\n\nBody two.\n",
    );
    fixture.write(
        "gamma.md",
        "---\ncreated: \"2026-08-09 10:00\"\n---\n\n# Gamma Note\n\nNot a note.\n",
    );

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Beta Note](beta) · date: 2026-08-05
            - [Alpha Note](alpha) · date: 2026-08-01

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ type: note }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn hook_reads_the_store_from_the_payload_cwd() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    let elsewhere = TempDir::new().expect("Failed to create temp directory");
    let transcripts = fixture.transcripts();
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command
        .arg("internal")
        .arg("claude")
        .arg("hook")
        .arg("session-start")
        .current_dir(elsewhere.path())
        .env_remove("IWE_MEMORY_TRANSCRIPTS")
        .stdin(Stdio::null());
    let output = without_knob_variables(&mut command)
        .output()
        .expect("Failed to run hook");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");

    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command
        .arg("internal")
        .arg("claude")
        .arg("hook")
        .arg("session-start")
        .current_dir(elsewhere.path())
        .env_remove("IWE_MEMORY_TRANSCRIPTS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = without_knob_variables(&mut command)
        .spawn()
        .expect("Failed to spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(
            serde_json::json!({ "cwd": fixture.store() })
                .to_string()
                .as_bytes(),
        )
        .expect("Failed to write payload");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("Failed to run hook");
    let _ = transcripts;

    assert_eq!(
        String::from_utf8(output.stdout).expect("Valid UTF-8 output"),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn internal_is_hidden_from_help() {
    let output = Command::new(crate::common::get_iwe_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute iwe --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("Valid UTF-8 output");
    assert!(!stdout.contains("internal"));
}

fn run_enable(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command
        .arg("internal")
        .arg("claude")
        .arg("enable")
        .args(args)
        .arg(root)
        .env("IWE_MEMORY_STATE", root.join("state"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.output().expect("Failed to run enable")
}

#[test]
fn enable_switches_a_bare_directory_on() {
    let root = TempDir::new().expect("Failed to create temp directory");

    let output = run_enable(root.path(), &["--queries"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wrote the MEMORY.md policy document"),
        "{}",
        stdout
    );
    assert!(stdout.contains("wrote the queries cookbook"), "{}", stdout);
    assert!(stdout.contains("nothing was committed"), "{}", stdout);

    let policy = read_to_string(root.path().join("MEMORY.md")).expect("policy written");
    assert!(policy.starts_with("---\ncreated: \""), "{}", policy);
    assert!(policy.contains("# Memory policy"));
    assert!(root.path().join("queries.md").is_file());
    let config = read_to_string(root.path().join(".iwe/config.toml")).expect("config written");
    assert!(config.contains("date_format = \"%Y-%m-%d\""), "{}", config);
    assert!(
        config.contains("time_format = \"%Y-%m-%d %H:%M\""),
        "{}",
        config
    );

    let again = run_enable(root.path(), &[]);
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("already memory-enabled"));
}

#[test]
fn enable_typed_installs_the_ontology_and_refuses_a_clash() {
    let root = TempDir::new().expect("Failed to create temp directory");

    let output = run_enable(root.path(), &["--typed"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = read_to_string(root.path().join(".iwe/config.toml")).expect("config written");
    assert!(config.contains("[templates.learning]"), "{}", config);
    assert!(root.path().join(".iwe/schemas/gotcha.yaml").is_file());
    let policy = read_to_string(root.path().join("MEMORY.md")).expect("policy written");
    assert!(
        policy.contains("--template <decision|learning|gotcha>"),
        "{}",
        policy
    );
    assert!(
        policy.contains(
            "\nknowledge_filter:\n  type: { $in: [decision, learning, gotcha, topic] }\n---\n"
        ),
        "{}",
        policy
    );

    let clashing = TempDir::new().expect("Failed to create temp directory");
    create_dir_all(clashing.path().join(".iwe")).expect("workspace");
    write(
        clashing.path().join(".iwe/config.toml"),
        "[templates.learning]\nkey_template = \"mine/{{slug}}\"\ndocument_template = \"# {{title}}\"\n",
    )
    .expect("config");
    let refused = run_enable(clashing.path(), &["--typed"]);
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("already defines: templates.learning")
    );
    let after = read_to_string(clashing.path().join(".iwe/config.toml")).expect("read");
    assert!(
        after.contains("key_template = \"mine/{{slug}}\""),
        "{}",
        after
    );
    assert!(!after.contains("[templates.decision]"), "{}", after);
    assert!(!after.contains("[actions.daily]"), "{}", after);
    assert!(!clashing.path().join(".iwe/schemas/learning.yaml").is_file());
    assert!(!clashing.path().join("MEMORY.md").is_file());
}

#[test]
fn enable_clears_the_state_directory_the_sweep_left_behind() {
    let root = TempDir::new().expect("Failed to create temp directory");
    create_dir_all(root.path().join(".iwe/claude-sessions")).expect("stale state");

    let output = run_enable(root.path(), &[]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("removed the empty .iwe/claude-sessions"),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!root.path().join(".iwe/claude-sessions").exists());
}

#[test]
fn enable_leaves_a_state_directory_that_still_holds_something() {
    let root = TempDir::new().expect("Failed to create temp directory");
    let stale = root.path().join(".iwe/claude-sessions/abc");
    create_dir_all(&stale).expect("stale state");
    write(stale.join("000420.md"), "# a chunk\n").expect("chunk");

    let output = run_enable(root.path(), &[]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stale.join("000420.md").is_file(),
        "the user deletes it, not us"
    );
}

#[test]
fn enable_body_writes_the_policy_verbatim() {
    let root = TempDir::new().expect("Failed to create temp directory");
    let body = root.path().join("policy-body.md");
    write(&body, "# My own policy\n\nThis store's shape.\n").expect("body");

    let mut command = Command::new(crate::common::get_iwe_binary_path());
    let output = command
        .arg("internal")
        .arg("claude")
        .arg("enable")
        .arg("--body")
        .arg(&body)
        .arg(root.path())
        .stdin(Stdio::null())
        .output()
        .expect("Failed to run enable");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let policy = read_to_string(root.path().join("MEMORY.md")).expect("policy written");
    assert!(policy.contains("# My own policy"), "{}", policy);
    assert!(policy.contains("This store's shape."));
    assert!(!policy.contains("# Memory policy"));
}

#[test]
fn enable_knobs_writes_them_into_the_policy_frontmatter() {
    let root = TempDir::new().expect("Failed to create temp directory");
    let body = root.path().join("policy-body.md");
    write(&body, "# My own policy\n\nThis store's shape.\n").expect("body");
    let knobs = root.path().join("knobs.yaml");
    write(
        &knobs,
        "recency_field: date\nknowledge_filter:\n  type: note\n",
    )
    .expect("knobs");

    let output = run_enable(
        root.path(),
        &[
            "--body",
            body.to_str().expect("path"),
            "--knobs",
            knobs.to_str().expect("path"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let policy = read_to_string(root.path().join("MEMORY.md")).expect("policy written");
    let (front, rest) = policy
        .trim_start_matches("---\n")
        .split_once("---\n")
        .expect("frontmatter");
    assert!(front.starts_with("created: \""), "{}", policy);
    assert_eq!(
        front.split_once('\n').expect("created line").1,
        "recency_field: date\nknowledge_filter:\n  type: note\n"
    );
    assert_eq!(rest, "\n# My own policy\n\nThis store's shape.\n");

    let broken = TempDir::new().expect("Failed to create temp directory");
    let bad = broken.path().join("knobs.yaml");
    write(&bad, "- not\n- a mapping\n").expect("knobs");
    let output = run_enable(broken.path(), &["--knobs", bad.to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--knobs must be a YAML mapping of knobs"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!broken.path().join("MEMORY.md").is_file());
}

#[test]
fn enable_config_installs_a_composed_ontology() {
    let root = TempDir::new().expect("Failed to create temp directory");
    let parts = TempDir::new().expect("Failed to create temp directory");
    let ontology = parts.path().join("ontology.toml");
    write(
        &ontology,
        "\n[templates.note]\nkey_template = \"notes/{{slug}}\"\ndocument_template = \"# {{title}}\"\n\n[schemas.note]\nmatch = \"notes/**\"\n",
    )
    .expect("ontology");
    let schema = parts.path().join("note.yaml");
    write(
        &schema,
        "$schema: https://document-schema.org/draft/2026-06/schema\nfrontmatter:\n  type: object\n",
    )
    .expect("schema");

    let output = run_enable(
        root.path(),
        &[
            "--config",
            ontology.to_str().expect("path"),
            "--schema",
            schema.to_str().expect("path"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("appended the composed ontology"),
        "{}",
        stdout
    );
    assert!(
        stdout.contains("wrote .iwe/schemas/note.yaml"),
        "{}",
        stdout
    );
    assert!(
        stdout.contains("session records will land under .iwe/claude/sessions\n"),
        "{}",
        stdout
    );
    assert_eq!(
        read_to_string(root.path().join(".iwe/claude/.gitignore"))
            .expect("the state gitignore is written"),
        ".reminded\n"
    );

    let config = read_to_string(root.path().join(".iwe/config.toml")).expect("config written");
    assert!(config.contains("[templates.note]"), "{}", config);
    assert!(root.path().join(".iwe/schemas/note.yaml").is_file());
    assert!(root.path().join("MEMORY.md").is_file());

    let clashing = TempDir::new().expect("Failed to create temp directory");
    create_dir_all(clashing.path().join(".iwe")).expect("workspace");
    write(
        clashing.path().join(".iwe/config.toml"),
        "[templates.note]\nkey_template = \"mine/{{slug}}\"\ndocument_template = \"# {{title}}\"\n",
    )
    .expect("config");
    let refused = run_enable(
        clashing.path(),
        &["--config", ontology.to_str().expect("path")],
    );
    assert_eq!(refused.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("already defines: templates.note"));
    assert!(!clashing.path().join("MEMORY.md").is_file());
}

const OLD_ENOUGH: u64 = 90;

fn backlog_fixture() -> HookFixture {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    for (session, lines, age) in [
        ("session-one", 12, OLD_ENOUGH + 2),
        ("session-two", 8, OLD_ENOUGH + 1),
        ("session-three", 4, OLD_ENOUGH),
    ] {
        fixture.transcript_with_timestamps(session, lines);
        fixture.age(session, age);
    }
    fixture
}

fn row_of<'a>(listing: &'a str, session: &str) -> &'a str {
    listing
        .lines()
        .find(|line| line.starts_with(session))
        .unwrap_or_else(|| panic!("no row for {} in\n{}", session, listing))
}

#[test]
fn list_shows_every_pending_session_newest_first() {
    let fixture = backlog_fixture();

    let listing = fixture.session_ok(&["list"]);

    let order: Vec<&str> = listing
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|word| word.starts_with("session-"))
        .collect();
    assert_eq!(order, vec!["session-three", "session-two", "session-one"]);
    assert!(
        row_of(&listing, "session-one").contains("pending"),
        "{}",
        listing
    );
    assert!(listing.contains("3 session(s) listed"), "{}", listing);
    assert!(
        listing.contains("3 pending over 24 undistilled line(s) carrying 24 user turn(s)"),
        "{}",
        listing
    );
}

#[test]
fn list_marks_the_current_session_and_never_lets_it_be_adopted() {
    let fixture = backlog_fixture();

    let listing = fixture.session_ok_env(&["list"], &[("CLAUDE_CODE_SESSION_ID", "session-two")]);
    assert!(
        listing.contains("current session: session-two"),
        "{}",
        listing
    );
    assert!(
        row_of(&listing, "session-two").contains("current"),
        "{}",
        listing
    );

    let report = fixture.session_ok_env(
        &["adopt", "session-two"],
        &[("CLAUDE_CODE_SESSION_ID", "session-two")],
    );
    assert!(report.contains("refused session-two"), "{}", report);
    assert!(report.contains("0 session(s) adopted"), "{}", report);
    assert!(!fixture.exists(".iwe/claude/sessions/session-two.yaml"));
}

#[test]
fn list_says_so_when_the_session_id_is_not_in_the_environment() {
    let fixture = backlog_fixture();

    let listing = fixture.session_ok(&["list"]);

    assert!(listing.contains("current session: unknown"), "{}", listing);
    assert!(listing.contains("confirm with the user"), "{}", listing);
    assert!(!listing.contains(" current "), "{}", listing);
}

#[test]
fn a_transcript_touched_moments_ago_is_active_and_left_alone() {
    let fixture = backlog_fixture();
    fixture.transcript_live("session-live", 9);

    let listing = fixture.session_ok(&["list"]);
    assert!(
        row_of(&listing, "session-live").contains("active"),
        "{}",
        listing
    );

    let report = fixture.session_ok(&["adopt"]);
    assert!(!report.contains("adopted session-live"), "{}", report);
    assert!(report.contains("3 session(s) adopted"), "{}", report);
    assert!(!fixture.exists(".iwe/claude/sessions/session-live.yaml"));
}

#[test]
fn subagent_transcripts_are_never_listed() {
    let fixture = backlog_fixture();
    create_dir_all(fixture.transcripts().join("session-one/subagents"))
        .expect("Failed to create the subagent directory");
    write(
        fixture
            .transcripts()
            .join("session-one/subagents/agent-1.jsonl"),
        "{\"type\":\"user\",\"message\":{\"content\":\"a subagent turn\"}}\n",
    )
    .expect("Failed to write the subagent transcript");

    let listing = fixture.session_ok(&["list", "--all"]);

    assert!(!listing.contains("agent-1"), "{}", listing);
    assert!(!listing.contains("subagents"), "{}", listing);
    assert_eq!(
        listing
            .lines()
            .filter(|line| line.starts_with("session-"))
            .count(),
        3,
        "{}",
        listing
    );
}

#[test]
fn read_serves_one_window_of_the_undistilled_span() {
    let fixture = backlog_fixture();

    let first = fixture.session_ok(&["read", "session-one"]);
    assert!(first.contains("session: session-one"), "{}", first);
    assert!(first.contains("covers_from: 0"), "{}", first);
    assert!(first.contains("max_proposals: 2"), "{}", first);
    assert!(first.contains("transcript_lines: 12"), "{}", first);
    assert!(first.contains("occurred: "), "{}", first);
    assert!(first.contains("[user]"), "{}", first);

    let covered: usize = first
        .lines()
        .find_map(|line| line.strip_prefix("covers_lines: "))
        .expect("covers_lines")
        .parse()
        .expect("a number");
    assert!(covered > 0 && covered < 12, "{}", first);

    let next = fixture.session_ok(&["read", "session-one", "--from", &covered.to_string()]);
    assert!(
        next.contains(&format!("covers_from: {}", covered)),
        "{}",
        next
    );
}

#[test]
fn read_starts_at_the_distilled_line_and_falls_silent_at_the_end() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-one", "--lines", "now"]);

    let read = fixture.session_ok(&["read", "session-one"]);

    assert!(read.contains("covers_from: 12"), "{}", read);
    assert!(read.contains("covers_lines: 12"), "{}", read);
    assert!(read.contains("nothing left to read"), "{}", read);
}

#[test]
fn read_defaults_to_the_current_session() {
    let fixture = backlog_fixture();

    let read = fixture.session_ok_env(&["read"], &[("CLAUDE_CODE_SESSION_ID", "session-two")]);

    assert!(read.contains("session: session-two"), "{}", read);
}

#[test]
fn read_without_a_session_id_anywhere_fails_loudly() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["read"]);

    assert!(stderr.contains("no session id"), "{}", stderr);
    assert!(stderr.contains("CLAUDE_CODE_SESSION_ID"), "{}", stderr);
}

#[test]
fn complete_advances_the_distilled_line_and_links_what_was_written() {
    let fixture = backlog_fixture();
    fixture.write(
        "cache-warmup-order.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Cache warmup order\n\nWarm the index first.\n",
    );

    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "12",
        "--wrote",
        "cache-warmup-order",
        "--title",
        "Cache warmup investigation",
        "--summary",
        "Traced the warmup order.",
    ]);
    assert!(
        report.contains("distilled through line 12, 1 link(s)"),
        "{}",
        report
    );

    let record = fixture.read(".iwe/claude/sessions/session-one.yaml");
    assert!(record.contains("distilled_lines: 12\n"), "{}", record);
    assert!(record.contains("distilled_at: "), "{}", record);
    assert!(record.contains("started: "), "{}", record);
    assert!(
        record.contains("title: Cache warmup investigation\n"),
        "{}",
        record
    );
    assert!(
        record.contains("summary: Traced the warmup order.\n"),
        "{}",
        record
    );
    assert!(
        record.contains("  through: 12\n  wrote:\n  - cache-warmup-order\n"),
        "{}",
        record
    );

    let listing = fixture.session_ok(&["list"]);
    assert!(!listing.contains("session-one"), "{}", listing);
    assert!(
        row_of(&listing, "session-two").contains("pending"),
        "{}",
        listing
    );
    assert!(listing.contains("1 settled and hidden"), "{}", listing);
    assert!(
        fixture.session_ok(&["list", "--all"]).contains("done"),
        "the completed session is listed with --all"
    );
}

#[test]
fn complete_with_lines_now_stamps_the_transcript_as_it_stands() {
    let fixture = backlog_fixture();

    let report = fixture.session_ok_env(
        &["complete", "--lines", "now"],
        &[("CLAUDE_CODE_SESSION_ID", "session-two")],
    );

    assert!(report.contains("distilled through line 8"), "{}", report);
    assert!(fixture
        .read(".iwe/claude/sessions/session-two.yaml")
        .contains("distilled_lines: 8"));
}

#[test]
fn complete_keeps_the_ledger_and_accumulates_across_calls() {
    let fixture = backlog_fixture();
    fixture.write(
        "kept-fact.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Kept fact\n\nIt holds.\n",
    );

    fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "12",
        "--wrote",
        "kept-fact",
        "--offered",
        "3",
        "--rejected",
        "Phantom decision",
        "--rejected",
        "Also phantom",
    ]);
    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--offered",
        "1",
        "--rejected",
        "Third phantom",
    ]);
    assert!(
        report.contains("distilled line unchanged at 12"),
        "{}",
        report
    );

    let record = fixture.read(".iwe/claude/sessions/session-one.yaml");
    assert!(record.contains("offered: 4"), "{}", record);
    assert!(record.contains("kept: 1"), "{}", record);
    assert!(record.contains("- Phantom decision"), "{}", record);
    assert!(record.contains("- Also phantom"), "{}", record);
    assert!(record.contains("- Third phantom"), "{}", record);
    assert!(record.contains("distilled_lines: 12"), "{}", record);
}

#[test]
fn a_declined_offer_is_recorded_without_moving_anything() {
    let fixture = backlog_fixture();

    let report = fixture.session_ok_env(
        &[
            "complete",
            "session-one",
            "--offered",
            "1",
            "--rejected",
            "A recommendation nobody confirmed",
        ],
        &[("TZ", "UTC")],
    );

    assert_eq!(
        report,
        "recorded session-one: distilled line unchanged at 0, 0 link(s)\n"
    );
    assert_eq!(
        fixture.read(".iwe/claude/sessions/session-one.yaml"),
        format!(
            "session: session-one\ntranscript: {}\nstarted: 2026-08-19 07:00\n\
             ended: 2026-08-19 07:11\ndistilled_lines: 0\noffered: 1\nrejected:\n\
             - A recommendation nobody confirmed\n",
            fixture.transcripts().join("session-one.jsonl").display()
        )
    );
    assert!(row_of(&fixture.session_ok(&["list"]), "session-one").contains("pending"));
}

#[test]
fn complete_never_walks_the_distilled_line_backwards() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-one", "--lines", "12"]);

    fixture.session_ok(&["complete", "session-one", "--lines", "4"]);

    assert!(fixture
        .read(".iwe/claude/sessions/session-one.yaml")
        .contains("distilled_lines: 12"));
}

#[test]
fn complete_refuses_a_missing_key_and_the_machinery_s_own_documents() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-two", "--lines", "8"]);

    fixture.write(
        "sessions/an-ordinary-note.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# An ordinary note\n\nIt holds.\n",
    );

    let missing = fixture.session_err(&["complete", "session-one", "--wrote", "nope"]);
    assert_eq!(
        missing,
        "error: --wrote nope: no such document in this store\n"
    );

    let refused = fixture.session_err(&["complete", "session-one", "--wrote", "MEMORY"]);
    assert_eq!(
        refused,
        "error: --wrote MEMORY: the machinery's own documents are not capture output\n"
    );
    assert!(!fixture.exists(".iwe/claude/sessions/session-one.yaml"));

    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--wrote",
        "sessions/an-ordinary-note",
    ]);
    assert_eq!(
        report,
        "recorded session-one: distilled line unchanged at 0, 1 link(s)\n"
    );
}

#[test]
fn complete_records_a_capture_and_no_graph_edge() {
    let fixture = backlog_fixture();
    fixture.write(
        "kept-fact.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Kept fact\n\nIt holds.\n",
    );

    fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "now",
        "--wrote",
        "kept-fact",
    ]);

    let record = fixture.read(".iwe/claude/sessions/session-one.yaml");
    assert!(record.contains("  wrote:\n  - kept-fact\n"), "{}", record);
    assert_eq!(
        fixture.iwe_ok(&["find", "--filter", "{ $includes: kept-fact }", "-f", "keys"]),
        ""
    );
    assert_eq!(
        fixture.iwe_ok(&["find", "--filter", "{}", "-f", "keys"]),
        "MEMORY\nkept-fact\n"
    );
}

#[test]
fn rename_leaves_session_records_alone() {
    let fixture = backlog_fixture();
    fixture.write(
        "kept-fact.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Kept fact\n\nIt holds.\n",
    );
    fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "now",
        "--wrote",
        "kept-fact",
    ]);
    let before = fixture.read(".iwe/claude/sessions/session-one.yaml");

    fixture.iwe(&["rename", "kept-fact", "moved-fact"]);

    assert!(fixture.exists("moved-fact.md"));
    assert_eq!(
        fixture.read(".iwe/claude/sessions/session-one.yaml"),
        before
    );
}

#[test]
fn record_title_and_summary_are_set_once() {
    let fixture = backlog_fixture();

    fixture.session_ok(&[
        "complete",
        "session-one",
        "--title",
        "The first subject",
        "--summary",
        "The first summary.",
    ]);
    fixture.session_ok(&[
        "complete",
        "session-one",
        "--title",
        "A later subject",
        "--summary",
        "A later summary.",
    ]);

    let record = fixture.read(".iwe/claude/sessions/session-one.yaml");
    assert!(record.contains("title: The first subject\n"), "{}", record);
    assert!(
        record.contains("summary: The first summary.\n"),
        "{}",
        record
    );
    assert!(!record.contains("later"), "{}", record);
}

const LEGACY_RECORD: &str = indoc! {"
    ---
    session: \"session-one\"
    created: \"2026-08-19 08:00\"
    distilled_lines: 12
    distilled_at: \"2026-08-19 08:30\"
    transcript: /elsewhere/session-one.jsonl
    transcript_bytes: 1034
    transcript_lines: 12
    started: \"2026-08-19 07:00\"
    ended: \"2026-08-19 07:11\"
    offered: 4
    kept: 2
    rejected:
    - A recommendation nobody confirmed
    ---

    # Cache warmup investigation

    Traced the warmup order.

    2026-08-19 08:10 — captured 1 item(s) through line 6

    [Kept fact](../kept-fact)

    2026-08-19 08:30 — captured 1 item(s)

    [Another fact](../another-fact)
"};

const LEGACY_ADOPTED: &str = indoc! {"
    ---
    session: \"session-two\"
    created: \"2026-08-19 08:00\"
    distilled_lines: 8
    ---

    # Session session-two

    Agent session in this workspace.
"};

fn legacy_fixture() -> HookFixture {
    let fixture = backlog_fixture();
    fixture.write("sessions/session-one.md", LEGACY_RECORD);
    fixture.write("sessions/session-two.md", LEGACY_ADOPTED);
    fixture.write("sessions/.gitignore", ".reminded\n");
    fixture
}

#[test]
fn migrate_moves_legacy_records_out_of_the_store() {
    let fixture = legacy_fixture();

    assert_eq!(
        fixture.migrate(),
        "migrated 2 session record(s) to .iwe/claude/sessions\n"
    );

    assert_eq!(
        fixture.read(".iwe/claude/sessions/session-one.yaml"),
        indoc! {"
            session: session-one
            title: Cache warmup investigation
            summary: Traced the warmup order.
            transcript: /elsewhere/session-one.jsonl
            transcript_bytes: 1034
            transcript_lines: 12
            started: 2026-08-19 07:00
            ended: 2026-08-19 07:11
            distilled_lines: 12
            distilled_at: 2026-08-19 08:30
            offered: 4
            kept: 2
            rejected:
            - A recommendation nobody confirmed
            captures:
            - at: 2026-08-19 08:10
              through: 6
              wrote:
              - kept-fact
            - at: 2026-08-19 08:30
              wrote:
              - another-fact
        "}
    );
    assert_eq!(
        fixture.read(".iwe/claude/sessions/session-two.yaml"),
        "session: session-two\ndistilled_lines: 8\n"
    );
    assert!(!fixture.exists("sessions/session-one.md"));
    assert!(!fixture.exists("sessions/session-two.md"));
    assert!(!fixture.exists("sessions/.gitignore"));
    assert!(!fixture.exists("sessions"));

    assert_eq!(
        fixture.migrate(),
        format!(
            "no session records under {}\n",
            fixture.legacy_records().display()
        )
    );
}

#[test]
fn brief_and_list_name_legacy_records_until_they_are_migrated() {
    let fixture = legacy_fixture();
    let notice = format!(
        "2 session record(s) still under {} — run `iwe internal claude session migrate`",
        fixture.legacy_records().display()
    );

    assert!(fixture.brief().contains(&notice), "{}", fixture.brief());
    assert!(
        fixture.session_ok(&["list"]).contains(&notice),
        "{}",
        fixture.session_ok(&["list"])
    );

    fixture.migrate();

    assert!(
        !fixture.brief().contains("still under"),
        "{}",
        fixture.brief()
    );
    assert!(
        !fixture.session_ok(&["list"]).contains("still under"),
        "{}",
        fixture.session_ok(&["list"])
    );
}

#[test]
fn complete_refuses_a_line_count_that_is_neither_a_number_nor_now() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["complete", "session-one", "--lines", "soon"]);

    assert!(
        stderr.contains("expected a line count or `now`"),
        "{}",
        stderr
    );
}

#[test]
fn adopt_stamps_every_pending_session_without_reading_one() {
    let fixture = backlog_fixture();

    let report = fixture.session_ok_env(&["adopt"], &[("TZ", "UTC")]);

    assert!(report.contains("3 session(s) adopted"), "{}", report);
    for (session, lines) in [
        ("session-one", 12),
        ("session-two", 8),
        ("session-three", 4),
    ] {
        let path = fixture.transcripts().join(format!("{}.jsonl", session));
        let size = std::fs::metadata(&path).expect("metadata").len();
        assert_eq!(
            fixture.read(&format!(".iwe/claude/sessions/{}.yaml", session)),
            format!(
                "session: {}\ntranscript: {}\ntranscript_bytes: {}\ntranscript_lines: {}\n\
                 started: 2026-08-19 07:00\nended: 2026-08-19 07:{:02}\ndistilled_lines: {}\n",
                session,
                path.display(),
                size,
                lines,
                lines - 1,
                lines
            )
        );
    }

    let listing = fixture.session_ok(&["list"]);
    assert!(listing.contains("0 pending"), "{}", listing);
    assert!(
        fixture.session_ok(&["list", "--all"]).contains("adopted"),
        "an adopted session reads as adopted, not distilled"
    );
}

#[test]
fn adopt_takes_named_sessions_only_when_named() {
    let fixture = backlog_fixture();

    let report = fixture.session_ok(&["adopt", "session-three"]);

    assert!(report.contains("adopted session-three"), "{}", report);
    assert!(report.contains("1 session(s) adopted"), "{}", report);
    assert!(!fixture.exists(".iwe/claude/sessions/session-one.yaml"));
}

#[test]
fn adopt_refuses_a_session_it_has_never_heard_of() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["adopt", "session-nine"]);

    assert_eq!(
        stderr,
        format!(
            "error: no transcript for session-nine under {}\n",
            fixture.transcripts().display()
        )
    );
    assert!(!fixture.exists(".iwe/claude/sessions/session-one.yaml"));
}

#[test]
fn adopt_takes_a_settled_tail_only_when_named() {
    let fixture = tail_fixture();
    fixture.session_ok(&["complete", "session-tail", "--lines", "now"]);
    fixture.append("session-tail", &ASSISTANT_TAIL);

    assert_eq!(
        fixture.session_ok(&["adopt"]),
        "\n0 session(s) adopted without reading, 0 refused\n"
    );
    assert_eq!(
        fixture.session_ok(&["adopt", "session-tail"]),
        "adopted session-tail at line 9\n\n1 session(s) adopted without reading, 0 refused\n"
    );
    assert_eq!(
        fixture.session_ok(&["adopt", "session-tail"]),
        "nothing to adopt in session-tail: it is distilled through its end already\n\n\
         0 session(s) adopted without reading, 0 refused\n"
    );
}

#[test]
fn a_title_that_begins_with_the_word_session_is_still_shown() {
    let fixture = backlog_fixture();
    fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "12",
        "--title",
        "Session handling notes",
    ]);

    let listing = fixture.session_ok(&["list", "--all"]);

    assert_eq!(
        row_of(&listing, "session-one").rsplit("  ").next(),
        Some("Session handling notes")
    );
}

#[test]
fn adopt_refuses_an_unsafe_session_id() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["adopt", "../escape"]);

    assert!(stderr.contains("is not a session id"), "{}", stderr);
}

const ASSISTANT_TAIL: [&str; 3] = [
    "{\"type\":\"assistant\",\"timestamp\":\"2026-08-19T07:30:00.000Z\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"recorded it\"}]}}",
    "{\"type\":\"assistant\",\"timestamp\":\"2026-08-19T07:31:00.000Z\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"and reported back\"}]}}",
    "{\"type\":\"assistant\",\"timestamp\":\"2026-08-19T07:32:00.000Z\",\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"done\"}]}}",
];

const ONE_MORE_TURN: &str =
    "{\"type\":\"user\",\"timestamp\":\"2026-08-19T07:40:00.000Z\",\"message\":{\"content\":\"one more thing\"}}";

fn tail_fixture() -> HookFixture {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.transcript_with_timestamps("session-tail", 6);
    fixture.age("session-tail", OLD_ENOUGH);
    fixture
}

#[test]
fn a_tail_of_assistant_lines_alone_is_done_and_out_of_the_count() {
    let fixture = tail_fixture();
    let transcripts = fixture.transcripts();
    let transcripts = transcripts.to_str().expect("path");
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.session_ok(&["complete", "session-tail", "--lines", "now"]);
    fixture.append("session-tail", &ASSISTANT_TAIL);

    let listing = fixture.session_ok(&["list", "--all"]);
    assert!(
        row_of(&listing, "session-tail").contains("done"),
        "a span with no user turn in it is nothing to distill: {}",
        listing
    );
    assert!(listing.contains("0 pending"), "{}", listing);
    assert!(
        row_of(&listing, "session-tail").contains(" 3 "),
        "the undistilled lines are still shown: {}",
        listing
    );

    let quiet =
        fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")]);
    assert!(!quiet.contains("are not distilled"), "{}", quiet);
    assert!(!quiet.contains("At the first natural pause"), "{}", quiet);

    fixture.append("session-tail", &[ONE_MORE_TURN]);

    let listing = fixture.session_ok(&["list"]);
    assert!(
        row_of(&listing, "session-tail").contains("pending"),
        "{}",
        listing
    );
    assert!(
        listing.contains("1 pending over 4 undistilled line(s) carrying 1 user turn(s)"),
        "{}",
        listing
    );

    let index =
        fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")]);
    assert!(index.contains("1 session (1 user turn)"), "{}", index);
}

#[test]
fn a_session_read_still_serves_a_tail_no_count_calls_pending() {
    let fixture = tail_fixture();
    fixture.session_ok(&["complete", "session-tail", "--lines", "now"]);
    fixture.append("session-tail", &ASSISTANT_TAIL);

    let read = fixture.session_ok(&["read", "session-tail"]);

    assert!(read.contains("covers_from: 6"), "{}", read);
    assert!(read.contains("transcript_lines: 9"), "{}", read);
}

#[test]
fn live_is_the_last_message_not_the_file_stamp() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.transcript_with_timestamps("session-touched", 6);
    fixture.age("session-touched", 0);
    fixture.transcript_live("session-quiet", 6);
    fixture.age("session-quiet", 4 * 24 * 60);

    let listing = fixture.session_ok(&["list"]);

    assert!(
        row_of(&listing, "session-touched").contains("pending"),
        "an old conversation whose file was touched is not live: {}",
        listing
    );
    assert!(
        row_of(&listing, "session-quiet").contains("active"),
        "a fresh message in an old file is live: {}",
        listing
    );
}

#[test]
fn a_transcript_without_a_single_stamp_falls_back_to_the_file_time() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.transcript("session-fresh", 6);
    fixture.transcript("session-stale", 6);
    fixture.age("session-stale", OLD_ENOUGH);

    let listing = fixture.session_ok(&["list"]);

    assert!(
        row_of(&listing, "session-fresh").contains("active"),
        "{}",
        listing
    );
    assert!(
        row_of(&listing, "session-stale").contains("pending"),
        "{}",
        listing
    );
}

#[test]
fn the_listing_columns_read_as_written() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-one", "--lines", "4"]);

    let listing = fixture.session_ok_env(
        &["list", "--all"],
        &[("CLAUDE_CODE_SESSION_ID", "session-three"), ("TZ", "UTC")],
    );

    let columns: Vec<&str> = row_of(&listing, "session-one").split_whitespace().collect();
    assert_eq!(
        columns,
        vec![
            "session-one",
            "2026-08-19",
            "07:00",
            "2026-08-19",
            "07:11",
            "12",
            "8",
            "4",
            "8",
            "pending",
        ],
        "{}",
        listing
    );
    let current: Vec<&str> = row_of(&listing, "session-three")
        .split_whitespace()
        .collect();
    assert_eq!(current.last(), Some(&"current"), "{}", listing);
    let header: Vec<&str> = listing
        .lines()
        .find(|line| line.starts_with("session "))
        .expect("the header")
        .split_whitespace()
        .collect();
    assert_eq!(
        header,
        vec![
            "session",
            "started",
            "last",
            "lines",
            "turns",
            "distilled",
            "pending",
            "state",
            "subject",
        ],
        "the column is the last activity, the state is active: {}",
        listing
    );
}

#[test]
fn a_ledger_only_completion_leaves_the_reminder_due() {
    let fixture = backlog_fixture();
    let transcripts = fixture.transcripts();
    let transcripts = transcripts.to_str().expect("path");
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write(".iwe/claude/.reminded", "2026-08-01 09:00\n");

    fixture.session_ok(&[
        "complete",
        "session-one",
        "--offered",
        "1",
        "--rejected",
        "A recommendation nobody confirmed",
    ]);

    assert_eq!(
        fixture.read(".iwe/claude/.reminded").trim(),
        "2026-08-01 09:00",
        "recording a no is not a distill run"
    );
    let index =
        fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")]);
    assert!(
        index.contains("At the first natural pause"),
        "an expired stamp is still due: {}",
        index
    );

    fixture.session_ok(&["complete", "session-one", "--lines", "now"]);

    assert_ne!(
        fixture.read(".iwe/claude/.reminded").trim(),
        "2026-08-01 09:00"
    );
    let quiet =
        fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")]);
    assert!(!quiet.contains("At the first natural pause"), "{}", quiet);
}

#[test]
fn complete_refuses_a_session_it_has_never_heard_of() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["complete", "oracle", "--offered", "1"]);

    assert!(
        stderr.contains("oracle: no transcript and no session record"),
        "{}",
        stderr
    );
    assert!(!fixture.exists(".iwe/claude/sessions/oracle.yaml"));
}

#[test]
fn complete_records_a_session_whose_transcript_is_gone() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-one", "--lines", "12"]);
    std::fs::remove_file(fixture.transcripts().join("session-one.jsonl")).expect("removed");

    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--offered",
        "1",
        "--rejected",
        "Late",
    ]);

    assert!(report.contains("recorded session-one"), "{}", report);
    assert!(fixture
        .read(".iwe/claude/sessions/session-one.yaml")
        .contains("- Late"));
}

#[test]
fn complete_refuses_a_line_count_past_the_end_of_the_transcript() {
    let fixture = backlog_fixture();

    let stderr = fixture.session_err(&["complete", "session-one", "--lines", "400"]);

    assert!(
        stderr.contains("--lines 400: session-one is 12 line(s) long"),
        "{}",
        stderr
    );
    assert!(!fixture.exists(".iwe/claude/sessions/session-one.yaml"));
}

#[test]
fn complete_refuses_lines_now_on_another_live_conversation() {
    let fixture = backlog_fixture();
    fixture.transcript_live("session-live", 9);

    let stderr = fixture.session_err(&["complete", "session-live", "--lines", "now"]);
    assert!(stderr.contains("another live conversation"), "{}", stderr);
    assert!(!fixture.exists(".iwe/claude/sessions/session-live.yaml"));

    let report = fixture.session_ok(&["complete", "session-live", "--lines", "9"]);
    assert!(report.contains("distilled through line 9"), "{}", report);

    let own = fixture.session_ok_env(
        &["complete", "session-live", "--lines", "now"],
        &[("CLAUDE_CODE_SESSION_ID", "session-live")],
    );
    assert!(own.contains("session-live"), "{}", own);
}

#[test]
fn adopt_refuses_a_named_live_session() {
    let fixture = backlog_fixture();
    fixture.transcript_live("session-live", 9);

    let report = fixture.session_ok(&["adopt", "session-live"]);

    assert!(report.contains("refused session-live"), "{}", report);
    assert!(
        report.contains("it is the active conversation"),
        "{}",
        report
    );
    assert!(report.contains("0 session(s) adopted"), "{}", report);
    assert!(!fixture.exists(".iwe/claude/sessions/session-live.yaml"));
}

#[test]
fn an_accepted_offer_alone_links_the_document_and_moves_nothing() {
    let fixture = backlog_fixture();
    fixture.write(
        "kept-fact.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Kept fact\n\nIt holds.\n",
    );

    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--offered",
        "1",
        "--wrote",
        "kept-fact",
    ]);

    assert!(
        report.contains("distilled line unchanged at 0, 1 link(s)"),
        "{}",
        report
    );
    let record = fixture.read(".iwe/claude/sessions/session-one.yaml");
    assert!(record.contains("distilled_lines: 0\n"), "{}", record);
    assert!(!record.contains("distilled_at:"), "{}", record);
    assert!(record.contains("kept: 1\n"), "{}", record);
    assert!(
        !record.contains("through:"),
        "an item established live covers no span: {}",
        record
    );
    assert!(record.contains("  wrote:\n  - kept-fact\n"), "{}", record);
}

#[test]
fn the_knobs_read_the_store_then_the_environment_then_the_default() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        chunk_chars: 30
        max_proposals_per_read: nine
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.transcript_with_timestamps("session-one", 12);
    fixture.age("session-one", OLD_ENOUGH);
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    let store_wins = fixture.session_ok_env(
        &["read", "session-one"],
        &[
            ("IWE_CHUNK_CHARS", "9000"),
            ("IWE_MAX_PROPOSALS_PER_READ", "4"),
        ],
    );
    assert!(
        store_wins.contains("max_proposals: 4"),
        "an unusable knob falls through: {}",
        store_wins
    );
    let covered: usize = store_wins
        .lines()
        .find_map(|line| line.strip_prefix("covers_lines: "))
        .expect("covers_lines")
        .parse()
        .expect("a number");
    assert!(
        covered < 12,
        "the store's chunk_chars wins over the variable: {}",
        store_wins
    );

    let default = fixture.session_ok(&["read", "session-one"]);
    assert!(default.contains("max_proposals: 5"), "{}", default);

    let transcripts = fixture.transcripts();
    let off = fixture.session_start_env(&[
        (
            "IWE_MEMORY_TRANSCRIPTS",
            transcripts.to_str().expect("path"),
        ),
        ("IWE_REMIND_EVERY_DAYS", "0"),
        ("TZ", "UTC"),
    ]);
    assert!(off.contains("are not distilled"), "{}", off);
    assert!(!off.contains("At the first natural pause"), "{}", off);
}

#[test]
fn brief_serves_the_policy_check_filter_schema_recent_documents_and_rejections() {
    let fixture = backlog_fixture();
    fixture.write(
        "kept-fact.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Kept fact\n\nIt holds.\n",
    );

    assert_eq!(
        fixture.brief(),
        indoc! {"
            === policy: MEMORY ===
            # Memory policy

            How this store is written.

            === policy check ===
            missing section: ## What to capture
            missing section: ## How to write it
            missing section: ## Dedup and updates

            === knowledge filter ===
            { $key: { $nin: [MEMORY, queries] } }

            === schema: 1 document(s) the filter selects ===
            | Field   | Types         | Coverage | Distinct | Values |
            | ------- | ------------- | -------- | -------- | --- |
            | created | string (100%) | 1 (100%) | 0        | --- |

            === schemas: what `--strict` enforces on those documents ===
            no schema binds any of the 1 document(s) the filter selects — nothing enforces \"how to write it\" (`iwe docs schema`; a reflect session writes one)

            === hubs: area documents and what they include ===
            no area hubs: no top-level document shares its key with a directory of these documents (a reflect session groups them when the policy allows)

            === recent: 1 of 1 document(s), newest by created ===
            kept-fact — Kept fact

            === rejected: 0 recent proposal(s) the user turned down ===
            nothing rejected yet
        "}
    );

    fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "12",
        "--offered",
        "2",
        "--rejected",
        "Sweep threshold tuned to 400",
    ]);

    let brief = fixture.brief();
    assert!(brief.contains("Sweep threshold tuned to 400"), "{}", brief);
    assert!(brief.contains("1 recent proposal(s)"), "{}", brief);
}

#[test]
fn brief_checks_the_policy_sections_the_filter_and_the_invocations() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        knowledge_filter:
          type: note
        ---

        # Memory policy

        ## What to capture

        Traps.

        ## How to write it

        `iwe create <slug> --strict --content -` with `type: note`.

        ## Dedup and updates

        `iwe find --lexcal \"<terms>\" --limit 5`
    "}));
    fixture.write(
        "note-one.md",
        "---\ntype: note\ncreated: \"2026-08-01 10:00\"\n---\n\n# Note One\n\nBody one.\n",
    );
    fixture.write(
        "gamma.md",
        "---\ncreated: \"2026-08-09 10:00\"\n---\n\n# Gamma Note\n\nNot a note.\n",
    );

    assert_eq!(
        fixture.brief(),
        indoc! {"
            === policy: MEMORY ===
            # Memory policy

            ## What to capture

            Traps.

            ## How to write it

            `iwe create <slug> --strict --content -` with `type: note`.

            ## Dedup and updates

            `iwe find --lexcal \"<terms>\" --limit 5`

            === policy check ===
            `iwe find --lexcal \"<terms>\" --limit 5`: unknown flag --lexcal

            === knowledge filter ===
            { type: note }

            === schema: 1 document(s) the filter selects ===
            | Field   | Types         | Coverage | Distinct | Values |
            | ------- | ------------- | -------- | -------- | --- |
            | created | string (100%) | 1 (100%) | 0        | --- |
            | type    | string (100%) | 1 (100%) | 1        | note (1) |

            === schemas: what `--strict` enforces on those documents ===
            no schema binds any of the 1 document(s) the filter selects — nothing enforces \"how to write it\" (`iwe docs schema`; a reflect session writes one)

            === hubs: area documents and what they include ===
            no area hubs: no top-level document shares its key with a directory of these documents (a reflect session groups them when the policy allows)

            === recent: 1 of 1 document(s), newest by created ===
            note-one — Note One

            === rejected: 0 recent proposal(s) the user turned down ===
            nothing rejected yet
        "}
    );

    let broken = HookFixture::new(Some(indoc! {"
        ---
        knowledge_filter:
          $bogus: 1
        ---

        # Memory policy

        ## What to capture

        ## How to write it

        ## Dedup and updates

        Search first.
    "}));
    let brief = broken.brief();
    let check = brief
        .split("=== policy check ===\n")
        .nth(1)
        .and_then(|rest| rest.split("\n=== knowledge filter ===\n").next())
        .expect("policy check");
    assert_eq!(
        check,
        "missing section: ## What to capture\nmissing section: ## How to write it\nknowledge_filter: unknown operator '$bogus'\n"
    );
    assert!(
        brief.contains("=== knowledge filter ===\n{ $key: { $nin: [MEMORY, queries] } }\n"),
        "{}",
        brief
    );
}

#[test]
fn session_start_takes_the_current_session_from_the_payload() {
    let fixture = backlog_fixture();
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    let transcripts = fixture.transcripts();
    let transcripts = transcripts.to_str().expect("path");
    let payload = format!(
        r#"{{"cwd":{},"session_id":"session-one"}}"#,
        json_string(&fixture.store().to_string_lossy())
    );

    let output = fixture.run_env(
        &["session-start"],
        Some(&payload),
        &[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")],
    );
    assert_eq!(output.status.code(), Some(0));
    let index = String::from_utf8(output.stdout).expect("Valid UTF-8 output");

    assert_eq!(
        index.lines().find(|line| line.ends_with("reads them with you.")),
        Some(
            "2 sessions (12 user turns) since 2026-08-19 are not distilled — `/iwe:distill` reads them with you."
        ),
        "{}",
        index
    );
}

#[test]
fn a_completion_remembers_the_transcript_size_and_length() {
    let fixture = backlog_fixture();
    let path = fixture.transcripts().join("session-three.jsonl");
    let size = std::fs::metadata(&path).expect("metadata").len();

    fixture.session_ok(&["complete", "session-three", "--lines", "now"]);
    let record = fixture.read(".iwe/claude/sessions/session-three.yaml");
    assert!(
        record.contains(&format!("transcript_bytes: {}\n", size)),
        "{}",
        record
    );
    assert!(record.contains("transcript_lines: 4\n"), "{}", record);

    fixture.session_ok(&["adopt", "session-two"]);
    let record = fixture.read(".iwe/claude/sessions/session-two.yaml");
    let size = std::fs::metadata(fixture.transcripts().join("session-two.jsonl"))
        .expect("metadata")
        .len();
    assert!(
        record.contains(&format!("transcript_bytes: {}\n", size)),
        "{}",
        record
    );
    assert!(record.contains("transcript_lines: 8\n"), "{}", record);
}

#[test]
fn adopt_records_the_size_of_a_session_settled_before_it_was_measured() {
    let fixture = backlog_fixture();
    fixture.write(
        ".iwe/claude/sessions/session-one.yaml",
        "session: session-one\ndistilled_lines: 12\ndistilled_at: 2026-08-19 08:00\n",
    );
    let size = std::fs::metadata(fixture.transcripts().join("session-one.jsonl"))
        .expect("metadata")
        .len();

    let report = fixture.session_ok(&["adopt", "session-one"]);

    assert_eq!(
        report,
        "nothing to adopt in session-one: it is distilled through its end already\n\n\
         0 session(s) adopted without reading, 0 refused\n"
    );
    assert_eq!(
        fixture.read(".iwe/claude/sessions/session-one.yaml"),
        format!(
            "session: session-one\ntranscript_bytes: {}\ntranscript_lines: 12\n\
             distilled_lines: 12\ndistilled_at: 2026-08-19 08:00\n",
            size
        )
    );
}

#[test]
fn an_unchanged_transcript_is_not_reopened() {
    let fixture = backlog_fixture();
    let path = fixture.transcripts().join("session-three.jsonl");
    fixture.session_ok(&["complete", "session-three", "--lines", "now"]);

    let glued = read_to_string(&path)
        .expect("transcript")
        .trim_end_matches('\n')
        .replace('\n', " ")
        + "\n";
    write(&path, glued).expect("rewrite the transcript at the same size");
    fixture.age("session-three", OLD_ENOUGH);

    let listing = fixture.session_ok(&["list", "--all"]);
    let columns: Vec<&str> = row_of(&listing, "session-three")
        .split_whitespace()
        .collect();
    assert_eq!(
        columns,
        vec!["session-three", "-", "-", "4", "0", "4", "0", "done"],
        "the remembered length stands in for a read: {}",
        listing
    );

    fixture.append("session-three", &[ONE_MORE_TURN]);
    fixture.age("session-three", OLD_ENOUGH);

    let listing = fixture.session_ok_env(&["list", "--all"], &[("TZ", "UTC")]);
    let columns: Vec<&str> = row_of(&listing, "session-three")
        .split_whitespace()
        .collect();
    assert_eq!(
        columns,
        vec![
            "session-three",
            "2026-08-19",
            "07:40",
            "2026-08-19",
            "07:40",
            "2",
            "0",
            "4",
            "0",
            "done",
        ],
        "a changed size forces a recount: {}",
        listing
    );
}

#[test]
fn session_start_counts_the_backlog_and_reminds_once_a_window() {
    let fixture = backlog_fixture();
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    let transcripts = fixture.transcripts();
    let transcripts = transcripts.to_str().expect("path");

    let first =
        fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts), ("TZ", "UTC")]);
    assert!(
        first.contains("3 sessions (24 user turns) since 2026-08-19 are not distilled"),
        "{}",
        first
    );
    assert!(
        first.contains("At the first natural pause in this session"),
        "{}",
        first
    );

    let second = fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts)]);
    assert!(second.contains("are not distilled"), "{}", second);
    assert!(
        !second.contains("At the first natural pause"),
        "the reminder is paced, not repeated: {}",
        second
    );

    fixture.session_ok(&["adopt"]);
    let quiet = fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts)]);
    assert!(!quiet.contains("are not distilled"), "{}", quiet);
}

#[test]
fn the_reminder_knob_can_switch_the_nudge_off() {
    let fixture = backlog_fixture();
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    let transcripts = fixture.transcripts();

    let index = fixture.session_start_env(&[
        (
            "IWE_MEMORY_TRANSCRIPTS",
            transcripts.to_str().expect("path"),
        ),
        ("IWE_REMIND_EVERY_DAYS", "0"),
    ]);

    assert!(index.contains("are not distilled"), "{}", index);
    assert!(!index.contains("At the first natural pause"), "{}", index);
    assert!(
        !fixture.exists(".iwe/claude/.reminded"),
        "nothing was stamped"
    );
}

#[test]
fn completing_a_distill_run_resets_the_reminder_window() {
    let fixture = backlog_fixture();
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    let transcripts = fixture.transcripts();
    let transcripts = transcripts.to_str().expect("path");

    fixture.session_ok(&["complete", "session-three", "--lines", "4"]);
    assert!(fixture.exists(".iwe/claude/.reminded"));

    let index = fixture.session_start_env(&[("IWE_MEMORY_TRANSCRIPTS", transcripts)]);
    assert!(index.contains("2 sessions"), "{}", index);
    assert!(!index.contains("At the first natural pause"), "{}", index);
}

#[test]
fn the_reminder_stamp_is_kept_out_of_git() {
    let fixture = backlog_fixture();
    fixture.session_ok(&["complete", "session-three", "--lines", "4"]);

    assert_eq!(fixture.read(".iwe/claude/.gitignore"), ".reminded\n");
}

const NOTE_SCHEMA: &str = indoc! {"
    $schema: https://document-schema.org/draft/2026-06/schema
    frontmatter:
      type: object
      required: [type]
      properties:
        type: { const: note }
      additionalProperties: true
"};

#[test]
fn a_document_written_outside_the_cli_is_normalized_in_place() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("notes/loose.md", "#  Loose   Title\n\n*  one\n*  two\n");

    let report = fixture.post_tool_report(&fixture.editor_write("notes/loose.md"));

    assert!(report.contains("written outside the CLI"));
    assert!(report.contains("`notes/loose`"));
    assert!(report.contains("iwe update -k notes/loose --strict --content -"));
    assert_eq!(
        fixture.read("notes/loose.md"),
        indoc! {"
            # Loose Title

            - one
            - two
        "}
    );
}

#[test]
fn a_document_already_in_canonical_form_is_left_alone() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("notes/clean.md", "# Clean\n\n- one\n");

    fixture.post_tool_quiet(&fixture.editor_write("notes/clean.md"));
    assert_eq!(fixture.read("notes/clean.md"), "# Clean\n\n- one\n");
}

#[test]
fn the_policy_document_is_never_rewritten_by_the_hook() {
    let fixture = HookFixture::new(Some("#  Memory   policy\n\n*  keep decisions\n"));

    fixture.post_tool_quiet(&fixture.editor_write("MEMORY.md"));
    assert_eq!(
        fixture.read("MEMORY.md"),
        "#  Memory   policy\n\n*  keep decisions\n"
    );
}

#[test]
fn the_machinery_s_own_document_is_never_rewritten_by_the_hook() {
    let fixture = HookFixture::new(Some("#  Memory\n\n*  keep decisions\n"));
    fixture.write("sessions/abc.md", "#  An   ordinary   note\n\n*  note\n");

    fixture.post_tool_quiet(&fixture.editor_write("MEMORY.md"));
    assert_eq!(
        fixture.read("MEMORY.md"),
        "#  Memory\n\n*  keep decisions\n"
    );

    let report = fixture.post_tool_report(&fixture.editor_write("sessions/abc.md"));
    assert!(report.contains("written outside the CLI"), "{}", report);
    assert_eq!(
        fixture.read("sessions/abc.md"),
        "# An ordinary note\n\n- note\n"
    );
}

#[test]
fn the_hook_is_inert_without_a_policy() {
    let fixture = HookFixture::new(None);
    fixture.write("notes/loose.md", "#  Loose\n\n*  one\n");

    fixture.post_tool_quiet(&fixture.editor_write("notes/loose.md"));
    assert_eq!(fixture.read("notes/loose.md"), "#  Loose\n\n*  one\n");
}

#[test]
fn a_file_of_another_type_is_not_touched() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("script.sh", "echo   hello\n");

    fixture.post_tool_quiet(&fixture.editor_write("script.sh"));
    assert_eq!(fixture.read("script.sh"), "echo   hello\n");
}

#[test]
fn a_document_outside_the_library_is_not_touched() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    let outside = fixture.root.path().join("elsewhere.md");
    write(&outside, "#  Outside\n\n*  one\n").expect("Failed to write");

    let payload = format!(
        r#"{{"cwd":{},"tool_name":"Write","tool_input":{{"file_path":{}}}}}"#,
        json_string(&fixture.store().to_string_lossy()),
        json_string(&outside.to_string_lossy())
    );
    fixture.post_tool_quiet(&payload);

    assert_eq!(
        read_to_string(&outside).expect("Failed to read"),
        "#  Outside\n\n*  one\n"
    );
}

#[test]
fn a_tool_that_does_not_write_is_ignored() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("notes/loose.md", "#  Loose\n\n*  one\n");

    let payload = format!(
        r#"{{"cwd":{},"tool_name":"Read","tool_input":{{"file_path":{}}}}}"#,
        json_string(&fixture.store().to_string_lossy()),
        json_string(&fixture.store().join("notes/loose.md").to_string_lossy())
    );
    fixture.post_tool_quiet(&payload);

    assert_eq!(fixture.read("notes/loose.md"), "#  Loose\n\n*  one\n");
}

#[test]
fn an_editor_write_that_breaks_the_schema_is_reported() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    let report = fixture.post_tool_report(&fixture.editor_write("notes/bare.md"));
    assert!(report.contains("does not match the schema"));
    assert!(report.contains(r#""type" is a required property"#));
}

#[test]
fn an_iwe_write_without_strict_reports_the_violation() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    let path = fixture.store().join("notes/bare.md");
    let report = fixture.post_tool_report(&fixture.bash_write(
        "iwe create notes/bare --content -",
        &format!("{}\n", path.display()),
    ));

    assert!(report.contains("does not match the schema"));
    assert!(report.contains("ran without `--strict`"));
}

#[test]
fn an_iwe_write_reports_the_key_the_updated_line_names() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    let report = fixture.post_tool_report(&fixture.bash_write(
        "iwe update -k notes/bare --content -",
        "Updated 'notes/bare'\n",
    ));
    assert!(report.contains("`notes/bare` does not match"));
}

#[test]
fn a_strict_write_is_not_checked_a_second_time() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    let path = fixture.store().join("notes/bare.md");
    fixture.post_tool_quiet(&fixture.bash_write(
        "iwe create notes/bare --strict --content -",
        &format!("{}\n", path.display()),
    ));
}

#[test]
fn a_command_that_does_not_write_is_ignored() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    let path = format!("{}\n", fixture.store().join("notes/bare.md").display());
    fixture.post_tool_quiet(&fixture.bash_write("iwe find --lexical bare", &path));
    fixture.post_tool_quiet(&fixture.bash_write("ls -la", &path));
    fixture.post_tool_quiet(&fixture.bash_write("iwe retrieve -k notes/bare", &path));
}

#[test]
fn a_write_whose_output_names_no_document_is_not_guessed_at() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");

    fixture.post_tool_quiet(&fixture.bash_write("iwe create notes/bare --content -", ""));
}

#[test]
fn an_iwe_write_is_never_normalized_by_the_hook() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("notes/loose.md", "#  Loose\n\n*  one\n");

    let path = fixture.store().join("notes/loose.md");
    fixture.post_tool_quiet(&fixture.bash_write(
        "iwe create notes/loose --content -",
        &format!("{}\n", path.display()),
    ));
    assert_eq!(fixture.read("notes/loose.md"), "#  Loose\n\n*  one\n");
}

#[test]
fn the_user_is_told_in_one_line_what_the_hook_did() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("notes/loose.md", "#  Loose\n\n*  one\n");

    let notice = fixture.post_tool_notice(&fixture.editor_write("notes/loose.md"));
    assert!(notice.starts_with("iwe normalized "));
    assert!(notice.ends_with("notes/loose.md"));
    assert_eq!(notice.lines().count(), 1);
}

#[test]
fn only_the_failing_document_is_named_in_the_notice() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");
    fixture.write("docs/fine.md", "# Fine\n\nUnbound.\n");

    let stdout = format!(
        "{}\n{}\n",
        fixture.store().join("docs/fine.md").display(),
        fixture.store().join("notes/bare.md").display()
    );
    let notice =
        fixture.post_tool_notice(&fixture.bash_write("iwe create notes/bare --content -", &stdout));

    assert_eq!(notice, "iwe: notes/bare does not match its schema");
}

#[test]
fn strict_is_judged_per_invocation_in_a_chained_command() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");
    let stdout = format!("{}\n", fixture.store().join("notes/bare.md").display());

    let notice = fixture.post_tool_notice(&fixture.bash_write(
        "iwe create docs/fine --strict --content - && iwe create notes/bare --content -",
        &stdout,
    ));
    assert_eq!(notice, "iwe: notes/bare does not match its schema");

    fixture.post_tool_quiet(&fixture.bash_write(
        "iwe create notes/bare --strict --content - && iwe find --lexical bare",
        &stdout,
    ));
}

#[test]
fn a_global_flag_with_a_value_does_not_hide_the_verb() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.bind_schema("note", "notes/**", NOTE_SCHEMA);
    fixture.write("notes/bare.md", "# Bare\n\nNo type field.\n");
    let stdout = format!("{}\n", fixture.store().join("notes/bare.md").display());

    let notice = fixture
        .post_tool_notice(&fixture.bash_write("iwe -v 1 create notes/bare --content -", &stdout));
    assert_eq!(notice, "iwe: notes/bare does not match its schema");
}

#[test]
fn the_net_covers_every_document_in_the_library() {
    let fixture = HookFixture::new(Some("# Memory\n\nKeep decisions.\n"));
    fixture.write("docs/guide.md", "#  Guide\n\n*  one\n");

    let report = fixture.post_tool_report(&fixture.editor_write("docs/guide.md"));
    assert!(report.contains("written outside the CLI"));
    assert_eq!(fixture.read("docs/guide.md"), "# Guide\n\n- one\n");
}

fn run_iwe(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(crate::common::get_iwe_binary_path());
    command
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.output().expect("Failed to run iwe")
}

const POOLING_BODY: &str = "Postgres connection pooling with pgbouncer sits in front of the primary database. Every worker opens one pooled connection at startup and keeps it for the life of the process, so the pool size equals the worker count plus a margin of four for migrations and ad hoc queries. Transaction pooling mode breaks prepared statements, so the application disables them in the driver configuration before it connects.";

#[test]
fn session_start_renders_the_injection_slices_in_order_without_repeats() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        injection:
          - { heading: \"Rules this store keeps:\", filter: { kind: rule }, limit: 5 }
          - { heading: \"Most recently recorded:\", recent: true }
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.write(
        "rule-one.md",
        "---\ncreated: \"2026-08-05 10:00\"\nkind: rule\n---\n\n# Rule One\n\nNever do X.\n",
    );
    fixture.write(
        "trap-one.md",
        "---\ncreated: \"2026-08-09 10:00\"\nkind: trap\n---\n\n# Trap One\n\nIt failed.\n",
    );

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory: markdown documents captured from past sessions, reviewed as ordinary diffs.
            Rules this store keeps:

            - [Rule One](rule-one) · created: 2026-08-05 10:00

            Most recently recorded:

            - [Trap One](trap-one) · created: 2026-08-09 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5 --filter '{ $key: { $nin: [MEMORY, queries] } }'`.
            `MEMORY.md` says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_falls_back_to_the_default_listing_when_the_injection_knob_is_broken() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        injection:
          - { limit: 3 }
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    let index = fixture.session_start(None);
    assert!(
        index.contains(
            "Most recently recorded, newest first — titles and keys only:\n\n- [Alpha Note](alpha)"
        ),
        "{}",
        index
    );
    let brief = fixture.brief();
    assert!(
        brief.contains("injection[1]: needs one of `filter`, `recent: true` or `changed: true`"),
        "{}",
        brief
    );
}

#[test]
fn session_start_changed_slice_lists_documents_naming_working_tree_files() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        injection:
          - { heading: \"Touching the working tree:\", changed: true }
        ---

        # Memory policy

        How this store is written.
    "}));
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(fixture.store())
        .output();
    if !git.is_ok_and(|output| output.status.success()) {
        return;
    }
    fixture.write("hooks.json", "{}\n");
    fixture.write(
        "matcher-trap.md",
        "---\ncreated: \"2026-08-05 10:00\"\n---\n\n# Matcher trap\n\nThe matcher in `hooks.json` never fires for forks.\n",
    );
    fixture.write(
        "unrelated.md",
        "---\ncreated: \"2026-08-09 10:00\"\n---\n\n# Unrelated\n\nNothing about that file.\n",
    );

    let index = fixture.session_start(None);
    assert!(
        index.contains("Touching the working tree:\n\n- [Matcher trap](matcher-trap) · created: 2026-08-05 10:00\n\nRead one"),
        "{}",
        index
    );
    assert!(!index.contains("Unrelated"), "{}", index);
}

#[test]
fn complete_links_a_written_document_into_its_area_hub_once() {
    let fixture = backlog_fixture();
    fixture.write(
        "postgres.md",
        "# Postgres\n\n[Existing](postgres/existing)\n",
    );
    fixture.write("postgres/existing.md", "# Existing\n\nAlready a member.\n");
    fixture.write(
        "postgres/pooling.md",
        "---\ncreated: \"2026-08-19 07:00\"\n---\n\n# Pooling\n\nUse pgbouncer.\n",
    );

    let report = fixture.session_ok(&[
        "complete",
        "session-one",
        "--lines",
        "12",
        "--wrote",
        "postgres/pooling",
    ]);
    assert!(
        report.contains("linked postgres/pooling into its area hub postgres"),
        "{}",
        report
    );
    let hub = fixture.read("postgres.md");
    assert_eq!(hub.matches("](postgres/pooling").count(), 1, "{}", hub);
    assert!(hub.contains("[Existing](postgres/existing"), "{}", hub);

    let again = fixture.session_ok(&[
        "complete",
        "session-two",
        "--lines",
        "8",
        "--wrote",
        "postgres/pooling",
    ]);
    assert!(!again.contains("linked"), "{}", again);
    assert_eq!(
        fixture
            .read("postgres.md")
            .matches("](postgres/pooling")
            .count(),
        1
    );

    let brief = fixture.brief();
    assert!(
        brief.contains("postgres — includes all 2 document(s) under postgres/"),
        "{}",
        brief
    );
}

#[test]
fn brief_reports_schema_coverage_and_the_hub_census() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.bind_schema(
        "note",
        "notes/**",
        "frontmatter:\n  type: object\n  required: [created]\n",
    );
    fixture.write("notes.md", "# Notes\n\n[Alpha](notes/alpha)\n");
    fixture.write(
        "notes/alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha\n\nBody.\n",
    );
    fixture.write(
        "notes/beta.md",
        "---\ncreated: \"2026-08-02 10:00\"\n---\n\n# Beta\n\nBody.\n",
    );
    fixture.write(
        "stray.md",
        "---\ncreated: \"2026-08-03 10:00\"\n---\n\n# Stray\n\nBody.\n",
    );

    let brief = fixture.brief();
    assert!(
        brief.contains("=== schemas: what `--strict` enforces on those documents ===\nnote — .iwe/schemas/note.yaml binds 2 document(s)\n2 document(s) bind no schema\n"),
        "{}",
        brief
    );
    assert!(
        brief.contains("=== hubs: area documents and what they include ===\nnotes — includes 1 of 2 document(s) under notes/; not included: notes/beta\n1 document(s) at the top level, outside every area: stray\n"),
        "{}",
        brief
    );
}

#[test]
fn post_tool_names_a_near_duplicate_after_a_create() {
    let fixture = HookFixture::new(Some(MEMORY_POLICY));
    fixture.write(
        "postgres-pooling.md",
        &format!(
            "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Postgres pooling\n\n{}\n",
            POOLING_BODY
        ),
    );
    fixture.write(
        "pooling-again.md",
        &format!(
            "---\ncreated: \"2026-08-02 10:00\"\n---\n\n# Pooling again\n\n{} The margin was measured on the staging cluster.\n",
            POOLING_BODY
        ),
    );
    let written = fixture.store().join("pooling-again.md");

    let report = fixture.post_tool_report(&fixture.bash_write(
        "iwe create pooling-again --strict --content -",
        &written.to_string_lossy(),
    ));
    assert!(
        report.contains("`pooling-again` closely matches `postgres-pooling`"),
        "{}",
        report
    );
    assert!(
        report.contains("iwe retrieve -k postgres-pooling"),
        "{}",
        report
    );

    fixture.post_tool_quiet(&fixture.bash_write(
        "iwe create pooling-again --strict --content - --dry-run",
        "",
    ));
}

#[test]
fn enable_installs_the_starter_schema_and_strict_refuses_a_bad_stamp() {
    let root = TempDir::new().expect("Failed to create temp directory");

    let output = run_enable(root.path(), &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wrote .iwe/schemas/memory.yaml"),
        "{}",
        stdout
    );
    let config = read_to_string(root.path().join(".iwe/config.toml")).expect("config written");
    assert!(config.contains("[schemas.memory]"), "{}", config);

    let bad = run_iwe(
        root.path(),
        &[
            "create",
            "bad-stamp",
            "--strict",
            "--content",
            "---\ncreated: \"2026-08-01\"\n---\n\n# Bad stamp\n\nBody.\n",
        ],
    );
    assert_ne!(bad.status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("does not match"),
        "{}",
        String::from_utf8_lossy(&bad.stderr)
    );
    assert!(!root.path().join("bad-stamp.md").exists());

    let good = run_iwe(
        root.path(),
        &[
            "create",
            "good-stamp",
            "--strict",
            "--content",
            "---\ncreated: \"2026-08-01 10:00\"\nsession: \"abc\"\n---\n\n# Good stamp\n\nBody.\n",
        ],
    );
    assert_eq!(
        good.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&good.stderr)
    );

    let plain = run_iwe(
        root.path(),
        &[
            "create",
            "plain",
            "--strict",
            "--content",
            "# Plain\n\nNo stamp at all.\n",
        ],
    );
    assert_eq!(
        plain.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&plain.stderr)
    );

    let again = run_enable(root.path(), &[]);
    assert_eq!(again.status.code(), Some(2));
}
