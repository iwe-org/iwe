use crate::model::config::DjotOptions;
use crate::model::document::{LinkType, MathType};
use crate::model::frontmatter::UNPARSED_FRONTMATTER;
use crate::model::inline::{
    append_refs_extension, detect_and_strip_checkbox, push_verbatim_lines, Attributes, Inline,
    Inlines, TextSink, TokenStream, FLATTENED_BLOCK,
};
use crate::model::node::ColumnAlignment;
use crate::model::writer::{frontmatter_to_yaml, same_list_kind, Block, Blocks};
use crate::model::{autolink_body, is_ref_url};

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
    let kept: Vec<&Block> = blocks
        .iter()
        .filter(|block| !(skip_frontmatter && block.is_frontmatter()))
        .collect();
    let mut alternate = false;
    let parts: Vec<String> = kept
        .iter()
        .enumerate()
        .map(|(i, block)| {
            alternate = i > 0 && same_list_kind(kept[i - 1], block) && !alternate;
            match block {
                Block::BulletList(items) if alternate => {
                    list_to_djot(items, options, false, true, indent)
                }
                Block::OrderedList(items) if alternate => {
                    list_to_djot(items, options, true, true, indent)
                }
                _ => block_to_djot(block, options, indent),
            }
        })
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
                "#".repeat((*level).min(6) as usize),
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
        Block::Comment(text) => format!("{{%{}%}}\n", text),
        Block::Html(text) => {
            let fence = "`".repeat(longest_backtick_run(text).max(2) + 1);
            format!("{}=html\n{}\n{}\n", fence, text, fence)
        }
        Block::CodeBlock(lang, text) => {
            let body = text.trim_matches('\n');
            let fence = "`".repeat(longest_backtick_run(body).max(2) + 1);
            match lang.clone().filter(|lang| !lang.trim().is_empty()) {
                Some(lang) => format!("{} {}\n{}\n{}\n", fence, lang, body, fence),
                None => format!("{}\n{}\n{}\n", fence, body, fence),
            }
        }
        Block::RawBlock(format, text) if format == UNPARSED_FRONTMATTER => {
            format!("{}\n", text.trim_end_matches('\n'))
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
        Block::BulletList(items) => list_to_djot(items, options, false, false, indent),
        Block::OrderedList(items) => list_to_djot(items, options, true, false, indent),
        Block::Table(header, alignment, rows) => table_to_djot(header, alignment, rows, options),
    }
}

fn list_to_djot(
    items: &[Blocks],
    options: &DjotOptions,
    ordered: bool,
    alternate: bool,
    indent: usize,
) -> String {
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        let marker = match (ordered, alternate) {
            (true, false) => format!("{}.", index + 1),
            (true, true) => format!("{})", index + 1),
            (false, false) => "-".to_string(),
            (false, true) => "*".to_string(),
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
                let first = format!("{} {}{}", marker, checkbox, line);
                out.push_str(first.trim_end());
                out.push('\n');
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

fn escape_cell_pipes(cell: &str) -> String {
    let chars: Vec<char> = cell.chars().collect();
    let mut out = String::with_capacity(cell.len());
    let mut verbatim: Option<usize> = None;
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' && verbatim.is_none() {
            out.push(ch);
            if let Some(&next) = chars.get(i + 1) {
                out.push(next);
            }
            i += 2;
            continue;
        }
        if ch == '`' {
            let run = chars[i..].iter().take_while(|&&c| c == '`').count();
            verbatim = match verbatim {
                None => Some(run),
                Some(open) if open == run => None,
                other => other,
            };
            out.extend(std::iter::repeat_n('`', run));
            i += run;
            continue;
        }
        if ch == '|' && verbatim.is_none() {
            out.push('\\');
        }
        out.push(ch);
        i += 1;
    }
    out
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
            .map(|cell| escape_cell_pipes(&inlines_to_djot(cell, options)))
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
    let mut skip = 0;
    if block_start {
        let lead_len = inlines
            .iter()
            .take_while(|inline| matches!(inline, Inline::Str(_) | Inline::Symbol(_)))
            .count();
        if lead_len > 1 {
            let lead: String = inlines[..lead_len]
                .iter()
                .filter_map(|inline| match inline {
                    Inline::Str(text) => Some(text.clone()),
                    Inline::Symbol(text) => Some(format!(":{}:", text)),
                    _ => None,
                })
                .collect();
            let followed_by_space = inlines
                .get(lead_len)
                .is_none_or(|inline| matches!(inline, Inline::Space));
            if let Some(escaped) = block_marker_escape(&lead, BlockPos::Start, followed_by_space) {
                out.push(&escaped);
                skip = lead_len;
                pos = BlockPos::Mid;
            }
        }
    }
    for (index, inline) in inlines.iter().enumerate().skip(skip) {
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
        Inline::Comment(text) => {
            out.push("{%");
            out.push(text);
            out.push("%}");
        }
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
        Inline::RawInline(format, content) if format == FLATTENED_BLOCK => {
            push_verbatim_lines(content, out);
        }
        Inline::RawInline(format, content) => {
            render_verbatim(content, out);
            out.push(&format!("{{={}}}", format));
        }
        Inline::Link(url, _, link_type, inlines) => {
            let inner = inlines_to_djot(inlines, options);
            if let Some(body) =
                autolink_body(url, &inner).filter(|_| *link_type == LinkType::Markdown)
            {
                out.push("<");
                out.push(body);
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
                append_refs_extension(&reference.target.unresolved_url(), &options.refs_extension);
            out.push("[");
            render_inlines_djot(&reference.inlines, options, out, false);
            out.push("](");
            out.push(&url);
            out.push(")");
        }
        Inline::Image(url, _, _, alt) => {
            out.push("![");
            render_inlines_djot(alt, options, out, false);
            out.push("](");
            out.push(url);
            out.push(")");
        }
    }
}

fn longest_backtick_run(body: &str) -> usize {
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
    max_run
}

fn render_verbatim<S: TextSink>(body: &str, out: &mut S) {
    let fence = "`".repeat(longest_backtick_run(body) + 1);
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
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '~' | '^' | '$' | '<'
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
            } else if all(':') {
                word.len() >= 3
            } else if followed_by_space && is_list_marker(word) {
                return Some(escape_list_marker(word));
            } else {
                false
            };
            marker.then(|| format!("\\{}", escape_djot(word)))
        }
        _ => None,
    }
}

fn is_list_marker(word: &str) -> bool {
    let body = match word.strip_prefix('(') {
        Some(rest) => match rest.strip_suffix(')') {
            Some(body) => body,
            None => return false,
        },
        None => match word.strip_suffix(['.', ')']) {
            Some(body) => body,
            None => return false,
        },
    };
    !body.is_empty()
        && (body.chars().all(|c| c.is_ascii_digit())
            || (body.chars().count() == 1 && body.chars().all(|c| c.is_ascii_alphabetic()))
            || body.chars().all(|c| "ivxlcdm".contains(c))
            || body.chars().all(|c| "IVXLCDM".contains(c)))
}

fn escape_list_marker(word: &str) -> String {
    match word.strip_prefix('(') {
        Some(rest) => format!("\\({}", rest),
        None => {
            let (body, last) = word.split_at(word.len() - 1);
            format!("{}\\{}", body, last)
        }
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
