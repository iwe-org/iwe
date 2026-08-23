use std::collections::HashMap;
use std::env::set_current_dir;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use chrono::{Duration, Local};
use diwe::config::{load_config, Configuration, IWE_MARKER};
use diwe::graph_from_path;
use liwe::graph::{Graph, GraphContext};
use liwe::model::{frontmatter_from_str, split_raw_frontmatter, Key};
use liwe::query::frontmatter::strip_reserved;
use liwe::query::{parse_filter_expression, Filter};
use log::debug;
use serde_json::Value as JsonValue;
use serde_yaml::{Mapping, Value as YamlValue};

pub struct HookPayload {
    fields: JsonValue,
}

pub fn read_hook_payload() -> HookPayload {
    if std::io::stdin().is_terminal() {
        return HookPayload {
            fields: JsonValue::Null,
        };
    }

    let mut buffer = String::new();
    if std::io::stdin().read_to_string(&mut buffer).is_err() {
        return HookPayload {
            fields: JsonValue::Null,
        };
    }

    HookPayload {
        fields: serde_json::from_str(&buffer).unwrap_or(JsonValue::Null),
    }
}

impl HookPayload {
    pub fn text(&self, name: &str) -> Option<String> {
        match self.fields.get(name) {
            Some(JsonValue::String(text)) if !text.is_empty() => Some(text.clone()),
            _ => None,
        }
    }

    pub fn is_true(&self, name: &str) -> bool {
        match self.fields.get(name) {
            Some(JsonValue::Bool(true)) => true,
            Some(JsonValue::String(text)) => text == "true",
            _ => false,
        }
    }
}

pub const SESSIONS_PREFIX: &str = "sessions";

pub const STATE_VARIABLE: &str = "IWE_MEMORY_STATE";

pub const STATE_DIRECTORY: &str = "claude-sessions";

const STATE_GITIGNORE: &str = "*\n";

const CACHEDIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by iwe.\n\
# For information about cache directory tags, see:\n\
#\thttps://bford.info/cachedir/\n";

const CHUNK_EXTENSION: &str = "md";

pub const DEFAULT_INFLIGHT_TTL_MINUTES: usize = 60;

