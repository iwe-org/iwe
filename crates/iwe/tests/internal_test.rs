use std::fs::{create_dir_all, read_to_string, write};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use chrono::Local;
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

    fn state(&self) -> PathBuf {
        self.root.path().join("state")
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

    fn stop_utc(&self, extra: &[&str]) -> String {
        let transcripts = self.transcripts();
        let mut args = vec!["stop", "--transcripts", transcripts.to_str().expect("path")];
        args.extend_from_slice(extra);
        let output = self.run_tz(&args, None, Some("UTC"));
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
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

    fn sweep_transcript(&self, name: &str, cycles: usize) {
        let body: String = (0..cycles)
            .map(|index| {
                format!(
                    concat!(
                        "{{\"type\":\"user\",\"isMeta\":true,\"message\":{{\"content\":\"Stop hook feedback:\\nsweep now\"}}}}\n",
                        "{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"id\":\"tool-{}\",\"name\":\"Agent\",\"input\":{{\"subagent_type\":\"plugin:distill\",\"prompt\":\"work the jobs\"}}}}]}}}}\n",
                        "{{\"type\":\"user\",\"message\":{{\"content\":[{{\"tool_use_id\":\"tool-{}\",\"type\":\"tool_result\",\"content\":\"agent launched\"}}]}}}}\n",
                        "{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"text\",\"text\":\"launched in the background\"}}]}}}}\n",
                        "{{\"type\":\"user\",\"origin\":{{\"kind\":\"task-notification\"}},\"message\":{{\"content\":\"agent finished\"}}}}\n",
                        "{{\"type\":\"assistant\",\"message\":{{\"content\":[{{\"type\":\"text\",\"text\":\"the queue is drained\"}}]}}}}\n",
                    ),
                    index, index
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
        collect_documents(&self.root.path().to_path_buf(), &self.state(), &mut entries);
        entries.sort();
        entries
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
            .env("IWE_MEMORY_STATE", self.state())
            .env_remove("IWE_MEMORY_TRANSCRIPTS")
            .env_remove("CLAUDE_CONFIG_DIR")
            .env_remove("CLAUDE_PLUGIN_ROOT")
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

    fn stop(&self, extra: &[&str], payload: Option<&str>) -> String {
        let transcripts = self.transcripts();
        let mut args = vec!["stop", "--transcripts", transcripts.to_str().expect("path")];
        args.extend_from_slice(extra);
        let output = self.run(&args, payload);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn plugin_root(&self, manifest: Option<&str>) -> PathBuf {
        let root = self.root.path().join("plugin");
        create_dir_all(root.join(".claude-plugin")).expect("Failed to create plugin root");
        if let Some(manifest) = manifest {
            write(root.join(".claude-plugin/plugin.json"), manifest)
                .expect("Failed to write plugin manifest");
        }
        root
    }

    fn stop_as_plugin(&self, manifest: Option<&str>) -> String {
        let root = self.plugin_root(manifest);
        self.stop_with_env(&[("CLAUDE_PLUGIN_ROOT", root.to_str().expect("path"))])
    }

    fn stop_with_env(&self, env: &[(&str, &str)]) -> String {
        let transcripts = self.transcripts();
        let args = ["stop", "--transcripts", transcripts.to_str().expect("path")];
        let output = self.run_env(&args, None, env);
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stderr, Vec::<u8>::new());
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

    fn chunk_file(&self, session: &str, from: usize) -> PathBuf {
        self.state().join(session).join(format!("{:06}.md", from))
    }

    fn chunk(&self, session: &str, from: usize) -> String {
        std::fs::read_to_string(self.chunk_file(session, from)).expect("Failed to read chunk")
    }

    fn chunk_exists(&self, session: &str, from: usize) -> bool {
        self.chunk_file(session, from).is_file()
    }

    fn write_chunk(&self, session: &str, from: usize, content: &str) {
        let path = self.chunk_file(session, from);
        create_dir_all(path.parent().expect("chunk parent")).expect("Failed to create parent");
        write(&path, content).expect("Failed to write chunk");
    }

    fn chunk_files(&self, session: &str) -> Vec<PathBuf> {
        self.chunk_keys(session)
            .into_iter()
            .map(|relative| self.root.path().join(relative))
            .collect()
    }

    fn chunk_keys(&self, session: &str) -> Vec<String> {
        self.tree()
            .into_iter()
            .map(|(path, _)| path)
            .filter(|path| path.starts_with(&format!("state/{}/", session)))
            .collect()
    }
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

fn now_stamp() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

fn created_stamp(document: &str) -> String {
    document
        .lines()
        .find_map(|line| line.strip_prefix("created: "))
        .expect("document carries a created stamp")
        .trim_matches('"')
        .to_string()
}

fn body_of(document: &str) -> String {
    document
        .split_once("\n---\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_else(|| document.to_string())
}

const CAPTURE_BLOCK: &str = indoc! {r#"
    {
      "decision": "block",
      "reason": "IWE memory sweep: call the Agent tool now with subagent_type \"distill\", run_in_background true, and the single-line prompt \"Work the capture jobs waiting in this workspace.\", then stop immediately without any other output or commentary."
    }
"#};

const SURVEY_HEADER: &str =
    "session                                   lines   captured   pending  chunks  signal\x20\x20";

const SWEEP_POLICY: &str = indoc! {"
    ---
    sweep_threshold_lines: 5
    chunk_chars: 30
    max_items_per_chunk: 2
    ---

    # Memory policy

    How this store is written.
"};

const KNOB_VARIABLES: [&str; 6] = [
    "IWE_SWEEP_THRESHOLD_LINES",
    "IWE_CHUNK_CHARS",
    "IWE_MAX_CHUNKS_PER_SWEEP",
    "IWE_MAX_ITEMS_PER_CHUNK",
    "IWE_INFLIGHT_TTL_MINUTES",
    "IWE_INJECTION_MAX_TOKENS",
];

fn without_knob_variables(command: &mut Command) -> &mut Command {
    for name in KNOB_VARIABLES {
        command.env_remove(name);
    }
    command
}

const PLAIN_POLICY: &str = indoc! {"
    # Memory policy

    How this store is written.
"};

#[test]
fn hook_without_the_memory_document_stays_silent() {
    let fixture = HookFixture::new(None);
    fixture.transcript("session-one", 20);

    assert_eq!(fixture.session_start(None), "");
    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(fixture.tree(), Vec::new());
}

#[test]
fn survey_outside_a_memory_store_fails_loudly() {
    let fixture = HookFixture::new(None);
    fixture.transcript("session-one", 20);
    let transcripts = fixture.transcripts();

    let output = fixture.run(
        &[
            "stop",
            "--transcripts",
            transcripts.to_str().expect("path"),
            "--survey",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a memory-enabled"), "{}", stderr);
    assert!(stderr.contains("/iwe:init"), "{}", stderr);
}

#[test]
fn survey_without_a_transcript_directory_fails_loudly() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));

    let output = fixture.run(
        &[
            "stop",
            "--transcripts",
            "/nonexistent/transcripts/for/this/test",
            "--survey",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no transcript directory"));
}

#[test]
fn hook_with_a_missing_cwd_stays_silent() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 20);

    let payload = "{\"cwd\":\"/nonexistent/directory/for/this/test\"}";
    assert_eq!(fixture.session_start(Some(payload)), "");
    assert_eq!(fixture.stop(&[], Some(payload)), "");
    assert_eq!(fixture.tree().len(), 1);
}

#[test]
fn session_start_indexes_dated_documents_newest_first() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
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
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Beta Note](beta) · created: 2026-08-05 11:00
            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_drops_the_oldest_entries_over_the_token_budget() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
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
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Gamma Note](gamma) · created: 2026-08-09 12:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_names_the_query_cookbook_when_present() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );
    fixture.write("queries.md", "# Queries\n\nCookbook.\n");

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            The `queries` document is this store's query cookbook.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_falls_back_to_undated_documents() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write("alpha.md", "# Alpha Note\n\nBody one.\n");

    assert_eq!(
        fixture.session_start(None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha)

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
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
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_is_silent_when_only_policy_documents_exist() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write("queries.md", "# Queries\n\nCookbook.\n");

    assert_eq!(fixture.session_start(None), "");
}

#[test]
fn session_start_closes_the_block_with_the_supplied_footer() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    assert_eq!(
        fixture.session_start_with(&["--footer", "Run `example one` to record a note."], None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Run `example one` to record a note.
            </iwe-memory>
        "}
    );
}

#[test]
fn session_start_falls_back_to_the_default_footer() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write(
        "alpha.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Alpha Note\n\nBody one.\n",
    );

    assert_eq!(
        fixture.session_start_with(&["--footer", "   "], None),
        indoc! {"
            <iwe-memory>
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

#[test]
fn stop_imports_the_whole_pending_span_as_chunks() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    let before = now_stamp();
    let stdout = fixture.stop(&[], None);
    let after = now_stamp();

    assert_eq!(stdout, CAPTURE_BLOCK);
    assert_eq!(
        fixture.chunk_keys("session-one"),
        vec![
            "state/session-one/000000.md",
            "state/session-one/000002.md",
            "state/session-one/000004.md",
        ]
    );

    let chunk = fixture.chunk("session-one", 2);
    let stamp = created_stamp(&chunk);
    assert!(stamp == before || stamp == after);
    assert_eq!(
        chunk,
        format!(
            indoc! {"
                ---
                session: session-one
                created: {}
                covers_from: 2
                covers_lines: 4
                max_items: 2
                claimed: {}
                ---

                # Capture chunk session-one lines 2-4

                [user]
                line 2

                [user]
                line 3
            "},
            stamp, stamp
        )
    );

    assert_eq!(
        fixture.read("sessions/session-one.md"),
        format!(
            indoc! {"
                ---
                session: session-one
                created: {}
                distilled_lines: 0
                transcript: {}
                ---

                # Session session-one

                Agent session in this workspace.
            "},
            stamp,
            fixture.transcripts().join("session-one.jsonl").display()
        )
    );
}

#[test]
fn stop_imports_nothing_below_the_threshold() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 4);

    assert_eq!(fixture.stop(&[], None), "");
    assert!(!fixture.exists("sessions/session-one.md"));
    assert!(fixture.chunk_keys("session-one").is_empty());
}

#[test]
fn chunk_boundaries_do_not_move_when_the_span_starts_later() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 8);
    fixture.stop(&[], None);
    let whole_span = fixture.chunk("session-one", 4);

    let later = HookFixture::new(Some(SWEEP_POLICY));
    later.transcript("session-one", 8);
    later.write(
        "sessions/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ndistilled_lines: 2\n---\n\n# Session session-one\n\nAgent session in this workspace.\n",
    );
    later.stop(&[], None);

    assert_eq!(
        later.chunk_keys("session-one"),
        vec![
            "state/session-one/000002.md",
            "state/session-one/000004.md",
            "state/session-one/000006.md",
        ]
    );
    assert_eq!(
        body_of(&later.chunk("session-one", 4)),
        body_of(&whole_span)
    );
}

#[test]
fn re_import_leaves_an_unchanged_chunk_byte_for_byte() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);
    let before = fixture.tree();

    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(fixture.tree(), before);
}

#[test]
fn re_import_grows_the_partial_tail_chunk_in_place() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 5);
    fixture.stop(&[], None);

    let tail = fixture.chunk("session-one", 4);
    let stamp = created_stamp(&tail);
    assert!(tail.contains("covers_lines: 5"));
    assert!(!tail.contains("line 5"));

    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);

    let grown = fixture.chunk("session-one", 4);
    assert_eq!(created_stamp(&grown), stamp);
    assert!(grown.contains("covers_lines: 6"));
    assert!(grown.contains("line 5"));
    assert_eq!(fixture.chunk_keys("session-one").len(), 3);
}

#[test]
fn re_import_never_rewrites_a_captured_chunk() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 5);
    fixture.stop(&[], None);
    for covers in [2, 4, 5] {
        fixture.job_complete_ok("session-one", covers, &[]);
    }
    let captured = fixture.chunk("session-one", 4);
    assert!(captured.contains("captured_at:"));

    let session = fixture.read("sessions/session-one.md");
    fixture.write(
        "sessions/session-one.md",
        &session.replace("distilled_lines: 5", "distilled_lines: 4"),
    );
    fixture.transcript("session-one", 20);
    fixture.stop(&[], None);

    assert_eq!(fixture.chunk("session-one", 4), captured);
}

