use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

use diwe::config::{ActionDefinition, Configuration, LinkType};
use liwe::model::config::LineBreakStyle;
use liwe::model::config::{
    DjotOptions, Format, FormattingOptions, MarkdownOptions, RefsPath, RefsText, WikiLinkPath,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum SettingId {
    LibraryPath,
    Format,
    LinkFormat,
    RefsExtension,
    RefsPath,
    WikiLinkPath,
    RefsText,
    KeyDateFormat,
    DisplayDateFormat,
    FrontmatterTitle,
    KeyTemplate,
    SearchLanguage,
    ListToken,
    OrderedListToken,
    IncrementOrderedListBullets,
    BulletListContentIndent,
    OrderedListContentIndent,
    EmphasisToken,
    StrongToken,
    CodeBlockToken,
    CodeBlockTokenCount,
    RuleToken,
    RuleTokenCount,
    WrapColumn,
    PreserveLineBreaks,
    LineBreakStyleId,
    PreserveNewlines,
    Agents,
}

pub const ALL_SETTINGS: [SettingId; 28] = [
    SettingId::LibraryPath,
    SettingId::Format,
    SettingId::LinkFormat,
    SettingId::RefsExtension,
    SettingId::RefsPath,
    SettingId::WikiLinkPath,
    SettingId::RefsText,
    SettingId::KeyDateFormat,
    SettingId::DisplayDateFormat,
    SettingId::FrontmatterTitle,
    SettingId::KeyTemplate,
    SettingId::SearchLanguage,
    SettingId::ListToken,
    SettingId::OrderedListToken,
    SettingId::IncrementOrderedListBullets,
    SettingId::BulletListContentIndent,
    SettingId::OrderedListContentIndent,
    SettingId::EmphasisToken,
    SettingId::StrongToken,
    SettingId::CodeBlockToken,
    SettingId::CodeBlockTokenCount,
    SettingId::RuleToken,
    SettingId::RuleTokenCount,
    SettingId::WrapColumn,
    SettingId::PreserveLineBreaks,
    SettingId::LineBreakStyleId,
    SettingId::PreserveNewlines,
    SettingId::Agents,
];

impl SettingId {
    pub fn key(&self) -> &'static str {
        match self {
            SettingId::LibraryPath => "library.path",
            SettingId::Format => "format",
            SettingId::LinkFormat => "completion.link_format",
            SettingId::RefsExtension => "markdown.refs_extension",
            SettingId::RefsPath => "markdown.refs_path",
            SettingId::WikiLinkPath => "markdown.wiki_link_path",
            SettingId::RefsText => "markdown.refs_text",
            SettingId::KeyDateFormat => "library.date_format",
            SettingId::DisplayDateFormat => "markdown.date_format",
            SettingId::FrontmatterTitle => "library.frontmatter_document_title",
            SettingId::KeyTemplate => "templates.default.key_template",
            SettingId::SearchLanguage => "search.language",
            SettingId::ListToken => "markdown.formatting.list_token",
            SettingId::OrderedListToken => "markdown.formatting.ordered_list_token",
            SettingId::IncrementOrderedListBullets => {
                "markdown.formatting.increment_ordered_list_bullets"
            }
            SettingId::BulletListContentIndent => "markdown.formatting.bullet_list_content_indent",
            SettingId::OrderedListContentIndent => {
                "markdown.formatting.ordered_list_content_indent"
            }
            SettingId::EmphasisToken => "markdown.formatting.emphasis_token",
            SettingId::StrongToken => "markdown.formatting.strong_token",
            SettingId::CodeBlockToken => "markdown.formatting.code_block_token",
            SettingId::CodeBlockTokenCount => "markdown.formatting.code_block_token_count",
            SettingId::RuleToken => "markdown.formatting.rule_token",
            SettingId::RuleTokenCount => "markdown.formatting.rule_token_count",
            SettingId::WrapColumn => "markdown.formatting.wrap_column",
            SettingId::PreserveLineBreaks => "markdown.formatting.preserve_line_breaks",
            SettingId::LineBreakStyleId => "markdown.formatting.line_break_style",
            SettingId::PreserveNewlines => "markdown.formatting.preserve_newlines",
            SettingId::Agents => "agents",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            SettingId::LibraryPath => "library",
            SettingId::Format => "format",
            SettingId::LinkFormat => "links",
            SettingId::RefsExtension => "link extension",
            SettingId::RefsPath => "link paths",
            SettingId::WikiLinkPath => "wiki paths",
            SettingId::RefsText => "link text",
            SettingId::KeyDateFormat => "daily notes",
            SettingId::DisplayDateFormat => "date display",
            SettingId::FrontmatterTitle => "titles",
            SettingId::KeyTemplate => "key naming",
            SettingId::SearchLanguage => "search language",
            SettingId::ListToken => "list bullets",
            SettingId::OrderedListToken => "ordered lists",
            SettingId::IncrementOrderedListBullets => "list numbering",
            SettingId::BulletListContentIndent => "bullet indent",
            SettingId::OrderedListContentIndent => "ordered indent",
            SettingId::EmphasisToken => "emphasis",
            SettingId::StrongToken => "strong",
            SettingId::CodeBlockToken => "code fences",
            SettingId::CodeBlockTokenCount => "fence length",
            SettingId::RuleToken => "rules",
            SettingId::RuleTokenCount => "rule length",
            SettingId::WrapColumn => "wrap",
            SettingId::PreserveLineBreaks => "line breaks",
            SettingId::LineBreakStyleId => "break style",
            SettingId::PreserveNewlines => "newlines",
            SettingId::Agents => "agents",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Value {
    Text(String),
    Bool(bool),
    Number(usize),
    Unset,
}

impl Value {
    pub fn text(value: impl Into<String>) -> Self {
        Value::Text(value.into())
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(flag) => Some(*flag),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<usize> {
        match self {
            Value::Number(number) => Some(*number),
            _ => None,
        }
    }
}

impl Display for Value {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Value::Text(text) if text.is_empty() => write!(formatter, "none"),
            Value::Text(text) => write!(formatter, "{}", text),
            Value::Bool(true) => write!(formatter, "on"),
            Value::Bool(false) => write!(formatter, "off"),
            Value::Number(number) => write!(formatter, "{}", number),
            Value::Unset => write!(formatter, "—"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    Detected,
    Assumed,
    Overridden,
    Asked,
}

impl Display for Confidence {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Confidence::Detected => write!(formatter, "detected"),
            Confidence::Assumed => write!(formatter, "assumed"),
            Confidence::Overridden => write!(formatter, "overridden"),
            Confidence::Asked => write!(formatter, "asked"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    values: BTreeMap<SettingId, Value>,
    confidence: BTreeMap<SettingId, Confidence>,
    notes: BTreeMap<SettingId, String>,
    mixed: BTreeMap<SettingId, bool>,
}

impl Settings {
    pub fn set(&mut self, id: SettingId, value: Value, confidence: Confidence, note: &str) {
        self.values.insert(id, value);
        self.confidence.insert(id, confidence);
        if !note.is_empty() {
            self.notes.insert(id, note.to_string());
        }
    }

    pub fn mark_mixed(&mut self, id: SettingId) {
        self.mixed.insert(id, true);
    }

    pub fn is_mixed(&self, id: SettingId) -> bool {
        self.mixed.get(&id).copied().unwrap_or(false)
    }

    pub fn get(&self, id: SettingId) -> Value {
        self.values.get(&id).cloned().unwrap_or(Value::Unset)
    }

    pub fn confidence(&self, id: SettingId) -> Confidence {
        self.confidence
            .get(&id)
            .copied()
            .unwrap_or(Confidence::Assumed)
    }

    pub fn note(&self, id: SettingId) -> &str {
        self.notes.get(&id).map(String::as_str).unwrap_or("")
    }

    pub fn adopt(&mut self, id: SettingId, other: &Settings) {
        self.values.insert(id, other.get(id));
        self.confidence.insert(id, other.confidence(id));
        match other.notes.get(&id) {
            Some(note) => {
                self.notes.insert(id, note.clone());
            }
            None => {
                self.notes.remove(&id);
            }
        }
        self.mixed.insert(id, other.is_mixed(id));
    }

    pub fn values(&self) -> BTreeMap<String, Value> {
        self.values
            .iter()
            .filter(|(id, _)| **id != SettingId::Agents)
            .map(|(id, value)| (id.key().to_string(), value.clone()))
            .collect()
    }

    pub fn confidences(&self) -> BTreeMap<String, Confidence> {
        self.confidence
            .iter()
            .filter(|(id, _)| **id != SettingId::Agents)
            .map(|(id, confidence)| (id.key().to_string(), *confidence))
            .collect()
    }

    pub fn differing(&self, other: &Settings) -> Vec<SettingId> {
        ALL_SETTINGS
            .iter()
            .copied()
            .filter(|id| self.get(*id) != other.get(*id))
            .collect()
    }

    pub fn agents_enabled(&self) -> bool {
        self.get(SettingId::Agents).as_bool().unwrap_or(false)
    }
}

pub fn defaults() -> Settings {
    let mut settings = Settings::default();
    let note = "iwe default";

    settings.set(
        SettingId::LibraryPath,
        Value::text(""),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::Format,
        Value::text("markdown"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::LinkFormat,
        Value::text("markdown"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::RefsExtension,
        Value::text(""),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::RefsPath,
        Value::text("relative"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::WikiLinkPath,
        Value::text("preserve"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::RefsText,
        Value::text("preserve"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::KeyDateFormat,
        Value::text("%Y-%m-%d"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::DisplayDateFormat,
        Value::text("%b %d, %Y"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::FrontmatterTitle,
        Value::Unset,
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::KeyTemplate,
        Value::text("{{slug}}"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::SearchLanguage,
        Value::text("english"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::ListToken,
        Value::text("-"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::OrderedListToken,
        Value::text("."),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::IncrementOrderedListBullets,
        Value::Bool(true),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::BulletListContentIndent,
        Value::Unset,
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::OrderedListContentIndent,
        Value::Unset,
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::EmphasisToken,
        Value::text("*"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::StrongToken,
        Value::text("**"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::CodeBlockToken,
        Value::text("`"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::CodeBlockTokenCount,
        Value::Number(3),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::RuleToken,
        Value::text("-"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::RuleTokenCount,
        Value::Number(72),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::WrapColumn,
        Value::Unset,
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::PreserveLineBreaks,
        Value::Bool(false),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::LineBreakStyleId,
        Value::text("backslash"),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::PreserveNewlines,
        Value::Bool(false),
        Confidence::Assumed,
        note,
    );
    settings.set(
        SettingId::Agents,
        Value::Bool(false),
        Confidence::Assumed,
        note,
    );

    settings
}

fn optional_text(settings: &Settings, id: SettingId) -> Option<String> {
    match settings.get(id) {
        Value::Text(text) => Some(text),
        _ => None,
    }
}

fn changed_text(settings: &Settings, baseline: &Settings, id: SettingId) -> Option<String> {
    match settings.get(id) == baseline.get(id) {
        true => None,
        false => optional_text(settings, id),
    }
}

fn changed_number(settings: &Settings, baseline: &Settings, id: SettingId) -> Option<usize> {
    match settings.get(id) == baseline.get(id) {
        true => None,
        false => settings.get(id).as_number(),
    }
}

fn changed_bool(settings: &Settings, baseline: &Settings, id: SettingId) -> Option<bool> {
    match settings.get(id) == baseline.get(id) {
        true => None,
        false => settings.get(id).as_bool(),
    }
}

fn formatting_options(settings: &Settings) -> FormattingOptions {
    let baseline = defaults();

    FormattingOptions {
        emphasis_token: changed_text(settings, &baseline, SettingId::EmphasisToken),
        strong_token: changed_text(settings, &baseline, SettingId::StrongToken),
        list_token: changed_text(settings, &baseline, SettingId::ListToken),
        ordered_list_token: changed_text(settings, &baseline, SettingId::OrderedListToken),
        code_block_token: changed_text(settings, &baseline, SettingId::CodeBlockToken),
        code_block_token_count: changed_number(settings, &baseline, SettingId::CodeBlockTokenCount),
        increment_ordered_list_bullets: changed_bool(
            settings,
            &baseline,
            SettingId::IncrementOrderedListBullets,
        ),
        ordered_list_content_indent: changed_number(
            settings,
            &baseline,
            SettingId::OrderedListContentIndent,
        ),
        bullet_list_content_indent: changed_number(
            settings,
            &baseline,
            SettingId::BulletListContentIndent,
        ),
        rule_token: changed_text(settings, &baseline, SettingId::RuleToken),
        rule_token_count: changed_number(settings, &baseline, SettingId::RuleTokenCount),
        wrap_column: changed_number(settings, &baseline, SettingId::WrapColumn),
        preserve_line_breaks: changed_bool(settings, &baseline, SettingId::PreserveLineBreaks),
        line_break_style: match changed_text(settings, &baseline, SettingId::LineBreakStyleId)
            .as_deref()
        {
            Some("spaces") => Some(LineBreakStyle::Spaces),
            Some("backslash") => Some(LineBreakStyle::Backslash),
            _ => None,
        },
        preserve_newlines: changed_bool(settings, &baseline, SettingId::PreserveNewlines),
    }
}

pub fn to_configuration(settings: &Settings) -> Configuration {
    let mut config = Configuration::template();

    config.library.path = optional_text(settings, SettingId::LibraryPath).unwrap_or_default();
    config.library.date_format = optional_text(settings, SettingId::KeyDateFormat);
    config.library.frontmatter_document_title =
        optional_text(settings, SettingId::FrontmatterTitle);

    config.format = match settings.get(SettingId::Format).as_text() {
        Some("djot") => Format::Djot,
        _ => Format::Markdown,
    };

    let link_format = match settings.get(SettingId::LinkFormat).as_text() {
        Some("wiki") => LinkType::WikiLink,
        _ => LinkType::Markdown,
    };
    config.completion.link_format = Some(link_format.clone());

    for action in config.actions.values_mut() {
        match action {
            ActionDefinition::Extract(extract) => extract.link_type = Some(link_format.clone()),
            ActionDefinition::ExtractAll(extract_all) => {
                extract_all.link_type = Some(link_format.clone())
            }
            ActionDefinition::Link(link) => link.link_type = Some(link_format.clone()),
            _ => {}
        }
    }

    let formatting = formatting_options(settings);

    config.markdown = MarkdownOptions {
        refs_extension: optional_text(settings, SettingId::RefsExtension).unwrap_or_default(),
        refs_path: match settings.get(SettingId::RefsPath).as_text() {
            Some("absolute") => RefsPath::Absolute,
            _ => RefsPath::Relative,
        },
        refs_text: match settings.get(SettingId::RefsText).as_text() {
            Some("normalize") => RefsText::Normalize,
            _ => RefsText::Preserve,
        },
        date_format: optional_text(settings, SettingId::DisplayDateFormat),
        time_format: None,
        locale: None,
        wiki_link_path: match settings.get(SettingId::WikiLinkPath).as_text() {
            Some("full") => WikiLinkPath::Full,
            Some("short") => WikiLinkPath::Short,
            _ => WikiLinkPath::Preserve,
        },
        formatting: formatting.clone(),
    };

    config.djot = DjotOptions {
        refs_extension: config.markdown.refs_extension.clone(),
        refs_path: config.markdown.refs_path,
        refs_text: config.markdown.refs_text,
        date_format: config.markdown.date_format.clone(),
        time_format: None,
        locale: None,
        formatting,
    };

    config.search.language =
        optional_text(settings, SettingId::SearchLanguage).unwrap_or_else(|| "english".to_string());

    if let Some(template) = config.templates.get_mut("default") {
        if let Some(key_template) = optional_text(settings, SettingId::KeyTemplate) {
            template.key_template = key_template;
        }
    }

    config
}
