use super::document::LinkType;
use crate::model;
use crate::model::config::MarkdownOptions;
use crate::model::document::{DocumentInline, DocumentInlines, MathType};
use crate::model::key_index::KeyIndex;
use crate::model::reference::{Reference, ReferenceType};
use crate::model::{InlinesContext, Key, Lang, LibraryUrl, Title};

pub type Inlines = Vec<Inline>;

#[derive(Debug, Clone, PartialEq, Default)]
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

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Code(Option<Lang>, String),
    Emph(Inlines),
    Image(LibraryUrl, Title, Inlines),
    LineBreak,
    Link(LibraryUrl, Title, LinkType, Inlines),
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
            Inline::Mark(inner) | Inline::Insert(inner) | Inline::Delete(inner) => {
                inner.iter().flat_map(|inline| inline.ref_keys()).collect()
            }
            Inline::Span(_, inner) => inner.iter().flat_map(|inline| inline.ref_keys()).collect(),
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
                let new_text = match reference.reference_type {
                    ReferenceType::Regular if context.normalize_ref_text() => context
                        .get_ref_title(&reference.key)
                        .unwrap_or_else(|| reference.text.clone()),
                    ReferenceType::Regular => reference.text.clone(),
                    ReferenceType::WikiLink => String::new(),
                    ReferenceType::WikiLinkPiped => reference.text.clone(),
                };

                let display_url = match reference.reference_type {
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => {
                        Some(context.wiki_display(&reference.key, &reference.url))
                    }
                    ReferenceType::Regular => None,
                };

                Inline::Reference(Reference {
                    key: reference.key.clone(),
                    text: new_text,
                    reference_type: reference.reference_type,
                    url: reference.url.clone(),
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
            Inline::Mark(inner) => Inline::Mark(
                inner
                    .iter()
                    .map(|i| i.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Insert(inner) => Inline::Insert(
                inner
                    .iter()
                    .map(|i| i.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Delete(inner) => Inline::Delete(
                inner
                    .iter()
                    .map(|i| i.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Span(attr, inner) => Inline::Span(
                attr.clone(),
                inner
                    .iter()
                    .map(|i| i.change_key(target_key, updated_key))
                    .collect(),
            ),
            Inline::Reference(reference) => {
                if reference.key.eq(target_key) {
                    return Inline::Reference(Reference {
                        key: updated_key.clone(),
                        text: reference.text.clone(),
                        reference_type: reference.reference_type,
                        url: updated_key.to_library_url(),
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

fn render_inlines<S: MarkdownSink>(
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

fn render_inline<S: MarkdownSink>(
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
            let url = reference.key.to_library_url();
            let inlines = text_to_inlines(&reference.text);
            emit_link(
                &url,
                reference.reference_type.to_link_type(),
                &inlines,
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

fn render_code_span<S: MarkdownSink>(body: &str, out: &mut S) {
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

fn escape_str<S: MarkdownSink>(text: &str, pos: LinePos, ctx: EscapeCtx, out: &mut S) {
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
            render_inlines(inlines, options, out, RenderCtx::Inline);
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
    let top_level = indent == 0;
    let Some(width) = options.formatting.wrap_column() else {
        let mut out = String::new();
        render_inlines(inlines, options, &mut out, RenderCtx::Block { top_level });
        return out;
    };
    let effective = width.saturating_sub(indent).max(20);
    let marker = options.formatting.line_break_marker();
    let mut stream = TokenStream::default();
    render_inlines(
        inlines,
        options,
        &mut stream,
        RenderCtx::Block { top_level },
    );

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
    render_inlines(content, options, &mut out, RenderCtx::Inline);
    out
}

pub(crate) fn append_refs_extension(url: &str, extension: &str) -> String {
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
