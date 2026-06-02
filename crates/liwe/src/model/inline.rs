use super::document::LinkType;
use crate::model;
use crate::model::config::MarkdownOptions;
use crate::model::document::{DocumentInline, DocumentInlines};
use crate::model::key_index::KeyIndex;
use crate::model::reference::{Reference, ReferenceType};
use crate::model::{InlinesContext, Key, Lang, LibraryUrl, Title};

pub type Inlines = Vec<Inline>;

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Code(Option<Lang>, String),
    Emph(Inlines),
    Image(LibraryUrl, Title, Inlines),
    LineBreak,
    Link(LibraryUrl, Title, LinkType, Inlines),
    Reference(Reference),
    Math(String),
    RawInline(Lang, String),
    SmallCaps(Inlines),
    SoftBreak,
    Space,
    Str(String),
    Strikeout(Inlines),
    Strong(Inlines),
    Subscript(Inlines),
    Superscript(Inlines),
    Underline(Inlines),
}

impl From<&str> for Inline {
    fn from(s: &str) -> Self {
        Inline::Str(s.to_string())
    }
}

impl From<String> for Inline {
    fn from(s: String) -> Self {
        Inline::Str(s)
    }
}

impl Inline {
    pub fn from_string(str: &str) -> Inlines {
        vec![Inline::Str(str.to_string())]
    }
    pub fn to_markdown(&self, options: &MarkdownOptions) -> String {
        let mut out = String::new();
        render_inline(self, options, &mut out);
        out
    }
    pub fn plain_text(&self) -> String {
        match self {
            Inline::Str(text) => text.clone(),
            Inline::Emph(emph) => to_plain_text(emph),
            Inline::Underline(underline) => to_plain_text(underline),
            Inline::Strong(strong) => to_plain_text(strong),
            Inline::Strikeout(strikeout) => to_plain_text(strikeout),
            Inline::Superscript(superscript) => to_plain_text(superscript),
            Inline::Subscript(subscript) => to_plain_text(subscript),
            Inline::SmallCaps(small_caps) => to_plain_text(small_caps),
            Inline::Code(_, text) => text.clone(),
            Inline::Space => " ".into(),
            Inline::SoftBreak => "\n".into(),
            Inline::LineBreak => "\n".into(),
            Inline::Link(_, _, _, inlines) => to_plain_text(inlines),
            Inline::Reference(reference) => reference.text.clone(),
            Inline::Image(_, _, inlines) => to_plain_text(inlines),
            Inline::RawInline(_, content) => content.clone(),
            _ => "".into(),
        }
    }