#[test]
fn stop_leaves_the_store_inbox_alone() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.write("inbox/errand.md", "# Errand\n\nTomorrow at nine.\n");
    fixture.write(
        "inbox/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ncovers_lines: 6\nmax_items: 5\n---\n\n# Capture job session-one\n\nlegacy\n",
    );
    let inbox_before: Vec<(String, String)> = fixture
        .tree()
        .into_iter()
        .filter(|(path, _)| path.starts_with("inbox/"))
        .collect();

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);

    let inbox_after: Vec<(String, String)> = fixture
        .tree()
        .into_iter()
        .filter(|(path, _)| path.starts_with("inbox/"))
        .collect();
    assert_eq!(inbox_after, inbox_before);
    assert_eq!(fixture.chunk_keys("session-one").len(), 3);
}

#[test]
fn stop_relocates_chunks_left_in_the_store() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 8);
    fixture.write(
        "sessions/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ndistilled_lines: 2\n---\n\n# Session session-one\n\nAgent session in this workspace.\n",
    );
    fixture.write(
        "sessions/session-one/000002.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ncovers_from: 2\ncovers_lines: 4\nmax_items: 2\nclaimed: \"2026-08-01 10:05\"\n---\n\n# Capture chunk session-one lines 2-4\n\n[user]\nline 2\n\n[user]\nline 3\n",
    );
    let captured = "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ncovers_from: 0\ncovers_lines: 2\nmax_items: 2\ncaptured_at: \"2026-08-01 10:10\"\n---\n\n# Capture chunk session-one lines 0-2\n\n[user]\nline 0\n\n[user]\nline 1\n";
    fixture.write("sessions/session-one/000000.md", captured);
    fixture.write(
        "sessions/session-one/notes.md",
        "# Notes on session one\n\nKept by hand.\n",
    );

    fixture.stop(&[], None);

    assert_eq!(
        fixture
            .tree()
            .into_iter()
            .map(|(path, _)| path)
            .filter(|path| path.starts_with("sessions/"))
            .collect::<Vec<_>>(),
        vec![
            "sessions/session-one.md".to_string(),
            "sessions/session-one/notes.md".to_string(),
        ]
    );
    assert_eq!(
        fixture.chunk_keys("session-one"),
        vec![
            "state/session-one/000000.md",
            "state/session-one/000002.md",
            "state/session-one/000004.md",
            "state/session-one/000006.md",
        ]
    );
    assert_eq!(fixture.chunk("session-one", 0), captured);
    assert_eq!(
        fixture.chunk("session-one", 2),
        format!(
            indoc! {"
                ---
                session: session-one
                created: 2026-08-01 10:00
                covers_from: 2
                covers_lines: 4
                max_items: 2
                claimed: {}
                ---

                # Capture chunk session-one lines 2-4

                [user]
                line 2

                [user]
                line 3
            "},
            now_stamp()
        )
    );
}

