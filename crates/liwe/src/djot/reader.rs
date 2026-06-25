use std::iter::once;
use std::ops::Range;

use jotdown::{Alignment, Container, Event, ListKind, Parser};
use serde_yaml::{Mapping, Value};

use crate::model::config::DjotOptions;
use crate::model::document::*;
use crate::model::node::ColumnAlignment;
use crate::model::*;

pub struct DjotEventsReader {
    options: DjotOptions,
    inlines_pos_stack: Vec<LineRange>,
    inlines_stack: Vec<DocumentInline>,
    blocks_stack: Vec<DocumentBlock>,
    blocks: DocumentBlocks,
    content: String,
    line_starts: Vec<usize>,
    offset: usize,
    frontmatter: Option<Mapping>,
    pending_checkbox: Option<bool>,
}

impl Default for DjotEventsReader {
    fn default() -> Self {
        Self::new()
    }
}

impl DjotEventsReader {
    pub fn new() -> DjotEventsReader {
        DjotEventsReader {
            options: DjotOptions::default(),
            inlines_pos_stack: Vec::new(),
            inlines_stack: Vec::new(),
            blocks_stack: Vec::new(),
            blocks: Vec::new(),
            content: String::new(),
            line_starts: Vec::new(),
            offset: 0,
            frontmatter: None,
            pending_checkbox: None,
        }
    }

    pub fn new_with_options(options: &DjotOptions) -> DjotEventsReader {
        DjotEventsReader {
            options: options.clone(),
            ..DjotEventsReader::new()
        }
    }

    pub fn blocks(&self) -> Vec<DocumentBlock> {
        self.blocks.clone()
    }

    pub fn frontmatter(&self) -> Option<Mapping> {
        self.frontmatter.clone()
    }

    fn top_block(&mut self) -> &mut DocumentBlock {
        self.blocks_stack
            .last_mut()
            .unwrap_or_else(|| panic!("parse djot:\n{}", &self.content))
    }

