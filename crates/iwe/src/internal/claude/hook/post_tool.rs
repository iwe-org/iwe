use std::path::{Component, Path, PathBuf};

use diwe::config::Configuration;
use diwe::schema::{render_reports_text, validate_pending_documents};
use diwe::search_query::build_index;
use diwe::stats::{mutation_findings, Rule};
use liwe::model::Key;

use crate::internal::claude::hook::store::{enter_memory_store, library_path_of, HookPayload};
use crate::new::normalize_content;

const EDITOR_TOOLS: [&str; 3] = ["Write", "Edit", "MultiEdit"];
const WRITE_VERBS: [&str; 5] = ["create", "update", "rename", "delete", "attach"];
const CREATE_VERBS: [&str; 2] = ["create", "new"];
const VALUE_FLAGS: [&str; 2] = ["-v", "--verbose"];
const KNOWN_EXTENSIONS: [&str; 2] = ["md", "dj"];

pub struct PostToolReport {
    pub notice: String,
    pub context: String,
}

pub fn post_tool_report(payload: &HookPayload) -> Option<PostToolReport> {
    let tool = payload.text("tool_name")?;

    if EDITOR_TOOLS.contains(&tool.as_str()) {
        return editor_report(payload);
    }
    if tool == "Bash" {
        return command_report(payload);
    }
    None
}

fn editor_report(payload: &HookPayload) -> Option<PostToolReport> {
    let written = PathBuf::from(payload.nested("tool_input", "file_path")?);
    let extension = written.extension()?.to_str()?.to_string();
    if !KNOWN_EXTENSIONS.contains(&extension.as_str()) {
        return None;
    }

    let store = enter_memory_store(payload.text("cwd"))?;
    let config = store.config();
    if extension != config.format.extension() {
        return None;
    }

    let library = library_path_of(config);
    let key = key_within(&library, &written)?;
    if is_machinery(&key) {
        return None;
    }

    let path = document_path(config, &library, &key);
    let raw = std::fs::read_to_string(&path).ok()?;

    let mut notices = Vec::new();
    let mut context = Vec::new();

    let content = match normalized_in_place(config, &key, &path, &raw) {
        Some(content) => {
            notices.push(format!("iwe normalized {}", path.display()));
            context.push(format!(
                "`{}` was written outside the CLI, so it landed unnormalized. It has been rewritten into this store's canonical form — re-read it before editing it again, and prefer `iwe update -k {} --strict --content -`, which normalizes and validates on the way in.",
                key, key
            ));
            content
        }
        None => raw,
    };

    if let Some(report) = schema_report(config, &key, &content) {
        notices.push(format!("iwe: {} does not match its schema", key));
        context.push(report);
    }

    report_of(notices, context)
}

fn command_report(payload: &HookPayload) -> Option<PostToolReport> {
    let command = payload.nested("tool_input", "command")?;
    let unchecked = has_unchecked_write(&command);
    let created = has_create(&command);
    if !unchecked && !created {
        return None;
    }

    let store = enter_memory_store(payload.text("cwd"))?;
    let config = store.config();
    let library = library_path_of(config);

    let stdout = payload
        .nested("tool_response", "stdout")
        .unwrap_or_default();
    let keys: Vec<Key> = written_keys(&library, &stdout)
        .into_iter()
        .filter(|key| !is_machinery(key))
        .collect();

    let mut notices = Vec::new();
    let mut context = Vec::new();

    if unchecked {
        let failing: Vec<(&Key, String)> = keys
            .iter()
            .filter_map(|key| {
                let path = document_path(config, &library, key);
                let content = std::fs::read_to_string(&path).ok()?;
                schema_report(config, key, &content).map(|report| (key, report))
            })
            .collect();
        if !failing.is_empty() {
            notices.extend(
                failing
                    .iter()
                    .map(|(key, _)| format!("iwe: {} does not match its schema", key)),
            );
            context.extend(failing.into_iter().map(|(_, report)| report));
            context.push(
                "That write ran without `--strict`, which would have refused it before anything landed. Fix the document and put the flag on every write.".to_string(),
            );
        }
    }

    if created && !keys.is_empty() {
        let index = build_index(store.graph(), config.search_language());
        for finding in mutation_findings(store.graph(), &index, &keys) {
            let (Rule::SimilarPage, Some(other)) = (&finding.rule, &finding.other) else {
                continue;
            };
            notices.push(format!("iwe: {}", finding.render()));
            context.push(format!(
                "`{}` closely matches `{}`, a document this store already has. Read it with `iwe retrieve -k {}`; if the two state the same fact, keep the older key and merge the way the policy's dedup section says, then `iwe delete <the newer key> --expect 1`. A related but different fact stays as it is.",
                finding.key, other, other
            ));
        }
    }

    report_of(notices, context)
}

