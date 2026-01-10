use pulldown_cmark::{Event, HeadingLevel, MetadataBlockKind, Tag, TagEnd};
use pulldown_cmark_to_cmark::{cmark_with_options, Options};

use crate::model::config::MarkdownOptions;
use crate::model::node::ColumnAlignment;
use crate::model::{
    document,
    graph::{inlines_to_markdown, GraphBlock, GraphInline, GraphInlines},
    is_ref_url,
};

pub struct MarkdownWriter {
    options: MarkdownOptions,
}

impl MarkdownWriter {
    pub fn new(options: MarkdownOptions) -> MarkdownWriter {
        MarkdownWriter { options }
    }

    pub fn write(&self, blocks: Vec<GraphBlock>) -> String {
        let mut buf = String::new();
        cmark_with_options(
            self.blocks_events(blocks).iter().cloned(),
            &mut buf,
            Options {
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
                code_block_token_count: 4,
                code_block_token: '`',
                list_token: '*',
                ordered_list_token: '.',
                increment_ordered_list_bullets: false,
                emphasis_token: '*',
                strong_token: "**",
                use_html_for_super_sub_script: false,
            },
        )
        .unwrap();
        buf
    }

    fn blocks_events(&self, iter: Vec<GraphBlock>) -> Vec<Event<'_>> {
        iter.into_iter()
            .flat_map(|block| self.block_events(block))
            .collect()
    }

    fn block_events(&self, block: GraphBlock) -> Vec<Event<'_>> {
        let mut events = Vec::new();
        match block {
            GraphBlock::Frontmatter(content) => {
                events.push(Event::Start(Tag::MetadataBlock(
                    MetadataBlockKind::YamlStyle,
                )));
                events.push(Event::Text(content.into()));
                events.push(Event::End(TagEnd::MetadataBlock(
                    MetadataBlockKind::YamlStyle,
                )));
            }
            GraphBlock::Header(level, inlines) => {
                events.push(Event::Start(Tag::Heading {
                    level: header_level(level),
                    id: None,
                    classes: vec![],
                    attrs: vec![],
                }));
                events.append(&mut self.inlines_to_events(inlines));
                events.push(Event::End(TagEnd::Heading(header_level(level))));
            }
            GraphBlock::BlockQuote(blocks) => {
                events.push(Event::Start(Tag::BlockQuote(None)));
                events.append(&mut self.blocks_events(blocks));
                events.push(Event::End(TagEnd::BlockQuote(None)));
            }
            GraphBlock::BulletList(items) => {
                events.push(Event::Start(Tag::List(None)));
                for item in items {
                    events.push(Event::Start(Tag::Item));
                    events.append(&mut self.blocks_events(item));
                    events.push(Event::End(TagEnd::Item));
                }
                events.push(Event::End(TagEnd::List(false)));
            }
            GraphBlock::OrderedList(items) => {
                events.push(Event::Start(Tag::List(Some(1))));
                for blocks in items {
                    events.push(Event::Start(Tag::Item));
                    events.append(&mut self.blocks_events(blocks));
                    events.push(Event::End(TagEnd::Item));
                }
                events.push(Event::End(TagEnd::List(true)));
            }
            GraphBlock::Para(inlines) => {
                events.push(Event::Start(Tag::Paragraph));
                events.append(&mut self.inlines_to_events(inlines));
                events.push(Event::End(TagEnd::Paragraph));
            }
            GraphBlock::RawBlock(_, content) => {
                events.push(Event::Html(content.into()));
            }
            GraphBlock::HorizontalRule => {
                events.push(Event::Rule);
            }
            GraphBlock::Plain(inlines) => {
                events.push(Event::Start(Tag::Paragraph));
                events.append(&mut self.inlines_to_events(inlines));
                events.push(Event::End(TagEnd::Paragraph));
            }
            GraphBlock::LineBlock(lines) => {
                events.push(Event::Start(Tag::Paragraph));
                lines.iter().for_each(|line| {
                    events.append(&mut self.inlines_to_events(line.clone()));
                    events.push(Event::HardBreak);
                });
                events.push(Event::End(TagEnd::Paragraph));
            }
            GraphBlock::CodeBlock(_, content) => {
                events.push(Event::Start(Tag::CodeBlock(
                    pulldown_cmark::CodeBlockKind::Fenced(content.into()),
                )));
                events.push(Event::End(TagEnd::CodeBlock));
            }
            GraphBlock::Table(header_row, alignment, rows) => {
                let table_md = self.render_aligned_table(&header_row, &alignment, &rows);
                events.push(Event::Html(table_md.into()));
            }
        }
        events
    }

    fn inlines_to_events(&self, inlines: GraphInlines) -> Vec<Event<'_>> {
        let mut events = Vec::new();
        for inline in inlines {
            match inline {
                GraphInline::Code(_, code) => {
                    events.push(Event::Start(Tag::CodeBlock(
                        pulldown_cmark::CodeBlockKind::Fenced(code.into()),
                    )));
                    events.push(Event::End(TagEnd::CodeBlock));
                }
                GraphInline::Emph(vec) => {
                    events.push(Event::Start(Tag::Emphasis));
                    events.extend(self.inlines_to_events(vec));
                    events.push(Event::End(TagEnd::Emphasis));
                }
                GraphInline::Image(url, title, _) => {
                    events.push(Event::Start(Tag::Image {
                        title: title.into(),
                        link_type: pulldown_cmark::LinkType::Autolink,
                        dest_url: url.into(),
                        id: "".into(),
                    }));
                    events.push(Event::End(TagEnd::Image));
                }
                GraphInline::LineBreak => {
                    events.push(Event::HardBreak);
                }
                GraphInline::Link(url, title, t, inlines) => {
                    let text = inlines_to_markdown(&inlines, &self.options);
                    if !is_ref_url(&url) && text.eq_ignore_ascii_case(&url) {
                        events.push(Event::Start(Tag::Link {
                            title: title.into(),
                            link_type: pulldown_cmark::LinkType::Autolink,
                            dest_url: url.into(),
                            id: "".into(),
                        }));
                        events.extend(self.inlines_to_events(inlines));
                        events.push(Event::End(TagEnd::Link));
                    } else {
                        events.push(Event::Start(Tag::Link {
                            title: title.into(),
                            link_type: link_type(t),
                            dest_url: url.into(),
                            id: "".into(),
                        }));
                        events.extend(self.inlines_to_events(inlines));
                        events.push(Event::End(TagEnd::Link));
                    }
                }
                GraphInline::Math(math) => {
                    events.push(Event::Html(format!("\\({}\\)", math).into()));
                }
                GraphInline::RawInline(_, content) => {
                    events.push(Event::Html(content.into()));
                }
                GraphInline::SmallCaps(vec) => {
                    events.extend(self.inlines_to_events(vec));
                }
                GraphInline::SoftBreak => {
                    events.push(Event::SoftBreak);
                }
                GraphInline::Space => {
                    events.push(Event::Text(" ".into()));
                }
                GraphInline::Str(text) => {
                    events.push(Event::Text(text.into()));
                }
                GraphInline::Strikeout(vec) => {
                    events.push(Event::Start(Tag::Strikethrough));
                    events.extend(self.inlines_to_events(vec));
                    events.push(Event::End(TagEnd::Strikethrough));
                }
                GraphInline::Strong(vec) => {
                    events.push(Event::Start(Tag::Strong));
                    events.extend(self.inlines_to_events(vec));
                    events.push(Event::End(TagEnd::Strong));
                }
                GraphInline::Subscript(vec) => {
                    events.extend(self.inlines_to_events(vec));
                }
                GraphInline::Superscript(vec) => {
                    events.extend(self.inlines_to_events(vec));
                }
                GraphInline::Underline(vec) => {
                    events.extend(self.inlines_to_events(vec));
                }
            }
        }
        events
    }

    fn render_aligned_table(
        &self,
        header: &[GraphInlines],
        alignment: &[ColumnAlignment],
        rows: &[Vec<GraphInlines>],
    ) -> String {
        let header_strs: Vec<String> = header
            .iter()
            .map(|c| inlines_to_markdown(c, &self.options))
            .collect();
        let row_strs: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|c| inlines_to_markdown(c, &self.options))
                    .collect()
            })
            .collect();

        let num_cols = header.len();
        let mut widths: Vec<usize> = header_strs.iter().map(|s| s.chars().count()).collect();

        for row in &row_strs {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.chars().count());
                }
            }
        }

        for w in &mut widths {
            if *w < 3 {
                *w = 3;
            }
        }

        let mut result = String::new();

        result.push('|');
        for (i, cell) in header_strs.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(3);
            result.push(' ');
            result.push_str(&pad_cell(cell, width, alignment.get(i).copied()));
            result.push_str(" |");
        }
        result.push('\n');

        result.push('|');
        for (i, &width) in widths.iter().enumerate() {
            let al = alignment.get(i).copied().unwrap_or(ColumnAlignment::None);
            result.push_str(&separator_cell(width, al));
            result.push('|');
        }
        result.push('\n');

        for row in &row_strs {
            result.push('|');
            for i in 0..num_cols {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                let width = widths.get(i).copied().unwrap_or(3);
                result.push(' ');
                result.push_str(&pad_cell(cell, width, alignment.get(i).copied()));
                result.push_str(" |");
            }
            result.push('\n');
        }

        result
    }
}

