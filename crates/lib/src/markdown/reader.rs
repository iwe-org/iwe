use std::iter::once;
use std::ops::Range;

use itertools::Itertools;
use pulldown_cmark::{CodeBlockKind, Tag, TagEnd};
use pulldown_cmark::{Event::*, Parser};

use crate::model::document::*;
use crate::model::*;

pub struct MarkdownEventsReader {
    inlines_pos_stack: Vec<LineRange>,
    inlines_stack: Vec<DocumentInline>,
    blocks_stack: Vec<DocumentBlock>,
    blocks: DocumentBlocks,
    line_starts: Vec<usize>,
}

impl MarkdownEventsReader {
    pub fn new() -> MarkdownEventsReader {
        MarkdownEventsReader {
            inlines_pos_stack: Vec::new(),
            inlines_stack: Vec::new(),
            blocks_stack: Vec::new(),
            blocks: Vec::new(),
            line_starts: Vec::new(),
        }
    }

    pub fn top_block(&mut self) -> &mut DocumentBlock {
        self.blocks_stack.last_mut().expect("to have element")
    }

    pub fn read(&mut self, content: &str) -> DocumentBlocks {
        let mut iter = Parser::new(content).into_offset_iter();
        self.line_starts = line_starts(content);

        while let Some((event, range)) = iter.next() {
            match event {
                Start(tag) => {
                    self.start_tag(tag, range);
                }
                End(tag) => {
                    self.end_tag(tag, range);
                }
                Text(text) => match self.top_block() {
                    DocumentBlock::CodeBlock(code_block) => {
                        code_block.text = format!("{}{}", code_block.text, text.to_string())
                    }
                    DocumentBlock::RawBlock(block) => block.text = text.to_string(),
                    default => {
                        self.push_inline(
                            DocumentInline::Str(text.to_string()),
                            self.to_line_ranges(range),
                        );
                        self.pop_inline();
                    }
                },
                Code(text) => {
                    self.push_inline(
                        DocumentInline::Code(document::Code {
                            attr: Attributes::default(),
                            text: text.to_string(),
                        }),
                        self.to_line_ranges(range),
                    );
                    self.pop_inline();
                }
                InlineMath(cow_str) => {
                    self.push_inline(
                        DocumentInline::Math(Math {
                            math_type: MathType::InlineMath,
                            content: cow_str.to_string(),
                        }),
                        self.to_line_ranges(range),
                    );
                    self.pop_inline();
                }
                DisplayMath(cow_str) => todo!(),
                Html(cow_str) => todo!(),
                InlineHtml(cow_str) => todo!(),
                FootnoteReference(cow_str) => todo!(),
                SoftBreak => {}
                HardBreak => {}
                Rule => {
                    self.push_block(DocumentBlock::HorizontalRule(HorizontalRule {
                        line_range: self.to_line_ranges(range),
                    }));
                    self.pop_block();
                }
                TaskListMarker(_) => todo!(),
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
                    line_range: self.to_line_ranges(range),
                    inlines: vec![],
                }));
            }
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => self.push_block(DocumentBlock::Header(Header {
                line_range: self.to_line_ranges(range),
                level: level as u8,
                inlines: vec![],
            })),
            Tag::BlockQuote(block_quote_kind) => {
                self.push_block(DocumentBlock::BlockQuote(BlockQuote {
                    line_range: self.to_line_ranges(range),
                    blocks: Vec::new(),
                }))
            }
            Tag::CodeBlock(code_block_kind) => {
                self.push_block(DocumentBlock::CodeBlock(CodeBlock {
                    line_range: self.to_line_ranges(range),
                    lang: match code_block_kind {
                        CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                        CodeBlockKind::Indented => None,
                    },
                    text: "".to_string(),
                }))
            }
            Tag::HtmlBlock => todo!(),
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
            Tag::FootnoteDefinition(str) => todo!(),
            Tag::DefinitionList => todo!(),
            Tag::DefinitionListTitle => todo!(),
            Tag::DefinitionListDefinition => todo!(),
            Tag::Table(vec) => todo!(),
            Tag::TableHead => todo!(),
            Tag::TableRow => todo!(),
            Tag::TableCell => todo!(),
            Tag::Emphasis => {
                self.push_inline(
                    DocumentInline::Emph(Emph { inlines: vec![] }),
                    self.to_line_ranges(range),
                );
            }
            Tag::Strong => {
                self.push_inline(
                    DocumentInline::Strong(Strong { inlines: vec![] }),
                    self.to_line_ranges(range),
                );
            }
            Tag::Strikethrough => {
                self.push_inline(
                    DocumentInline::Strikeout(Strikeout { inlines: vec![] }),
                    self.to_line_ranges(range),
                );
            }
            Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            } => {
                self.push_inline(
                    DocumentInline::Link(Link {
                        inlines: vec![],
                        target: Target {
                            url: dest_url.to_string(),
                            title: title.to_string(),
                        },
                        attr: Default::default(),
                    }),
                    self.to_line_ranges(range),
                );
            }
            Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            } => {
                self.push_inline(
                    DocumentInline::Image(Image {
                        inlines: vec![],
                        target: Target {
                            url: dest_url.to_string(),
                            title: title.to_string(),
                        },
                        attr: Default::default(),
                    }),
                    self.to_line_ranges(range),
                );
            }
            Tag::MetadataBlock(metadata_block_kind) => todo!(),
        }
    }

    fn end_tag(&mut self, tag: TagEnd, range: Range<usize>) {
        match tag {
            TagEnd::Paragraph => {
                self.pop_block();
            }
            TagEnd::Heading(heading_level) => self.pop_block(),
            TagEnd::BlockQuote(block_quote_kind) => self.pop_block(),
            TagEnd::CodeBlock => {
                self.pop_block();
            }
            TagEnd::HtmlBlock => todo!(),
            TagEnd::List(_) => {
                self.pop_block();
            }
            TagEnd::Item => {}
            TagEnd::Emphasis => self.pop_inline(),
            TagEnd::Strong => self.pop_inline(),
            TagEnd::Strikethrough => self.pop_inline(),
            TagEnd::Link => self.pop_inline(),
            TagEnd::DefinitionList => todo!(),
            TagEnd::DefinitionListDefinition => todo!(),
            TagEnd::DefinitionListTitle => todo!(),
            TagEnd::FootnoteDefinition => todo!(),
            TagEnd::Image => self.pop_inline(),
            TagEnd::MetadataBlock(metadata_block_kind) => todo!(),
            TagEnd::Table => todo!(),
            TagEnd::TableCell => todo!(),
            TagEnd::TableHead => todo!(),
            TagEnd::TableRow => todo!(),
        }
    }

    fn to_line_ranges(&self, range: Range<usize>) -> LineRange {
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

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use crate::markdown::reader::{line_starts, MarkdownEventsReader};
    use crate::model::document::*;

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
        - line1
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