pub fn inflight_cutoff(ttl: usize) -> String {
    (Local::now() - Duration::minutes(ttl as i64))
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub struct MemoryStore {
    config: Configuration,
    graph: Graph,
    knobs: Mapping,
    state: PathBuf,
}

pub struct ChunkRecord {
    pub session: String,
    pub path: PathBuf,
    pub from: usize,
    pub covers: usize,
    pub max_items: usize,
    pub created: String,
    pub occurred: Option<String>,
    pub claimed: Option<String>,
    pub skipped: Option<String>,
    pub captured_at: Option<String>,
}

impl ChunkRecord {
    pub fn is_pending(&self) -> bool {
        self.captured_at.is_none()
    }

    pub fn is_claimed(&self, cutoff: &str) -> bool {
        matches!(&self.claimed, Some(claimed) if claimed.as_str() >= cutoff)
    }

    pub fn is_skipped(&self, cutoff: &str) -> bool {
        matches!(&self.skipped, Some(skipped) if skipped.as_str() >= cutoff)
    }
}

pub fn enter_memory_store(cwd: Option<String>) -> Option<MemoryStore> {
    if let Some(cwd) = cwd {
        let path = PathBuf::from(&cwd);
        if !path.is_absolute() || !path.is_dir() {
            return None;
        }
        set_current_dir(&path).ok()?;
    }

    let config = match load_config() {
        Ok(config) => config,
        Err(error) => {
            debug!(
                "memory hooks disabled, configuration failed to load: {}",
                error
            );
            return None;
        }
    };
    let graph = load_store_graph(&config);
    let memory_key = Key::name("MEMORY");
    (&graph).get_node_id(&memory_key)?;
    let knobs = graph.frontmatter(&memory_key).cloned().unwrap_or_default();
    let state = state_directory(&workspace_root());

    Some(MemoryStore {
        config,
        graph,
        knobs,
        state,
    })
}

pub fn state_directory(root: &Path) -> PathBuf {
    if let Some(explicit) = non_empty_var(STATE_VARIABLE) {
        return PathBuf::from(explicit);
    }

    root.join(IWE_MARKER).join(STATE_DIRECTORY)
}

pub fn workspace_root() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

pub fn ensure_state_directory(state: &Path) -> Result<(), String> {
    let cannot = |error: std::io::Error| {
        format!(
            "cannot write capture chunks under {}: {}",
            state.display(),
            error
        )
    };
    if state.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(state).map_err(cannot)?;
    std::fs::write(state.join(".gitignore"), STATE_GITIGNORE).map_err(cannot)?;
    std::fs::write(state.join("CACHEDIR.TAG"), CACHEDIR_TAG).map_err(cannot)
}

pub fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

pub fn project_slug(project: &str) -> String {
    project
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn load_store_graph(config: &Configuration) -> Graph {
    graph_from_path(
        &library_path_of(config),
        false,
        config.format_options(),
        config.library.frontmatter_document_title.clone(),
    )
}

pub fn library_path_of(config: &Configuration) -> PathBuf {
    let mut path = std::env::current_dir().unwrap_or_default();
    if !config.library.path.is_empty() {
        path.push(config.library.path.clone());
    }
    path
}

fn single_line(value: Option<&str>) -> Option<String> {
    let line = value?.lines().next().unwrap_or("").trim();
    (!line.is_empty()).then(|| line.to_string())
}

fn replace_whole_line(content: &str, from: &str, to: &str) -> String {
    let needle = format!("\n{}\n", from);
    if content.contains(&needle) {
        content.replacen(&needle, &format!("\n{}\n", to), 1)
    } else {
        content.to_string()
    }
}

fn order_chunks(records: &mut [ChunkRecord]) {
    let mut recency: HashMap<String, String> = HashMap::new();
    for record in records.iter() {
        let stamp = record
            .occurred
            .clone()
            .unwrap_or_else(|| record.created.clone());
        recency
            .entry(record.session.clone())
            .and_modify(|latest| {
                if stamp > *latest {
                    latest.clone_from(&stamp);
                }
            })
            .or_insert(stamp);
    }

    records.sort_by(|left, right| {
        recency
            .get(&right.session)
            .cmp(&recency.get(&left.session))
            .then_with(|| left.session.cmp(&right.session))
            .then_with(|| left.from.cmp(&right.from))
    });
}

pub fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn frontmatter_body(block: &str) -> &str {
    let inner = block
        .strip_prefix("---\r\n")
        .or_else(|| block.strip_prefix("---\n"))
        .unwrap_or(block);
    inner
        .strip_suffix("---\r\n")
        .or_else(|| inner.strip_suffix("---\n"))
        .or_else(|| inner.strip_suffix("---"))
        .unwrap_or(inner)
}

pub fn field_text(fields: &Mapping, name: &str) -> Option<String> {
    fields
        .get(YamlValue::String(name.to_string()))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .filter(|text| !text.is_empty())
}

pub fn field_int(fields: &Mapping, name: &str) -> Option<usize> {
    fields
        .get(YamlValue::String(name.to_string()))
        .and_then(|value| value.as_u64())
        .map(|value| value as usize)
}

fn scalar_text(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::Null => None,
        YamlValue::Bool(false) => None,
        YamlValue::Bool(true) => Some("true".to_string()),
        YamlValue::String(text) => Some(text.clone()),
        YamlValue::Number(number) => Some(number.to_string()),
        other => serde_json::to_string(other).ok(),
    }
}

fn knob_count(text: &str) -> Option<usize> {
    if text.is_empty() || !text.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn knob_env(name: &str) -> Option<String> {
    std::env::var(format!("IWE_{}", name.to_uppercase())).ok()
}

fn fields_at(path: &Path) -> Option<Mapping> {
    let raw = std::fs::read_to_string(path).ok()?;
    let (front, _) = split_raw_frontmatter(&raw);
    Some(
        front
            .and_then(|block| frontmatter_from_str(frontmatter_body(block)))
            .unwrap_or_default(),
    )
}

fn body_at(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let (_, body) = split_raw_frontmatter(&raw);
    Some(body.to_string())
}

fn write_raw_at(path: &Path, content: &str) -> bool {
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    std::fs::write(path, content).is_ok()
}

fn rewrite_at(path: &Path, fields: &[(&str, YamlValue)], append: Option<&str>) -> bool {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return false,
    };
    let (front, body) = split_raw_frontmatter(&raw);

    let mut mapping = front
        .and_then(|block| frontmatter_from_str(frontmatter_body(block)))
        .unwrap_or_default();
    for (name, value) in fields {
        mapping.insert(YamlValue::String(name.to_string()), value.clone());
    }
    strip_reserved(&mut mapping);

    let front = if mapping.is_empty() {
        String::new()
    } else {
        match serde_yaml::to_string(&mapping) {
            Ok(serialized) => format!("---\n{}---\n", serialized),
            Err(_) => return false,
        }
    };

    let body = match append {
        Some(paragraph) => {
            let settled = body.trim_end_matches('\n');
            if settled.is_empty() {
                format!("{}\n", paragraph)
            } else {
                format!("{}\n\n{}\n", settled, paragraph)
            }
        }
        None => body.to_string(),
    };

    let content = format!("{}{}", front, body);
    if content == raw {
        return true;
    }
    write_raw_at(path, &content)
}

fn move_file(from: &Path, to: &Path) -> bool {
    if let Some(parent) = to.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    if std::fs::rename(from, to).is_ok() {
        return true;
    }
    std::fs::copy(from, to).is_ok() && std::fs::remove_file(from).is_ok()
}

fn session_directories(root: &Path) -> Vec<(String, PathBuf)> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().to_string();
            is_safe_id(&id).then(|| (id, entry.path()))
        })
        .collect()
}

