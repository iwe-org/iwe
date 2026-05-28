use serde_yaml::Mapping;

use super::document::LinkType;
use crate::markdown::writer::MarkdownWriter;
use crate::model;
use crate::model::config::{FormattingOptions, MarkdownOptions};
use crate::model::document::{DocumentInline, DocumentInlines};
use crate::model::node::ColumnAlignment;
use crate::model::reference::{Reference, ReferenceType};
use crate::model::{InlinesContext, Key, Lang, Level, LibraryUrl, Title};

pub type Blocks = Vec<GraphBlock>;
pub type GraphInlines = Vec<GraphInline>;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphBlock {
    Frontmatter(Mapping),
    Plain(GraphInlines),
    Para(GraphInlines),
    LineBlock(Vec<GraphInlines>),
    CodeBlock(Option<Lang>, String),
    RawBlock(String, String),
    BlockQuote(Blocks),
    OrderedList(Vec<Blocks>),
    BulletList(Vec<Blocks>),
    Header(Level, GraphInlines),
    HorizontalRule,
    Table(
        Vec<GraphInlines>,
        Vec<ColumnAlignment>,
        Vec<Vec<GraphInlines>>,
    ),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphInline {
    Code(Option<Lang>, String),
    Emph(GraphInlines),
    Image(LibraryUrl, Title, GraphInlines),
    LineBreak,
    Link(LibraryUrl, Title, LinkType, GraphInlines),
    Reference(Reference),
    Math(String),
    RawInline(Lang, String),
    SmallCaps(GraphInlines),
    SoftBreak,
    Space,
    Str(String),
    Strikeout(GraphInlines),
    Strong(GraphInlines),
    Subscript(GraphInlines),
    Superscript(GraphInlines),
    Underline(GraphInlines),
}

impl From<&str> for GraphInline {
    fn from(s: &str) -> Self {
        GraphInline::Str(s.to_string())
    }
}

impl From<String> for GraphInline {
    fn from(s: String) -> Self {
        GraphInline::Str(s)
    }
}

#[allow(dead_code)]
impl GraphBlock {
    fn is_sparce_list(&self) -> bool {
        match self {
            GraphBlock::BulletList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            GraphBlock::OrderedList(items) => items
                .iter()
                .any(|item| item.iter().filter(|block| block.is_paragraph()).count() > 1),
            _ => false,
        }
    }

    fn is_list(&self) -> bool {
        matches!(self, GraphBlock::BulletList(_) | GraphBlock::OrderedList(_))
    }

    fn is_paragraph(&self) -> bool {
        matches!(self, GraphBlock::Plain(_) | GraphBlock::Para(_))
    }

    fn is_frontmatter(&self) -> bool {
        matches!(self, GraphBlock::Frontmatter(_))
    }

    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        self.to_markdown_indented(options, 0)
    }

    pub fn to_markdown_indented(&self, options: &MarkdownOptions, indent: usize) -> String {
        match self {
            GraphBlock::Frontmatter(mapping) => {
                format!("---\n{}---\n", frontmatter_to_yaml(mapping))
            }
            GraphBlock::Plain(inlines) => format!("{}\n", wrap_inlines(inlines, options, indent)),
            GraphBlock::Para(inlines) => format!("{}\n", wrap_inlines(inlines, options, indent)),
            GraphBlock::LineBlock(lines) => lines
                .iter()
                .map(|line| inlines_to_markdown(line, options))
                .collect::<Vec<String>>()
                .join("\n"),
            GraphBlock::CodeBlock(lang, text) => {
                let fence = options
                    .formatting
                    .code_block_token()
                    .repeat(options.formatting.code_block_token_count());
                lang.clone()
                    .filter(|lang| !lang.trim().is_empty())
                    .map(|lang| {
                        format!(
                            "{} {}\n{}\n{}\n",
                            fence,
                            lang,
                            text.trim_matches('\n'),
                            fence
                        )
                    })
                    .unwrap_or_else(|| {
                        format!("{}\n{}\n{}\n", fence, text.trim_matches('\n'), fence)
                    })
            }
            GraphBlock::RawBlock(_, text) => text.clone(),
            GraphBlock::BlockQuote(blocks) => {
                blocks_to_markdown_sparce_indented(blocks, options, indent + 2)
                    .lines()
                    .map(|line| format!("> {}", line))
                    .map(|line| line.trim().to_string())
                    .collect::<Vec<String>>()
                    .join("\n")
                    + "\n"
            }
            GraphBlock::OrderedList(items) => {
                let child_indent = indent + ordered_prefix_indent(items.len(), &options.formatting);
                items
                    .iter()
                    .enumerate()
                    .map(|(n, item)| {
                        let num = if options.formatting.increment_ordered_list_bullets() {
                            n + 1
                        } else {
                            1
                        };
                        left_pad_and_prefix_num(
                            &blocks_to_markdown_and_indented(
                                item,
                                self.is_sparce_list(),
                                options,
                                child_indent,
                            ),
                            num,
                            options.formatting.ordered_list_token_char(),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(if self.is_sparce_list() { "\n" } else { "" })
            }
            GraphBlock::BulletList(items) => {
                let child_indent = indent + options.formatting.list_token().chars().count() + 1;
                items
                    .iter()
                    .map(|item| {
                        left_pad_and_prefix(
                            &blocks_to_markdown_and_indented(
                                item,
                                self.is_sparce_list(),
                                options,
                                child_indent,
                            ),
                            options.formatting.list_token(),
                        )
                    })
                    .collect::<Vec<String>>()
                    .join(if self.is_sparce_list() { "\n" } else { "" })
            }
            GraphBlock::Header(level, inlines) => {
                format!(
                    "{} {}\n",
                    "#".repeat(*level as usize),
                    inlines_to_markdown(inlines, options)
                )
            }
            GraphBlock::HorizontalRule => {
                let fmt = &options.formatting;
                format!("{}\n", fmt.rule_token().repeat(fmt.rule_token_count()))
            }
            GraphBlock::Table(_, _, _) => {
                let writer = MarkdownWriter::new(options.clone());
                format!("{}\n", writer.write(vec![self.clone()]))
            }
        }
    }
}

fn ordered_prefix_indent(item_count: usize, formatting: &FormattingOptions) -> usize {
    let last = if formatting.increment_ordered_list_bullets() {
        item_count.max(1)
    } else {
        1
    };
    let prefix = format!(
        "{}{}{}",
        last,
        formatting.ordered_list_token_char(),
        if last > 9 { "" } else { " " }
    );
    prefix.len() + 1
}

impl GraphInline {
    pub fn from_string(str: &str) -> GraphInlines {
        vec![GraphInline::Str(str.to_string())]
    }
    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        let mut out = String::new();
        render_inline(self, options, &mut out);
        out
    }
    pub fn plain_text(&self) -> String {
        match self {
            GraphInline::Str(text) => text.clone(),
            GraphInline::Emph(emph) => to_plain_text(emph),
            GraphInline::Underline(underline) => to_plain_text(underline),
            GraphInline::Strong(strong) => to_plain_text(strong),
            GraphInline::Strikeout(strikeout) => to_plain_text(strikeout),
            GraphInline::Superscript(superscript) => to_plain_text(superscript),
            GraphInline::Subscript(subscript) => to_plain_text(subscript),
            GraphInline::SmallCaps(small_caps) => to_plain_text(small_caps),
            GraphInline::Code(_, text) => text.clone(),
            GraphInline::Space => " ".into(),
            GraphInline::SoftBreak => "\n".into(),
            GraphInline::LineBreak => "\n".into(),
            GraphInline::Link(_, _, _, inlines) => to_plain_text(inlines),
            GraphInline::Reference(reference) => reference.text.clone(),
            GraphInline::Image(_, _, inlines) => to_plain_text(inlines),
            GraphInline::RawInline(_, content) => content.clone(),
            _ => "".into(),
        }
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        match self {
            GraphInline::Emph(emph) => emph.iter().flat_map(|inline| inline.ref_keys()).collect(),
            GraphInline::Underline(underline) => underline
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Strong(strong) => {
                strong.iter().flat_map(|inline| inline.ref_keys()).collect()
            }
            GraphInline::Strikeout(strikeout) => strikeout
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Superscript(superscript) => superscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Subscript(subscript) => subscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::SmallCaps(small_caps) => small_caps
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Link(_, _, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            GraphInline::Reference(reference) => vec![reference.key.clone()],
            GraphInline::Image(_, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            _ => vec![],
        }
    }

    pub fn normalize(&self, context: impl InlinesContext) -> GraphInline {
        match self {
            GraphInline::Emph(emph) => GraphInline::Emph(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            GraphInline::Strong(emph) => GraphInline::Strong(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Underline(emph) => GraphInline::Underline(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            GraphInline::Strikeout(emph) => GraphInline::Strikeout(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Superscript(emph) => GraphInline::Superscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Subscript(emph) => GraphInline::Subscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::SmallCaps(emph) => GraphInline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            GraphInline::Reference(reference) => {
                let new_text = match reference.reference_type {
                    ReferenceType::Regular => context
                        .get_ref_title(&reference.key)
                        .unwrap_or_else(|| reference.text.clone()),
                    ReferenceType::WikiLink => String::new(),
                    ReferenceType::WikiLinkPiped => reference.text.clone(),
                };

                GraphInline::Reference(Reference {
                    key: reference.key.clone(),
                    text: new_text,
                    reference_type: reference.reference_type,
                })
            }
            _ => self.clone(),
        }
    }

    pub fn change_key(&self, target_key: &Key, updated_key: &Key) -> GraphInline {
        match self {
            GraphInline::Emph(emph) => GraphInline::Emph(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            GraphInline::Strong(emph) => GraphInline::Strong(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Underline(emph) => GraphInline::Underline(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            GraphInline::Strikeout(emph) => GraphInline::Strikeout(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Superscript(emph) => GraphInline::Superscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Subscript(emph) => GraphInline::Subscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::SmallCaps(emph) => GraphInline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            GraphInline::Reference(reference) => {
                if reference.key.eq(target_key) {
                    return GraphInline::Reference(Reference {
                        key: updated_key.clone(),
                        text: reference.text.clone(),
                        reference_type: reference.reference_type,
                    });
                }
                self.clone()
            }
            _ => self.clone(),
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, GraphInline::Reference(_))
    }
}

fn left_pad_and_prefix(text: &str, list_token: &str) -> String {
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!("{} {}\n", list_token, line));
        } else {
            result.push_str(&format!("{} {}\n", " ".repeat(list_token.len()), line));
        }
    }

    result
}

fn left_pad_and_prefix_num(text: &str, num: usize, ordered_list_token: char) -> String {
    let prefix = format!(
        "{}{}{}",
        num,
        ordered_list_token,
        if num > 9 { "" } else { " " }
    );
    let mut result = String::new();
    for (n, line) in text.lines().enumerate() {
        if line.is_empty() {
            result.push('\n');
        } else if n == 0 {
            result.push_str(&format!("{} {}\n", prefix, line));
        } else {
            result.push_str(&format!("{} {}\n", " ".repeat(prefix.len()), line));
        }
    }

    result
}

trait MarkdownSink {
    fn push(&mut self, s: &str);
    fn space(&mut self);
    fn soft_break(&mut self);
    fn line_break(&mut self, marker: &str);
}

impl MarkdownSink for String {
    fn push(&mut self, s: &str) {
        self.push_str(s);
    }
    fn space(&mut self) {
        String::push(self, ' ');
    }
    fn soft_break(&mut self) {
        String::push(self, '\n');
    }
    fn line_break(&mut self, marker: &str) {
        self.push_str(marker);
    }
}

enum WrapToken {
    Word(String),
    Break,
}

#[derive(Default)]
struct TokenStream {
    tokens: Vec<WrapToken>,
    current: String,
}

impl TokenStream {
    fn flush(&mut self) {
        if !self.current.is_empty() {
            self.tokens
                .push(WrapToken::Word(std::mem::take(&mut self.current)));
        }
    }

    fn finish(mut self) -> Vec<WrapToken> {
        self.flush();
        self.tokens
    }
}

impl MarkdownSink for TokenStream {
    fn push(&mut self, s: &str) {
        self.current.push_str(s);
    }
    fn space(&mut self) {
        self.flush();
    }
    fn soft_break(&mut self) {
        self.flush();
    }
    fn line_break(&mut self, _marker: &str) {
        self.flush();
        self.tokens.push(WrapToken::Break);
    }
}

fn render_inlines<S: MarkdownSink>(inlines: &GraphInlines, options: &MarkdownOptions, out: &mut S) {
    for inline in inlines {
        render_inline(inline, options, out);
    }
}

fn render_inline<S: MarkdownSink>(inline: &GraphInline, options: &MarkdownOptions, out: &mut S) {
    match inline {
        GraphInline::Str(text) => out.push(text),
        GraphInline::Space => out.space(),
        GraphInline::SoftBreak => out.soft_break(),
        GraphInline::LineBreak => out.line_break(options.formatting.line_break_marker()),
        GraphInline::Code(_, body) | GraphInline::RawInline(_, body) => {
            out.push("`");
            out.push(body);
            out.push("`");
        }
        GraphInline::Math(body) => {
            out.push("$");
            out.push(body);
            out.push("$");
        }
        GraphInline::Emph(inner) => {
            let t = options.formatting.emphasis_token();
            out.push(t);
            render_inlines(inner, options, out);
            out.push(t);
        }
        GraphInline::Strong(inner) => {
            let t = options.formatting.strong_token();
            out.push(t);
            render_inlines(inner, options, out);
            out.push(t);
        }
        GraphInline::Strikeout(inner) => {
            out.push("~~");
            render_inlines(inner, options, out);
            out.push("~~");
        }
        GraphInline::Superscript(inner) => {
            out.push("^");
            render_inlines(inner, options, out);
            out.push("^");
        }
        GraphInline::Subscript(inner) => {
            out.push("~");
            render_inlines(inner, options, out);
            out.push("~");
        }
        GraphInline::SmallCaps(inner) | GraphInline::Underline(inner) => {
            render_inlines(inner, options, out)
        }
        GraphInline::Link(url, _, link_type, inlines) => {
            if *link_type == LinkType::Markdown && !model::is_ref_url(url) {
                let inner = inlines_to_markdown(inlines, options);
                if inner.eq_ignore_ascii_case(url) {
                    out.push("<");
                    out.push(url);
                    out.push(">");
                    return;
                }
            }
            emit_link(url, *link_type, inlines, options, out);
        }
        GraphInline::Reference(reference) => {
            let url = reference.key.to_library_url();
            let inlines = text_to_inlines(&reference.text);
            emit_link(
                &url,
                reference.reference_type.to_link_type(),
                &inlines,
                options,
                out,
            );
        }
        GraphInline::Image(url, _, alt) => {
            out.push("![");
            render_inlines(alt, options, out);
            out.push("](");
            out.push(url);
            out.push(")");
        }
    }
}

fn emit_link<S: MarkdownSink>(
    url: &str,
    link_type: LinkType,
    inlines: &GraphInlines,
    options: &MarkdownOptions,
    out: &mut S,
) {
    match link_type {
        LinkType::WikiLinkPiped => {
            let inner_text = inlines_to_markdown(inlines, options);
            out.push("[[");
            out.push(url);
            out.push("|");
            out.push(&inner_text);
            out.push("]]");
        }
        LinkType::WikiLink => {
            out.push("[[");
            out.push(url);
            out.push("]]");
        }
        LinkType::Markdown => {
            let final_url = if model::is_ref_url(url) {
                append_refs_extension(url, &options.refs_extension)
            } else {
                url.to_string()
            };
            out.push("[");
            render_inlines(inlines, options, out);
            out.push("](");
            out.push(&final_url);
            out.push(")");
        }
    }
}

fn text_to_inlines(text: &str) -> Vec<GraphInline> {
    let mut out = Vec::new();
    split_text_words(text, &mut out);
    out
}

fn wrap_inlines(inlines: &GraphInlines, options: &MarkdownOptions, indent: usize) -> String {
    let Some(width) = options.formatting.wrap_column() else {
        return inlines_to_markdown(inlines, options);
    };
    let effective = width.saturating_sub(indent).max(20);
    let marker = options.formatting.line_break_marker();
    let mut stream = TokenStream::default();
    render_inlines(inlines, options, &mut stream);

    let mut segments: Vec<String> = Vec::new();
    let mut buf: Vec<String> = Vec::new();
    for token in stream.finish() {
        match token {
            WrapToken::Word(s) => buf.push(s),
            WrapToken::Break => {
                segments.push(greedy_wrap(&buf, effective));
                buf.clear();
            }
        }
    }
    segments.push(greedy_wrap(&buf, effective));
    segments.join(marker)
}

fn greedy_wrap(tokens: &[String], width: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for token in tokens {
        if current.is_empty() {
            current.push_str(token);
        } else if current.chars().count() + 1 + token.chars().count() <= width {
            current.push(' ');
            current.push_str(token);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(token);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines.join("\n")
}

pub fn to_plain_text(content: &GraphInlines) -> String {
    content
        .iter()
        .map(|i| i.plain_text())
        .collect::<Vec<String>>()
        .join("")
}

pub fn frontmatter_to_yaml(mapping: &Mapping) -> String {
    if mapping.is_empty() {
        return "{}\n".to_string();
    }
    serde_yaml::to_string(mapping).unwrap_or_default()
}

pub fn inlines_to_markdown(content: &GraphInlines, options: &MarkdownOptions) -> String {
    let mut out = String::new();
    render_inlines(content, options, &mut out);
    out
}

fn append_refs_extension(url: &str, extension: &str) -> String {
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };

    let new_path = if has_file_extension(path) {
        path.to_string()
    } else {
        format!("{path}{extension}")
    };

    match fragment {
        Some(f) => format!("{new_path}#{f}"),
        None => new_path,
    }
}

fn has_file_extension(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.chars().any(|c| c.is_ascii_alphabetic()))
        .unwrap_or(false)
}

fn ensure_trailing_newline(s: String) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s
    } else {
        s + "\n"
    }
}

pub fn blocks_to_markdown_and(blocks: &Blocks, sparce: bool, options: &MarkdownOptions) -> String {
    blocks_to_markdown_and_indented(blocks, sparce, options, 0)
}

pub fn blocks_to_markdown_and_indented(
    blocks: &Blocks,
    sparce: bool,
    options: &MarkdownOptions,
    indent: usize,
) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown_indented(options, indent))
            .collect::<Vec<String>>()
            .join(if sparce { "\n" } else { "" }),
    )
}

pub fn blocks_to_markdown(blocks: &Blocks, options: &MarkdownOptions) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown(options))
            .collect::<Vec<String>>()
            .join(""),
    )
}

pub fn blocks_to_markdown_sparce(blocks: &Blocks, options: &MarkdownOptions) -> String {
    blocks_to_markdown_sparce_indented(blocks, options, 0)
}

pub fn blocks_to_markdown_sparce_indented(
    blocks: &Blocks,
    options: &MarkdownOptions,
    indent: usize,
) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .map(|block| block.to_markdown_indented(options, indent))
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

pub fn blocks_to_markdown_sparce_skip_frontmatter(
    blocks: &Blocks,
    options: &MarkdownOptions,
) -> String {
    ensure_trailing_newline(
        blocks
            .iter()
            .filter_map(|block| (!block.is_frontmatter()).then_some(block.to_markdown(options)))
            .collect::<Vec<String>>()
            .join("\n"),
    )
}

pub fn to_graph_inlines(content: &DocumentInlines, relative_to: &str) -> Vec<GraphInline> {
    let mut out = Vec::new();
    for inline in content {
        match inline {
            DocumentInline::Str(text) => split_text_words(text, &mut out),
            other => out.push(other.to_graph_inline(relative_to)),
        }
    }
    out
}

fn split_text_words(text: &str, out: &mut Vec<GraphInline>) {
    let mut word_start: Option<usize> = None;
    let mut in_ws = false;
    let mut ws_has_newline = false;
    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = word_start.take() {
                out.push(GraphInline::Str(text[start..i].to_string()));
            }
            if !in_ws {
                in_ws = true;
                ws_has_newline = false;
            }
            if ch == '\n' {
                ws_has_newline = true;
            }
        } else {
            if in_ws {
                in_ws = false;
                out.push(if ws_has_newline {
                    GraphInline::SoftBreak
                } else {
                    GraphInline::Space
                });
            }
            if word_start.is_none() {
                word_start = Some(i);
            }
        }
    }
    if let Some(start) = word_start {
        out.push(GraphInline::Str(text[start..].to_string()));
    } else if in_ws {
        out.push(if ws_has_newline {
            GraphInline::SoftBreak
        } else {
            GraphInline::Space
        });
    }
}

