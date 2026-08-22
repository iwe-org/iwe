use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Result};
use std::path::Path;

use serde_json::Value;

const TOOL_INPUT_FIELDS: [&str; 8] = [
    "command",
    "description",
    "file_path",
    "path",
    "pattern",
    "query",
    "prompt",
    "url",
];

const SWEEP_FEEDBACK_MARKER: &str = "Stop hook feedback:";
const SWEEP_AGENT_TOOL: &str = "Agent";
const SWEEP_AGENT_SUFFIX: &str = "distill";
const SWEEP_NOTIFICATION_KIND: &str = "task-notification";
const QUEUED_COMMAND_KIND: &str = "queued_command";
const HUMAN_ORIGIN_KIND: &str = "human";

const TOOL_RESULT_LIMIT: usize = 240;
const TOOL_ERROR_LIMIT: usize = 600;
const TOOL_INPUT_LIMIT: usize = 200;
const UNKNOWN_BLOCK_LIMIT: usize = 200;

pub struct Digest {
    pub covered: usize,
    pub text: String,
}

pub struct Chunk {
    pub from: usize,
    pub covered: usize,
    pub text: String,
    pub occurred: Option<String>,
}

pub fn digest_claude_chunks(
    path: &Path,
    from: usize,
    chunk_chars: usize,
    max_chunks: usize,
) -> Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    if max_chunks == 0 {
        return Ok(chunks);
    }

    let mut reader = BufReader::new(File::open(path)?);
    let mut state = DigestState::new(from, chunk_chars);
    let mut span = SweepSpan::new();
    let mut buffer = Vec::new();
    let mut skipped = 0;
    let mut index = from;

    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        if skipped < from {
            skipped += 1;
            continue;
        }
        let line = String::from_utf8_lossy(strip_line_ending(&buffer));
        let value: Option<Value> = serde_json::from_str(&line).ok();
        let rendered = value
            .as_ref()
            .filter(|value| !span.swallows(value))
            .and_then(render_claude_value);
        state.absorb(
            index,
            rendered,
            value.as_ref().and_then(value_occurred),
            &mut chunks,
        );
        if chunks.len() >= max_chunks {
            chunks.truncate(max_chunks);
            return Ok(chunks);
        }
        index += 1;
    }

    if let Some(last) = state.flush() {
        chunks.push(last);
    }
    Ok(chunks)
}

pub fn count_user_turns(path: &Path, from: usize) -> Result<usize> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut span = SweepSpan::new();
    let mut buffer = Vec::new();
    let mut skipped = 0;
    let mut turns = 0;

    loop {
        buffer.clear();
        if reader.read_until(b'\n', &mut buffer)? == 0 {
            break;
        }
        if skipped < from {
            skipped += 1;
            continue;
        }
        let line = String::from_utf8_lossy(strip_line_ending(&buffer));
        if serde_json::from_str::<Value>(&line)
            .ok()
            .is_some_and(|value| !span.swallows(&value) && is_user_turn(&value))
        {
            turns += 1;
        }
    }

    Ok(turns)
}

struct SweepSpan {
    open: bool,
    launches: HashSet<String>,
}

impl SweepSpan {
    fn new() -> Self {
        SweepSpan {
            open: false,
            launches: HashSet::new(),
        }
    }

    fn swallows(&mut self, value: &Value) -> bool {
        if is_sweep_feedback(value) {
            self.open = true;
            return true;
        }
        if !self.open || !is_conversation_line(value) {
            return false;
        }
        if let Some(id) = sweep_launch_id(value) {
            self.launches.insert(id);
            return true;
        }
        if is_sweep_launch_result(value, &self.launches)
            || is_sweep_notification(value)
            || is_assistant_commentary(value)
        {
            return true;
        }
        self.open = false;
        false
    }
}

fn is_conversation_line(value: &Value) -> bool {
    matches!(
        value.get("type").and_then(Value::as_str),
        Some("user") | Some("assistant")
    )
}

fn is_sweep_feedback(value: &Value) -> bool {
    if value.get("type").and_then(Value::as_str) != Some("user") {
        return false;
    }
    if value.get("isMeta") != Some(&Value::Bool(true)) {
        return false;
    }
    plain_text(&value["message"]["content"])
        .is_some_and(|text| text.trim_start().starts_with(SWEEP_FEEDBACK_MARKER))
}

fn is_sweep_notification(value: &Value) -> bool {
    value.get("type").and_then(Value::as_str) == Some("user")
        && value
            .get("origin")
            .and_then(|origin| origin.get("kind"))
            .and_then(Value::as_str)
            == Some(SWEEP_NOTIFICATION_KIND)
}

