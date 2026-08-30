use std::env::set_current_dir;
use std::io::{IsTerminal, Read};
use std::path::{Path, PathBuf};

use diwe::config::{load_config, Configuration};
use diwe::graph_from_path;
use liwe::graph::{Graph, GraphContext};
use liwe::model::{frontmatter_from_str, split_raw_frontmatter, Key};
use liwe::query::{parse_filter_expression, parse_filter_mapping, Filter, KeyOp, SortDir};
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

    pub fn nested(&self, parent: &str, name: &str) -> Option<String> {
        match self.fields.get(parent)?.get(name) {
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

pub const SESSION_START_SECTION: &str = "At session start";

pub const DEFAULT_HEADING: &str = "Most recently recorded, newest first — titles and keys only:";

pub const MACHINERY_KEYS: [&str; 2] = ["MEMORY", "queries"];

pub fn machinery_exclusion() -> Filter {
    Filter::Key(KeyOp::Nin(
        MACHINERY_KEYS.iter().map(|key| Key::name(key)).collect(),
    ))
}

pub struct Slice {
    pub heading: String,
    pub filter: Option<(Filter, String)>,
    pub sort: Option<(String, SortDir)>,
    pub limit: Option<usize>,
}

impl Slice {
    pub fn default_slices() -> Vec<Slice> {
        let filter = parse_filter_expression("{ created: { $exists: true } }")
            .expect("the default slice filter parses");
        vec![Slice {
            heading: DEFAULT_HEADING.to_string(),
            filter: Some((filter, "{ created: { $exists: true } }".to_string())),
            sort: Some(("created".to_string(), SortDir::Desc)),
            limit: None,
        }]
    }
}

pub struct MemoryStore {
    config: Configuration,
    graph: Graph,
    knobs: Mapping,
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

    Some(MemoryStore {
        config,
        graph,
        knobs,
    })
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
    std::env::var(format!("IWE_{}", name.to_uppercase()))
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub fn flow_yaml(value: &YamlValue) -> String {
    match value {
        YamlValue::Null => "null".to_string(),
        YamlValue::Bool(flag) => flag.to_string(),
        YamlValue::Number(number) => number.to_string(),
        YamlValue::String(text) => flow_scalar(text),
        YamlValue::Sequence(items) => format!(
            "[{}]",
            items.iter().map(flow_yaml).collect::<Vec<_>>().join(", ")
        ),
        YamlValue::Mapping(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let pairs: Vec<String> = map
                .iter()
                .map(|(name, value)| format!("{}: {}", flow_yaml(name), flow_yaml(value)))
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }
        YamlValue::Tagged(tagged) => flow_yaml(&tagged.value),
    }
}

fn flow_scalar(text: &str) -> String {
    let plain = text.chars().any(|c| c.is_ascii_alphabetic())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.' | '$'))
        && !matches!(text, "true" | "false" | "null");
    if plain {
        text.to_string()
    } else {
        serde_json::to_string(text).unwrap_or_default()
    }
}

fn filter_from_text(text: &str, label: &str) -> Result<(Filter, String), String> {
    parse_filter_expression(text)
        .map(|filter| (filter, text.trim().to_string()))
        .map_err(|error| format!("{}: {}", label, error))
}

fn filter_from_value(value: &YamlValue, label: &str) -> Result<(Filter, String), String> {
    match value {
        YamlValue::Mapping(map) => parse_filter_mapping(map.clone())
            .map(|filter| (filter, flow_yaml(value)))
            .map_err(|error| format!("{}: {}", label, error)),
        YamlValue::String(text) => filter_from_text(text, label),
        other => Err(format!(
            "{}: expected a mapping, got {}",
            label,
            flow_yaml(other)
        )),
    }
}

fn slices_from_value(value: &YamlValue) -> Result<Vec<Slice>, String> {
    let items = match value {
        YamlValue::Sequence(items) => items,
        other => {
            return Err(format!(
                "injection: expected a list of slices, got {}",
                flow_yaml(other)
            ))
        }
    };
    if items.is_empty() {
        return Err("injection: the list is empty; remove the knob to get the default".to_string());
    }
    items
        .iter()
        .enumerate()
        .map(|(at, item)| slice_from_value(at + 1, item))
        .collect()
}

fn slice_from_value(at: usize, value: &YamlValue) -> Result<Slice, String> {
    let label = format!("injection[{}]", at);
    let map = match value {
        YamlValue::Mapping(map) => map,
        other => {
            return Err(format!(
                "{}: expected a mapping, got {}",
                label,
                flow_yaml(other)
            ))
        }
    };
    let field = |name: &str| map.get(YamlValue::String(name.to_string()));

    for name in map.keys() {
        let known = matches!(name.as_str(), Some("heading" | "filter" | "limit" | "sort"));
        if !known {
            return Err(format!("{}: unknown key {}", label, flow_yaml(name)));
        }
    }

    let filter = match field("filter") {
        Some(value) => Some(filter_from_value(value, &format!("{}.filter", label))?),
        None => None,
    };

    let limit = match field("limit") {
        None => None,
        Some(value) => match value.as_u64() {
            Some(count) if count > 0 => Some(count as usize),
            _ => {
                return Err(format!(
                    "{}: limit must be a positive number, got {}",
                    label,
                    flow_yaml(value)
                ))
            }
        },
    };

    let sort = match field("sort") {
        None => None,
        Some(YamlValue::String(text)) => Some(parse_sort_spec(text).ok_or_else(|| {
            format!(
                "{}: sort must be `<field>:1` or `<field>:-1`, got {}",
                label,
                flow_yaml(&YamlValue::String(text.clone()))
            )
        })?),
        Some(other) => {
            return Err(format!(
                "{}: sort must be `<field>:1` or `<field>:-1`, got {}",
                label,
                flow_yaml(other)
            ))
        }
    };

    if filter.is_none() && sort.is_none() {
        return Err(format!("{}: needs a `filter` or a `sort`", label));
    }

    let heading = match field("heading") {
        Some(YamlValue::String(text)) if !text.trim().is_empty() => text.trim().to_string(),
        Some(other) => {
            return Err(format!(
                "{}: heading must be a non-empty string, got {}",
                label,
                flow_yaml(other)
            ))
        }
        None => match (&filter, &sort) {
            (Some((_, text)), _) => format!("Matching `{}`:", text),
            (None, Some((field, SortDir::Desc))) => format!("By `{}`, newest first:", field),
            (None, Some((field, SortDir::Asc))) => format!("By `{}`, oldest first:", field),
            (None, None) => unreachable!(),
        },
    };

    Ok(Slice {
        heading,
        filter,
        sort,
        limit,
    })
}

fn parse_sort_spec(text: &str) -> Option<(String, SortDir)> {
    let (field, direction) = text.trim().rsplit_once(':')?;
    let field = field.trim();
    if field.is_empty() || field.chars().any(char::is_whitespace) {
        return None;
    }
    let direction = match direction.trim() {
        "1" => SortDir::Asc,
        "-1" => SortDir::Desc,
        _ => return None,
    };
    Some((field.to_string(), direction))
}

fn section_of(body: &str, title: &str) -> Option<String> {
    let mut inside = false;
    let mut fenced = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        if !fenced {
            if let Some(heading) = line.strip_prefix("## ") {
                if inside {
                    break;
                }
                inside = heading.trim().eq_ignore_ascii_case(title);
                continue;
            }
            if inside && line.starts_with("# ") {
                break;
            }
        }
        if inside {
            lines.push(line);
        }
    }
    let text = lines.join("\n").trim().to_string();
    (inside && !text.is_empty()).then_some(text)
}

pub fn fields_at(path: &Path) -> Option<Mapping> {
    let raw = std::fs::read_to_string(path).ok()?;
    let (front, _) = split_raw_frontmatter(&raw);
    Some(
        front
            .and_then(|block| frontmatter_from_str(frontmatter_body(block)))
            .unwrap_or_default(),
    )
}

pub fn body_at(path: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(path).ok()?;
    let (_, body) = split_raw_frontmatter(&raw);
    Some(body.to_string())
}

impl MemoryStore {
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn config(&self) -> &Configuration {
        &self.config
    }

    pub fn reloaded_graph(&self) -> Graph {
        load_store_graph(&self.config)
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

    fn knob_value(&self, name: &str) -> Option<&YamlValue> {
        self.knobs
            .get(YamlValue::String(name.to_string()))
            .filter(|value| !matches!(value, YamlValue::Null))
    }

    pub fn injection_slices(&self) -> Result<Vec<Slice>, String> {
        if let Some(value) = self.knob_value("injection") {
            return slices_from_value(value);
        }
        if let Some(text) = knob_env("injection") {
            let value: YamlValue = serde_yaml::from_str(&text)
                .map_err(|error| format!("injection: IWE_INJECTION does not parse: {}", error))?;
            return slices_from_value(&value);
        }
        Ok(Slice::default_slices())
    }

    pub fn injection_scope(&self) -> Filter {
        let slices = self
            .injection_slices()
            .unwrap_or_else(|_| Slice::default_slices());
        let mut parts: Vec<Filter> = slices
            .into_iter()
            .map(|slice| match slice.filter {
                Some((filter, _)) => filter,
                None => Filter::And(Vec::new()),
            })
            .collect();
        let selected = if parts.len() == 1 {
            parts.pop().expect("one slice")
        } else {
            Filter::Or(parts)
        };
        Filter::And(vec![selected, machinery_exclusion()])
    }

    pub fn policy_body(&self) -> String {
        self.body_of("MEMORY").unwrap_or_default()
    }

    pub fn policy_section(&self, title: &str) -> Option<String> {
        section_of(&self.policy_body(), title)
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

    pub fn filter_of(&self, expression: &str) -> Option<Filter> {
        parse_filter_expression(expression).ok()
    }
}