    pub fn ref_keys(&self) -> Vec<Key> {
        match self {
            Inline::Emph(emph) => emph.iter().flat_map(|inline| inline.ref_keys()).collect(),
            Inline::Underline(underline) => underline
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Strong(strong) => strong.iter().flat_map(|inline| inline.ref_keys()).collect(),
            Inline::Strikeout(strikeout) => strikeout
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Superscript(superscript) => superscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Subscript(subscript) => subscript
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::SmallCaps(small_caps) => small_caps
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Link(_, _, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            Inline::Reference(reference) => vec![reference.key.clone()],
            Inline::Image(_, _, inlines) => inlines
                .iter()
                .flat_map(|inline| inline.ref_keys())
                .collect(),
            _ => vec![],
        }
    }

    pub fn normalize(&self, context: impl InlinesContext) -> Inline {
        match self {
            Inline::Emph(emph) => Inline::Emph(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            Inline::Strong(emph) => Inline::Strong(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Underline(emph) => Inline::Underline(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),

            Inline::Strikeout(emph) => Inline::Strikeout(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Superscript(emph) => Inline::Superscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Subscript(emph) => Inline::Subscript(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::SmallCaps(emph) => Inline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.normalize(context))
                    .collect(),
            ),
            Inline::Reference(reference) => {
                let new_text = match reference.reference_type {
                    ReferenceType::Regular => context
                        .get_ref_title(&reference.key)
                        .unwrap_or_else(|| reference.text.clone()),
                    ReferenceType::WikiLink => String::new(),
                    ReferenceType::WikiLinkPiped => reference.text.clone(),
                };

                let display_url = match reference.reference_type {
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => {
                        Some(context.shorten_wiki(&reference.key))
                    }
                    ReferenceType::Regular => None,
                };

                Inline::Reference(Reference {
                    key: reference.key.clone(),
                    text: new_text,
                    reference_type: reference.reference_type,
                    display_url,
                })
            }
            _ => self.clone(),
        }
    }

    pub fn change_key(&self, target_key: &Key, updated_key: &Key) -> Inline {
        match self {
            Inline::Emph(emph) => Inline::Emph(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            Inline::Strong(emph) => Inline::Strong(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Underline(emph) => Inline::Underline(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),

            Inline::Strikeout(emph) => Inline::Strikeout(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Superscript(emph) => Inline::Superscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Subscript(emph) => Inline::Subscript(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::SmallCaps(emph) => Inline::SmallCaps(
                emph.iter()
                    .map(|inline| inline.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Reference(reference) => {
                if reference.key.eq(target_key) {
                    return Inline::Reference(Reference {
                        key: updated_key.clone(),
                        text: reference.text.clone(),
                        reference_type: reference.reference_type,
                        display_url: None,
                    });
                }
                self.clone()
            }
            _ => self.clone(),
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, Inline::Reference(_))
    }
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

fn render_inlines<S: MarkdownSink>(inlines: &Inlines, options: &MarkdownOptions, out: &mut S) {
    for inline in inlines {
        render_inline(inline, options, out);
    }
}

fn render_inline<S: MarkdownSink>(inline: &Inline, options: &MarkdownOptions, out: &mut S) {
    match inline {
        Inline::Str(text) => out.push(text),
        Inline::Space => out.space(),
        Inline::SoftBreak => out.soft_break(),
        Inline::LineBreak => out.line_break(options.formatting.line_break_marker()),
        Inline::Code(_, body) | Inline::RawInline(_, body) => {
            out.push("`");
            out.push(body);
            out.push("`");
        }
        Inline::Math(body) => {
            out.push("$");
            out.push(body);
            out.push("$");
        }
        Inline::Emph(inner) => {
            let t = options.formatting.emphasis_token();
            out.push(t);
            render_inlines(inner, options, out);
            out.push(t);
        }
        Inline::Strong(inner) => {
            let t = options.formatting.strong_token();
            out.push(t);
            render_inlines(inner, options, out);
            out.push(t);
        }
        Inline::Strikeout(inner) => {
            out.push("~~");
            render_inlines(inner, options, out);
            out.push("~~");
        }
        Inline::Superscript(inner) => {
            out.push("^");
            render_inlines(inner, options, out);
            out.push("^");
        }
        Inline::Subscript(inner) => {
            out.push("~");
            render_inlines(inner, options, out);
            out.push("~");
        }
        Inline::SmallCaps(inner) | Inline::Underline(inner) => render_inlines(inner, options, out),
        Inline::Link(url, _, link_type, inlines) => {
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
        Inline::Reference(reference) => {
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
        Inline::Image(url, _, alt) => {
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
    inlines: &Inlines,
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

fn text_to_inlines(text: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    split_text_words(text, &mut out);
    out
}

pub(crate) fn wrap_inlines(inlines: &Inlines, options: &MarkdownOptions, indent: usize) -> String {
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

pub fn detect_and_strip_checkbox(inlines: &Inlines) -> (Option<bool>, Inlines) {
    if let Some(Inline::Str(first)) = inlines.first() {
        for (prefix, checked) in [("[x] ", true), ("[X] ", true), ("[ ] ", false)] {
            if let Some(rest) = first.strip_prefix(prefix) {
                let mut stripped = Vec::new();
                if !rest.is_empty() {
                    stripped.push(Inline::Str(rest.to_string()));
                }
                stripped.extend(inlines[1..].iter().cloned());
                return (Some(checked), stripped);
            }
        }
    }

    let is_open = matches!(&inlines.first(), Some(Inline::Str(s)) if s == "[");
    let is_close = matches!(inlines.get(2), Some(Inline::Str(s)) if s == "]");
    let trailing_space = matches!(inlines.get(3), Some(Inline::Space));

    if inlines.len() >= 4 && is_open && is_close && trailing_space {
        match &inlines[1] {
            Inline::Str(mark) if mark == "x" || mark == "X" => {
                return (Some(true), inlines[4..].to_vec());
            }
            Inline::Space => {
                return (Some(false), inlines[4..].to_vec());
            }
            _ => {}
        }
    }
    (None, inlines.clone())
}

pub fn prepend_checkbox(checked: Option<bool>, inlines: Inlines) -> Inlines {
    match checked {
        Some(true) => {
            let mut result = vec![Inline::Str("[x] ".to_string())];
            result.extend(inlines);
            result
        }
        Some(false) => {
            let mut result = vec![Inline::Str("[ ] ".to_string())];
            result.extend(inlines);
            result
        }
        None => inlines,
    }
}

pub fn to_plain_text(content: &Inlines) -> String {
    content
        .iter()
        .map(|i| i.plain_text())
        .collect::<Vec<String>>()
        .join("")
}

pub fn inlines_to_markdown(content: &Inlines, options: &MarkdownOptions) -> String {
    let mut out = String::new();
    render_inlines(content, options, &mut out);
    out
}

fn append_refs_extension(url: &str, extension: &str) -> String {
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };

    let new_path = if path.is_empty() || has_file_extension(path) {
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

pub fn to_graph_inlines(
    content: &DocumentInlines,
    relative_to: &str,
    key_index: &KeyIndex,
) -> Vec<Inline> {
    let mut out = Vec::new();
    for inline in content {
        match inline {
            DocumentInline::Str(text) => split_text_words(text, &mut out),
            other => out.push(other.to_graph_inline(relative_to, key_index)),
        }
    }
    out
}

fn split_text_words(text: &str, out: &mut Vec<Inline>) {
    let mut word_start: Option<usize> = None;
    let mut in_ws = false;
    let mut ws_has_newline = false;
    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(start) = word_start.take() {
                out.push(Inline::Str(text[start..i].to_string()));
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
                    Inline::SoftBreak
                } else {
                    Inline::Space
                });
            }
            if word_start.is_none() {
                word_start = Some(i);
            }
        }
    }
    if let Some(start) = word_start {
        out.push(Inline::Str(text[start..].to_string()));
    } else if in_ws {
        out.push(if ws_has_newline {
            Inline::SoftBreak
        } else {
            Inline::Space
        });
    }
}