#[cfg(test)]
pub mod tests {
    use crate::model::config::MarkdownOptions;
    use crate::model::graph::blocks_to_markdown;
    use crate::model::graph::{GraphBlock, GraphInline};
    use indoc::indoc;

    fn plain(text: &str) -> GraphBlock {
        GraphBlock::Plain(vec![GraphInline::Str(text.into())])
    }

    #[test]
    fn test_ordered_list_to_markdown() {
        let list = vec![GraphBlock::OrderedList(vec![
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
            vec![plain("item")],
        ])];
        assert_eq!(
            indoc! {"
                1.  item
                2.  item
                3.  item
                4.  item
                5.  item
                6.  item
                7.  item
                8.  item
                9.  item
                10. item
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_ordered_list_with_para() {
        let list = vec![GraphBlock::OrderedList(vec![vec![
            plain("item1"),
            plain("para"),
        ]])];
        assert_eq!(
            indoc! {"
                1.  item1

                    para
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_to_markdown() {
        let list = vec![GraphBlock::BulletList(vec![
            vec![plain("item1")],
            vec![plain("item2")],
        ])];
        assert_eq!(
            indoc! {"
                - item1
                - item2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_sub_list() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            GraphBlock::BulletList(vec![vec![plain("item2")]]),
        ]])];
        assert_eq!(
            indoc! {"
                - item1
                  - item2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_with_para() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            plain("para"),
        ]])];
        assert_eq!(
            indoc! {"
                - item1

                  para
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_list_with_para2() {
        let list = vec![GraphBlock::BulletList(vec![
            vec![plain("item1"), plain("para1")],
            vec![plain("item2"), plain("para2")],
        ])];
        assert_eq!(
            indoc! {"
                - item1

                  para1

                - item2

                  para2
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }

    #[test]
    fn test_sub_sub_list() {
        let list = vec![GraphBlock::BulletList(vec![vec![
            plain("item1"),
            GraphBlock::BulletList(vec![vec![
                plain("item2"),
                GraphBlock::BulletList(vec![vec![plain("item3")]]),
            ]]),
        ]])];
        assert_eq!(
            indoc! {"
                - item1
                  - item2
                    - item3
                "},
            blocks_to_markdown(&list, &MarkdownOptions::default()),
        );
    }
}
