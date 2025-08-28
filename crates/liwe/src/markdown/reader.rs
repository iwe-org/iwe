use std::iter::once;
use std::ops::Range;
use std::sync::LazyLock;

use crate::model::node::ColumnAlignment;
use pulldown_cmark::{Alignment, CodeBlockKind, LinkType, Options, Tag, TagEnd};
use pulldown_cmark::{CowStr, Event::*, Parser};
use regex::Regex;

use crate::model::document::*;
use crate::model::*;

pub struct MarkdownEventsReader {
    inlines_pos_stack: Vec<LineRange>,
    inlines_stack: Vec<DocumentInline>,
    blocks_stack: Vec<DocumentBlock>,
    blocks: DocumentBlocks,
    line_starts: Vec<usize>,
    metadata_block: bool,
    metadata: Option<String>,
}

impl MarkdownEventsReader {
    pub fn new() -> MarkdownEventsReader {
        MarkdownEventsReader {
            inlines_pos_stack: Vec::new(),
            inlines_stack: Vec::new(),
            blocks_stack: Vec::new(),
            blocks: Vec::new(),
            line_starts: Vec::new(),
            metadata_block: false,
            metadata: None,
        }
    }

    pub fn blocks(&self) -> Vec<DocumentBlock> {
        self.blocks.clone()
    }

    pub fn metadata(&self) -> Option<String> {
        self.metadata.clone()
    }

    pub fn top_block(&mut self) -> &mut DocumentBlock {
        self.blocks_stack.last_mut().expect("to have element")
    }