#[test]
fn stop_skips_an_unsafe_session_id() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("not a safe id", 20);

    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(fixture.tree().len(), 1);
}

#[test]
fn stop_writes_chunks_under_the_workspace_iwe_directory_by_default() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 8);
    let transcripts = fixture.transcripts();
    let args = ["stop", "--transcripts", transcripts.to_str().expect("path")];

    let output = fixture.run_env(&args, None, &[("IWE_MEMORY_STATE", "")]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, Vec::<u8>::new());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"decision\": \"block\""));

    let chunks = fixture.store().join(".iwe/claude-sessions/session-one");
    assert!(chunks.join("000000.md").is_file());
    assert!(!fixture.state().exists());
    assert_eq!(
        read_to_string(fixture.store().join(".iwe/claude-sessions/.gitignore")).expect("ignore"),
        "*\n"
    );
    assert_eq!(
        read_to_string(fixture.store().join(".iwe/claude-sessions/CACHEDIR.TAG")).expect("tag"),
        indoc! {"
            Signature: 8a477f597d28d172789f06886806bc55
            # This file is a cache directory tag created by iwe.
            # For information about cache directory tags, see:
            #\thttps://bford.info/cachedir/
        "}
    );

    let survey = fixture.run_env(
        &[
            "stop",
            "--survey",
            "--transcripts",
            transcripts.to_str().expect("path"),
        ],
        None,
        &[("IWE_MEMORY_STATE", "")],
    );
    let stdout = String::from_utf8_lossy(&survey.stdout);
    let chunks_line = stdout
        .lines()
        .find(|line| line.starts_with("chunks: "))
        .expect("survey names the chunk directory");
    let expected = Path::new("store").join(".iwe").join("claude-sessions");
    assert!(
        chunks_line.ends_with(&format!(
            "{}{}",
            std::path::MAIN_SEPARATOR,
            expected.display()
        )),
        "{}",
        chunks_line
    );
}

#[test]
fn stop_fails_loudly_when_the_chunk_directory_cannot_be_created() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 8);
    let blocker = fixture.root.path().join("blocked");
    write(&blocker, "").expect("blocker");
    let state = blocker.join("state");
    let transcripts = fixture.transcripts();
    let args = ["stop", "--transcripts", transcripts.to_str().expect("path")];

    let output = fixture.run_env(
        &args,
        None,
        &[("IWE_MEMORY_STATE", state.to_str().expect("path"))],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot write capture chunks under"));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert!(!fixture.exists("sessions/session-one.md"));
}

#[test]
fn stop_stamps_a_pending_chunk_left_below_the_watermark() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 4);
    fixture.write(
        "sessions/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ndistilled_lines: 4\n---\n\n# Session session-one\n\nAgent session in this workspace.\n",
    );
    fixture.write_chunk(
        "session-one",
        2,
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ncovers_from: 2\ncovers_lines: 4\nmax_items: 2\nclaimed: \"2026-08-01 10:05\"\n---\n\n# Capture chunk session-one lines 2-4\n\n[user]\nline 2\n\n[user]\nline 3\n",
    );

    assert_eq!(fixture.stop(&[], None), "");

    assert_eq!(
        fixture.chunk("session-one", 2),
        format!(
            indoc! {"
                ---
                session: session-one
                created: 2026-08-01 10:00
                covers_from: 2
                covers_lines: 4
                max_items: 2
                claimed: 2026-08-01 10:05
                captured_at: {}
                ---

                # Capture chunk session-one lines 2-4

                [user]
                line 2

                [user]
                line 3
            "},
            now_stamp()
        )
    );
    assert_eq!(
        fixture
            .read("sessions/session-one.md")
            .matches("distilled_lines: 4")
            .count(),
        1
    );
}

#[test]
fn removing_the_chunk_directory_ignore_file_sticks() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);
    assert_eq!(
        read_to_string(fixture.state().join(".gitignore")).expect("ignore"),
        "*\n"
    );

    std::fs::remove_file(fixture.state().join(".gitignore")).expect("Failed to remove ignore");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);

    assert!(!fixture.state().join(".gitignore").exists());
    assert!(fixture.chunk_exists("session-two", 0));
}

#[test]
fn adopt_stamps_a_pending_chunk_left_below_the_watermark() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);
    assert!(!fixture.chunk("session-one", 0).contains("captured_at:"));

    fixture.stop(&["--adopt"], None);

    for from in [0, 2, 4] {
        let chunk = fixture.chunk("session-one", from);
        assert_eq!(
            chunk
                .lines()
                .filter(|line| line.starts_with("captured_at: "))
                .collect::<Vec<_>>(),
            vec![format!("captured_at: {}", now_stamp())],
            "{}",
            chunk
        );
    }
    assert_eq!(fixture.job_next(), "");
}

#[test]
fn stop_is_silent_when_the_hook_is_already_active() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 20);
    let before = fixture.tree();

    assert_eq!(fixture.stop(&[], Some("{\"stop_hook_active\":true}")), "");
    assert_eq!(fixture.tree(), before);
}

#[test]
fn stop_respawns_over_a_stale_claim_without_touching_the_chunk() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);

    for from in [0, 2, 4] {
        let chunk = fixture.chunk("session-one", from);
        fixture.write_chunk(
            "session-one",
            from,
            &chunk.replace(
                &format!("claimed: {}", created_stamp(&chunk)),
                "claimed: 2020-01-01 00:00",
            ),
        );
    }
    let bodies: Vec<String> = [0, 2, 4]
        .iter()
        .map(|from| body_of(&fixture.chunk("session-one", *from)))
        .collect();

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);

    let stamp = now_stamp();
    for (index, from) in [0, 2, 4].iter().enumerate() {
        let chunk = fixture.chunk("session-one", *from);
        assert!(chunk.contains(&format!("claimed: {}", stamp)), "{}", chunk);
        assert_eq!(body_of(&chunk), bodies[index]);
    }
}

