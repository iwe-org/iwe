use std::iter::once;
use std::ops::Range;

use crate::model::config::MarkdownOptions;
use crate::model::node::ColumnAlignment;
use pulldown_cmark::{Alignment, CodeBlockKind, LinkType, Options, Tag, TagEnd};
use pulldown_cmark::{Event::*, Parser};
use serde_yaml::{Mapping, Value};

use crate::model::document::SoftBreak as DocumentSoftBreak;
use crate::model::document::*;
use crate::model::*;

pub(crate) const PARSER_OPTIONS: Options = Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
    .union(Options::ENABLE_WIKILINKS)
    .union(Options::ENABLE_TABLES);

pub struct MarkdownEventsReader {
    markdown_options: MarkdownOptions,
    inlines_pos_stack: Vec<LineRange>,
    inlines_stack: Vec<DocumentInline>,
    blocks_stack: Vec<DocumentBlock>,
    blocks: DocumentBlocks,
    line_starts: Vec<usize>,
    line_offset: usize,
    metadata_block: bool,
    html_block: bool,
    html_text: String,
    html_start: usize,
    frontmatter: Option<Mapping>,
    content: Option<String>,
}

impl Default for MarkdownEventsReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownEventsReader {
    pub fn new() -> MarkdownEventsReader {
        MarkdownEventsReader {
            markdown_options: MarkdownOptions::default(),
            inlines_pos_stack: Vec::new(),
            inlines_stack: Vec::new(),
            blocks_stack: Vec::new(),
            blocks: Vec::new(),
            line_starts: Vec::new(),
            line_offset: 0,
            metadata_block: false,
            html_block: false,
            html_text: String::new(),
            html_start: 0,
            frontmatter: None,
            content: None,
        }
    }

    pub fn new_with_options(markdown_options: &MarkdownOptions) -> MarkdownEventsReader {
        MarkdownEventsReader {
            markdown_options: markdown_options.clone(),
            inlines_pos_stack: Vec::new(),
            inlines_stack: Vec::new(),
            blocks_stack: Vec::new(),
            blocks: Vec::new(),
            line_starts: Vec::new(),
            line_offset: 0,
            metadata_block: false,
            html_block: false,
            html_text: String::new(),
            html_start: 0,
            frontmatter: None,
            content: None,
        }
    }

    pub fn blocks(&self) -> Vec<DocumentBlock> {
        self.blocks.clone()
    }

    pub fn frontmatter(&self) -> Option<Mapping> {
        self.frontmatter.clone()
    }

    pub fn top_block(&mut self) -> &mut DocumentBlock {
        self.blocks_stack.last_mut().unwrap_or_else(|| {
            panic!(
                "parse markdown:\n{}",
                &self.content.clone().unwrap_or_default()
            )
        })
    }

