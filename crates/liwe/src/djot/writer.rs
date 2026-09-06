use crate::model::config::DjotOptions;
use crate::model::document::{LinkType, MathType};
use crate::model::inline::{
    append_refs_extension, detect_and_strip_checkbox, text_to_inlines, Attributes, Inline, Inlines,
    TextSink, TokenStream,
};
use crate::model::is_ref_url;
use crate::model::node::ColumnAlignment;
use crate::model::writer::{frontmatter_to_yaml, Block, Blocks};

pub struct DjotWriter {}

impl Default for DjotWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl DjotWriter {
    pub fn new() -> DjotWriter {
        DjotWriter {}
    }
}

impl DjotWriter {
    pub fn write(&self, blocks: &Blocks, options: &DjotOptions) -> String {
        blocks_to_djot(blocks, options, false, 0)
    }

    pub fn write_skip_frontmatter(&self, blocks: &Blocks, options: &DjotOptions) -> String {
        blocks_to_djot(blocks, options, true, 0)
    }
}

fn blocks_to_djot(
    blocks: &Blocks,
    options: &DjotOptions,
    skip_frontmatter: bool,
    indent: usize,
) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .filter(|block| !(skip_frontmatter && matches!(block, Block::Frontmatter(_))))
        .map(|block| block_to_djot(block, options, indent))
        .collect();
    ensure_trailing_newline(parts.join("\n"))
}

fn block_to_djot(block: &Block, options: &DjotOptions, indent: usize) -> String {
    match block {
        Block::Frontmatter(mapping) => {
            format!("---\n{}---\n", frontmatter_to_yaml(mapping))
        }
        Block::Header(level, inlines) => {
            format!(
                "{} {}\n",
                "#".repeat(*level as usize),
                inlines_to_djot(inlines, options)
            )
        }
        Block::Para(inlines) | Block::Plain(inlines) => {
            format!("{}\n", wrap_inlines_djot(inlines, options, indent))
        }
        Block::LineBlock(lines) => {
            let body = lines
                .iter()
                .map(|line| inlines_to_djot(line, options))
                .collect::<Vec<String>>()
                .join("\\\n");
            format!("{}\n", body)
        }
        Block::HorizontalRule => "----\n".to_string(),
        Block::CodeBlock(lang, text) => {
            let body = text.trim_matches('\n');
            match lang.clone().filter(|lang| !lang.trim().is_empty()) {
                Some(lang) => format!("``` {}\n{}\n```\n", lang, body),
                None => format!("```\n{}\n```\n", body),
            }
        }
        Block::RawBlock(_, text) => ensure_trailing_newline(text.clone()),
        Block::BlockQuote(blocks) => {
            let inner = blocks_to_djot(blocks, options, false, indent + 2);
            let quoted = inner
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        ">".to_string()
                    } else {
                        format!("> {}", line)
                    }
                })
                .collect::<Vec<String>>()
                .join("\n");
            format!("{}\n", quoted)
        }
        Block::BulletList(items) => list_to_djot(items, options, false, indent),
        Block::OrderedList(items) => list_to_djot(items, options, true, indent),
        Block::Table(header, alignment, rows) => table_to_djot(header, alignment, rows, options),
    }
}

fn list_to_djot(items: &[Blocks], options: &DjotOptions, ordered: bool, indent: usize) -> String {
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        let marker = if ordered {
            format!("{}.", index + 1)
        } else {
            "-".to_string()
        };
        let (checkbox, item) = strip_item_checkbox(item);
        let pad = marker.chars().count() + 1;
        let item_text: String = item
            .iter()
            .map(|block| block_to_djot(block, options, indent + pad))
            .collect::<Vec<String>>()
            .join("\n");
        for (n, line) in item_text.lines().enumerate() {
            if n == 0 {
                out.push_str(&format!("{} {}{}\n", marker, checkbox, line));
            } else if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(&format!("{}{}\n", " ".repeat(pad), line));
            }
        }
    }
    out
}

fn strip_item_checkbox(item: &Blocks) -> (&'static str, Blocks) {
    let inlines = match item.first() {
        Some(Block::Para(inlines)) | Some(Block::Plain(inlines)) => inlines,
        _ => return ("", item.clone()),
    };
    let (checked, stripped) = detect_and_strip_checkbox(inlines);
    let prefix = match checked {
        Some(true) => "[x] ",
        Some(false) => "[ ] ",
        None => return ("", item.clone()),
    };
    let mut item = item.clone();
    item[0] = match &item[0] {
        Block::Para(_) => Block::Para(stripped),
        Block::Plain(_) => Block::Plain(stripped),
        other => other.clone(),
    };
    (prefix, item)
}