#[test]
fn stop_answers_with_the_supplied_capture_reason() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(
        fixture.stop(
            &[
                "--capture-reason",
                "Run the example one routine, then stop."
            ],
            None
        ),
        indoc! {r#"
            {
              "decision": "block",
              "reason": "Run the example one routine, then stop."
            }
        "#}
    );
}

#[test]
fn stop_names_the_plugin_agent_when_run_as_a_plugin_hook() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(
        fixture.stop_as_plugin(Some(r#"{"name": "iwe", "version": "0.12.0"}"#)),
        indoc! {r#"
            {
              "decision": "block",
              "reason": "IWE memory sweep: call the Agent tool now with subagent_type \"iwe:distill\", run_in_background true, and the single-line prompt \"Work the capture jobs waiting in this workspace.\", then stop immediately without any other output or commentary."
            }
        "#}
    );
}

#[test]
fn stop_names_the_bare_agent_when_the_plugin_root_has_no_manifest() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(fixture.stop_as_plugin(None), CAPTURE_BLOCK);
}

#[test]
fn stop_names_the_bare_agent_when_the_plugin_name_is_not_a_safe_id() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(
        fixture.stop_as_plugin(Some(r#"{"name": "my plugin\", \"x"}"#)),
        CAPTURE_BLOCK
    );
}

#[test]
fn stop_keeps_a_supplied_capture_reason_verbatim_as_a_plugin_hook() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    let root = fixture.plugin_root(Some(r#"{"name": "iwe"}"#));
    let transcripts = fixture.transcripts();

    let output = fixture.run_env(
        &[
            "stop",
            "--transcripts",
            transcripts.to_str().expect("path"),
            "--capture-reason",
            "Run the {{ agent }} routine, then stop.",
        ],
        None,
        &[("CLAUDE_PLUGIN_ROOT", root.to_str().expect("path"))],
    );

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).expect("Valid UTF-8 output"),
        indoc! {r#"
            {
              "decision": "block",
              "reason": "Run the {{ agent }} routine, then stop."
            }
        "#}
    );
}

#[test]
fn stop_falls_back_on_a_blank_capture_reason() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(
        fixture.stop(&["--capture-reason", "  "], None),
        CAPTURE_BLOCK
    );
}

#[test]
fn stop_does_not_spawn_a_second_agent_inside_the_claim_ttl() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    let claimed = fixture.tree();

    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(fixture.tree(), claimed);
}

#[test]
fn survey_reports_the_backlog_and_the_queue_without_writing() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 12);
    fixture.stop(&[], None);
    fixture.job_complete_ok("session-one", 2, &[]);
    let before = fixture.tree();

    let stdout = fixture.stop(&["--survey"], None);

    assert_eq!(
        stdout,
        format!(
            "transcripts: {}\nchunks: {}\nthreshold: 5 lines\n\n{}\nsession-one                                  12          2        10       5      10  pending, claimed\n\n1 transcript(s), 1 pending over 10 uncaptured line(s) carrying 10 user turn(s), 5 pending chunk(s) of which 5 claimed, 0 skipped\n",
            fixture.transcripts().display(),
            fixture.state().display(),
            SURVEY_HEADER
        )
    );
    assert_eq!(fixture.tree(), before);
}

#[test]
fn survey_marks_the_current_session_and_the_threshold() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 3);
    let transcript = fixture.transcripts().join("session-one.jsonl");

    let stdout = fixture.stop(
        &["--survey"],
        Some(&serde_json::json!({ "transcript_path": transcript }).to_string()),
    );

    assert_eq!(
        stdout,
        format!(
            "transcripts: {}\nchunks: {}\nthreshold: 5 lines\n\n{}\nsession-one                                   3          0         3       0       0  under the threshold, this session\n\n1 transcript(s), 0 pending over 0 uncaptured line(s) carrying 0 user turn(s), 0 pending chunk(s) of which 0 claimed, 0 skipped\n",
            fixture.transcripts().display(),
            fixture.state().display(),
            SURVEY_HEADER
        )
    );
}

#[test]
fn adopt_stamps_every_transcript_at_its_current_length() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 7);

    let stdout = fixture.stop(&["--adopt"], None);

    assert_eq!(
        stdout,
        indoc! {"
            adopted session-one at line 7

            1 session(s) adopted without capture; memory starts from here
        "}
    );

    let session = fixture.read("sessions/session-one.md");
    assert_eq!(
        session,
        format!(
            indoc! {"
                ---
                session: session-one
                created: {}
                distilled_lines: 7
                transcript: {}
                ---

                # Session session-one

                Agent session in this workspace.
            "},
            created_stamp(&session),
            fixture.transcripts().join("session-one.jsonl").display()
        )
    );
    assert!(fixture.chunk_keys("session-one").is_empty());
}

#[test]
fn survey_and_adopt_conflict() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    let transcripts = fixture.transcripts();
    let output = fixture.run(
        &[
            "stop",
            "--survey",
            "--adopt",
            "--transcripts",
            transcripts.to_str().expect("path"),
        ],
        None,
    );

    assert!(!output.status.success());
    assert_eq!(output.stdout, Vec::<u8>::new());
}

#[test]
fn stop_honours_the_max_chunks_flag_across_sessions() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 20);
    fixture.transcript("session-two", 20);

    assert_eq!(fixture.stop(&["--max-chunks", "3"], None), CAPTURE_BLOCK);

    let imported = fixture
        .tree()
        .into_iter()
        .filter(|(path, _)| path.starts_with("state/"))
        .count();
    assert_eq!(imported, 3);
}

#[test]
fn a_transcript_of_stop_hook_turns_alone_never_blocks() {
    let fixture = HookFixture::new(Some(PLAIN_POLICY));
    fixture.sweep_transcript("session-one", 21);

    let before = now_stamp();
    let stdout = fixture.stop(&[], None);
    let after = now_stamp();

    assert_eq!(stdout, "");
    assert_eq!(
        fixture.chunk_keys("session-one"),
        vec!["state/session-one/000000.md"]
    );

    let chunk = fixture.chunk("session-one", 0);
    let stamp = created_stamp(&chunk);
    assert!(stamp == before || stamp == after);
    assert_eq!(
        chunk,
        format!(
            indoc! {"
                ---
                session: \"{}\"
                created: \"{}\"
                covers_from: 0
                covers_lines: 126
                max_items: 3
                captured_at: \"{}\"
                ---

                # Capture chunk session-one lines 0-126
            "},
            "session-one", stamp, stamp
        )
    );
}

#[test]
fn a_second_sweep_over_stop_hook_turns_advances_the_watermark() {
    let fixture = HookFixture::new(Some(PLAIN_POLICY));
    fixture.sweep_transcript("session-one", 21);

    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(fixture.stop(&[], None), "");
    assert_eq!(
        fixture.chunk_keys("session-one"),
        vec!["state/session-one/000000.md"]
    );
}

#[test]
fn stop_falls_back_on_unusable_knobs() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        sweep_threshold_lines: \"lots\"
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.transcript("session-one", 150);

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    assert!(fixture.chunk_exists("session-one", 0));
    assert!(fixture.exists("sessions/session-one.md"));
    assert_eq!(fixture.chunk_keys("session-one").len(), 1);
}

