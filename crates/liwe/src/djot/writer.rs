use crate::model::config::DjotOptions;
use crate::model::document::{LinkType, MathType};
use crate::model::inline::{
    append_refs_extension, detect_and_strip_checkbox, Attributes, Inline, Inlines,
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
        blocks_to_djot(blocks, options, false)
    }

    pub fn write_skip_frontmatter(&self, blocks: &Blocks, options: &DjotOptions) -> String {
        blocks_to_djot(blocks, options, true)
    }
}

fn blocks_to_djot(blocks: &Blocks, options: &DjotOptions, skip_frontmatter: bool) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .filter(|block| !(skip_frontmatter && matches!(block, Block::Frontmatter(_))))
        .map(|block| block_to_djot(block, options))
        .collect();
    ensure_trailing_newline(parts.join("\n"))
}

fn block_to_djot(block: &Block, options: &DjotOptions) -> String {
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
            format!("{}\n", inlines_to_djot(inlines, options))
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
            let inner = blocks_to_djot(blocks, options, false);
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
        Block::BulletList(items) => list_to_djot(items, options, false),
        Block::OrderedList(items) => list_to_djot(items, options, true),
        Block::Table(header, alignment, rows) => table_to_djot(header, alignment, rows, options),
    }
}

fn list_to_djot(items: &[Blocks], options: &DjotOptions, ordered: bool) -> String {
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
            .map(|block| block_to_djot(block, options))
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
    for inline in inlines {
        render_inline_djot(inline, options, &mut out);
    }
    out
}

fn render_inline_djot(inline: &Inline, options: &DjotOptions, out: &mut String) {
    match inline {
        Inline::Str(text) => out.push_str(&escape_djot(text)),
        Inline::Space => out.push(' '),
        Inline::SoftBreak => out.push('\n'),
        Inline::LineBreak => out.push_str("\\\n"),
        Inline::Emph(inner) => {
            out.push('_');
            out.push_str(&inlines_to_djot(inner, options));
            out.push('_');
        }
        Inline::Strong(inner) => {
            out.push('*');
            out.push_str(&inlines_to_djot(inner, options));
            out.push('*');
        }
        Inline::Strikeout(inner) => {
            out.push_str("{-");
            out.push_str(&inlines_to_djot(inner, options));
            out.push_str("-}");
        }
        Inline::Underline(inner) => {
            out.push_str("{+");
            out.push_str(&inlines_to_djot(inner, options));
            out.push_str("+}");
        }
        Inline::Insert(inner) => {
            out.push_str("{+");
            out.push_str(&inlines_to_djot(inner, options));
            out.push_str("+}");
        }
        Inline::Delete(inner) => {
            out.push_str("{-");
            out.push_str(&inlines_to_djot(inner, options));
            out.push_str("-}");
        }
        Inline::Mark(inner) => {
            out.push_str("{=");
            out.push_str(&inlines_to_djot(inner, options));
            out.push_str("=}");
        }
        Inline::Symbol(text) => {
            out.push(':');
            out.push_str(text);
            out.push(':');
        }
        Inline::Span(attr, inner) => {
            out.push('[');
            out.push_str(&inlines_to_djot(inner, options));
            out.push(']');
            out.push_str(&render_attributes(attr));
        }
        Inline::Superscript(inner) => {
            out.push('^');
            out.push_str(&inlines_to_djot(inner, options));
            out.push('^');
        }
        Inline::Subscript(inner) => {
            out.push('~');
            out.push_str(&inlines_to_djot(inner, options));
            out.push('~');
        }
        Inline::SmallCaps(inner) => out.push_str(&inlines_to_djot(inner, options)),
        Inline::Code(_, body) => render_verbatim(body, out),
        Inline::Math(math_type, body) => {
            out.push_str(if *math_type == MathType::DisplayMath {
                "$$"
            } else {
                "$"
            });
            render_verbatim(body, out);
        }
        Inline::RawInline(_, content) => out.push_str(content),
        Inline::Link(url, _, link_type, inlines) => {
            let inner = inlines_to_djot(inlines, options);
            if *link_type == LinkType::Markdown
                && !is_ref_url(url)
                && inner.eq_ignore_ascii_case(url)
            {
                out.push('<');
                out.push_str(url);
                out.push('>');
                return;
            }
            let final_url = if is_ref_url(url) {
                append_refs_extension(url, &options.refs_extension)
            } else {
                url.to_string()
            };
            out.push('[');
            out.push_str(&inner);
            out.push_str("](");
            out.push_str(&final_url);
            out.push(')');
        }
        Inline::Reference(reference) => {
            let url =
                append_refs_extension(&reference.key.to_library_url(), &options.refs_extension);
            out.push('[');
            out.push_str(&escape_djot(&reference.text));
            out.push_str("](");
            out.push_str(&url);
            out.push(')');
        }
        Inline::Image(url, _, alt) => {
            out.push_str("![");
            out.push_str(&inlines_to_djot(alt, options));
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }
    }
}

fn render_verbatim(body: &str, out: &mut String) {
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
    out.push_str(&fence);
    if padded {
        out.push(' ');
    }
    out.push_str(body);
    if padded {
        out.push(' ');
    }
    out.push_str(&fence);
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