fn sweep_launch_id(value: &Value) -> Option<String> {
    if value.get("type").and_then(Value::as_str) != Some("assistant") {
        return None;
    }
    content_elements(value)?.iter().find_map(|element| {
        if element.get("type").and_then(Value::as_str) != Some("tool_use") {
            return None;
        }
        if element.get("name").and_then(Value::as_str) != Some(SWEEP_AGENT_TOOL) {
            return None;
        }
        let agent = element["input"]["subagent_type"].as_str()?;
        if agent.rsplit(':').next() != Some(SWEEP_AGENT_SUFFIX) {
            return None;
        }
        element
            .get("id")
            .and_then(Value::as_str)
            .map(|id| id.to_string())
    })
}

fn is_sweep_launch_result(value: &Value, launches: &HashSet<String>) -> bool {
    if value.get("type").and_then(Value::as_str) != Some("user") {
        return false;
    }
    let Some(elements) = content_elements(value) else {
        return false;
    };
    !elements.is_empty()
        && elements.iter().all(|element| {
            element.get("type").and_then(Value::as_str) == Some("tool_result")
                && element
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .is_some_and(|id| launches.contains(id))
        })
}

fn is_assistant_commentary(value: &Value) -> bool {
    if value.get("type").and_then(Value::as_str) != Some("assistant") {
        return false;
    }
    let Some(elements) = content_elements(value) else {
        return false;
    };
    elements.iter().all(|element| {
        matches!(
            element.get("type").and_then(Value::as_str),
            Some("text") | Some("thinking")
        )
    })
}

fn content_elements(value: &Value) -> Option<&Vec<Value>> {
    value["message"]["content"].as_array()
}

fn plain_text(content: &Value) -> Option<&str> {
    match content {
        Value::String(text) => Some(text),
        Value::Array(elements) => elements
            .iter()
            .find(|element| element.get("type").and_then(Value::as_str) == Some("text"))
            .and_then(|element| element.get("text").and_then(Value::as_str)),
        _ => None,
    }
}

fn is_user_turn(value: &Value) -> bool {
    match value.get("type").and_then(Value::as_str) {
        Some("user") => {
            if value.get("isMeta") == Some(&Value::Bool(true)) {
                return false;
            }
            user_parts(value)
                .iter()
                .any(|part| part.starts_with("[user]"))
        }
        Some("attachment") => !queued_command_parts(value).is_empty(),
        _ => false,
    }
}

fn queued_command_parts(line: &Value) -> Vec<String> {
    let attachment = &line["attachment"];
    if attachment.get("type").and_then(Value::as_str) != Some(QUEUED_COMMAND_KIND) {
        return Vec::new();
    }
    if attachment
        .get("origin")
        .and_then(|origin| origin.get("kind"))
        .and_then(Value::as_str)
        != Some(HUMAN_ORIGIN_KIND)
    {
        return Vec::new();
    }
    let trimmed = attachment
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if trimmed.is_empty() {
        Vec::new()
    } else {
        vec![format!("[user]\n{}", trimmed)]
    }
}

pub fn digest_claude_transcript(path: &Path, from: usize, max_chars: usize) -> Result<Digest> {
    let chunks = digest_claude_chunks(path, from, max_chars, 1)?;
    Ok(match chunks.into_iter().next() {
        Some(chunk) => Digest {
            covered: chunk.covered - from,
            text: chunk.text,
        },
        None => Digest {
            covered: 0,
            text: String::new(),
        },
    })
}

fn strip_line_ending(buffer: &[u8]) -> &[u8] {
    let without_newline = buffer.strip_suffix(b"\n").unwrap_or(buffer);
    without_newline
        .strip_suffix(b"\r")
        .unwrap_or(without_newline)
}

struct DigestState {
    from: usize,
    text: String,
    length: usize,
    covered: usize,
    occurred: Option<String>,
    max_chars: usize,
}

impl DigestState {
    fn new(from: usize, max_chars: usize) -> Self {
        DigestState {
            from,
            text: String::new(),
            length: 0,
            covered: from,
            occurred: None,
            max_chars,
        }
    }

    fn absorb(
        &mut self,
        index: usize,
        rendered: Option<String>,
        occurred: Option<String>,
        out: &mut Vec<Chunk>,
    ) {
        let rendered = match rendered {
            Some(rendered) => rendered,
            None => {
                if self.occurred.is_none() {
                    self.occurred = occurred;
                }
                self.covered = index + 1;
                return;
            }
        };

        let length = rendered.chars().count();
        if !self.text.is_empty() && self.length + 2 + length > self.max_chars {
            out.push(self.take(index));
        }
        if self.occurred.is_none() {
            self.occurred = occurred;
        }

        if self.text.is_empty() {
            if length > self.max_chars {
                self.text = format!(
                    "{}\n[truncated at {} chars; the line rendered {}]",
                    take_chars(&rendered, self.max_chars),
                    self.max_chars,
                    length
                );
                self.covered = index + 1;
                out.push(self.take(index + 1));
                return;
            }
            self.text = rendered;
            self.length = length;
        } else {
            self.text.push_str("\n\n");
            self.text.push_str(&rendered);
            self.length += 2 + length;
        }
        self.covered = index + 1;
    }

