use serde::{Deserialize, Serialize};

use super::document::LinkType;
use crate::model;
use crate::model::config::MarkdownOptions;
use crate::model::document::{DocumentInline, DocumentInlines, MathType};
use crate::model::reference::{Reference, ReferenceType, Target};
use crate::model::{has_file_extension, InlinesContext, Lang, Title, WorkspaceUrl};

pub type Inlines = Vec<Inline>;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Attributes {
    pub id: String,
    pub classes: Vec<String>,
    pub pairs: Vec<(String, String)>,
}

impl Attributes {
    pub fn is_empty(&self) -> bool {
        self.id.is_empty() && self.classes.is_empty() && self.pairs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inline {
    Code(Option<Lang>, String),
    Emph(Inlines),
    Image(WorkspaceUrl, Title, Inlines),
    LineBreak,
    Link(WorkspaceUrl, Title, LinkType, Inlines),
    Reference(Reference),
    Math(MathType, String),
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
    Mark(Inlines),
    Insert(Inlines),
    Delete(Inlines),
    Symbol(String),
    Span(Attributes, Inlines),
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
        let plain = self.plain_text();
        let ctx = EscapeCtx {
            top_level: false,
            asterisk_pair: plain.matches('*').count() >= 2,
            underscore_pair: plain.matches('_').count() >= 2,
            backtick_pair: plain.matches('`').count() >= 2,
        };
        render_inline(self, options, &mut out, LinePos::Mid, ctx);
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
            Inline::Mark(inner) | Inline::Insert(inner) | Inline::Delete(inner) => {
                to_plain_text(inner)
            }
            Inline::Span(_, inner) => to_plain_text(inner),
            Inline::Symbol(text) => format!(":{}:", text),
            Inline::Code(_, text) => text.clone(),
            Inline::Space => " ".into(),
            Inline::SoftBreak => "\n".into(),
            Inline::LineBreak => "\n".into(),
            Inline::Link(_, _, _, inlines) => to_plain_text(inlines),
            Inline::Reference(reference) => reference.text(),
            Inline::Image(_, _, inlines) => to_plain_text(inlines),
            Inline::RawInline(_, content) => content.clone(),
            _ => "".into(),
        }
    }

