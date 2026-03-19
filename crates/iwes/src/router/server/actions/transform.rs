use std::io::Write;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

use liwe::markdown::MarkdownReader;
use liwe::model::config::Command;
use liwe::model::node::{NodeIter, NodePointer};
use liwe::operations::Changes;

use super::templates;
use super::{Action, ActionContext, ActionProvider};

pub struct TransformBlockAction {
    pub title: String,
    pub identifier: String,
    pub command: String,
    pub input_template: String,
}

fn expand_env_var(value: &str) -> String {
    let mut result = value.to_string();
    let mut i = 0;
    while i < result.len() {
        if result[i..].starts_with('$') {
            let rest = &result[i + 1..];
            let (var_name, end_offset) = if rest.starts_with('{') {
                if let Some(close) = rest.find('}') {
                    (&rest[1..close], close + 2)
                } else {
                    i += 1;
                    continue;
                }
            } else {
                let end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                if end == 0 {
                    i += 1;
                    continue;
                }
                (&rest[..end], end + 1)
            };
            let replacement = std::env::var(var_name).unwrap_or_default();
            result = format!("{}{}{}", &result[..i], replacement, &result[i + end_offset..]);
            i += replacement.len();
        } else {
            i += 1;
        }
    }
    result
}

fn execute_command(cmd: &Command, input: &str) -> Option<String> {
    let timeout = cmd.timeout_seconds.unwrap_or(120);
    let use_shell = cmd.shell.unwrap_or(true);

    let mut process = if use_shell {
        let mut p = ProcessCommand::new("sh");
        p.arg("-c").arg(&cmd.run);
        p
    } else {
        let mut p = ProcessCommand::new(&cmd.run);
        if let Some(args) = &cmd.args {
            p.args(args);
        }
        p
    };

    if let Some(cwd) = &cmd.cwd {
        process.current_dir(cwd);
    }

    if let Some(env) = &cmd.env {
        for (key, value) in env {
            let expanded = expand_env_var(value);
            process.env(key, expanded);
        }
    }

    process
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = process.spawn().ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).ok()?;
    }

    let output = wait_with_timeout(&mut child, Duration::from_secs(timeout))?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Option<std::process::Output> {
    use std::thread;
    use std::time::Instant;

    let start = Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child
                    .stdout
                    .take()
                    .map(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = Vec::new();
                        std::io::Read::read_to_end(&mut s, &mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                return Some(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return None;
                }
                thread::sleep(poll_interval);
            }
            Err(_) => return None,
        }
    }
}

impl ActionProvider for TransformBlockAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier)
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let _target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        Some(Action {
            title: self.title.clone(),
            identifier: self.identifier(),
            key: key.clone(),
            range: selection.clone(),
        })
    }

    fn changes(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Changes> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;

        let tree = &context.collect(&key);

        let target_id = tree
            .get_surrounding_top_level_block(target_id)
            .unwrap_or(target_id);

        let input = templates::render_input_template(&self.input_template, target_id, tree);

        let command = context.get_command(&self.command)?;

        if command.run.is_empty() {
            return None;
        }

        let generated = execute_command(&command, &input)?;

        let mut patch = context.patch();

        patch.from_markdown("new".into(), &generated, MarkdownReader::new());
        let tree = patch.maybe_key(&"new".into()).unwrap().collect_tree();

        let markdown = context
            .collect(&key)
            .replace(target_id, &tree)
            .iter()
            .to_markdown(&key.parent(), context.markdown_options());

        Some(Changes::new().update(key, markdown))
    }
}