    pub fn read(&mut self, content: &str) -> DocumentBlocks {
        let normalized = normalize_line_endings(content);
        let (mut content, stripped_lines) = strip_empty_frontmatter(&normalized);
        self.line_offset = stripped_lines;
        if stripped_lines > 0 {
            self.frontmatter = Some(Mapping::new());
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        self.content = Some(content.clone());
        let iter = Parser::new_ext(&content, PARSER_OPTIONS).into_offset_iter();
        self.line_starts = line_starts(&content);

        for (event, range) in iter {
            match event {
                Start(tag) => {
                    self.start_tag(tag, range);
                }
                End(tag) => {
                    self.end_tag(tag, range);
                }
                Text(text) => {
                    if self.metadata_block {
                        self.frontmatter = Some(parse_frontmatter(&text));
                    } else if !self.html_block {
                        match self.top_block() {
                            DocumentBlock::CodeBlock(code_block) => {
                                code_block.text = format!("{}{}", code_block.text, text)
                            }
                            DocumentBlock::RawBlock(block) => block.text = text.to_string(),
                            _ => {
                                self.push_inline(
                                    DocumentInline::Str(text.to_string()),
                                    self.to_line_range(range),
                                );
                                self.pop_inline();
                            }
                        }
                    }
                }
                Code(text) => {
                    self.push_inline(
                        DocumentInline::Code(document::Code {
                            attr: Attributes::default(),
                            text: text.to_string(),
                            inline_range: self.to_inline_range(range.clone()),
                        }),
                        self.to_line_range(range),
                    );
                    self.pop_inline();
                }
                InlineMath(cow_str) => {
                    self.push_inline(
                        DocumentInline::Math(Math {
                            math_type: MathType::InlineMath,
                            content: cow_str.to_string(),
                            inline_range: self.to_inline_range(range.clone()),
                        }),
                        self.to_line_range(range),
                    );
                    self.pop_inline();
                }
                DisplayMath(_) => {}
                Html(text) => {
                    if self.html_block {
                        self.html_text.push_str(&text);
                    }
                }
                InlineHtml(text) => {
                    let inline = match single_comment(&text) {
                        Some(body) => DocumentInline::Comment(body.to_string()),
                        None => DocumentInline::RawInline(RawInline {
                            format: Format("html".to_string()),
                            content: text.to_string(),
                            inline_range: self.to_inline_range(range.clone()),
                        }),
                    };
                    self.push_inline(inline, self.to_line_range(range));
                    self.pop_inline();
                }
                FootnoteReference(_) => {}
                SoftBreak => {
                    let inline = if self.markdown_options.formatting.preserve_newlines() {
                        DocumentInline::SoftBreak(DocumentSoftBreak {
                            inline_range: InlineRange::default(),
                        })
                    } else {
                        DocumentInline::Space(Space {
                            inline_range: InlineRange::default(),
                        })
                    };
                    self.push_inline(inline, self.to_line_range(range));
                    self.pop_inline();
                }
                HardBreak => {
                    let inline = if self.markdown_options.formatting.preserve_line_breaks() {
                        DocumentInline::LineBreak(LineBreak {
                            inline_range: InlineRange::default(),
                        })
                    } else {
                        DocumentInline::Space(Space {
                            inline_range: InlineRange::default(),
                        })
                    };
                    self.push_inline(inline, self.to_line_range(range.clone()));
                    self.pop_inline();
                }
                Rule => {
                    self.push_block(DocumentBlock::HorizontalRule(HorizontalRule {
                        line_range: self.to_line_range(range),
                    }));
                    self.pop_block();
                }
                TaskListMarker(_) => {}
            }
        }

        self.blocks.clone()
    }

    fn push_inline(&mut self, inline: DocumentInline, lines_range: LineRange) {
        self.inlines_stack.push(inline);
        self.inlines_pos_stack.push(lines_range);
    }

    fn push_block(&mut self, block: DocumentBlock) {
        self.blocks_stack.push(block);
    }

    fn pop_inline(&mut self) {
        let inline = self
            .inlines_stack
            .pop()
            .expect("pop_inline: inlines stack underflow");
        let pos = self
            .inlines_pos_stack
            .pop()
            .expect("pop_inline: inlines pos stack underflow");

        if self.inlines_stack.is_empty() {
            self.top_block().append_inline(inline, pos);
            return;
        }

        self.inlines_stack
            .last_mut()
            .expect("pop_inline: inlines stack should not be empty")
            .apppen(inline);
    }

    fn pop_block(&mut self) {
        let block = self
            .blocks_stack
            .pop()
            .expect("pop_block: blocks stack underflow");

        if self.blocks_stack.is_empty() {
            self.blocks.push(block);
            return;
        }

        if self.top_block().is_container() {
            self.top_block().append_block(block);
        }
    }

    fn start_tag(&mut self, tag: Tag, range: Range<usize>) {
        match tag {
            Tag::Paragraph => {
                self.push_block(DocumentBlock::Para(Para {
                    line_range: self.to_line_range(range),
                    inlines: vec![],
                }));
            }
            Tag::Heading { level, .. } => self.push_block(DocumentBlock::Header(Header {
                line_range: self.to_line_range(range),
                level: level as u8,
                inlines: vec![],
            })),
            Tag::BlockQuote(_) => self.push_block(DocumentBlock::BlockQuote(BlockQuote {
                line_range: self.to_line_range(range),
                blocks: Vec::new(),
            })),
            Tag::CodeBlock(code_block_kind) => {
                self.push_block(DocumentBlock::CodeBlock(CodeBlock {
                    line_range: self.to_line_range(range),
                    lang: match code_block_kind {
                        CodeBlockKind::Fenced(lang) => {
                            Some(lang.to_string()).filter(|f| !f.is_empty())
                        }
                        CodeBlockKind::Indented => None,
                    },
                    text: String::default(),
                }))
            }
            Tag::HtmlBlock => {
                self.html_block = true;
                self.html_text.clear();
                self.html_start = range.start;
            }
            Tag::List(num) => {
                let line_range = self.to_line_range(range);
                if num.is_some() {
                    self.push_block(DocumentBlock::OrderedList(OrderedList {
                        line_range,
                        items: vec![],
                    }));
                } else {
                    self.push_block(DocumentBlock::BulletList(BulletList {
                        line_range,
                        items: vec![],
                    }));
                }
            }
            Tag::Item => {
                self.top_block().append_item();
            }
            Tag::FootnoteDefinition(_) => {}
            Tag::DefinitionList => {}
            Tag::DefinitionListTitle => {}
            Tag::DefinitionListDefinition => {}
            Tag::Table(alignment) => {
                self.push_block(DocumentBlock::Table(Table {
                    line_range: self.to_line_range(range),
                    alignment: alignment
                        .iter()
                        .map(|a| match a {
                            Alignment::None => ColumnAlignment::None,
                            Alignment::Left => ColumnAlignment::Left,
                            Alignment::Center => ColumnAlignment::Center,
                            Alignment::Right => ColumnAlignment::Right,
                        })
                        .collect(),
                    rows: vec![],
                    header: vec![],
                }));
            }
            Tag::TableHead => {}
            Tag::TableRow => {
                self.top_block().append_row();
            }
            Tag::TableCell => {
                self.top_block().append_cell();
            }
            Tag::Emphasis => {
                self.push_inline(
                    DocumentInline::Emph(Emph {
                        inlines: vec![],
                        inline_range: self.to_inline_range(range.clone()),
                    }),
                    self.to_line_range(range),
                );
            }
            Tag::Strong => {
                self.push_inline(
                    DocumentInline::Strong(Strong {
                        inlines: vec![],
                        inline_range: self.to_inline_range(range.clone()),
                    }),
                    self.to_line_range(range),
                );
            }
            Tag::Strikethrough => {
                self.push_inline(
                    DocumentInline::Strikeout(Strikeout {
                        inlines: vec![],
                        inline_range: self.to_inline_range(range.clone()),
                    }),
                    self.to_line_range(range),
                );
            }
            Tag::Link {
                dest_url,
                title,
                link_type,
                id: _,
            } => {
                self.push_inline(
                    DocumentInline::Link(Link {
                        inlines: vec![],
                        target: Target {
                            url: if link_type == LinkType::Email {
                                email_autolink_url(dest_url.as_ref())
                            } else {
                                normalize_url(
                                    dest_url.as_ref(),
                                    &self.markdown_options.refs_extension,
                                )
                            },
                            title: title.to_string(),
                        },
                        title: title.to_string(),
                        attr: Default::default(),
                        inline_range: self.to_inline_range(range.clone()),
                        link_type: to_link_type(link_type),
                    }),
                    self.to_line_range(range),
                );
            }
            Tag::Image {
                link_type,
                dest_url,
                title,
                ..
            } => {
                self.push_inline(
                    DocumentInline::Image(Image {
                        inlines: vec![],
                        target: Target {
                            url: dest_url.to_string(),
                            title: title.to_string(),
                        },
                        attr: Default::default(),
                        inline_range: self.to_inline_range(range.clone()),
                        link_type: to_link_type(link_type),
                    }),
                    self.to_line_range(range),
                );
            }
            Tag::MetadataBlock(_) => self.metadata_block = true,
            Tag::Superscript => {}
            Tag::Subscript => {}
        }
    }

    fn end_tag(&mut self, tag: TagEnd, _: Range<usize>) {
        match tag {
            TagEnd::Paragraph => {
                self.pop_block();
            }
            TagEnd::Heading(_) => self.pop_block(),
            TagEnd::BlockQuote(_) => self.pop_block(),
            TagEnd::CodeBlock => {
                self.pop_block();
            }
            TagEnd::HtmlBlock => {
                self.html_block = false;
                self.push_html_blocks();
            }
            TagEnd::List(_) => {
                self.pop_block();
            }
            TagEnd::Item => {}
            TagEnd::Emphasis => self.pop_inline(),
            TagEnd::Strong => self.pop_inline(),
            TagEnd::Strikethrough => self.pop_inline(),
            TagEnd::Link => self.pop_inline(),
            TagEnd::DefinitionList => {}
            TagEnd::DefinitionListDefinition => {}
            TagEnd::DefinitionListTitle => {}
            TagEnd::FootnoteDefinition => {}
            TagEnd::Image => self.pop_inline(),
            TagEnd::MetadataBlock(_) => self.metadata_block = false,
            TagEnd::Table => {
                self.pop_block();
            }
            TagEnd::TableCell => {}
            TagEnd::TableHead => {}
            TagEnd::TableRow => {}
            TagEnd::Superscript => {}
            TagEnd::Subscript => {}
        }
    }

    fn push_html_blocks(&mut self) {
        let html = std::mem::take(&mut self.html_text);
        let start_line = self.line_index(self.html_start) + self.line_offset;
        let Some(comments) = comment_block_bodies(&html) else {
            let text = html.trim_end_matches('\n');
            if text.trim().is_empty() {
                return;
            }
            let span = text.matches('\n').count() + 1;
            self.push_block(DocumentBlock::Html(HtmlBlock {
                line_range: start_line..start_line + span,
                text: text.to_string(),
            }));
            self.pop_block();
            return;
        };
        for (offset, body) in comments {
            let line = start_line + html[..offset].matches('\n').count();
            let span = body.matches('\n').count() + 1;
            self.push_block(DocumentBlock::Comment(Comment {
                line_range: line..line + span,
                text: body.to_string(),
            }));
            self.pop_block();
        }
    }

    fn to_inline_range(&self, range: Range<usize>) -> InlineRange {
        let content = self.content.as_deref().unwrap_or("");

        let start_line = self.line_index(range.start);
        let start_char = content[self.line_starts[start_line]..range.start]
            .encode_utf16()
            .count();

        let end_line = self.line_index(range.end);
        let end_char = content[self.line_starts[end_line]..range.end]
            .encode_utf16()
            .count();

        Position {
            line: start_line + self.line_offset,
            character: start_char,
        }..Position {
            line: end_line + self.line_offset,
            character: end_char,
        }
    }

    fn line_index(&self, offset: usize) -> usize {
        self.line_starts
            .partition_point(|&line_start| line_start <= offset)
            .saturating_sub(1)
    }

    fn to_line_range(&self, range: Range<usize>) -> LineRange {
        let start = self.line_index(range.start);
        let mut end = self.line_index(range.end);

        if start == end {
            end += 1;
        }

        (start + self.line_offset)..(end + self.line_offset)
    }
}

fn parse_frontmatter(text: &str) -> Mapping {
    if text.trim().is_empty() {
        return Mapping::new();
    }
    match serde_yaml::from_str::<Value>(text) {
        Ok(Value::Mapping(m)) => m,
        Ok(_) => {
            log::warn!("Frontmatter is not a YAML mapping, treating as empty");
            Mapping::new()
        }
        Err(e) => {
            log::warn!("Failed to parse frontmatter YAML: {}", e);
            Mapping::new()
        }
    }
}

fn single_comment(text: &str) -> Option<&str> {
    let body = text.strip_prefix("<!--")?.strip_suffix("-->")?;
    (!body.contains("-->")).then_some(body)
}

fn comment_block_bodies(html: &str) -> Option<Vec<(usize, &str)>> {
    let mut comments = Vec::new();
    let mut cursor = 0;
    loop {
        let rest = &html[cursor..];
        let trimmed = rest.trim_start();
        if trimmed.is_empty() {
            break;
        }
        let start = cursor + (rest.len() - trimmed.len());
        let after_open = trimmed.strip_prefix("<!--")?;
        let close = after_open.find("-->")?;
        let body = &after_open[..close];
        comments.push((start, body));
        cursor = start + 4 + close + 3;
        let line_rest = html[cursor..].split('\n').next().unwrap_or("");
        if !line_rest.trim().is_empty() && !line_rest.trim_start().starts_with("<!--") {
            return None;
        }
    }
    (!comments.is_empty()).then_some(comments)
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn strip_empty_frontmatter(content: &str) -> (String, usize) {
    if content.starts_with("---\n---\n") {
        (content["---\n---\n".len()..].to_string(), 2)
    } else if content == "---\n---" {
        (String::new(), 2)
    } else {
        (content.to_string(), 0)
    }
}

fn line_starts(content: &str) -> Vec<usize> {
    once(0)
        .chain(
            content
                .lines()
                .map(|line| line.len() + 1)
                .scan(0, |start, len| {
                    *start += len;
                    Some(*start)
                }),
        )
        .collect()
}

fn to_link_type(link_type: LinkType) -> document::LinkType {
    match link_type {
        LinkType::WikiLink { has_pothole } => match has_pothole {
            true => document::LinkType::WikiLinkPiped,
            false => document::LinkType::WikiLink,
        },
        _ => document::LinkType::Markdown,
    }
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::markdown::reader::{line_starts, MarkdownEventsReader};
    use crate::model::{document::*, InlineRange, Position};

    #[test]
    fn test_link_positions() {
        let content = indoc! {"
        [link](to)
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::Para(Para {
            line_range: 0..1,
            inlines: vec![DocumentInline::Link(Link {
                inlines: vec![DocumentInline::Str("link".to_string())],
                target: Target {
                    url: "to".to_string(),
                    title: String::default(),
                },
                attr: Default::default(),
                title: String::default(),
                inline_range: InlineRange {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 10,
                    },
                },
                link_type: LinkType::Markdown,
            })],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_link_position_inside_text() {
        let content = indoc! {"
            para

            text [link](to) text

            para
            "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Para(Para {
                line_range: 0..1,
                inlines: vec![DocumentInline::Str("para".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 2..3,
                inlines: vec![
                    DocumentInline::Str("text ".to_string()),
                    DocumentInline::Link(Link {
                        inlines: vec![DocumentInline::Str("link".to_string())],
                        target: Target {
                            url: "to".to_string(),
                            title: String::default(),
                        },
                        attr: Default::default(),
                        title: String::default(),
                        inline_range: InlineRange {
                            start: Position {
                                line: 2,
                                character: 5,
                            },
                            end: Position {
                                line: 2,
                                character: 15,
                            },
                        },
                        link_type: LinkType::Markdown,
                    }),
                    DocumentInline::Str(" text".to_string()),
                ],
            }),
            DocumentBlock::Para(Para {
                line_range: 4..5,
                inlines: vec![DocumentInline::Str("para".to_string())],
            }),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_link_position_inside_text_crlf() {
        let content = "para\r\n\r\ntext [link](to) text\r\n\r\npara\r\n";
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Para(Para {
                line_range: 0..1,
                inlines: vec![DocumentInline::Str("para".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 2..3,
                inlines: vec![
                    DocumentInline::Str("text ".to_string()),
                    DocumentInline::Link(Link {
                        inlines: vec![DocumentInline::Str("link".to_string())],
                        target: Target {
                            url: "to".to_string(),
                            title: String::default(),
                        },
                        attr: Default::default(),
                        title: String::default(),
                        inline_range: InlineRange {
                            start: Position {
                                line: 2,
                                character: 5,
                            },
                            end: Position {
                                line: 2,
                                character: 15,
                            },
                        },
                        link_type: LinkType::Markdown,
                    }),
                    DocumentInline::Str(" text".to_string()),
                ],
            }),
            DocumentBlock::Para(Para {
                line_range: 4..5,
                inlines: vec![DocumentInline::Str("para".to_string())],
            }),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_link_position_after_astral_character() {
        let content = "\u{1F5FA} [link](to)\n";
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::Para(Para {
            line_range: 0..1,
            inlines: vec![
                DocumentInline::Str("\u{1F5FA} ".to_string()),
                DocumentInline::Link(Link {
                    inlines: vec![DocumentInline::Str("link".to_string())],
                    target: Target {
                        url: "to".to_string(),
                        title: String::default(),
                    },
                    attr: Default::default(),
                    title: String::default(),
                    inline_range: InlineRange {
                        start: Position {
                            line: 0,
                            character: 3,
                        },
                        end: Position {
                            line: 0,
                            character: 13,
                        },
                    },
                    link_type: LinkType::Markdown,
                }),
            ],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_list_nested_item_positions() {
        let content = indoc! {"
        - line1
          1.  line2
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::BulletList(BulletList {
            line_range: 0..2,
            items: vec![vec![
                DocumentBlock::Para(Para {
                    line_range: 0..1,
                    inlines: vec![DocumentInline::Str("line1".to_string())],
                }),
                DocumentBlock::OrderedList(OrderedList {
                    line_range: 1..2,
                    items: vec![vec![DocumentBlock::Para(Para {
                        line_range: 1..2,
                        inlines: vec![DocumentInline::Str("line2".to_string())],
                    })]],
                }),
            ]],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_list_item_positions() {
        let content = indoc! {"
        - line1
        - line1
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::BulletList(BulletList {
            line_range: 0..2,
            items: vec![
                vec![DocumentBlock::Para(Para {
                    line_range: 0..1,
                    inlines: vec![DocumentInline::Str("line1".to_string())],
                })],
                vec![DocumentBlock::Para(Para {
                    line_range: 1..2,
                    inlines: vec![DocumentInline::Str("line1".to_string())],
                })],
            ],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_one_header_position() {
        let content = indoc! {"
        # test"};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::Header(Header {
            line_range: 0..1,
            inlines: vec![DocumentInline::Str("test".to_string())],
            level: 1,
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_positions_with_empty_frontmatter() {
        let content = indoc! {"
        ---
        ---
        # line1

        text [link](to) text
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Header(Header {
                line_range: 2..3,
                inlines: vec![DocumentInline::Str("line1".to_string())],
                level: 1,
            }),
            DocumentBlock::Para(Para {
                line_range: 4..5,
                inlines: vec![
                    DocumentInline::Str("text ".to_string()),
                    DocumentInline::Link(Link {
                        inlines: vec![DocumentInline::Str("link".to_string())],
                        target: Target {
                            url: "to".to_string(),
                            title: String::default(),
                        },
                        attr: Default::default(),
                        title: String::default(),
                        inline_range: InlineRange {
                            start: Position {
                                line: 4,
                                character: 5,
                            },
                            end: Position {
                                line: 4,
                                character: 15,
                            },
                        },
                        link_type: LinkType::Markdown,
                    }),
                    DocumentInline::Str(" text".to_string()),
                ],
            }),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_header_positions() {
        let content = indoc! {"
        # line1

        ## line2
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Header(Header {
                line_range: 0..1,
                inlines: vec![DocumentInline::Str("line1".to_string())],
                level: 1,
            }),
            DocumentBlock::Header(Header {
                line_range: 2..3,
                inlines: vec![DocumentInline::Str("line2".to_string())],
                level: 2,
            }),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_block_line_positions() {
        let content = indoc! {"
        line1

        line2

        line3


        line4
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Para(Para {
                line_range: 0..1,
                inlines: vec![DocumentInline::Str("line1".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 2..3,
                inlines: vec![DocumentInline::Str("line2".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 4..5,
                inlines: vec![DocumentInline::Str("line3".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 7..8,
                inlines: vec![DocumentInline::Str("line4".to_string())],
            }),
        ];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_ranges() {
        let content = indoc! {"
        1

        2

        3
        "};
        let ranges = line_starts(content);
        assert_eq!(vec![0, 2, 3, 5, 6, 8], ranges);
    }

    #[test]
    fn test_link_position_after_multibyte_text() {
        let content = "\u{03B1}\u{03B2} [link](to)\n";
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let link = actual[0]
            .child_inlines()
            .into_iter()
            .find(|i| i.is_link())
            .unwrap();
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 3,
                },
                end: Position {
                    line: 0,
                    character: 13,
                },
            },
            link.inline_range()
        );
    }

    #[test]
    fn test_wiki_link_position_after_multibyte_text() {
        let content = "\u{03B1}\u{03B2} [[target]]\n";
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let link = actual[0]
            .child_inlines()
            .into_iter()
            .find(|i| i.is_link())
            .unwrap();
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 3,
                },
                end: Position {
                    line: 0,
                    character: 13,
                },
            },
            link.inline_range()
        );
    }

    fn key_range_of(content: &str) -> InlineRange {
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        actual[0]
            .child_inlines()
            .into_iter()
            .find(|i| i.is_link())
            .unwrap()
            .key_range()
            .unwrap()
    }

    #[test]
    fn test_markdown_link_key_range() {
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 0,
                    character: 9,
                },
            },
            key_range_of("[link](to)\n")
        );
    }

    #[test]
    fn test_wiki_link_key_range() {
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 8,
                },
            },
            key_range_of("[[target]]\n")
        );
    }

    #[test]
    fn test_wiki_link_piped_key_range() {
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 2,
                },
                end: Position {
                    line: 0,
                    character: 8,
                },
            },
            key_range_of("[[target|display]]\n")
        );
    }

    #[test]
    fn test_wiki_link_key_range_after_multibyte_text() {
        assert_eq!(
            InlineRange {
                start: Position {
                    line: 0,
                    character: 5,
                },
                end: Position {
                    line: 0,
                    character: 11,
                },
            },
            key_range_of("\u{03B1}\u{03B2} [[target]]\n")
        );
    }

    #[test]
    fn test_link_position_on_late_line() {
        let mut content = String::new();
        for _ in 0..500 {
            content.push_str("\u{03B1}\u{03B2} filler text\n\n");
        }
        content.push_str("\u{03B1}\u{03B2} [link](to)\n");

        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(&content);
        let link = actual
            .last()
            .unwrap()
            .child_inlines()
            .into_iter()
            .find(|i| i.is_link())
            .unwrap();

        assert_eq!(
            InlineRange {
                start: Position {
                    line: 1000,
                    character: 3,
                },
                end: Position {
                    line: 1000,
                    character: 13,
                },
            },
            link.inline_range()
        );
    }

    #[test]
    fn test_large_document_parses_quickly() {
        let mut content = String::new();
        for _ in 0..4000 {
            content.push_str(
                "Some longer paragraph text that contains a [link](target-url) inside it.\n\n",
            );
        }

        let mut reader = MarkdownEventsReader::new();
        let start = std::time::Instant::now();
        let actual = reader.read(&content);
        let elapsed = start.elapsed();

        assert_eq!(4000, actual.len());
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "parsing took {elapsed:?}, expected well under 5s"
        );
    }
}