fn files_with_extension(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|found| found.to_str()) == Some(extension)
        })
        .collect()
}

impl MemoryStore {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn state_directory(&self) -> &Path {
        &self.state
    }

    pub fn knob(&self, name: &str) -> Option<String> {
        let text = self.knobs.get(YamlValue::String(name.to_string()))?;
        let text = scalar_text(text)?;
        if text.is_empty() || text == "null" {
            return None;
        }
        Some(text)
    }

    pub fn knob_int(&self, name: &str, default: usize) -> usize {
        self.knob(name)
            .and_then(|text| knob_count(&text))
            .or_else(|| knob_env(name).and_then(|text| knob_count(&text)))
            .unwrap_or(default)
    }

    pub fn session_key(&self, id: &str) -> String {
        format!("{}/{}", SESSIONS_PREFIX, id)
    }

    pub fn chunk_path(&self, id: &str, from: usize) -> PathBuf {
        self.state
            .join(id)
            .join(format!("{:06}.{}", from, CHUNK_EXTENSION))
    }

    pub fn has_document(&self, key: &str) -> bool {
        (&self.graph).get_node_id(&Key::name(key)).is_some()
    }

    pub fn document_path(&self, key: &str) -> PathBuf {
        library_path_of(&self.config).join(format!(
            "{}.{}",
            Key::name(key),
            self.config.format.extension()
        ))
    }

    pub fn document_exists(&self, key: &str) -> bool {
        self.document_path(key).is_file()
    }

    pub fn fields_of(&self, key: &str) -> Option<Mapping> {
        fields_at(&self.document_path(key))
    }

    pub fn body_of(&self, key: &str) -> Option<String> {
        body_at(&self.document_path(key))
    }

    pub fn chunk_fields(&self, path: &Path) -> Option<Mapping> {
        fields_at(path)
    }

    pub fn chunk_body(&self, path: &Path) -> Option<String> {
        body_at(path)
    }

    pub fn write_chunk(&self, path: &Path, content: &str) -> bool {
        write_raw_at(path, content)
    }

    pub fn remove_chunk(&self, path: &Path) -> bool {
        std::fs::remove_file(path).is_ok()
    }

    pub fn stamp_chunk(&self, path: &Path, fields: &[(&str, YamlValue)]) -> bool {
        rewrite_at(path, fields, None)
    }

