use pulldown_cmark_to_cmark::Options as CmarkOptions;

use serde::{Deserialize, Serialize};

use crate::graph::GraphContext;

use super::node::NodeIter;
use super::NodeId;

pub const DEFAULT_KEY_DATE_FORMAT: &str = "%Y-%m-%d";

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct MarkdownOptions {
    #[serde(default)]
    pub refs_extension: String,
    #[serde(default)]
    pub refs_path: RefsPath,
    pub date_format: Option<String>,
    pub time_format: Option<String>,
    pub locale: Option<String>,
    #[serde(default)]
    pub wiki_link_path: WikiLinkPath,
    #[serde(default)]
    pub formatting: FormattingOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Markdown,
    Djot,
}

impl Format {
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Markdown => "md",
            Format::Djot => "dj",
        }
    }

    pub fn from_extension(extension: &str) -> Option<Format> {
        match extension {
            "md" => Some(Format::Markdown),
            "dj" => Some(Format::Djot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct DjotOptions {
    #[serde(default)]
    pub refs_extension: String,
    #[serde(default)]
    pub refs_path: RefsPath,
    pub date_format: Option<String>,
    pub time_format: Option<String>,
    pub locale: Option<String>,
    #[serde(default)]
    pub formatting: FormattingOptions,
}

impl Default for DjotOptions {
    fn default() -> Self {
        Self {
            refs_extension: String::new(),
            refs_path: RefsPath::default(),
            date_format: Some("%b %d, %Y".into()),
            time_format: None,
            locale: None,
            formatting: FormattingOptions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormatOptions {
    Markdown(MarkdownOptions),
    Djot(DjotOptions),
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions::Markdown(MarkdownOptions::default())
    }
}

impl From<MarkdownOptions> for FormatOptions {
    fn from(options: MarkdownOptions) -> Self {
        FormatOptions::Markdown(options)
    }
}

impl From<DjotOptions> for FormatOptions {
    fn from(options: DjotOptions) -> Self {
        FormatOptions::Djot(options)
    }
}

impl FormatOptions {
    pub fn format(&self) -> Format {
        match self {
            FormatOptions::Markdown(_) => Format::Markdown,
            FormatOptions::Djot(_) => Format::Djot,
        }
    }

    pub fn extension(&self) -> &'static str {
        self.format().extension()
    }

    pub fn markdown_options(&self) -> MarkdownOptions {
        match self {
            FormatOptions::Markdown(options) => options.clone(),
            FormatOptions::Djot(_) => MarkdownOptions::default(),
        }
    }

    pub fn refs_extension(&self) -> &str {
        match self {
            FormatOptions::Markdown(options) => &options.refs_extension,
            FormatOptions::Djot(options) => &options.refs_extension,
        }
    }

    pub fn refs_path(&self) -> RefsPath {
        match self {
            FormatOptions::Markdown(options) => options.refs_path,
            FormatOptions::Djot(options) => options.refs_path,
        }
    }

    pub fn date_format(&self) -> Option<&str> {
        match self {
            FormatOptions::Markdown(options) => options.date_format.as_deref(),
            FormatOptions::Djot(options) => options.date_format.as_deref(),
        }
    }

    pub fn time_format(&self) -> Option<&str> {
        match self {
            FormatOptions::Markdown(options) => options.time_format.as_deref(),
            FormatOptions::Djot(options) => options.time_format.as_deref(),
        }
    }

    pub fn locale(&self) -> Option<&str> {
        match self {
            FormatOptions::Markdown(options) => options.locale.as_deref(),
            FormatOptions::Djot(options) => options.locale.as_deref(),
        }
    }

    pub fn formatting(&self) -> &FormattingOptions {
        match self {
            FormatOptions::Markdown(options) => &options.formatting,
            FormatOptions::Djot(options) => &options.formatting,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WikiLinkPath {
    Full,
    Short,
    #[default]
    Preserve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefsPath {
    #[default]
    Relative,
    Absolute,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            refs_extension: String::new(),
            refs_path: RefsPath::default(),
            date_format: Some("%b %d, %Y".into()),
            time_format: None,
            locale: None,
            wiki_link_path: WikiLinkPath::Preserve,
            formatting: FormattingOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineBreakStyle {
    Backslash,
    Spaces,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct FormattingOptions {
    pub emphasis_token: Option<String>,
    pub strong_token: Option<String>,
    pub list_token: Option<String>,
    pub ordered_list_token: Option<String>,
    pub code_block_token: Option<String>,
    pub code_block_token_count: Option<usize>,
    pub increment_ordered_list_bullets: Option<bool>,
    pub ordered_list_content_indent: Option<usize>,
    pub bullet_list_content_indent: Option<usize>,
    pub rule_token: Option<String>,
    pub rule_token_count: Option<usize>,
    pub wrap_column: Option<usize>,
    pub preserve_line_breaks: Option<bool>,
    pub line_break_style: Option<LineBreakStyle>,
    pub preserve_newlines: Option<bool>,
}

impl FormattingOptions {
    pub fn validated(self) -> Self {
        Self {
            emphasis_token: self
                .emphasis_token
                .filter(|v| matches!(v.as_str(), "*" | "_")),
            strong_token: self
                .strong_token
                .filter(|v| matches!(v.as_str(), "**" | "__")),
            list_token: self
                .list_token
                .filter(|v| matches!(v.as_str(), "*" | "-" | "+")),
            ordered_list_token: self
                .ordered_list_token
                .filter(|v| matches!(v.as_str(), "." | ")")),
            code_block_token: self
                .code_block_token
                .filter(|v| matches!(v.as_str(), "`" | "~")),
            code_block_token_count: self.code_block_token_count.filter(|&v| v >= 3),
            increment_ordered_list_bullets: self.increment_ordered_list_bullets,
            ordered_list_content_indent: self
                .ordered_list_content_indent
                .filter(|&v| (2..=4).contains(&v)),
            bullet_list_content_indent: self
                .bullet_list_content_indent
                .filter(|&v| (2..=4).contains(&v)),
            rule_token: self
                .rule_token
                .filter(|v| matches!(v.as_str(), "-" | "*" | "_")),
            rule_token_count: self.rule_token_count.filter(|&v| v >= 3),
            wrap_column: self.wrap_column.filter(|&v| v >= 20),
            preserve_line_breaks: self.preserve_line_breaks,
            line_break_style: self.line_break_style,
            preserve_newlines: self.preserve_newlines,
        }
    }

    pub fn emphasis_token(&self) -> &str {
        self.emphasis_token.as_deref().unwrap_or("*")
    }

    pub fn strong_token(&self) -> &str {
        self.strong_token.as_deref().unwrap_or("**")
    }

    pub fn list_token(&self) -> &str {
        self.list_token.as_deref().unwrap_or("-")
    }

    pub fn list_token_char(&self) -> char {
        self.list_token().chars().next().unwrap_or('-')
    }

    pub fn ordered_list_token(&self) -> &str {
        self.ordered_list_token.as_deref().unwrap_or(".")
    }

    pub fn ordered_list_token_char(&self) -> char {
        self.ordered_list_token().chars().next().unwrap_or('.')
    }

    pub fn code_block_token(&self) -> &str {
        self.code_block_token.as_deref().unwrap_or("`")
    }

    pub fn code_block_token_char(&self) -> char {
        self.code_block_token().chars().next().unwrap_or('`')
    }

    pub fn code_block_token_count(&self) -> usize {
        self.code_block_token_count.unwrap_or(3)
    }

    pub fn increment_ordered_list_bullets(&self) -> bool {
        self.increment_ordered_list_bullets.unwrap_or(true)
    }

    pub fn ordered_list_content_indent(&self) -> Option<usize> {
        self.ordered_list_content_indent
    }

    pub fn bullet_list_content_indent(&self) -> Option<usize> {
        self.bullet_list_content_indent
    }

    pub fn rule_token(&self) -> &str {
        self.rule_token.as_deref().unwrap_or("-")
    }

    pub fn rule_token_count(&self) -> usize {
        self.rule_token_count.unwrap_or(72)
    }

    pub fn wrap_column(&self) -> Option<usize> {
        self.wrap_column
    }

    pub fn preserve_line_breaks(&self) -> bool {
        self.preserve_line_breaks.unwrap_or(false)
    }

    pub fn preserve_newlines(&self) -> bool {
        self.preserve_newlines.unwrap_or(false)
    }

    pub fn line_break_style(&self) -> LineBreakStyle {
        self.line_break_style.unwrap_or(LineBreakStyle::Backslash)
    }

    pub fn line_break_marker(&self) -> &'static str {
        match self.line_break_style() {
            LineBreakStyle::Backslash => "\\\n",
            LineBreakStyle::Spaces => "  \n",
        }
    }

    pub fn to_cmark_options(&self) -> CmarkOptions<'_> {
        CmarkOptions {
            newlines_after_headline: 2,
            newlines_after_paragraph: 2,
            newlines_after_codeblock: 2,
            newlines_after_htmlblock: 1,
            newlines_after_table: 2,
            newlines_after_rule: 2,
            newlines_after_list: 2,
            newlines_after_blockquote: 2,
            newlines_after_rest: 1,
            newlines_after_metadata: 1,
            code_block_token_count: self.code_block_token_count(),
            code_block_token: self.code_block_token_char(),
            list_token: self.list_token_char(),
            ordered_list_token: self.ordered_list_token_char(),
            increment_ordered_list_bullets: self.increment_ordered_list_bullets(),
            emphasis_token: self.emphasis_token_char(),
            strong_token: self.strong_token(),
            use_html_for_super_sub_script: false,
        }
    }

    fn emphasis_token_char(&self) -> char {
        self.emphasis_token().chars().next().unwrap_or('*')
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum LinkType {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "wiki")]
    WikiLink,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum InlineType {
    #[serde(rename = "section")]
    Section,
    #[serde(rename = "quote")]
    Quote,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Operation {
    Replace,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum TargetType {
    Block,
    Paragraph,
    List,
}

impl TargetType {
    pub fn acceptable_target(&self, id: NodeId, context: impl GraphContext) -> Option<NodeId> {
        match self {
            TargetType::Block => Some(id).filter(|id| !context.node(*id).is_section()),
            TargetType::Paragraph => Some(id).filter(|id| context.node(*id).is_leaf()),
            TargetType::List => Some(id).filter(|id| context.node(*id).is_leaf()),
        }
    }
}