fn report_of(notices: Vec<String>, context: Vec<String>) -> Option<PostToolReport> {
    if context.is_empty() {
        return None;
    }
    Some(PostToolReport {
        notice: notices.join("; "),
        context: context.join("\n\n"),
    })
}

fn normalized_in_place(
    config: &Configuration,
    key: &Key,
    path: &Path,
    raw: &str,
) -> Option<String> {
    let normalized = normalize_content(config, key, raw);
    if normalized == raw {
        return None;
    }
    std::fs::write(path, &normalized).ok()?;
    Some(normalized)
}

fn schema_report(config: &Configuration, key: &Key, content: &str) -> Option<String> {
    let run = validate_pending_documents(config, &[(key.clone(), content.to_string())]).ok()?;
    if run.reports.is_empty() {
        return None;
    }
    Some(format!(
        "`{}` does not match the schema this store binds to it:\n{}",
        key,
        render_reports_text(&run.reports).trim_end()
    ))
}

fn document_path(config: &Configuration, library: &Path, key: &Key) -> PathBuf {
    library.join(format!("{}.{}", key, config.format.extension()))
}

fn is_machinery(key: &Key) -> bool {
    key.to_string() == "MEMORY"
}

fn key_within(library: &Path, path: &Path) -> Option<Key> {
    let library = library.canonicalize().ok()?;
    let path = path.canonicalize().ok()?;
    let relative = path.strip_prefix(&library).ok()?;

    let name = relative
        .with_extension("")
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");

    (!name.is_empty()).then(|| Key::name(&name))
}

fn has_unchecked_write(command: &str) -> bool {
    command
        .split(['\n', ';', '|', '&'])
        .any(|segment| is_iwe_verb(segment, &WRITE_VERBS) && !segment.contains("--strict"))
}

fn has_create(command: &str) -> bool {
    command
        .split(['\n', ';', '|', '&'])
        .any(|segment| is_iwe_verb(segment, &CREATE_VERBS) && !segment.contains("--dry-run"))
}

fn is_iwe_verb(segment: &str, verbs: &[&str]) -> bool {
    let tokens: Vec<&str> = segment.split_whitespace().collect();

    tokens.iter().enumerate().any(|(at, token)| {
        let name = token.rsplit('/').next().unwrap_or(token);
        name == "iwe" && verb_of(&tokens[at + 1..]).is_some_and(|verb| verbs.contains(&verb))
    })
}

fn verb_of<'a>(tokens: &[&'a str]) -> Option<&'a str> {
    let mut rest = tokens.iter();
    while let Some(token) = rest.next() {
        if VALUE_FLAGS.contains(token) {
            rest.next();
            continue;
        }
        if token.starts_with('-') {
            continue;
        }
        return Some(token);
    }
    None
}

fn written_keys(library: &Path, stdout: &str) -> Vec<Key> {
    let mut keys: Vec<Key> = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        let found = match line.strip_prefix("Updated '").and_then(|rest| {
            rest.strip_suffix('\'')
                .filter(|name| !name.is_empty())
                .map(Key::name)
        }) {
            Some(key) => Some(key),
            None => key_within(library, Path::new(line)),
        };

        if let Some(key) = found {
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
    }

    keys
}