fn table_to_djot(
    header: &[Inlines],
    alignment: &[ColumnAlignment],
    rows: &[Vec<Inlines>],
    options: &DjotOptions,
) -> String {
    let mut out = String::new();
    let render_row = |cells: &[Inlines]| -> String {
        let rendered = cells
            .iter()
            .map(|cell| inlines_to_djot(cell, options).replace('|', "\\|"))
            .collect::<Vec<String>>()
            .join(" | ");
        format!("| {} |\n", rendered)
    };

    if !header.is_empty() {
        out.push_str(&render_row(header));
        let separator = header
            .iter()
            .enumerate()
            .map(
                |(i, _)| match alignment.get(i).copied().unwrap_or(ColumnAlignment::None) {
                    ColumnAlignment::Left => ":---".to_string(),
                    ColumnAlignment::Right => "---:".to_string(),
                    ColumnAlignment::Center => ":---:".to_string(),
                    ColumnAlignment::None => "---".to_string(),
                },
            )
            .collect::<Vec<String>>()
            .join(" | ");
        out.push_str(&format!("| {} |\n", separator));
    }

    for row in rows {
        out.push_str(&render_row(row));
    }

    out
}

fn inlines_to_djot(inlines: &Inlines, options: &DjotOptions) -> String {
    let mut out = String::new();
    render_inlines_djot(inlines, options, &mut out, false);
    out
}

#[derive(Clone, Copy, PartialEq)]
enum BlockPos {
    Start,
    AfterDigits,
    Mid,
}

fn render_inlines_djot<S: TextSink>(
    inlines: &Inlines,
    options: &DjotOptions,
    out: &mut S,
    block_start: bool,
) {
    let mut pos = if block_start {
        BlockPos::Start
    } else {
        BlockPos::Mid
    };
    for (index, inline) in inlines.iter().enumerate() {
        let next = inlines.get(index + 1);
        let followed_by_space = next.is_none_or(|inline| matches!(inline, Inline::Space));
        render_inline_djot(inline, options, out, pos, followed_by_space);
        pos = advance_block_pos(pos, inline);
    }
}

fn advance_block_pos(pos: BlockPos, inline: &Inline) -> BlockPos {
    match inline {
        Inline::Space if pos == BlockPos::Start => BlockPos::Start,
        Inline::Str(text)
            if pos == BlockPos::Start
                && !text.is_empty()
                && text.chars().all(|c| c.is_ascii_digit()) =>
        {
            BlockPos::AfterDigits
        }
        _ => BlockPos::Mid,
    }
}

fn wrap_inlines_djot(inlines: &Inlines, options: &DjotOptions, indent: usize) -> String {
    let Some(width) = options.formatting.wrap_column() else {
        let mut out = String::new();
        render_inlines_djot(inlines, options, &mut out, true);
        return out;
    };
    let mut stream = TokenStream::default();
    render_inlines_djot(inlines, options, &mut stream, true);
    stream.wrap(width.saturating_sub(indent).max(20))
}

