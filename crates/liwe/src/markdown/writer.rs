use pulldown_cmark::{Alignment, Event, HeadingLevel, Tag, TagEnd};
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
            self.blocks_events(blocks).iter().map(|e| e.clone()),
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
            },
        )
        .unwrap();
        buf
    }

    fn blocks_events(&self, iter: Vec<GraphBlock>) -> Vec<Event> {
        iter.into_iter()
            .flat_map(|block| self.block_events(block))
            .collect()
    }

    fn block_events(&self, block: GraphBlock) -> Vec<Event> {
        let mut events = Vec::new();
        match block {
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
                events.push(Event::Start(Tag::Table(
                    alignment.iter().map(|al| to_alignment(*al)).collect(),
                )));
                events.push(Event::Start(Tag::TableHead));
                for cell in header_row {
                    events.push(Event::Start(Tag::TableCell));
                    events.append(&mut self.inlines_to_events(cell));
                    events.push(Event::End(TagEnd::TableCell));
                }
                events.push(Event::End(TagEnd::TableHead));
                for row in rows {
                    events.push(Event::Start(Tag::TableRow));
                    for cell in row {
                        events.push(Event::Start(Tag::TableCell));
                        events.append(&mut self.inlines_to_events(cell));
                        events.push(Event::End(TagEnd::TableCell));
                    }
                    events.push(Event::End(TagEnd::TableRow));
                }
                events.push(Event::End(TagEnd::Table));
            }
        }
        events
    }

    fn inlines_to_events<'a>(&self, inlines: GraphInlines) -> Vec<Event> {
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
                GraphInline::Tag(text) => {
                    events.push(Event::Text(format!("#{text}").into()));
                }
                GraphInline::Task(checked) => {
                    events.push(Event::Text(if checked { "[x] " } else { "[ ] " }.into()));
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
}

fn link_type(link_type: document::LinkType) -> pulldown_cmark::LinkType {
    match link_type {
        document::LinkType::Regular => pulldown_cmark::LinkType::Inline,
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
        _ => HeadingLevel::H6, // default to H1 if level is out of range
    }
}

fn to_alignment(al: ColumnAlignment) -> Alignment {
    match al {
        ColumnAlignment::Left => Alignment::Left,
        ColumnAlignment::Center => Alignment::Center,
        ColumnAlignment::Right => Alignment::Right,
        ColumnAlignment::None => Alignment::None,
    }
}