    fn take(&mut self, next_from: usize) -> Chunk {
        let chunk = Chunk {
            from: self.from,
            covered: self.covered,
            text: std::mem::take(&mut self.text),
            occurred: self.occurred.take(),
        };
        self.from = next_from;
        self.covered = next_from;
        self.length = 0;
        chunk
    }

    fn flush(&mut self) -> Option<Chunk> {
        let covered = self.covered;
        (covered > self.from).then(|| self.take(covered))
    }
}

fn take_chars(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

pub fn value_occurred(value: &Value) -> Option<String> {
    let stamp = value.get("timestamp").and_then(Value::as_str)?;
    chrono::DateTime::parse_from_rfc3339(stamp)
        .ok()
        .map(|parsed| {
            parsed
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
}

fn render_claude_value(value: &Value) -> Option<String> {
    let parts = match value.get("type").and_then(Value::as_str)? {
        "user" => {
            if value.get("isMeta") == Some(&Value::Bool(true)) {
                return None;
            }
            user_parts(value)
        }
        "assistant" => assistant_parts(value),
        "attachment" => queued_command_parts(value),
        _ => return None,
    };

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

fn user_parts(line: &Value) -> Vec<String> {
    match &line["message"]["content"] {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![format!("[user]\n{}", trimmed)]
            }
        }
        Value::Array(elements) => elements.iter().filter_map(user_content_part).collect(),
        _ => Vec::new(),
    }
}

fn user_content_part(element: &Value) -> Option<String> {
    match element.get("type").and_then(Value::as_str) {
        Some("text") => text_part(element, "[user]"),
        Some("tool_result") => {
            let text = tool_result_text(element);
            if element.get("is_error") == Some(&Value::Bool(true)) {
                Some(format!("[tool error] {}", excerpt(&text, TOOL_ERROR_LIMIT)))
            } else {
                Some(format!(
                    "[tool result] {}",
                    excerpt(&text, TOOL_RESULT_LIMIT)
                ))
            }
        }
        _ => Some(unknown_block(element)),
    }
}

fn assistant_parts(line: &Value) -> Vec<String> {
    match &line["message"]["content"] {
        Value::Array(elements) => elements.iter().filter_map(assistant_content_part).collect(),
        _ => Vec::new(),
    }
}

fn assistant_content_part(element: &Value) -> Option<String> {
    match element.get("type").and_then(Value::as_str) {
        Some("text") => text_part(element, "[assistant]"),
        Some("thinking") => thinking_part(element),
        Some("tool_use") => {
            let name = element
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let head = format!("[tool: {}]", name);
            let summary = tool_input_summary(element.get("input").unwrap_or(&Value::Null));
            if summary.is_empty() {
                Some(head)
            } else {
                Some(format!("{} {}", head, summary))
            }
        }
        _ => Some(unknown_block(element)),
    }
}

fn thinking_part(element: &Value) -> Option<String> {
    let trimmed = element
        .get("thinking")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("[thinking]\n{}", trimmed))
    }
}

fn text_part(element: &Value, label: &str) -> Option<String> {
    let trimmed = element
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("{}\n{}", label, trimmed))
    }
}

fn tool_result_text(element: &Value) -> String {
    match element.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn tool_input_summary(input: &Value) -> String {
    let object = match input {
        Value::Object(object) => object,
        _ => return String::new(),
    };

    for field in TOOL_INPUT_FIELDS {
        if let Some(text) = object.get(field).and_then(Value::as_str) {
            if text.chars().any(|character| !character.is_whitespace()) {
                return excerpt(text, TOOL_INPUT_LIMIT);
            }
        }
    }

    let serialized = serde_json::to_string(input).unwrap_or_default();
    if serialized == "{}" || serialized == "null" {
        String::new()
    } else {
        excerpt(&serialized, TOOL_INPUT_LIMIT)
    }
}

fn unknown_block(element: &Value) -> String {
    let kind = element
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let head = format!("[block: {}]", kind);

    let mut leaves = Vec::new();
    match element {
        Value::Object(entries) => entries
            .iter()
            .filter(|(name, _)| name.as_str() != "type")
            .for_each(|(_, value)| collect_string_leaves(value, &mut leaves)),
        other => collect_string_leaves(other, &mut leaves),
    }
    let body = excerpt(&leaves.join(" "), UNKNOWN_BLOCK_LIMIT);

    if body.is_empty() {
        head
    } else {
        format!("{} {}", head, body)
    }
}

fn collect_string_leaves<'a>(value: &'a Value, leaves: &mut Vec<&'a str>) {
    match value {
        Value::String(text) => leaves.push(text),
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_string_leaves(item, leaves)),
        Value::Object(entries) => entries
            .values()
            .for_each(|entry| collect_string_leaves(entry, leaves)),
        _ => {}
    }
}

fn excerpt(text: &str, limit: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let length = flat.chars().count();
    if length > limit {
        format!("{}… (+{} chars)", take_chars(&flat, limit), length - limit)
    } else {
        flat
    }
}