    pub fn read(&mut self, content: &str) -> DocumentBlocks {
        let mut iter = Parser::new_ext(
            content,
            Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
                | Options::ENABLE_WIKILINKS
                | Options::ENABLE_TABLES
                | Options::ENABLE_TASKLISTS,
        )
        .into_offset_iter();
        self.line_starts = line_starts(content);

        while let Some((event, range)) = iter.next() {
            match event {
                Start(tag) => {
                    self.start_tag(tag, range);
                }
                End(tag) => {
                    self.end_tag(tag, range);
                }
                Text(text) => {
                    self.text(text, range);
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
                Html(_) => {}
                InlineHtml(text) => {
                    self.push_inline(
                        DocumentInline::Str(text.to_string()),
                        self.to_line_range(range),
                    );
                    self.pop_inline();
                }
                FootnoteReference(_) => {}
                SoftBreak => {}
                HardBreak => {}
                Rule => {
                    self.push_block(DocumentBlock::HorizontalRule(HorizontalRule {
                        line_range: self.to_line_range(range),
                    }));
                    self.pop_block();
                }
                TaskListMarker(checked) => {
                    self.push_inline(DocumentInline::Task(checked), self.to_line_range(range));
                    self.pop_inline();
                }
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
        let inline = self.inlines_stack.pop().unwrap();
        let pos = self.inlines_pos_stack.pop().unwrap();

        if self.inlines_stack.len() == 0 {
            self.top_block().append_inline(inline, pos);
            return;
        }

        self.inlines_stack.last_mut().unwrap().apppen(inline);
    }

    fn pop_block(&mut self) {
        let block = self.blocks_stack.pop().unwrap();

        if self.blocks_stack.len() == 0 {
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
            Tag::HtmlBlock => {}
            Tag::List(num) => {
                if num.is_some() {
                    self.push_block(DocumentBlock::OrderedList(OrderedList { items: vec![] }));
                } else {
                    self.push_block(DocumentBlock::BulletList(BulletList { items: vec![] }));
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
                            url: dest_url.to_string(),
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
                dest_url, title, ..
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
            TagEnd::HtmlBlock => {}
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

    fn text(&mut self, text: CowStr, range: Range<usize>) {
        // https://help.obsidian.md/tags#Tag+format
        static RE_TAG: LazyLock<Regex> =
            // Check for tag ending with whitespace or line end is not included, because we would
            // loose tags with a single whitespace in between.
            LazyLock::new(|| {
                Regex::new(r"(?:^|[\t ])#([a-zA-Z0-9_\-/]+)").expect("Invalid regex")
            });

        if !self.metadata_block {
            match self.top_block() {
                DocumentBlock::CodeBlock(code_block) => {
                    code_block.text = format!("{}{}", code_block.text, text.to_string())
                }
                DocumentBlock::RawBlock(block) => block.text = text.to_string(),
                _ => {
                    let mut pos = 0;
                    for tag in RE_TAG
                        .captures_iter(text.chars().as_str())
                        .filter_map(|c| c.get(1))
                    {
                        // Text *before* match
                        if tag.start() > pos + 1 {
                            self.push_inline(
                                DocumentInline::Str(text[pos..tag.start() - 1].to_string()),
                                self.to_line_range(range.clone()),
                            );
                            self.pop_inline();
                        }
                        // Hashtag
                        self.push_inline(
                            DocumentInline::Tag(tag.as_str().to_string()),
                            self.to_line_range(range.clone()),
                        );
                        self.pop_inline();
                        pos = tag.end();
                    }
                    // Remaining text
                    if pos < text.len() {
                        self.push_inline(
                            DocumentInline::Str(text[pos..text.len()].to_string()),
                            self.to_line_range(range),
                        );
                        self.pop_inline();
                    }
                }
            }
        } else {
            self.metadata = Some(text.to_string());
        }
    }

    fn to_inline_range(&self, range: Range<usize>) -> InlineRange {
        let mut start = 0;
        let mut start_char = 0;
        let mut end = 0;
        let mut end_char = 0;

        for (line, &line_start) in self.line_starts.iter().enumerate() {
            if line_start <= range.start {
                start = line;
                start_char = range.start - line_start;
            }
            if line_start <= range.end {
                end = line;
                end_char = range.end - line_start;
            }
        }

        Position {
            line: start,
            character: start_char,
        }..Position {
            line: end,
            character: end_char,
        }
    }

    fn to_line_range(&self, range: Range<usize>) -> LineRange {
        let mut start = 0;
        let mut end = 0;

        for (line, &line_start) in self.line_starts.iter().enumerate() {
            if line_start <= range.start {
                start = line;
            }
            if line_start <= range.end {
                end = line;
            }
        }

        if start == end {
            end += 1;
        }

        start..end
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
        _ => document::LinkType::Regular,
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
                link_type: LinkType::Regular,
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
                        link_type: LinkType::Regular,
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
    fn test_list_nested_item_positions() {
        let content = indoc! {"
        - line1
          1.  line2
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::BulletList(BulletList {
            items: vec![vec![
                DocumentBlock::Para(Para {
                    line_range: 0..1,
                    inlines: vec![DocumentInline::Str("line1".to_string())],
                }),
                DocumentBlock::OrderedList(OrderedList {
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
        - line2
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::BulletList(BulletList {
            items: vec![
                vec![DocumentBlock::Para(Para {
                    line_range: 0..1,
                    inlines: vec![DocumentInline::Str("line1".to_string())],
                })],
                vec![DocumentBlock::Para(Para {
                    line_range: 1..2,
                    inlines: vec![DocumentInline::Str("line2".to_string())],
                })],
            ],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_task_items() {
        let content = indoc! {"
        - [ ] todo1
              second line
        - [x] todo2
        - no task
        - [X] todo3
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![DocumentBlock::BulletList(BulletList {
            items: vec![
                vec![DocumentBlock::Para(Para {
                    line_range: 0..1,
                    inlines: vec![
                        DocumentInline::Task(false),
                        DocumentInline::Str("todo1".to_string()),
                        DocumentInline::Str("second line".to_string()),
                    ],
                })],
                vec![DocumentBlock::Para(Para {
                    line_range: 2..3,
                    inlines: vec![
                        DocumentInline::Task(true),
                        DocumentInline::Str("todo2".to_string()),
                    ],
                })],
                vec![DocumentBlock::Para(Para {
                    line_range: 3..4,
                    inlines: vec![DocumentInline::Str("no task".to_string())],
                })],
                vec![DocumentBlock::Para(Para {
                    line_range: 4..5,
                    inlines: vec![
                        DocumentInline::Task(true),
                        DocumentInline::Str("todo3".to_string()),
                    ],
                })],
            ],
        })];

        assert_eq!(expected, actual);
    }

    #[test]
    fn test_hastags() {
        let content = indoc! {"
        Text #tag1 end
        #tag2
        no#tag
        no # tag
        #invalid#tag
        #multiple #tags in line
        #nested/tag
        #tag_with-delimiters
        #invalid.tag
        - [ ] todo #tag3
        - [x] #tag4
        # Tag in #title
        ## #title2
        `ignored code #tag`
        End #tag5
        "};
        let mut reader = MarkdownEventsReader::new();
        let actual = reader.read(content);
        let expected = vec![
            DocumentBlock::Para(Para {
                line_range: 0..9,
                inlines: vec![
                    DocumentInline::Str("Text ".to_string()),
                    DocumentInline::Tag("tag1".to_string()),
                    DocumentInline::Str(" end".to_string()),
                    DocumentInline::Tag("tag2".to_string()),
                    DocumentInline::Str("no#tag".to_string()),
                    DocumentInline::Str("no # tag".to_string()),
                    DocumentInline::Tag("invalid".to_string()),
                    DocumentInline::Str("#tag".to_string()),
                    DocumentInline::Tag("multiple".to_string()),
                    DocumentInline::Str(" ".to_string()),
                    DocumentInline::Tag("tags".to_string()),
                    DocumentInline::Str(" in line".to_string()),
                    DocumentInline::Tag("nested/tag".to_string()),
                    DocumentInline::Tag("tag_with-delimiters".to_string()),
                    DocumentInline::Tag("invalid".to_string()),
                    DocumentInline::Str(".tag".to_string()),
                ],
            }),
            DocumentBlock::BulletList(BulletList {
                items: vec![
                    vec![DocumentBlock::Para(Para {
                        line_range: 9..10,
                        inlines: vec![
                            DocumentInline::Task(false),
                            DocumentInline::Str("todo ".to_string()),
                            DocumentInline::Tag("tag3".to_string()),
                        ],
                    })],
                    vec![DocumentBlock::Para(Para {
                        line_range: 10..11,
                        inlines: vec![
                            DocumentInline::Task(true),
                            DocumentInline::Tag("tag4".to_string()),
                        ],
                    })],
                ],
            }),
            DocumentBlock::Header(Header {
                line_range: 11..12,
                level: 1,
                inlines: vec![
                    DocumentInline::Str("Tag in ".to_string()),
                    DocumentInline::Tag("title".to_string()),
                ],
            }),
            DocumentBlock::Header(Header {
                line_range: 12..13,
                level: 2,
                inlines: vec![DocumentInline::Tag("title2".to_string())],
            }),
            DocumentBlock::Para(Para {
                line_range: 13..15,
                inlines: vec![
                    DocumentInline::Code(Code {
                        attr: Attributes {
                            identifier: "".to_string(),
                            classes: vec![],
                            attributes: vec![],
                            inline_range: Position {
                                line: 0,
                                character: 0,
                            }..Position {
                                line: 0,
                                character: 0,
                            },
                        },
                        text: "ignored code #tag".to_string(),
                        inline_range: Position {
                            line: 13,
                            character: 0,
                        }..Position {
                            line: 13,
                            character: 19,
                        },
                    }),
                    DocumentInline::Str("End ".to_string()),
                    DocumentInline::Tag("tag5".to_string()),
                ],
            }),
        ];
        assert_eq!(expected, actual);

        let doc = Document {
            blocks: reader.blocks(),
            metadata: reader.metadata(),
        };
        assert_eq!(
            doc.tags().collect::<Vec<String>>(),
            vec![
                "tag1",
                "tag2",
                "invalid",
                "multiple",
                "tags",
                "nested/tag",
                "tag_with-delimiters",
                "invalid",
                "tag3",
                "tag4",
                "title",
                "title2",
                "tag5"
            ]
        );
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
}