    pub fn read(&mut self, content: &str) -> DocumentBlocks {
        let normalized = normalize_line_endings(content);
        self.content = normalized.clone();
        self.line_starts = line_starts(&normalized);

        let (body, frontmatter, offset) = strip_frontmatter(&normalized);
        self.frontmatter = frontmatter;
        self.offset = offset;

        let events: Vec<(Event, Range<usize>)> = Parser::new(&body).into_offset_iter().collect();

        for (event, range) in events {
            let range = (range.start + self.offset)..(range.end + self.offset);
            match event {
                Event::Start(container, attrs) => self.start_container(container, attrs, range),
                Event::End(container) => self.end_container(container),
                Event::Str(text) => self.text(&text, range),
                Event::Symbol(text) => {
                    self.push_inline(
                        DocumentInline::Symbol(Symbol {
                            text: text.to_string(),
                            inline_range: self.to_inline_range(range.clone()),
                        }),
                        self.to_line_range(range),
                    );
                    self.pop_inline();
                }
                Event::FootnoteReference(_) => {}
                Event::Softbreak => {
                    self.push_inline(
                        DocumentInline::Space(Space {
                            inline_range: InlineRange::default(),
                        }),
                        self.to_line_range(range),
                    );
                    self.pop_inline();
                }
                Event::Hardbreak => {
                    let inline = if self.options.formatting.preserve_line_breaks() {
                        DocumentInline::LineBreak(LineBreak {
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
                Event::Escape => {}
                Event::Blankline => {}
                Event::ThematicBreak(_) => {
                    self.push_block(DocumentBlock::HorizontalRule(HorizontalRule {
                        line_range: self.to_line_range(range),
                    }));
                    self.pop_block();
                }
                Event::Attributes(_) => {}
                Event::LeftSingleQuote => self.text("\u{2018}", range),
                Event::RightSingleQuote => self.text("\u{2019}", range),
                Event::LeftDoubleQuote => self.text("\u{201C}", range),
                Event::RightDoubleQuote => self.text("\u{201D}", range),
                Event::Ellipsis => self.text("\u{2026}", range),
                Event::EnDash => self.text("\u{2013}", range),
                Event::EmDash => self.text("\u{2014}", range),
                Event::NonBreakingSpace => self.text("\u{00A0}", range),
            }
        }

        self.blocks.clone()
    }

    fn text(&mut self, text: &str, range: Range<usize>) {
        if self.append_verbatim_text(text) {
            return;
        }
        match self.blocks_stack.last_mut() {
            Some(DocumentBlock::CodeBlock(code_block)) => {
                code_block.text = format!("{}{}", code_block.text, text);
            }
            Some(DocumentBlock::RawBlock(raw_block)) => {
                raw_block.text = format!("{}{}", raw_block.text, text);
            }
            _ => {
                self.push_inline(
                    DocumentInline::Str(text.to_string()),
                    self.to_line_range(range),
                );
                self.pop_inline();
            }
        }
    }

    fn append_verbatim_text(&mut self, text: &str) -> bool {
        match self.inlines_stack.last_mut() {
            Some(DocumentInline::Code(code)) => {
                code.text = format!("{}{}", code.text, text);
                true
            }
            Some(DocumentInline::Math(math)) => {
                math.content = format!("{}{}", math.content, text);
                true
            }
            Some(DocumentInline::RawInline(raw)) => {
                raw.content = format!("{}{}", raw.content, text);
                true
            }
            _ => false,
        }
    }

    fn push_inline(&mut self, inline: DocumentInline, lines_range: LineRange) {
        self.inlines_stack.push(inline);
        self.inlines_pos_stack.push(lines_range);
    }

    fn inject_checkbox(&mut self, checked: bool, range: Range<usize>) {
        let space = || {
            DocumentInline::Space(Space {
                inline_range: InlineRange::default(),
            })
        };
        let mark = if checked {
            DocumentInline::Str("x".to_string())
        } else {
            space()
        };
        for inline in [
            DocumentInline::Str("[".to_string()),
            mark,
            DocumentInline::Str("]".to_string()),
            space(),
        ] {
            self.push_inline(inline, self.to_line_range(range.clone()));
            self.pop_inline();
        }
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
            if let Some(block) = self.blocks_stack.last_mut() {
                block.append_inline(inline, pos);
            }
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

    fn start_container(
        &mut self,
        container: Container,
        attrs: jotdown::Attributes,
        range: Range<usize>,
    ) {
        match container {
            Container::Document | Container::Section { .. } | Container::Div { .. } => {}
            Container::Paragraph => {
                self.push_block(DocumentBlock::Para(Para {
                    line_range: self.to_line_range(range.clone()),
                    inlines: vec![],
                }));
                if let Some(checked) = self.pending_checkbox.take() {
                    self.inject_checkbox(checked, range);
                }
            }
            Container::Heading { level, .. } => {
                self.push_block(DocumentBlock::Header(Header {
                    line_range: self.to_line_range(range),
                    level: level.min(6) as u8,
                    inlines: vec![],
                }));
            }
            Container::Blockquote => {
                self.push_block(DocumentBlock::BlockQuote(BlockQuote {
                    line_range: self.to_line_range(range),
                    blocks: Vec::new(),
                }));
            }
            Container::CodeBlock { language } => {
                self.push_block(DocumentBlock::CodeBlock(CodeBlock {
                    line_range: self.to_line_range(range),
                    lang: Some(language.to_string()).filter(|l| !l.is_empty()),
                    text: String::new(),
                }));
            }
            Container::RawBlock { format } => {
                self.push_block(DocumentBlock::RawBlock(RawBlock {
                    line_range: self.to_line_range(range),
                    format: format.to_string(),
                    text: String::new(),
                }));
            }
            Container::List { kind, .. } => {
                let line_range = self.to_line_range(range);
                match kind {
                    ListKind::Ordered { .. } => {
                        self.push_block(DocumentBlock::OrderedList(OrderedList {
                            line_range,
                            items: vec![],
                        }));
                    }
                    _ => {
                        self.push_block(DocumentBlock::BulletList(BulletList {
                            line_range,
                            items: vec![],
                        }));
                    }
                }
            }
            Container::ListItem => {
                self.top_block().append_item();
            }
            Container::TaskListItem { checked } => {
                self.top_block().append_item();
                self.pending_checkbox = Some(checked);
            }
            Container::Table => {
                self.push_block(DocumentBlock::Table(Table {
                    line_range: self.to_line_range(range),
                    alignment: vec![],
                    rows: vec![],
                    header: vec![],
                }));
            }
            Container::TableRow { head } => {
                if !head {
                    self.top_block().append_row();
                }
            }
            Container::TableCell { alignment, head } => {
                if head {
                    if let DocumentBlock::Table(table) = self.top_block() {
                        table.alignment.push(to_column_alignment(alignment));
                    }
                }
                self.top_block().append_cell();
            }
            Container::Emphasis => self.push_inline(
                DocumentInline::Emph(Emph {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Strong => self.push_inline(
                DocumentInline::Strong(Strong {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Delete => self.push_inline(
                DocumentInline::Delete(Delete {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Insert => self.push_inline(
                DocumentInline::Insert(Insert {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Mark => self.push_inline(
                DocumentInline::Mark(Mark {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Superscript => self.push_inline(
                DocumentInline::Superscript(Superscript {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Subscript => self.push_inline(
                DocumentInline::Subscript(Subscript {
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Span => self.push_inline(
                DocumentInline::Span(Span {
                    attr: to_document_attributes(&attrs),
                    inlines: vec![],
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Verbatim => self.push_inline(
                DocumentInline::Code(Code {
                    attr: Attributes::default(),
                    text: String::new(),
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Math { display } => self.push_inline(
                DocumentInline::Math(Math {
                    math_type: if display {
                        MathType::DisplayMath
                    } else {
                        MathType::InlineMath
                    },
                    content: String::new(),
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::RawInline { format } => self.push_inline(
                DocumentInline::RawInline(RawInline {
                    format: Format(format.to_string()),
                    content: String::new(),
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Link(url, _) => self.push_inline(
                DocumentInline::Link(Link {
                    inlines: vec![],
                    target: Target {
                        url: normalize_url(url.as_ref(), &self.options.refs_extension),
                        title: String::new(),
                    },
                    title: String::new(),
                    attr: Default::default(),
                    inline_range: self.to_inline_range(range.clone()),
                    link_type: document::LinkType::Markdown,
                }),
                self.to_line_range(range),
            ),
            Container::Image(url, _) => self.push_inline(
                DocumentInline::Image(Image {
                    inlines: vec![],
                    target: Target {
                        url: url.to_string(),
                        title: String::new(),
                    },
                    attr: Default::default(),
                    inline_range: self.to_inline_range(range.clone()),
                }),
                self.to_line_range(range),
            ),
            Container::Footnote { .. }
            | Container::LinkDefinition { .. }
            | Container::DescriptionList
            | Container::DescriptionTerm
            | Container::DescriptionDetails
            | Container::Caption => {}
        }
    }

    fn end_container(&mut self, container: Container) {
        match container {
            Container::Document | Container::Section { .. } | Container::Div { .. } => {}
            Container::Paragraph
            | Container::Heading { .. }
            | Container::Blockquote
            | Container::CodeBlock { .. }
            | Container::RawBlock { .. }
            | Container::List { .. }
            | Container::Table => self.pop_block(),
            Container::ListItem | Container::TaskListItem { .. } => {
                self.pending_checkbox = None;
            }
            Container::TableRow { .. } | Container::TableCell { .. } | Container::Caption => {}
            Container::Emphasis
            | Container::Strong
            | Container::Delete
            | Container::Insert
            | Container::Mark
            | Container::Superscript
            | Container::Subscript
            | Container::Verbatim
            | Container::Math { .. }
            | Container::RawInline { .. }
            | Container::Link(..)
            | Container::Image(..)
            | Container::Span => self.pop_inline(),
            Container::Footnote { .. }
            | Container::LinkDefinition { .. }
            | Container::DescriptionList
            | Container::DescriptionTerm
            | Container::DescriptionDetails => {}
        }
    }

    fn to_inline_range(&self, range: Range<usize>) -> InlineRange {
        let start_line = self.line_index(range.start);
        let start_char = self.content[self.line_starts[start_line]..range.start]
            .encode_utf16()
            .count();

        let end_line = self.line_index(range.end);
        let end_char = self.content[self.line_starts[end_line]..range.end]
            .encode_utf16()
            .count();

        Position {
            line: start_line,
            character: start_char,
        }..Position {
            line: end_line,
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

        start..end
    }
}

fn to_document_attributes(attrs: &jotdown::Attributes) -> Attributes {
    let mut result = Attributes::default();
    for (kind, value) in attrs {
        match kind {
            jotdown::AttributeKind::Id => result.identifier = value.to_string(),
            jotdown::AttributeKind::Class => result.classes.push(value.to_string()),
            jotdown::AttributeKind::Pair { key } => {
                result.attributes.push((key.to_string(), value.to_string()))
            }
            jotdown::AttributeKind::Comment => {}
        }
    }
    result
}

fn to_column_alignment(alignment: Alignment) -> ColumnAlignment {
    match alignment {
        Alignment::Unspecified => ColumnAlignment::None,
        Alignment::Left => ColumnAlignment::Left,
        Alignment::Center => ColumnAlignment::Center,
        Alignment::Right => ColumnAlignment::Right,
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

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn strip_frontmatter(content: &str) -> (String, Option<Mapping>, usize) {
    let Some(rest) = content.strip_prefix("---\n") else {
        return (content.to_string(), None, 0);
    };
    let Some(end) = rest.find("\n---\n").or_else(|| {
        rest.strip_suffix("\n---")
            .map(|trimmed| trimmed.len())
            .filter(|_| rest.ends_with("\n---"))
    }) else {
        return (content.to_string(), None, 0);
    };
    let yaml = &rest[..end];
    let consumed = "---\n".len() + end + "\n---\n".len();
    let consumed = consumed.min(content.len());
    let body = content[consumed..].to_string();
    (body, Some(parse_frontmatter(yaml)), consumed)
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