#[test]
fn stop_takes_the_threshold_from_the_environment_when_the_store_is_silent() {
    let fixture = HookFixture::new(Some(PLAIN_POLICY));
    fixture.transcript("session-one", 150);

    assert_eq!(
        fixture.stop_with_env(&[("IWE_SWEEP_THRESHOLD_LINES", "200")]),
        ""
    );
    assert!(!fixture.exists("sessions/session-one.md"));

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    assert!(fixture.exists("sessions/session-one.md"));
}

#[test]
fn stop_prefers_the_store_knob_over_the_environment() {
    let fixture = HookFixture::new(Some(indoc! {"
        ---
        sweep_threshold_lines: 500
        ---

        # Memory policy

        How this store is written.
    "}));
    fixture.transcript("session-one", 85);

    assert_eq!(
        fixture.stop_with_env(&[("IWE_SWEEP_THRESHOLD_LINES", "5")]),
        ""
    );
    assert!(!fixture.exists("sessions/session-one.md"));
}

#[test]
fn stop_falls_back_on_an_unusable_environment_knob() {
    let fixture = HookFixture::new(Some(PLAIN_POLICY));
    fixture.transcript("session-one", 150);

    assert_eq!(
        fixture.stop_with_env(&[("IWE_SWEEP_THRESHOLD_LINES", "lots")]),
        CAPTURE_BLOCK
    );
    assert!(fixture.chunk_exists("session-one", 0));
    assert!(fixture.exists("sessions/session-one.md"));
}

#[test]
fn stop_tolerates_an_unparseable_payload() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);

    assert_eq!(fixture.stop(&[], Some("not json at all")), CAPTURE_BLOCK);
    assert!(fixture.chunk_exists("session-one", 0));
}

#[test]
fn hook_reads_the_store_from_the_payload_cwd() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
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
            This repository is an IWE workspace with durable memory in it: markdown documents, captured from past sessions and reviewed as ordinary diffs.
            Most recently recorded, newest first — titles and keys only, not content:

            - [Alpha Note](alpha) · created: 2026-08-01 10:00

            Read one with `iwe retrieve -k <key>`; search with `iwe find --lexical \"<terms>\" --limit 5`.
            The `MEMORY.md` document says what this store keeps and how it is written: `iwe retrieve -k MEMORY`.
            Use `/iwe:distill` to record something worth keeping, `/iwe:reflect` to reorganize the store.
            </iwe-memory>
        "}
    );
}

impl HookFixture {
    fn job(&self, args: &[&str]) -> Output {
        let mut command = Command::new(crate::common::get_iwe_binary_path());
        command
            .arg("internal")
            .arg("claude")
            .arg("job")
            .args(args)
            .current_dir(self.store())
            .env("IWE_MEMORY_STATE", self.state())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        without_knob_variables(&mut command)
            .output()
            .expect("Failed to execute iwe internal claude job")
    }

    fn job_next(&self) -> String {
        let output = self.job(&["next"]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn job_complete(&self, session: &str, lines: usize, wrote: &[&str]) -> Output {
        let lines = lines.to_string();
        let mut args = vec!["complete", session, "--lines", &lines];
        for key in wrote {
            args.push("--wrote");
            args.push(key);
        }
        self.job(&args)
    }

    fn job_complete_ok(&self, session: &str, lines: usize, wrote: &[&str]) -> String {
        let output = self.job_complete(session, lines, wrote);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stderr, Vec::<u8>::new());
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn job_complete_err(&self, session: &str, lines: usize, wrote: &[&str]) -> String {
        let output = self.job_complete(session, lines, wrote);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, Vec::<u8>::new());
        String::from_utf8(output.stderr).expect("Valid UTF-8 output")
    }

    fn job_complete_ok_with(
        &self,
        session: &str,
        lines: usize,
        wrote: &[&str],
        extra: &[&str],
    ) -> String {
        let lines = lines.to_string();
        let mut args = vec!["complete", session, "--lines", &lines];
        for key in wrote {
            args.push("--wrote");
            args.push(key);
        }
        args.extend_from_slice(extra);
        let output = self.job(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn import(&self, session: &str, lines: usize) -> String {
        self.transcript(session, lines);
        assert_eq!(self.stop(&[], None), CAPTURE_BLOCK);
        created_stamp(&self.chunk(session, 0))
    }

    fn restamp(&self, session: &str, field: &str, stamp: &str) {
        for path in self.chunk_files(session) {
            let rewritten = std::fs::read_to_string(&path)
                .expect("Failed to read chunk")
                .lines()
                .map(|line| {
                    if line.starts_with(&format!("{}: ", field)) {
                        format!("{}: {}", field, stamp)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            write(&path, &rewritten).expect("Failed to write chunk");
        }
    }

    fn job_ok(&self, args: &[&str]) -> String {
        let output = self.job(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("Valid UTF-8 output")
    }

    fn served_sessions(&self, batch: &str) -> Vec<String> {
        batch
            .lines()
            .filter_map(|line| line.strip_prefix("session: "))
            .map(str::to_string)
            .collect()
    }
}

#[test]
fn job_next_serves_the_chunk_at_the_watermark() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    let stamp = fixture.import("session-one", 6);

    assert_eq!(
        fixture.job_next(),
        format!(
            indoc! {"
                session: session-one
                covers_from: 0
                covers_lines: 2
                max_items: 2
                created: {}

                # Capture chunk session-one lines 0-2

                [user]
                line 0

                [user]
                line 1
            "},
            stamp
        )
    );

    assert_eq!(fixture.job_next(), fixture.job_next());
}

#[test]
fn job_next_walks_the_queue_and_then_falls_silent() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);

    for covers in [2, 4, 6] {
        assert!(fixture
            .job_next()
            .contains(&format!("covers_lines: {}\n", covers)));
        fixture.job_complete_ok("session-one", covers, &[]);
    }

    assert_eq!(fixture.job_next(), "");
}

#[test]
fn job_next_serves_the_newest_session_first() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);

    assert!(fixture.job_next().starts_with("session: session-two\n"));

    for covers in [2, 4, 6] {
        fixture.job_complete_ok("session-two", covers, &[]);
    }

    assert!(fixture.job_next().starts_with("session: session-one\n"));
}

#[test]
fn job_next_ranks_sessions_by_when_the_conversation_happened() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript_with_timestamps("session-one", 6);
    fixture.transcript_with_timestamps("session-two", 6);
    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);

    fixture.restamp("session-one", "occurred", "2019-01-01 00:00");

    assert!(fixture.job_next().starts_with("session: session-two\n"));
}

#[test]
fn job_skip_parks_one_session_and_serves_the_next() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);
    assert!(fixture.job_next().starts_with("session: session-two\n"));

    let output = fixture.job(&["skip", "session-two", "--lines", "2"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("skipped session-two at line 2"));

    assert!(fixture.job_next().starts_with("session: session-one\n"));
    assert!(fixture
        .chunk("session-two", 0)
        .contains(&format!("skipped: {}", now_stamp())));

    let survey = fixture.stop(&["--survey"], None);
    assert!(survey.contains("skipped"), "{}", survey);
    assert!(
        survey.ends_with("6 pending chunk(s) of which 4 claimed, 1 skipped\n"),
        "{}",
        survey
    );
}