    pub fn targets(&self) -> Vec<Target> {
        match self {
            Inline::Reference(reference) => vec![reference.target.clone()],
            _ => self
                .children()
                .map(|children| {
                    children
                        .iter()
                        .flat_map(|inline| inline.targets())
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn children(&self) -> Option<&Inlines> {
        match self {
            Inline::Emph(inner)
            | Inline::Underline(inner)
            | Inline::Strong(inner)
            | Inline::Strikeout(inner)
            | Inline::Superscript(inner)
            | Inline::Subscript(inner)
            | Inline::SmallCaps(inner)
            | Inline::Mark(inner)
            | Inline::Insert(inner)
            | Inline::Delete(inner)
            | Inline::Span(_, inner)
            | Inline::Link(_, _, _, inner)
            | Inline::Image(_, _, inner) => Some(inner),
            _ => None,
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
            Inline::Mark(inner) => {
                Inline::Mark(inner.iter().map(|i| i.normalize(context)).collect())
            }
            Inline::Insert(inner) => {
                Inline::Insert(inner.iter().map(|i| i.normalize(context)).collect())
            }
            Inline::Delete(inner) => {
                Inline::Delete(inner.iter().map(|i| i.normalize(context)).collect())
            }
            Inline::Span(attr, inner) => Inline::Span(
                attr.clone(),
                inner.iter().map(|i| i.normalize(context)).collect(),
            ),
            Inline::Reference(reference) => {
                let inlines = match reference.reference_type {
                    ReferenceType::Regular if context.normalize_ref_text() => reference
                        .target
                        .path_key()
                        .and_then(|key| context.get_ref_title(key))
                        .map(|title| text_to_inlines(&title))
                        .unwrap_or_else(|| reference.inlines.clone()),
                    ReferenceType::Regular | ReferenceType::WikiLinkPiped => {
                        reference.inlines.clone()
                    }
                    ReferenceType::WikiLink => Vec::new(),
                };

                let display_url = match reference.reference_type {
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => {
                        Some(context.wiki_display(&reference.target))
                    }
                    ReferenceType::Regular => None,
                };

                Inline::Reference(Reference {
                    target: reference.target.clone(),
                    inlines,
                    reference_type: reference.reference_type,
                    display_url,
                })
            }
            _ => self.clone(),
        }
    }

    pub fn rewrite_targets(&self, rewrite: &dyn Fn(&Target) -> Option<Target>) -> Inline {
        let map = |inner: &Inlines| -> Inlines {
            inner
                .iter()
                .map(|inline| inline.rewrite_targets(rewrite))
                .collect()
        };
        match self {
            Inline::Emph(inner) => Inline::Emph(map(inner)),
            Inline::Strong(inner) => Inline::Strong(map(inner)),
            Inline::Underline(inner) => Inline::Underline(map(inner)),
            Inline::Strikeout(inner) => Inline::Strikeout(map(inner)),
            Inline::Superscript(inner) => Inline::Superscript(map(inner)),
            Inline::Subscript(inner) => Inline::Subscript(map(inner)),
            Inline::SmallCaps(inner) => Inline::SmallCaps(map(inner)),
            Inline::Mark(inner) => Inline::Mark(map(inner)),
            Inline::Insert(inner) => Inline::Insert(map(inner)),
            Inline::Delete(inner) => Inline::Delete(map(inner)),
            Inline::Span(attr, inner) => Inline::Span(attr.clone(), map(inner)),
            Inline::Link(url, title, link_type, inner) => {
                Inline::Link(url.clone(), title.clone(), *link_type, map(inner))
            }
            Inline::Image(url, title, inner) => {
                Inline::Image(url.clone(), title.clone(), map(inner))
            }
            Inline::Reference(reference) => match rewrite(&reference.target) {
                Some(target) => Inline::Reference(Reference {
                    target,
                    inlines: map(&reference.inlines),
                    reference_type: reference.reference_type,
                    display_url: None,
                }),
                None => Inline::Reference(Reference {
                    inlines: map(&reference.inlines),
                    ..reference.clone()
                }),
            },
            _ => self.clone(),
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, Inline::Reference(_))
    }
}

pub(crate) trait TextSink {
    fn push(&mut self, s: &str);
    fn space(&mut self);
    fn soft_break(&mut self);
    fn line_break(&mut self, marker: &str);
}

impl TextSink for String {
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
    Break(String),
}

#[derive(Default)]
pub(crate) struct TokenStream {
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

    #[cfg(feature = "djot")]
    pub(crate) fn wrap(self, width: usize) -> String {
        self.wrap_lines(width, false)
    }

    pub(crate) fn wrap_escaping_markers(self, width: usize) -> String {
        self.wrap_lines(width, true)
    }

    fn wrap_lines(self, width: usize, escape_markers: bool) -> String {
        let mut wrapped = String::new();
        let mut buf: Vec<String> = Vec::new();
        for token in self.finish() {
            match token {
                WrapToken::Word(s) => buf.push(s),
                WrapToken::Break(separator) => {
                    wrapped.push_str(&greedy_wrap(&buf, width, escape_markers));
                    wrapped.push_str(&separator);
                    buf.clear();
                }
            }
        }
        wrapped.push_str(&greedy_wrap(&buf, width, escape_markers));
        wrapped
    }
}

impl TextSink for TokenStream {
    fn push(&mut self, s: &str) {
        self.current.push_str(s);
    }
    fn space(&mut self) {
        self.flush();
    }
    fn soft_break(&mut self) {
        self.flush();
        self.tokens.push(WrapToken::Break("\n".to_string()));
    }
    fn line_break(&mut self, marker: &str) {
        self.flush();
        self.tokens.push(WrapToken::Break(marker.to_string()));
    }
}

#[derive(Clone, Copy, PartialEq)]
enum LinePos {
    Start,
    AfterDigits,
    Mid,
}

#[derive(Clone, Copy)]
struct EscapeCtx {
    top_level: bool,
    asterisk_pair: bool,
    underscore_pair: bool,
    backtick_pair: bool,
}

enum RenderCtx {
    Inline,
    Block { top_level: bool },
}

fn render_inlines<S: TextSink>(
    inlines: &Inlines,
    options: &MarkdownOptions,
    out: &mut S,
    context: RenderCtx,
) {
    let (at_line_start, top_level) = match context {
        RenderCtx::Block { top_level } => (true, top_level),
        RenderCtx::Inline => (false, false),
    };
    let (mut stars, mut unders, mut ticks) = (0usize, 0usize, 0usize);
    for inline in inlines {
        if let Inline::Str(text) = inline {
            for b in text.bytes() {
                match b {
                    b'*' => stars += 1,
                    b'_' => unders += 1,
                    b'`' => ticks += 1,
                    _ => {}
                }
            }
        }
    }
    let ctx = EscapeCtx {
        top_level,
        asterisk_pair: stars >= 2,
        underscore_pair: unders >= 2,
        backtick_pair: ticks >= 2,
    };
    let mut pos = if at_line_start {
        LinePos::Start
    } else {
        LinePos::Mid
    };
    if pos == LinePos::Start
        && (starts_with_literal_checkbox(inlines) || (top_level && is_thematic_break(inlines)))
    {
        out.push("\\");
    }
    for inline in inlines {
        pos = render_inline(inline, options, out, pos, ctx);
    }
}

fn render_inline<S: TextSink>(
    inline: &Inline,
    options: &MarkdownOptions,
    out: &mut S,
    pos: LinePos,
    ctx: EscapeCtx,
) -> LinePos {
    match inline {
        Inline::Str(text) => {
            escape_str(text, pos, ctx, out);
            if text.is_empty() {
                pos
            } else if pos != LinePos::Mid && text.chars().all(|c| c.is_ascii_digit()) {
                LinePos::AfterDigits
            } else {
                LinePos::Mid
            }
        }
        Inline::Space => {
            out.space();
            if pos == LinePos::Start {
                LinePos::Start
            } else {
                LinePos::Mid
            }
        }
        Inline::SoftBreak => {
            out.soft_break();
            LinePos::Start
        }
        Inline::LineBreak => {
            out.line_break(options.formatting.line_break_marker());
            LinePos::Start
        }
        Inline::Code(_, body) | Inline::RawInline(_, body) => {
            render_code_span(body, out);
            LinePos::Mid
        }
        Inline::Math(math_type, body) => {
            out.push(if *math_type == MathType::DisplayMath {
                "$$"
            } else {
                "$"
            });
            out.push(body);
            LinePos::Mid
        }
        Inline::Emph(inner) => {
            let t = options.formatting.emphasis_token();
            out.push(t);
            render_inlines(inner, options, out, RenderCtx::Inline);
            out.push(t);
            LinePos::Mid
        }
        Inline::Strong(inner) => {
            let t = options.formatting.strong_token();
            out.push(t);
            render_inlines(inner, options, out, RenderCtx::Inline);
            out.push(t);
            LinePos::Mid
        }
        Inline::Strikeout(inner) => {
            out.push("~~");
            render_inlines(inner, options, out, RenderCtx::Inline);
            out.push("~~");
            LinePos::Mid
        }
        Inline::Superscript(inner) => {
            out.push("^");
            render_inlines(inner, options, out, RenderCtx::Inline);
            out.push("^");
            LinePos::Mid
        }
        Inline::Subscript(inner) => {
            out.push("~");
            render_inlines(inner, options, out, RenderCtx::Inline);
            out.push("~");
            LinePos::Mid
        }
        Inline::SmallCaps(inner)
        | Inline::Underline(inner)
        | Inline::Mark(inner)
        | Inline::Insert(inner)
        | Inline::Delete(inner) => {
            render_inlines(inner, options, out, RenderCtx::Inline);
            LinePos::Mid
        }
        Inline::Symbol(text) => {
            out.push(":");
            out.push(text);
            out.push(":");
            LinePos::Mid
        }
        Inline::Span(_, inner) => {
            render_inlines(inner, options, out, RenderCtx::Inline);
            LinePos::Mid
        }
        Inline::Link(url, _, link_type, inlines) => {
            if *link_type == LinkType::Markdown && !model::is_ref_url(url) {
                let inner = inlines_to_markdown(inlines, options);
                if inner.eq_ignore_ascii_case(url) {
                    out.push("<");
                    out.push(url);
                    out.push(">");
                    return LinePos::Mid;
                }
            }
            emit_link(url, *link_type, inlines, options, out);
            LinePos::Mid
        }
        Inline::Reference(reference) => {
            let url = reference.target.unresolved_url();
            emit_link(
                &url,
                reference.reference_type.to_link_type(),
                &reference.inlines,
                options,
                out,
            );
            LinePos::Mid
        }
        Inline::Image(url, _, alt) => {
            out.push("![");
            render_inlines(alt, options, out, RenderCtx::Inline);
            out.push("](");
            out.push(url);
            out.push(")");
            LinePos::Mid
        }
    }
}

fn render_code_span<S: TextSink>(body: &str, out: &mut S) {
    let mut max_run = 0;
    let mut run = 0;
    for ch in body.chars() {
        if ch == '`' {
            run += 1;
            max_run = max_run.max(run);
        } else {
            run = 0;
        }
    }
    let fence = "`".repeat(max_run + 1);
    let padded = body.starts_with('`')
        || body.ends_with('`')
        || (body.starts_with(' ') && body.ends_with(' ') && !body.trim_matches(' ').is_empty());
    out.push(&fence);
    if padded {
        out.push(" ");
    }
    out.push(body);
    if padded {
        out.push(" ");
    }
    out.push(&fence);
}

fn escape_str<S: TextSink>(text: &str, pos: LinePos, ctx: EscapeCtx, out: &mut S) {
    let line_start = pos == LinePos::Start;
    let block_start = ctx.top_level && line_start;
    let lead = text.as_bytes().first().copied();
    let lead_block_marker = block_start
        && lead
            .is_some_and(|b| matches!(b, b'#' | b'>' | b'-' | b'+' | b'*') || b.is_ascii_digit());
    let lead_ordered_marker = ctx.top_level
        && pos == LinePos::AfterDigits
        && lead.is_some_and(|b| matches!(b, b'.' | b')'));
    let has_inline_marker = text
        .bytes()
        .any(|b| matches!(b, b'\\' | b'*' | b'_' | b'[' | b']' | b'`'));
    if !lead_block_marker && !lead_ordered_marker && !has_inline_marker {
        out.push(text);
        return;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut result = String::with_capacity(text.len());
    for (i, &ch) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let trailing_marker_space = next.is_none_or(|c| c == ' ' || c == '\t');
        let escape = match ch {
            '\\' => true,
            '*' if block_start && i == 0 && trailing_marker_space => true,
            '*' => ctx.asterisk_pair,
            '_' if ctx.underscore_pair => {
                let between_alnum = prev.is_some_and(|c| c.is_alphanumeric())
                    && next.is_some_and(|c| c.is_alphanumeric());
                !between_alnum
            }
            ']' => next == Some('('),
            '[' => next == Some('['),
            '`' if ctx.top_level => ctx.backtick_pair,
            '#' if block_start && i == 0 => {
                let hashes = chars.iter().take_while(|&&c| c == '#').count();
                hashes <= 6 && chars.get(hashes).is_none_or(|&c| c == ' ' || c == '\t')
            }
            '>' if block_start && i == 0 => true,
            '-' | '+' if block_start && i == 0 => trailing_marker_space,
            '.' | ')' if ctx.top_level => {
                trailing_marker_space
                    && ((pos == LinePos::AfterDigits && i == 0)
                        || (line_start && i > 0 && chars[..i].iter().all(|c| c.is_ascii_digit())))
            }
            _ => false,
        };
        if escape {
            result.push('\\');
        }
        result.push(ch);
    }
    out.push(&result);
}

fn starts_with_literal_checkbox(inlines: &Inlines) -> bool {
    if let Some(Inline::Str(first)) = inlines.first() {
        if first == "[x] " || first == "[X] " || first == "[ ] " {
            return false;
        }
    }
    let mut prefix = String::new();
    for inline in inlines {
        match inline {
            Inline::Str(text) => prefix.push_str(text),
            Inline::Space => prefix.push(' '),
            _ => break,
        }
        if prefix.len() >= 4 {
            break;
        }
    }
    prefix.starts_with("[x] ") || prefix.starts_with("[X] ") || prefix.starts_with("[ ] ")
}

fn is_thematic_break(inlines: &Inlines) -> bool {
    let mut iter = inlines.iter();
    let dashes = match iter.next() {
        Some(Inline::Str(text)) if !text.is_empty() && text.bytes().all(|b| b == b'-') => {
            text.len()
        }
        _ => return false,
    };
    dashes >= 3 && iter.all(|inline| matches!(inline, Inline::Space))
}

fn emit_link<S: TextSink>(
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
            let final_url = if model::is_document_url(url) {
                append_refs_extension(url, &options.refs_extension)
            } else {
                url.to_string()
            };
            out.push("[");
            render_inlines(inlines, options, out, RenderCtx::Inline);
            out.push("](");
            out.push(&final_url);
            out.push(")");
        }
    }
}

pub(crate) fn text_to_inlines(text: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    split_text_words(text, &mut out);
    out
}

pub(crate) fn wrap_inlines(inlines: &Inlines, options: &MarkdownOptions, indent: usize) -> String {
    let top_level = indent == 0;
    let Some(width) = options.formatting.wrap_column() else {
        let mut out = String::new();
        render_inlines(inlines, options, &mut out, RenderCtx::Block { top_level });
        return out;
    };
    let effective = width.saturating_sub(indent).max(20);
    let mut stream = TokenStream::default();
    render_inlines(
        inlines,
        options,
        &mut stream,
        RenderCtx::Block { top_level },
    );
    stream.wrap_escaping_markers(effective)
}

fn greedy_wrap(tokens: &[String], width: usize, escape_markers: bool) -> String {
    let mut lines: Vec<Vec<&str>> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut current_width = 0;
    for token in tokens {
        let token_width = token.chars().count();
        if current.is_empty() {
            current.push(token);
            current_width = token_width;
        } else if current_width + 1 + token_width <= width {
            current.push(token);
            current_width += 1 + token_width;
        } else {
            lines.push(std::mem::take(&mut current));
            current.push(token);
            current_width = token_width;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if escape_markers && index > 0 {
                join_escaping_line_start(line)
            } else {
                line.join(" ")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn join_escaping_line_start(line: &[&str]) -> String {
    let mut out = escaped_line_start(line).unwrap_or_else(|| line[0].to_string());
    for token in &line[1..] {
        out.push(' ');
        out.push_str(token);
    }
    out
}

fn escaped_line_start(line: &[&str]) -> Option<String> {
    let token = line[0];
    let bytes = token.as_bytes();
    let first = *bytes.first()?;
    let bullet = matches!(bytes, [b'-'] | [b'+'] | [b'*']);
    let quote = first == b'>';
    let heading = bytes.len() <= 6 && bytes.iter().all(|b| *b == b'#');
    let underline =
        line.len() == 1 && (bytes.iter().all(|b| *b == b'-') || bytes.iter().all(|b| *b == b'='));
    if bullet || quote || heading || underline || starts_html_block(token) {
        return Some(format!("\\{token}"));
    }
    if let Some(run) = code_fence_run(line) {
        let (fence, rest) = token.split_at(run);
        let escaped = fence.chars().flat_map(|c| ['\\', c]).collect::<String>();
        return Some(format!("{escaped}{rest}"));
    }
    if let Some((marker, digits)) = bytes.split_last() {
        if matches!(marker, b'.' | b')')
            && !digits.is_empty()
            && digits.iter().all(u8::is_ascii_digit)
        {
            return Some(format!(
                "{}\\{}",
                &token[..token.len() - 1],
                &token[token.len() - 1..]
            ));
        }
    }
    None
}

fn starts_html_block(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('<') else {
        return false;
    };
    if rest.starts_with('!') || rest.starts_with('?') {
        return true;
    }
    let name = rest.strip_prefix('/').unwrap_or(rest);
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return false;
    }
    let tail = name.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-');
    tail.is_empty() || tail.starts_with(['>', '/']) || tail.starts_with(char::is_whitespace)
}

fn code_fence_run(line: &[&str]) -> Option<usize> {
    let token = line[0];
    let fence = *token.as_bytes().first()?;
    if !matches!(fence, b'`' | b'~') {
        return None;
    }
    let run = token.bytes().take_while(|b| *b == fence).count();
    if run < 3 {
        return None;
    }
    if fence == b'`'
        && (token[run..].contains('`') || line[1..].iter().any(|rest| rest.contains('`')))
    {
        return None;
    }
    Some(run)
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
    render_inlines(content, options, &mut out, RenderCtx::Inline);
    out
}

pub(crate) fn append_refs_extension(url: &str, extension: &str) -> String {
    let (path, fragment) = match url.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (url, None),
    };

    let new_path = if path.is_empty() || path.ends_with('/') || has_file_extension(path) {
        path.to_string()
    } else {
        format!("{path}{extension}")
    };

    match fragment {
        Some(f) => format!("{new_path}#{f}"),
        None => new_path,
    }
}

pub fn to_graph_inlines(content: &DocumentInlines, relative_to: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    for inline in content {
        match inline {
            DocumentInline::Str(text) => split_text_words(text, &mut out),
            other => out.push(other.to_graph_inline(relative_to)),
        }
    }
    out
}

fn split_text_words(text: &str, out: &mut Vec<Inline>) {
    let mut text = text;
    if let Some(rest) = text.strip_prefix('[') {
        let ws_len: usize = rest
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .map(|c| c.len_utf8())
            .sum();
        if ws_len > 0 {
            out.push(Inline::Str(text[..1 + ws_len].to_string()));
            text = &text[1 + ws_len..];
        }
    }
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

pub(crate) fn encode_inlines(inlines: &Inlines) -> Box<[u8]> {
    postcard::to_allocvec(inlines)
        .expect("inlines serialize")
        .into_boxed_slice()
}

pub(crate) fn decode_inlines(bytes: &[u8]) -> Inlines {
    postcard::from_bytes(bytes).expect("inlines deserialize")
}

#[cfg(test)]
mod test {
    use super::{decode_inlines, encode_inlines, Attributes, Inline, Inlines};
    use crate::model::document::{LinkType, MathType};
    use crate::model::reference::{Reference, ReferenceType, Target};
    use crate::model::Key;

    fn round_trip(inlines: &Inlines) -> Inlines {
        decode_inlines(&encode_inlines(inlines))
    }

    fn words() -> Inlines {
        vec![
            Inline::Str("first".to_string()),
            Inline::Space,
            Inline::Str("second".to_string()),
        ]
    }

    fn reference(target: Target, reference_type: ReferenceType) -> Inline {
        Inline::Reference(Reference {
            target,
            inlines: words(),
            reference_type,
            display_url: Some("display".to_string()),
        })
    }

    #[test]
    fn round_trip_every_variant() {
        let inlines = vec![
            Inline::Code(None, "code".to_string()),
            Inline::Code(Some("lang".to_string()), "code".to_string()),
            Inline::Emph(words()),
            Inline::Image("url".to_string(), "title".to_string(), words()),
            Inline::LineBreak,
            Inline::Link(
                "url".to_string(),
                "title".to_string(),
                LinkType::Markdown,
                words(),
            ),
            Inline::Link(
                "url".to_string(),
                String::new(),
                LinkType::WikiLink,
                words(),
            ),
            Inline::Link(
                "url".to_string(),
                String::new(),
                LinkType::WikiLinkPiped,
                words(),
            ),
            reference(
                Target::Path {
                    key: Key::name("key"),
                    fragment: None,
                },
                ReferenceType::Regular,
            ),
            reference(
                Target::Path {
                    key: Key::name("dir/key"),
                    fragment: Some("fragment".to_string()),
                },
                ReferenceType::Regular,
            ),
            reference(Target::name("key"), ReferenceType::WikiLink),
            reference(Target::name("key"), ReferenceType::WikiLinkPiped),
            Inline::Math(MathType::InlineMath, "x".to_string()),
            Inline::Math(MathType::DisplayMath, "y".to_string()),
            Inline::RawInline("html".to_string(), "<br>".to_string()),
            Inline::SmallCaps(words()),
            Inline::SoftBreak,
            Inline::Space,
            Inline::Str("text".to_string()),
            Inline::Strikeout(words()),
            Inline::Strong(vec![Inline::Emph(words())]),
            Inline::Subscript(words()),
            Inline::Superscript(words()),
            Inline::Underline(words()),
            Inline::Mark(words()),
            Inline::Insert(words()),
            Inline::Delete(words()),
            Inline::Symbol("symbol".to_string()),
            Inline::Span(Attributes::default(), words()),
            Inline::Span(
                Attributes {
                    id: "id".to_string(),
                    classes: vec!["first".to_string(), "second".to_string()],
                    pairs: vec![("name".to_string(), "value".to_string())],
                },
                words(),
            ),
        ];

        assert_eq!(round_trip(&inlines), inlines);
    }

    #[test]
    fn round_trip_exact_key() {
        let inlines = vec![reference(
            Target::Path {
                key: Key::from_stripped("notes.md"),
                fragment: None,
            },
            ReferenceType::Regular,
        )];

        let decoded = round_trip(&inlines);

        assert_eq!(decoded, inlines);
        let Inline::Reference(decoded_reference) = &decoded[0] else {
            panic!("expected reference");
        };
        assert_eq!(
            decoded_reference.target.path_key().map(|key| key.as_str()),
            Some("notes.md")
        );
    }
}