    pub fn claim_cutoff(&self) -> String {
        inflight_cutoff(self.knob_int("inflight_ttl_minutes", DEFAULT_INFLIGHT_TTL_MINUTES))
    }

    pub fn session_watermark(&self, id: &str) -> usize {
        self.fields_of(&self.session_key(id))
            .and_then(|fields| field_int(&fields, "distilled_lines"))
            .unwrap_or(0)
    }

    pub fn chunks(&self) -> Vec<ChunkRecord> {
        let mut records = Vec::new();
        for (id, directory) in session_directories(&self.state) {
            for path in files_with_extension(&directory, CHUNK_EXTENSION) {
                if let Some(record) = self.chunk_record(&id, &path) {
                    records.push(record);
                }
            }
        }

        order_chunks(&mut records);
        records
    }

    fn chunk_record(&self, session: &str, path: &Path) -> Option<ChunkRecord> {
        let fields = fields_at(path)?;
        Some(ChunkRecord {
            session: session.to_string(),
            from: field_int(&fields, "covers_from")?,
            covers: field_int(&fields, "covers_lines")?,
            max_items: field_int(&fields, "max_items").unwrap_or(0),
            created: field_text(&fields, "created").unwrap_or_default(),
            occurred: field_text(&fields, "occurred"),
            claimed: field_text(&fields, "claimed"),
            skipped: field_text(&fields, "skipped"),
            captured_at: field_text(&fields, "captured_at"),
            path: path.to_path_buf(),
        })
    }

    pub fn relocate_store_chunks(&self) -> usize {
        let root = library_path_of(&self.config).join(SESSIONS_PREFIX);
        let mut moved = 0;
        for (id, directory) in session_directories(&root) {
            for path in files_with_extension(&directory, self.config.format.extension()) {
                let Some(fields) = fields_at(&path) else {
                    continue;
                };
                let is_chunk = field_text(&fields, "session").as_deref() == Some(id.as_str())
                    && field_int(&fields, "covers_lines").is_some();
                let Some(from) = field_int(&fields, "covers_from").filter(|_| is_chunk) else {
                    continue;
                };
                let target = self.chunk_path(&id, from);
                let done = if target.is_file() {
                    std::fs::remove_file(&path).is_ok()
                } else {
                    move_file(&path, &target)
                };
                if done {
                    moved += 1;
                }
            }
            std::fs::remove_dir(&directory).ok();
        }
        moved
    }

    pub fn filter_of(&self, expression: &str) -> Option<Filter> {
        parse_filter_expression(expression).ok()
    }

    pub fn set_fields(&self, key: &str, fields: &[(&str, YamlValue)]) -> bool {
        rewrite_at(&self.document_path(key), fields, None)
    }

    pub fn set_fields_and_append(
        &self,
        key: &str,
        fields: &[(&str, YamlValue)],
        paragraph: &str,
    ) -> bool {
        rewrite_at(&self.document_path(key), fields, Some(paragraph))
    }

    pub fn retitle_session(
        &self,
        session: &str,
        title: Option<&str>,
        summary: Option<&str>,
    ) -> bool {
        let key = self.session_key(session);
        let path = self.document_path(&key);
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => return false,
        };

        let mut content = raw.clone();
        if let Some(title) = single_line(title) {
            content = replace_whole_line(
                &content,
                &format!("# Session {}", session),
                &format!("# {}", title),
            );
        }
        if let Some(summary) = single_line(summary) {
            content = replace_whole_line(&content, "Agent session in this workspace.", &summary);
        }

        if content == raw {
            return true;
        }
        std::fs::write(&path, &content).is_ok()
    }

    pub fn ensure_session_document(&self, session: &str, now: &str) -> bool {
        let key = self.session_key(session);
        if self.document_exists(&key) {
            return true;
        }

        let content = format!(
            "---\nsession: \"{}\"\ncreated: \"{}\"\ndistilled_lines: 0\n---\n\n# Session {}\n\nAgent session in this workspace.\n",
            session, now, session
        );
        write_raw_at(&self.document_path(&key), &content)
    }
}