#[test]
fn job_skip_expires_and_the_session_returns() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    let output = fixture.job(&["skip", "session-one", "--lines", "2"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(fixture.job_next(), "");

    let stale: String = fixture
        .chunk("session-one", 0)
        .lines()
        .map(|line| {
            if line.starts_with("skipped:") {
                "skipped: 2020-01-01 00:00".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fixture.write_chunk("session-one", 0, &stale);

    assert!(fixture.job_next().starts_with("session: session-one\n"));
}

fn make_read_only(path: &Path) {
    let mut permissions = std::fs::metadata(path)
        .expect("Failed to read permissions")
        .permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(path, permissions).expect("Failed to set permissions");
}

#[test]
fn job_next_fails_loudly_when_the_chunk_cannot_be_claimed() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "claimed", "2020-01-01 00:00");
    make_read_only(&fixture.chunk_file("session-one", 0));

    let output = fixture.job(&["next"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "error: cannot claim {}: the chunk is not writable, so the queue cannot serve it\n",
            fixture.chunk_file("session-one", 0).display()
        )
    );
    assert!(fixture
        .chunk("session-one", 0)
        .contains("claimed: 2020-01-01 00:00"));
}

#[test]
fn job_frontier_fails_loudly_when_a_chunk_cannot_be_claimed() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);
    fixture.restamp("session-one", "claimed", "2020-01-01 00:00");
    make_read_only(&fixture.chunk_file("session-one", 0));

    let output = fixture.job(&["frontier"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, Vec::<u8>::new());
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!(
            "error: cannot claim {}: the chunk is not writable, so the queue cannot serve it\n",
            fixture.chunk_file("session-one", 0).display()
        )
    );
}

#[test]
fn job_skip_refuses_a_line_count_the_chunk_does_not_cover() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);

    let output = fixture.job(&["skip", "session-one", "--lines", "6"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("covers through line 2"));
    assert!(!fixture.chunk("session-one", 0).contains("skipped:"));
}

#[test]
fn job_next_refreshes_the_claim_so_stop_does_not_double_spawn() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    for from in [0, 2, 4] {
        let chunk = fixture.chunk("session-one", from);
        fixture.write_chunk(
            "session-one",
            from,
            &chunk.replace(
                &format!("claimed: {}", created_stamp(&chunk)),
                "claimed: 2020-01-01 00:00",
            ),
        );
    }

    assert!(fixture.job_next().starts_with("session: session-one\n"));

    assert!(fixture
        .chunk("session-one", 0)
        .contains(&format!("claimed: {}", now_stamp())));
    assert_eq!(fixture.stop(&[], None), "");
}

#[test]
fn job_reset_rewinds_the_watermark_and_drops_the_chunks() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.job_complete_ok("session-one", 2, &[]);

    let output = fixture.job(&["reset", "session-one", "--to", "0"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "reset session-one to line 0: removed 3 chunk(s); the next sweep re-imports the span\n"
    );
    assert_eq!(fixture.chunk_keys("session-one"), Vec::<String>::new());
    assert!(fixture
        .read("sessions/session-one.md")
        .contains("distilled_lines: 0"));

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    assert!(fixture.job_next().starts_with("session: session-one\n"));
    assert!(fixture.job_next().contains("covers_from: 0\n"));
}

#[test]
fn job_reset_keeps_the_chunks_at_or_before_the_line() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 8);
    fixture.job_complete_ok("session-one", 2, &[]);
    fixture.job_complete_ok("session-one", 4, &[]);

    let output = fixture.job(&["reset", "session-one", "--to", "2"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "reset session-one to line 2: removed 3 chunk(s); the next sweep re-imports the span\n"
    );
    assert_eq!(
        fixture.chunk_keys("session-one"),
        vec!["state/session-one/000000.md".to_string()]
    );
    assert!(fixture
        .read("sessions/session-one.md")
        .contains("distilled_lines: 2"));

    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    assert!(fixture.job_next().contains("covers_from: 2\n"));
}

#[test]
fn job_frontier_serves_one_chunk_per_session_newest_first() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);

    let batch = fixture.job_ok(&["frontier"]);

    assert!(
        batch.starts_with("frontier: 2 of 2 servable session(s)\n"),
        "{}",
        batch
    );
    assert_eq!(
        fixture.served_sessions(&batch),
        vec!["session-two".to_string(), "session-one".to_string()]
    );
    assert!(batch.contains("=== chunk 1 of 2 ===\nsession: session-two\n"));
    assert!(batch.contains("=== chunk 2 of 2 ===\nsession: session-one\n"));
    assert_eq!(batch.matches("covers_from: 0\n").count(), 2);
    assert_eq!(batch.matches("covers_lines: 2\n").count(), 2);
}

#[test]
fn job_frontier_honours_the_limit() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);

    let batch = fixture.job_ok(&["frontier", "--limit", "1"]);

    assert!(
        batch.starts_with("frontier: 1 of 2 servable session(s)\n"),
        "{}",
        batch
    );
    assert_eq!(
        fixture.served_sessions(&batch),
        vec!["session-two".to_string()]
    );
}

#[test]
fn job_frontier_stops_at_the_character_budget_rather_than_cutting_a_digest() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);
    fixture.restamp("session-one", "claimed", "2020-01-01 00:00");

    let one = fixture.job_ok(&["frontier", "--max-chars", "1"]);

    assert!(
        one.starts_with("frontier: 1 of 2 servable session(s)\n"),
        "{}",
        one
    );
    assert_eq!(
        fixture.served_sessions(&one),
        vec!["session-two".to_string()]
    );
    assert!(
        one.contains("[user]\nline 0\n\n[user]\nline 1\n"),
        "{}",
        one
    );
    assert!(fixture
        .chunk("session-one", 0)
        .contains("claimed: 2020-01-01 00:00"));

    let both = fixture.job_ok(&["frontier"]);
    assert_eq!(fixture.served_sessions(&both).len(), 2);
    assert!(both.len() > one.len());
}

#[test]
fn job_frontier_claims_what_it_serves_so_stop_does_not_double_spawn() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "claimed", "2020-01-01 00:00");

    fixture.job_ok(&["frontier"]);

    assert!(fixture
        .chunk("session-one", 0)
        .contains(&format!("claimed: {}", now_stamp())));
    assert_eq!(fixture.stop(&[], None), "");
}

#[test]
fn job_frontier_passes_over_a_parked_session() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.restamp("session-one", "created", "2020-01-01 00:00");
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);
    assert_eq!(
        fixture
            .job(&["skip", "session-two", "--lines", "2"])
            .status
            .code(),
        Some(0)
    );

    let batch = fixture.job_ok(&["frontier"]);

    assert!(
        batch.starts_with("frontier: 1 of 1 servable session(s)\n"),
        "{}",
        batch
    );
    assert_eq!(
        fixture.served_sessions(&batch),
        vec!["session-one".to_string()]
    );
}

