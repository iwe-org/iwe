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
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            result.push(c);
            continue;
        }
        match chars.peek() {
            Some('{') => {
                chars.next();
                let mut name = String::new();
                let mut closed = false;
                for nc in chars.by_ref() {
                    if nc == '}' {
                        closed = true;
                        break;
                    }
                    name.push(nc);
                }
                if closed {
                    result.push_str(&std::env::var(&name).unwrap_or_default());
                } else {
                    result.push('$');
                    result.push('{');
                    result.push_str(&name);
                }
            }
            Some(nc) if nc.is_alphanumeric() || *nc == '_' => {
                let mut name = String::new();
                while let Some(nc) = chars.peek() {
                    if nc.is_alphanumeric() || *nc == '_' {
                        name.push(*nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                result.push_str(&std::env::var(&name).unwrap_or_default());
            }
            _ => result.push('$'),
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
        let _ = stdin.write_all(input.as_bytes());
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

        let generated = execute_command(command, &input)?;

        let mut patch = context.patch();

        patch.from_markdown("new".into(), &generated, MarkdownReader::new());
        let tree = patch.maybe_key(&"new".into()).unwrap().collect_tree();

        let markdown = context
            .collect(&key)
            .replace(target_id, &tree)
            .iter()
            .to_text(&key.parent(), &context.format_options());

        Some(Changes::new().update(key, markdown))
    }
}

#[cfg(test)]
mod tests {
    use super::expand_env_var;

    #[test]
    fn multibyte_value_with_env_var_does_not_panic() {
        std::env::set_var("IWE_TEST_EXPAND_VALUE", "replaced");
        assert_eq!(
            expand_env_var("héllo $IWE_TEST_EXPAND_VALUE"),
            "héllo replaced"
        );
        assert_eq!(
            expand_env_var("${IWE_TEST_EXPAND_VALUE} café"),
            "replaced café"
        );
        assert_eq!(expand_env_var("café $missing_var €"), "café  €");
    }

    #[test]
    fn lone_dollar_and_unclosed_brace_are_kept() {
        assert_eq!(expand_env_var("price $ 5"), "price $ 5");
        assert_eq!(expand_env_var("trailing $"), "trailing $");
        assert_eq!(expand_env_var("open ${brace"), "open ${brace");
    }
}