fn pad_cell(content: &str, width: usize, alignment: Option<ColumnAlignment>) -> String {
    let content_len = content.chars().count();
    if content_len >= width {
        return content.to_string();
    }
    let padding = width - content_len;
    match alignment.unwrap_or(ColumnAlignment::None) {
        ColumnAlignment::Right => format!("{}{}", " ".repeat(padding), content),
        ColumnAlignment::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!("{}{}{}", " ".repeat(left), content, " ".repeat(right))
        }
        _ => format!("{}{}", content, " ".repeat(padding)),
    }
}

fn separator_cell(width: usize, alignment: ColumnAlignment) -> String {
    match alignment {
        ColumnAlignment::Left => format!(":{}", "-".repeat(width + 1)),
        ColumnAlignment::Right => format!("{}:", "-".repeat(width + 1)),
        ColumnAlignment::Center => format!(":{}:", "-".repeat(width)),
        ColumnAlignment::None => format!(" {} ", "-".repeat(width)),
    }
}

fn link_type(link_type: document::LinkType) -> pulldown_cmark::LinkType {
    match link_type {
        document::LinkType::Markdown => pulldown_cmark::LinkType::Inline,
        document::LinkType::WikiLink => pulldown_cmark::LinkType::WikiLink { has_pothole: false },
        document::LinkType::WikiLinkPiped => {
            pulldown_cmark::LinkType::WikiLink { has_pothole: true }
        }
    }
}

fn header_level(level: u8) -> HeadingLevel {
    match level {
        1 => HeadingLevel::H1,
        2 => HeadingLevel::H2,
        3 => HeadingLevel::H3,
        4 => HeadingLevel::H4,
        5 => HeadingLevel::H5,
        6 => HeadingLevel::H6,
        _ => HeadingLevel::H6,
    }
}