#[test]
fn job_frontier_falls_silent_on_a_drained_queue() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    for covers in [2, 4, 6] {
        fixture.job_complete_ok("session-one", covers, &[]);
    }

    assert_eq!(fixture.job_ok(&["frontier"]), "");
}

#[test]
fn a_frontier_chunk_completes_through_the_ordinary_verb() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.transcript("session-two", 6);
    fixture.stop(&[], None);

    fixture.job_ok(&["frontier"]);
    fixture.job_complete_ok("session-two", 2, &[]);
    fixture.job_complete_ok("session-one", 2, &[]);

    let batch = fixture.job_ok(&["frontier"]);
    assert_eq!(batch.matches("covers_from: 2\n").count(), 2);
}

#[test]
fn job_brief_serves_the_policy_the_schema_and_the_recent_documents() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.write(
        "deploy-order.md",
        "---\ncreated: 2026-08-20 09:00\n---\n\n# Deploy order\n\nMigrations first.\n",
    );
    fixture.write(
        "parser-vendored.md",
        "---\ncreated: 2026-08-21 09:00\norigin: user\n---\n\n# Vendored parser\n\nNot a dependency.\n",
    );

    let brief = fixture.job_ok(&["brief"]);

    assert!(
        brief
            .starts_with("=== policy: MEMORY ===\n# Memory policy\n\nHow this store is written.\n"),
        "{}",
        brief
    );
    assert!(brief.contains("=== schema: 2 document(s), machinery excluded ==="));
    assert!(brief.contains("| origin  |"));
    assert!(!brief.contains("distilled_lines"));
    assert!(!brief.contains("covers_lines"));
    assert!(brief.ends_with(indoc! {"
        === recent: 2 of 2 document(s) ===
        parser-vendored — Vendored parser
        deploy-order — Deploy order
    "}));
}

#[test]
fn job_brief_says_so_when_the_store_holds_nothing_to_imitate() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));

    let brief = fixture.job_ok(&["brief"]);

    assert!(brief.contains("=== schema: 0 document(s), machinery excluded ==="));
    assert!(brief.contains("no frontmatter yet: the policy is your only guide"));
    assert!(brief.ends_with("=== recent: 0 of 0 document(s) ===\n"));
}

#[test]
fn the_survey_counts_user_turns_and_not_the_noise_around_them() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    write(
        fixture.transcripts().join("session-one.jsonl"),
        indoc! {r#"
            {"type":"user","message":{"content":"the one real turn"}}
            {"type":"user","isMeta":true,"message":{"content":"a meta line"}}
            {"type":"user","message":{"content":[{"type":"tool_result","content":"not a turn"}]}}
            {"type":"assistant","message":{"content":[{"type":"text","text":"not a turn either"}]}}
            {"type":"summary","summary":"nor this"}
            not json at all
        "#},
    )
    .expect("Failed to write transcript");

    let survey = fixture.stop(&["--survey"], None);

    assert!(
        survey.contains("session-one                                   6          0         6       0       1  pending\n"),
        "{}",
        survey
    );
    assert!(survey.contains("carrying 1 user turn(s)"), "{}", survey);
}

#[test]
fn job_complete_advances_the_watermark_appends_the_note_and_stamps_the_chunk() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    let stamp = fixture.import("session-one", 6);
    fixture.write(
        "cache-warmup-order.md",
        "---\ncreated: \"2026-08-01 10:00\"\n---\n\n# Cache Warmup Order\n\nA fact.\n",
    );
    fixture.write(
        "retry-backoff.md",
        "---\ncreated: \"2026-08-01 11:00\"\n---\n\n# Retry Backoff\n\nAnother fact.\n",
    );

    let stdout = fixture.job_complete_ok(
        "session-one",
        2,
        &["cache-warmup-order", "retry-backoff", "cache-warmup-order"],
    );

    assert_eq!(
        stdout,
        "completed session-one: watermark at line 2, 2 link(s)\n"
    );
    assert!(fixture
        .chunk("session-one", 0)
        .contains(&format!("captured_at: {}", now_stamp())));
    assert_eq!(
        fixture.read("sessions/session-one.md"),
        format!(
            indoc! {"
                ---
                session: session-one
                created: {}
                distilled_lines: 2
                transcript: {}
                distilled_at: {}
                ---

                # Session session-one

                Agent session in this workspace.

                {} — captured 2 item(s) through line 2

                [Cache Warmup Order](../cache-warmup-order)

                [Retry Backoff](../retry-backoff)
            "},
            stamp,
            fixture.transcripts().join("session-one.jsonl").display(),
            stamp,
            stamp
        )
    );
}

#[test]
fn job_complete_with_no_links_still_advances_the_watermark() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    let stamp = fixture.import("session-one", 6);

    let stdout = fixture.job_complete_ok("session-one", 2, &[]);

    assert_eq!(
        stdout,
        "completed session-one: watermark at line 2, 0 link(s)\n"
    );
    let session = fixture.read("sessions/session-one.md");
    assert!(session.contains("distilled_lines: 2"));
    assert!(session.contains(&format!("{} — captured 0 item(s) through line 2", stamp)));
}

#[test]
fn job_complete_refuses_a_line_count_the_chunk_does_not_cover() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    let chunk = fixture.chunk("session-one", 0);

    let stderr = fixture.job_complete_err("session-one", 4, &[]);

    assert_eq!(
        stderr,
        "error: --lines 4: the chunk at line 0 covers through line 2\n"
    );
    assert_eq!(fixture.chunk("session-one", 0), chunk);
    assert!(fixture
        .read("sessions/session-one.md")
        .contains("distilled_lines: 0"));
}

#[test]
fn job_complete_names_the_expected_start_line_when_the_chunk_is_missing() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    std::fs::remove_file(fixture.chunk_file("session-one", 0)).expect("Failed to remove the chunk");

    let stderr = fixture.job_complete_err("session-one", 2, &[]);

    assert_eq!(
        stderr,
        "error: no pending capture chunk starting at line 0 for session-one; \
         the next sweep imports it\n"
    );
}

#[test]
fn job_complete_refuses_a_missing_wrote_key_and_keeps_the_chunk() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    let chunk = fixture.chunk("session-one", 0);

    let stderr = fixture.job_complete_err("session-one", 2, &["no-such-doc"]);

    assert_eq!(
        stderr,
        "error: --wrote no-such-doc: no such document in this store\n"
    );
    assert_eq!(fixture.chunk("session-one", 0), chunk);
    assert!(fixture
        .read("sessions/session-one.md")
        .contains("distilled_lines: 0"));
}

#[test]
fn job_complete_refuses_machinery_documents_as_capture_output() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);

    for key in [
        "MEMORY",
        "sessions/session-one",
        "sessions/session-one/000000",
    ] {
        let stderr = fixture.job_complete_err("session-one", 2, &[key]);
        assert_eq!(
            stderr,
            format!(
                "error: --wrote {}: the machinery's own documents are not capture output\n",
                key
            )
        );
    }
    assert!(fixture.chunk_exists("session-one", 0));
}