fn render_inline_djot<S: TextSink>(
    inline: &Inline,
    options: &DjotOptions,
    out: &mut S,
    pos: BlockPos,
    followed_by_space: bool,
) {
    match inline {
        Inline::Str(text) => out.push(&escape_djot_at(text, pos, followed_by_space)),
        Inline::Space => out.space(),
        Inline::SoftBreak => out.soft_break(),
        Inline::LineBreak => out.line_break("\\\n"),
        Inline::Emph(inner) => {
            out.push("_");
            render_inlines_djot(inner, options, out, false);
            out.push("_");
        }
        Inline::Strong(inner) => {
            out.push("*");
            render_inlines_djot(inner, options, out, false);
            out.push("*");
        }
        Inline::Strikeout(inner) => {
            out.push("{-");
            render_inlines_djot(inner, options, out, false);
            out.push("-}");
        }
        Inline::Underline(inner) => {
            out.push("{+");
            render_inlines_djot(inner, options, out, false);
            out.push("+}");
        }
        Inline::Insert(inner) => {
            out.push("{+");
            render_inlines_djot(inner, options, out, false);
            out.push("+}");
        }
        Inline::Delete(inner) => {
            out.push("{-");
            render_inlines_djot(inner, options, out, false);
            out.push("-}");
        }
        Inline::Mark(inner) => {
            out.push("{=");
            render_inlines_djot(inner, options, out, false);
            out.push("=}");
        }
        Inline::Symbol(text) => {
            out.push(":");
            out.push(text);
            out.push(":");
        }
        Inline::Span(attr, inner) => {
            out.push("[");
            render_inlines_djot(inner, options, out, false);
            out.push("]");
            out.push(&render_attributes(attr));
        }
        Inline::Superscript(inner) => {
            out.push("^");
            render_inlines_djot(inner, options, out, false);
            out.push("^");
        }
        Inline::Subscript(inner) => {
            out.push("~");
            render_inlines_djot(inner, options, out, false);
            out.push("~");
        }
        Inline::SmallCaps(inner) => render_inlines_djot(inner, options, out, false),
        Inline::Code(_, body) => render_verbatim(body, out),
        Inline::Math(math_type, body) => {
            out.push(if *math_type == MathType::DisplayMath {
                "$$"
            } else {
                "$"
            });
            render_verbatim(body, out);
        }
        Inline::RawInline(_, content) => out.push(content),
        Inline::Link(url, _, link_type, inlines) => {
            let inner = inlines_to_djot(inlines, options);
            if *link_type == LinkType::Markdown
                && !is_ref_url(url)
                && inner.eq_ignore_ascii_case(url)
            {
                out.push("<");
                out.push(url);
                out.push(">");
                return;
            }
            let final_url = if is_ref_url(url) {
                append_refs_extension(url, &options.refs_extension)
            } else {
                url.to_string()
            };
            out.push("[");
            render_inlines_djot(inlines, options, out, false);
            out.push("](");
            out.push(&final_url);
            out.push(")");
        }
        Inline::Reference(reference) => {
            let url =
                append_refs_extension(&reference.key.to_library_url(), &options.refs_extension);
            out.push("[");
            render_inlines_djot(&text_to_inlines(&reference.text), options, out, false);
            out.push("](");
            out.push(&url);
            out.push(")");
        }
        Inline::Image(url, _, alt) => {
            out.push("![");
            render_inlines_djot(alt, options, out, false);
            out.push("](");
            out.push(url);
            out.push(")");
        }
    }
}

fn render_verbatim<S: TextSink>(body: &str, out: &mut S) {
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
    let padded = body.starts_with('`') || body.ends_with('`');
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

fn escape_djot_at(text: &str, pos: BlockPos, followed_by_space: bool) -> String {
    match block_marker_escape(text, pos, followed_by_space) {
        Some(escaped) => escaped,
        None => escape_djot(text),
    }
}

fn escape_djot(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(
            ch,
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '~' | '^' | '$'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn block_marker_escape(word: &str, pos: BlockPos, followed_by_space: bool) -> Option<String> {
    if word.is_empty() {
        return None;
    }
    let all = |c: char| word.chars().all(|ch| ch == c);

    match pos {
        BlockPos::AfterDigits if followed_by_space && matches!(word, "." | ")") => {
            Some(format!("\\{}", word))
        }
        BlockPos::Start => {
            let marker = if word.starts_with('|') {
                true
            } else if all('-') {
                followed_by_space || word.len() >= 3
            } else if word == "+" || word == ":" || word.starts_with('>') {
                followed_by_space
            } else if all('#') {
                word.len() <= 6 && followed_by_space
            } else {
                false
            };
            marker.then(|| format!("\\{}", escape_djot(word)))
        }
        _ => None,
    }
}

fn render_attributes(attr: &Attributes) -> String {
    if attr.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    if !attr.id.is_empty() {
        parts.push(format!("#{}", attr.id));
    }
    for class in &attr.classes {
        parts.push(format!(".{}", class));
    }
    for (key, value) in &attr.pairs {
        if value
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '}')
        {
            parts.push(format!("{}=\"{}\"", key, value.replace('"', "\\\"")));
        } else {
            parts.push(format!("{}={}", key, value));
        }
    }
    format!("{{{}}}", parts.join(" "))
}

fn ensure_trailing_newline(s: String) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s
    } else {
        s + "\n"
    }
}
