use crate::key::without_extension;
use crate::model;
use crate::model::graph::Inline;
use crate::model::{Key, Lang, LineRange};

use super::graph::ReferenceType;
use super::{InlineRange, Position};

pub struct Document {
    pub blocks: DocumentBlocks,
    pub metadata: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DocumentBlock {
    Plain(Plain),
    Para(Para),
    CodeBlock(CodeBlock),
    RawBlock(RawBlock),
    BlockQuote(BlockQuote),
    OrderedList(OrderedList),
    BulletList(BulletList),
    Header(Header),
    HorizontalRule(HorizontalRule),
    Div(Div),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum DocumentInline {
    Code(Code),
    Emph(Emph),
    Image(Image),
    LineBreak(LineBreak),
    Link(Link),
    Math(Math),
    RawInline(RawInline),
    SmallCaps(SmallCaps),
    SoftBreak(SoftBreak),
    Space(Space),
    Str(String),
    Strikeout(Strikeout),
    Strong(Strong),
    Subscript(Subscript),
    Superscript(Superscript),
    Underline(Underline),
}

impl Document {
    pub fn link_at(&self, position: Position) -> Option<DocumentInline> {
        self.block_at_position(position)
            .into_iter()
            .flat_map(|block| block.child_inlines())
            .find_map(|inline| inline.link_at_position(position))
    }

    fn block_at_position(&self, position: Position) -> Option<DocumentBlock> {
        self.blocks
            .iter()
            .find_map(|block| block.block_at_position(position))
    }
}

impl DocumentBlock {
    pub fn is_ref(&self) -> bool {
        match self {
            DocumentBlock::Para(para) => para.inlines.len() == 1 && para.inlines[0].is_ref(),
            _ => false,
        }
    }

    pub fn ref_title(&self) -> Option<String> {
        match self {
            DocumentBlock::Para(para) => para.inlines.first().and_then(|inline| inline.ref_title()),
            _ => None,
        }
    }

    pub fn ref_text(&self) -> Option<String> {
        match self {
            DocumentBlock::Para(para) => para
                .inlines
                .first()
                .map(|inline| inline.to_node_inline().to_plain_text()),
            _ => None,
        }
    }

    pub fn ref_key(&self) -> Option<String> {
        match self {
            DocumentBlock::Para(para) => para.inlines.first().and_then(|inline| inline.ref_key()),
            _ => None,
        }
    }

    pub fn ref_type(&self) -> Option<ReferenceType> {
        match self {
            DocumentBlock::Para(para) => para.inlines.first().and_then(|inline| inline.ref_type()),
            _ => None,
        }
    }

    pub fn is_container(&self) -> bool {
        match self {
            DocumentBlock::Plain(_) => false,
            DocumentBlock::Para(_) => false,
            DocumentBlock::CodeBlock(_) => false,
            DocumentBlock::RawBlock(_) => false,
            DocumentBlock::BlockQuote(_) => true,
            DocumentBlock::OrderedList(_) => true,
            DocumentBlock::BulletList(_) => true,
            DocumentBlock::Header(_) => false,
            DocumentBlock::HorizontalRule(_) => false,
            DocumentBlock::Div(_) => true,
        }
    }

    pub fn append_block(&mut self, block: DocumentBlock) {
        match self {
            DocumentBlock::BulletList(list) => {
                list.items.last_mut().unwrap().push(block);
            }
            DocumentBlock::OrderedList(list) => {
                list.items.last_mut().unwrap().push(block);
            }
            DocumentBlock::BlockQuote(quote) => {
                quote.blocks.push(block);
            }
            _ => panic!(),
        }
    }

    pub fn append_item(&mut self) {
        match self {
            DocumentBlock::BulletList(list) => {
                list.items.push(Vec::new());
            }
            DocumentBlock::OrderedList(list) => {
                list.items.push(Vec::new());
            }
            _ => panic!(),
        }
    }

    pub fn append_inline(&mut self, inline: DocumentInline, line_range: LineRange) {
        match self {
            DocumentBlock::Plain(plain) => plain.inlines.push(inline),
            DocumentBlock::Para(para) => para.inlines.push(inline),
            DocumentBlock::CodeBlock(_) => {}
            DocumentBlock::RawBlock(_) => {}
            DocumentBlock::BlockQuote(block_quote) => {
                if block_quote.blocks.is_empty() {
                    block_quote.blocks.push(DocumentBlock::Para(Para {
                        line_range: line_range.clone(),
                        inlines: Vec::new(),
                    }));
                }
                let last_block = block_quote.blocks.last_mut().unwrap();
                last_block.append_inline(inline, line_range);
            }
            DocumentBlock::OrderedList(list) => {
                let item = list.items.last_mut().unwrap();

                if item.is_empty() {
                    item.push(DocumentBlock::Para(Para {
                        line_range: line_range.clone(),
                        inlines: Vec::new(),
                    }));
                }
                let last_block = item.last_mut().unwrap();
                last_block.append_inline(inline, line_range.clone());
            }
            DocumentBlock::BulletList(list) => {
                let item = list.items.last_mut().unwrap();

                if item.is_empty() {
                    item.push(DocumentBlock::Para(Para {
                        line_range: line_range.clone(),
                        inlines: Vec::new(),
                    }));
                }
                let last_block = item.last_mut().unwrap();
                last_block.append_inline(inline, line_range.clone());
            }
            DocumentBlock::Header(header) => header.inlines.push(inline),
            DocumentBlock::HorizontalRule(_) => {}
            DocumentBlock::Div(_) => {}
        }
    }

    pub fn line_range(&self) -> LineRange {
        match self {
            DocumentBlock::Plain(plain) => plain.line_range.clone(),
            DocumentBlock::Para(para) => para.line_range.clone(),
            DocumentBlock::CodeBlock(code) => code.line_range.clone(),
            DocumentBlock::RawBlock(raw) => raw.line_range.clone(),
            DocumentBlock::BlockQuote(quote) => quote.line_range.clone(),
            DocumentBlock::OrderedList(list) => {
                list.items.first().unwrap().first().unwrap().line_range()
            }
            DocumentBlock::BulletList(list) => {
                list.items.first().unwrap().first().unwrap().line_range()
            }
            DocumentBlock::Header(header) => header.line_range.clone(),
            DocumentBlock::HorizontalRule(hr) => hr.line_range.clone(),
            DocumentBlock::Div(div) => div.line_range.clone(),
        }
    }

    fn child_blocks(&self) -> Vec<&DocumentBlock> {
        match self {
            DocumentBlock::Plain(_) => vec![],
            DocumentBlock::Para(_) => vec![],
            DocumentBlock::CodeBlock(_) => vec![],
            DocumentBlock::RawBlock(_) => vec![],
            DocumentBlock::BlockQuote(quote) => quote.blocks.iter().collect(),
            DocumentBlock::OrderedList(list) => list.items.iter().flat_map(|i| i).collect(),
            DocumentBlock::BulletList(list) => list.items.iter().flat_map(|i| i).collect(),
            DocumentBlock::Header(_) => vec![],
            DocumentBlock::HorizontalRule(_) => vec![],
            DocumentBlock::Div(div) => div.blocks.iter().collect(),
        }
    }

    fn block_at_position(&self, position: Position) -> Option<DocumentBlock> {
        self.child_blocks()
            .iter()
            .find_map(|child| child.block_at_position(position))
            .or(Some(self.clone()).filter(|block| block.line_range().contains(&position.line)))
    }

    pub fn child_inlines(&self) -> Vec<DocumentInline> {
        match self {
            DocumentBlock::Plain(plain) => plain.inlines.clone(),
            DocumentBlock::Para(para) => para.inlines.clone(),
            DocumentBlock::Header(header) => header.inlines.clone(),
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plain {
    pub line_range: LineRange,
    pub inlines: DocumentInlines,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Para {
    pub line_range: LineRange,
    pub inlines: DocumentInlines,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineBlock {
    pub line_range: LineRange,
    pub inlines: Vec<DocumentInlines>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlock {
    pub line_range: LineRange,
    pub lang: Option<Lang>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawBlock {
    pub line_range: LineRange,
    pub format: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub line_range: LineRange,
    pub level: u8,
    pub inlines: DocumentInlines,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockQuote {
    pub line_range: LineRange,
    pub blocks: DocumentBlocks,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderedList {
    pub items: Vec<DocumentBlocks>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BulletList {
    pub items: Vec<DocumentBlocks>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Div {
    pub line_range: LineRange,
    pub blocks: DocumentBlocks,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefinitionList {
    pub line_range: LineRange,
    pub items: Vec<(DocumentInlines, Vec<DocumentInlines>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalRule {
    pub line_range: LineRange,
}

impl DocumentInline {
    pub fn apppen(&mut self, inline: DocumentInline) {
        match self {
            DocumentInline::Emph(emph) => emph.inlines.push(inline),
            DocumentInline::Image(image) => image.inlines.push(inline),
            DocumentInline::Link(link) => link.inlines.push(inline),
            DocumentInline::Strikeout(strikeout) => strikeout.inlines.push(inline),
            DocumentInline::Strong(strong) => strong.inlines.push(inline),
            DocumentInline::Subscript(subscript) => subscript.inlines.push(inline),
            DocumentInline::Superscript(superscript) => superscript.inlines.push(inline),
            DocumentInline::Underline(underline) => underline.inlines.push(inline),
            DocumentInline::SmallCaps(small_caps) => small_caps.inlines.push(inline),
            DocumentInline::LineBreak(_) => panic!("cannot append inline to line break"),
            DocumentInline::SoftBreak(_) => panic!("cannot append inline to soft break"),
            DocumentInline::Code(_) => panic!("cannot append inline to code"),
            DocumentInline::Math(_) => panic!("cannot append inline to math"),
            DocumentInline::RawInline(_) => panic!("cannot append inline to raw inline"),
            DocumentInline::Space(_) => panic!("cannot append inline to space"),
            DocumentInline::Str(_) => panic!("cannot append inline to str"),
        }
    }

    pub fn to_node_inline(&self) -> Inline {
        match self {
            DocumentInline::Str(text) => Inline::Str(text.clone()),
            DocumentInline::Emph(emph) => Inline::Emph(
                emph.inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Underline(underline) => Inline::Underline(
                underline
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Strong(strong) => Inline::Strong(
                strong
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Strikeout(strikeout) => Inline::Strikeout(
                strikeout
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Superscript(superscript) => Inline::Superscript(
                superscript
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Subscript(subscript) => Inline::Subscript(
                subscript
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::SmallCaps(small_caps) => Inline::SmallCaps(
                small_caps
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Code(code) => Inline::Code(None, code.text.clone()),
            DocumentInline::Space(_) => Inline::Space,
            DocumentInline::SoftBreak(_) => Inline::SoftBreak,
            DocumentInline::LineBreak(_) => Inline::LineBreak,
            DocumentInline::Link(link) => Inline::Link(
                link.target.url.clone(),
                link.target.title.clone(),
                link.link_type,
                link.inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::RawInline(raw_inline) => {
                Inline::RawInline(raw_inline.format.0.clone(), raw_inline.content.clone())
            }
            DocumentInline::Image(image) => Inline::Image(
                image.target.url.clone(),
                image.target.title.clone(),
                image
                    .inlines
                    .iter()
                    .map(|inline| inline.to_node_inline())
                    .collect(),
            ),
            DocumentInline::Math(math) => Inline::Math(math.content.clone()),
        }
    }

    pub fn from_string(s: &str) -> DocumentInlines {
        vec![DocumentInline::Str(s.to_string())]
    }

    pub fn child_inlines(&self) -> Vec<&DocumentInline> {
        match self {
            DocumentInline::Code(_) => vec![],
            DocumentInline::Emph(emph) => emph.inlines.iter().collect(),
            DocumentInline::Image(image) => image.inlines.iter().collect(),
            DocumentInline::LineBreak(_) => vec![],
            DocumentInline::Link(link) => link.inlines.iter().collect(),
            DocumentInline::Math(_) => vec![],
            DocumentInline::RawInline(_) => vec![],
            DocumentInline::SmallCaps(small_caps) => small_caps.inlines.iter().collect(),
            DocumentInline::SoftBreak(_) => vec![],
            DocumentInline::Space(_) => vec![],
            DocumentInline::Str(_) => vec![],
            DocumentInline::Strikeout(strikeout) => strikeout.inlines.iter().collect(),
            DocumentInline::Strong(strong) => strong.inlines.iter().collect(),
            DocumentInline::Subscript(subscript) => subscript.inlines.iter().collect(),
            DocumentInline::Superscript(superscript) => superscript.inlines.iter().collect(),
            DocumentInline::Underline(underline) => underline.inlines.iter().collect(),
        }
    }

    fn is_ref(&self) -> bool {
        match self {
            DocumentInline::Link(link) => model::is_ref_url(&link.target.url),
            _ => false,
        }
    }

    fn ref_title(&self) -> Option<String> {
        match self {
            DocumentInline::Link(_) => Some(self.to_node_inline().to_plain_text()),
            _ => None,
        }
    }

    pub fn ref_key(&self) -> Option<String> {
        match self {
            DocumentInline::Link(link) => Some(without_extension(&link.target.url)),
            _ => None,
        }
    }

    fn ref_type(&self) -> Option<ReferenceType> {
        match self {
            DocumentInline::Link(link) => Some(link.link_type.to_ref_type()),
            _ => None,
        }
    }

    pub fn key_range(&self) -> Option<InlineRange> {
        match self {
            DocumentInline::Link(link) => {
                Some(InlineRange {
                    start: Position {
                        line: link.inline_range.start.line,
                        // Exclude title and parentheses from the range
                        character: link.inline_range.start.character
                            + self.to_node_inline().to_plain_text().len()
                            + 3,
                    },
                    end: Position {
                        line: link.inline_range.end.line,
                        // Exclude title and parentheses from the range
                        character: link.inline_range.end.character - 1,
                    },
                })
            }
            _ => None,
        }
    }

    pub fn inline_range(&self) -> InlineRange {
        match self {
            DocumentInline::Code(code) => code.inline_range.clone(),
            DocumentInline::Emph(emph) => emph.inline_range.clone(),
            DocumentInline::Image(image) => image.inline_range.clone(),
            DocumentInline::LineBreak(line_break) => line_break.inline_range.clone(),
            DocumentInline::Link(link) => link.inline_range.clone(),
            DocumentInline::Math(math) => math.inline_range.clone(),
            DocumentInline::RawInline(raw_inline) => raw_inline.inline_range.clone(),
            DocumentInline::SmallCaps(small_caps) => small_caps.inline_range.clone(),
            DocumentInline::SoftBreak(soft_break) => soft_break.inline_range.clone(),
            DocumentInline::Space(space) => space.inline_range.clone(),
            DocumentInline::Str(_) => InlineRange::default(),
            DocumentInline::Strikeout(strikeout) => strikeout.inline_range.clone(),
            DocumentInline::Strong(strong) => strong.inline_range.clone(),
            DocumentInline::Subscript(subscript) => subscript.inline_range.clone(),
            DocumentInline::Superscript(superscript) => superscript.inline_range.clone(),
            DocumentInline::Underline(underline) => underline.inline_range.clone(),
        }
    }

    pub fn is_link(&self) -> bool {
        match self {
            DocumentInline::Link(_) => true,
            _ => false,
        }
    }

    pub fn key(&self) -> Option<Key> {
        match self {
            DocumentInline::Link(link) => Some(without_extension(&link.target.url)),
            _ => None,
        }
    }

    pub fn link_at_position(&self, position: Position) -> Option<DocumentInline> {
        if self.inline_range().contains(&position) && self.is_link() {
            return Some(self.clone());
        }

        self.child_inlines()
            .iter()
            .find_map(|child| child.link_at_position(position))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum LinkType {
    Regular,
    WikiLink,
    WikiLinkPiped,
}

impl LinkType {
    pub fn to_ref_type(&self) -> ReferenceType {
        match self {
            LinkType::Regular => ReferenceType::Regular,
            LinkType::WikiLink => ReferenceType::WikiLink,
            LinkType::WikiLinkPiped => ReferenceType::WikiLinkPiped,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Link {
    pub target: Target,
    pub attr: Attributes,
    pub inlines: DocumentInlines,
    pub title: String,
    pub inline_range: InlineRange,
    pub link_type: LinkType,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ref {
    pub key: Key,
    pub title: String,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Image {
    pub attr: Attributes,
    pub target: Target,
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Quoted {
    pub quote_type: QuoteType,
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Cite {
    pub citations: Vec<Citation>,
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum QuoteType {
    SingleQuote,
    DoubleQuote,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Citation {
    pub citation_id: String,
    pub citation_prefix: DocumentInlines,
    pub citation_suffix: DocumentInlines,
    pub citation_mode: CitationMode,
    pub citation_note_num: i32,
    pub citation_hash: i32,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum CitationMode {
    AuthorInText,
    SuppressAuthor,
    NormalCitation,
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Attributes {
    pub identifier: String,
    pub classes: Vec<String>,
    pub attributes: Vec<(String, String)>,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Str {
    pub text: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Emph {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Underline {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Strong {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Strikeout {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Superscript {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Subscript {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SmallCaps {
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Code {
    pub attr: Attributes,
    pub text: String,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Space {
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SoftBreak {
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LineBreak {
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Target {
    pub url: String,
    pub title: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub attr: Attributes,
    pub inlines: DocumentInlines,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Math {
    pub math_type: MathType,
    pub content: String,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RawInline {
    pub format: Format,
    pub content: String,
    pub inline_range: InlineRange,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Format(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MathType {
    DisplayMath,
    InlineMath,
}

pub type DocumentInlines = Vec<DocumentInline>;
pub type DocumentBlocks = Vec<DocumentBlock>;