#[test]
fn job_complete_after_a_lost_stamp_marks_the_chunk_and_keeps_the_watermark() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);
    fixture.write(
        "sessions/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ndistilled_lines: 2\n---\n\n# Session session-one\n\nAgent session in this workspace.\n",
    );
    let session = fixture.read("sessions/session-one.md");

    let stdout = fixture.job_complete_ok("session-one", 2, &[]);

    assert_eq!(
        stdout,
        "already complete: session-one is at line 2; stamped 1 leftover chunk(s)\n"
    );
    assert!(fixture.chunk("session-one", 0).contains("captured_at:"));
    assert!(!fixture.chunk("session-one", 2).contains("captured_at:"));
    assert_eq!(fixture.read("sessions/session-one.md"), session);
}

#[test]
fn job_complete_without_a_chunk_reports_already_complete() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write(
        "sessions/session-one.md",
        "---\nsession: \"session-one\"\ncreated: \"2026-08-01 10:00\"\ndistilled_lines: 6\n---\n\n# Session session-one\n\nAgent session in this workspace.\n",
    );

    let stdout = fixture.job_complete_ok("session-one", 6, &[]);

    assert_eq!(
        stdout,
        "already complete: session-one is at line 6; stamped 0 leftover chunk(s)\n"
    );
}

#[test]
fn job_complete_without_a_chunk_or_session_fails() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));

    let stderr = fixture.job_complete_err("session-one", 6, &[]);

    assert_eq!(
        stderr,
        "error: no capture chunk for session-one and no session document sessions/session-one\n"
    );
}

#[test]
fn job_complete_honours_the_refs_extension() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write(".iwe/config.toml", "[markdown]\nrefs_extension = \".md\"\n");
    fixture.import("session-one", 6);
    fixture.write("alpha.md", "# Alpha Note\n\nA fact.\n");

    fixture.job_complete_ok("session-one", 2, &["alpha"]);

    assert!(fixture
        .read("sessions/session-one.md")
        .contains("[Alpha Note](../alpha.md)"));
}

#[test]
fn chunks_never_enter_the_store() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.write("alpha.md", "# Alpha Note\n\nA fact.\n");
    fixture.import("session-one", 6);
    fixture.job_complete_ok("session-one", 2, &["alpha"]);

    assert_eq!(
        fixture
            .tree()
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>(),
        vec![
            "MEMORY.md".to_string(),
            "alpha.md".to_string(),
            "sessions/session-one.md".to_string(),
            "state/session-one/000000.md".to_string(),
            "state/session-one/000002.md".to_string(),
            "state/session-one/000004.md".to_string(),
        ]
    );
    assert!(!fixture.store().join("sessions/session-one").exists());
}

#[test]
fn job_commands_outside_a_memory_store_fail_loudly() {
    let fixture = HookFixture::new(None);

    for output in [
        fixture.job_complete("session-one", 6, &[]),
        fixture.job(&["next"]),
    ] {
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "error: this directory is not a memory-enabled iwe workspace\n"
        );
    }
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

#[test]
fn stop_stamps_occurred_and_the_session_time_range() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript_with_timestamps("session-one", 6);

    assert_eq!(fixture.stop_utc(&[]), CAPTURE_BLOCK);

    assert!(fixture
        .chunk("session-one", 0)
        .contains("occurred: 2026-08-19 07:00"));
    assert!(fixture
        .chunk("session-one", 2)
        .contains("occurred: 2026-08-19 07:02"));

    let session = fixture.read("sessions/session-one.md");
    assert!(session.contains("started: 2026-08-19 07:00"), "{}", session);
    assert!(session.contains("ended: 2026-08-19 07:05"), "{}", session);
}

#[test]
fn job_next_prints_the_occurred_stamp() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript_with_timestamps("session-one", 6);
    fixture.stop_utc(&[]);

    let next = fixture.job_next();

    assert!(next.contains("\noccurred: 2026-08-19 07:00\n"), "{}", next);
}

#[test]
fn transcripts_without_timestamps_get_no_time_fields() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop_utc(&[]);

    assert!(!fixture.chunk("session-one", 0).contains("occurred:"));
    let session = fixture.read("sessions/session-one.md");
    assert!(!session.contains("started:"));
    assert!(!session.contains("ended:"));
    assert!(!fixture.job_next().contains("occurred:"));
}

#[test]
fn adopt_stamps_the_session_time_range() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript_with_timestamps("session-one", 7);

    fixture.stop_utc(&["--adopt"]);

    let session = fixture.read("sessions/session-one.md");
    assert!(session.contains("started: 2026-08-19 07:00"), "{}", session);
    assert!(session.contains("ended: 2026-08-19 07:06"), "{}", session);
}

#[test]
fn job_complete_titles_the_session_record_only_while_default() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.import("session-one", 6);

    fixture.job_complete_ok_with(
        "session-one",
        2,
        &[],
        &[
            "--title",
            "Cache warmup investigation",
            "--summary",
            "Traced the warmup order.",
        ],
    );

    let session = fixture.read("sessions/session-one.md");
    assert!(
        session.contains("# Cache warmup investigation"),
        "{}",
        session
    );
    assert!(session.contains("Traced the warmup order."), "{}", session);
    assert!(!session.contains("# Session session-one"));
    assert!(!session.contains("Agent session in this workspace."));

    fixture.job_complete_ok_with("session-one", 4, &[], &["--title", "A second opinion"]);

    let session = fixture.read("sessions/session-one.md");
    assert!(session.contains("# Cache warmup investigation"));
    assert!(!session.contains("A second opinion"));
}

#[test]
fn stop_reconciles_a_watermark_stranded_behind_a_captured_chunk() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    fixture.stop(&[], None);
    fixture.job_complete_ok("session-one", 2, &[]);
    fixture.job_complete_ok("session-one", 4, &[]);

    let session = fixture.read("sessions/session-one.md");
    fixture.write(
        "sessions/session-one.md",
        &session.replace("distilled_lines: 4", "distilled_lines: 2"),
    );
    assert_eq!(fixture.job_next(), "");

    fixture.stop(&[], None);

    assert!(fixture
        .read("sessions/session-one.md")
        .contains("distilled_lines: 4"));
    assert!(fixture.job_next().contains("covers_from: 4"));
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
        stdout.contains("capture chunks stay out of the graph, under"),
        "{}",
        stdout
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

#[test]
fn stop_does_not_double_spawn_over_fresh_imports_while_an_agent_works() {
    let fixture = HookFixture::new(Some(SWEEP_POLICY));
    fixture.transcript("session-one", 6);
    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);

    fixture.job_complete_ok("session-one", 2, &[]);
    fixture.transcript("session-two", 6);

    assert_eq!(fixture.stop(&[], None), "");
    assert!(!fixture.chunk_keys("session-two").is_empty());
    assert!(!fixture.chunk("session-two", 0).contains("claimed:"));

    for covers in [4, 6] {
        fixture.job_complete_ok("session-one", covers, &[]);
    }
    assert_eq!(fixture.stop(&[], None), CAPTURE_BLOCK);
    assert!(fixture.chunk("session-two", 0).contains("claimed:"));
}
