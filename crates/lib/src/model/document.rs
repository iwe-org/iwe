use crate::key::without_extension;
use crate::model;
use crate::model::graph::Inline;
use crate::model::{Key, Lang, LineRange};

#[derive(Clone, Debug, PartialEq)]
pub enum DocumentBlock {
    Plain(Plain),
    Para(Para),
    LineBlock(LineBlock),
    CodeBlock(CodeBlock),
    RawBlock(RawBlock),
    BlockQuote(BlockQuote),
    OrderedList(OrderedList),
    BulletList(BulletList),
    DefinitionList(DefinitionList),
    Header(Header),
    HorizontalRule(HorizontalRule),
    Div(Div),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum DocumentInline {
    Cite(Cite),
    Code(Code),
    Emph(Emph),
    Image(Image),
    LineBreak(LineBreak),
    Link(Link),
    Math(Math),
    Quoted(Quoted),
    RawInline(RawInline),
    SmallCaps(SmallCaps),
    SoftBreak(SoftBreak),
    Space(Space),
    Span(Span),
    Str(String),
    Strikeout(Strikeout),
    Strong(Strong),
    Subscript(Subscript),
    Superscript(Superscript),
    Underline(Underline),
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

    pub fn ref_key(&self) -> Option<String> {
        match self {
            DocumentBlock::Para(para) => para.inlines.first().and_then(|inline| inline.ref_key()),
            _ => None,
        }
    }

    pub fn is_container(&self) -> bool {
        match self {
            DocumentBlock::Plain(_) => false,
            DocumentBlock::Para(_) => false,
            DocumentBlock::LineBlock(_) => false,
            DocumentBlock::CodeBlock(_) => false,
            DocumentBlock::RawBlock(_) => false,
            DocumentBlock::BlockQuote(_) => true,
            DocumentBlock::OrderedList(_) => true,
            DocumentBlock::BulletList(_) => true,
            DocumentBlock::DefinitionList(_) => true,
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
            default => panic!(),
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
            default => panic!(),
        }
    }

    pub fn append_inline(&mut self, inline: DocumentInline, line_range: LineRange) {
        match self {
            DocumentBlock::Plain(plain) => plain.inlines.push(inline),
            DocumentBlock::Para(para) => para.inlines.push(inline),
            DocumentBlock::LineBlock(line_block) => {
                if let Some(last) = line_block.inlines.last_mut() {
                    last.push(inline);
                }
            }
            DocumentBlock::CodeBlock(code) => {}
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
            DocumentBlock::DefinitionList(definition_list) => {}
            DocumentBlock::Header(header) => header.inlines.push(inline),
            DocumentBlock::HorizontalRule(_) => {}
            DocumentBlock::Div(div) => {}
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
    fn is_ref_inline(&self) -> bool {
        match self {
            DocumentInline::Link(link) => model::is_ref_url(&link.target.url),
            _ => false,
        }
    }

    pub fn apppen(&mut self, inline: DocumentInline) {
        match self {
            DocumentInline::Cite(cite) => todo!(),
            DocumentInline::Emph(emph) => emph.inlines.push(inline),
            DocumentInline::Image(image) => image.inlines.push(inline),
            DocumentInline::LineBreak(line_break) => todo!(),
            DocumentInline::Link(link) => link.inlines.push(inline),
            DocumentInline::Quoted(quoted) => todo!(),
            DocumentInline::SmallCaps(small_caps) => todo!(),
            DocumentInline::SoftBreak(soft_break) => todo!(),
            DocumentInline::Span(span) => span.inlines.push(inline),
            DocumentInline::Strikeout(strikeout) => strikeout.inlines.push(inline),
            DocumentInline::Strong(strong) => strong.inlines.push(inline),
            DocumentInline::Subscript(subscript) => subscript.inlines.push(inline),
            DocumentInline::Superscript(superscript) => superscript.inlines.push(inline),
            DocumentInline::Underline(underline) => underline.inlines.push(inline),
            default => panic!("cannot append to {:?}", default),
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
            DocumentInline::Cite(cite) => todo!(),
            DocumentInline::Quoted(quoted) => todo!(),
            DocumentInline::Span(span) => todo!(),
        }
    }

    pub fn from_string(s: &str) -> DocumentInlines {
        vec![DocumentInline::Str(s.to_string())]
    }

    fn child_inlines(&self) -> Vec<&DocumentInline> {
        match self {
            DocumentInline::Cite(cite) => cite.inlines.iter().collect(),
            DocumentInline::Code(_) => vec![],
            DocumentInline::Emph(emph) => emph.inlines.iter().collect(),
            DocumentInline::Image(image) => image.inlines.iter().collect(),
            DocumentInline::LineBreak(_) => vec![],
            DocumentInline::Link(link) => link.inlines.iter().collect(),
            DocumentInline::Math(_) => vec![],
            DocumentInline::Quoted(quoted) => quoted.inlines.iter().collect(),
            DocumentInline::RawInline(_) => vec![],
            DocumentInline::SmallCaps(small_caps) => small_caps.inlines.iter().collect(),
            DocumentInline::SoftBreak(_) => vec![],
            DocumentInline::Space(_) => vec![],
            DocumentInline::Span(span) => span.inlines.iter().collect(),
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
            DocumentInline::Link(link) => Some(self.to_node_inline().to_plain_text()),
            _ => None,
        }
    }

    fn ref_key(&self) -> Option<String> {
        match self {
            DocumentInline::Link(link) => Some(without_extension(&link.target.url)),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Link {
    pub target: Target,
    pub attr: Attributes,
    pub inlines: DocumentInlines,
}

impl Link {
    pub fn from_strings(target: &str, line: &str) -> DocumentInline {
        DocumentInline::Link(Link {
            target: Target {
                url: target.to_string(),
                title: "".to_string(),
            },
            attr: Attributes::default(),
            inlines: DocumentInline::from_string(line),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ref {
    pub key: Key,
    pub title: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Image {
    pub attr: Attributes,
    pub target: Target,
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Quoted {
    pub quote_type: QuoteType,
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Cite {
    pub citations: Vec<Citation>,
    pub inlines: DocumentInlines,
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
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Str {
    pub text: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Emph {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Underline {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Strong {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Strikeout {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Superscript {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Subscript {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SmallCaps {
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Code {
    pub attr: Attributes,
    pub text: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Space {}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SoftBreak {}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct LineBreak {}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Target {
    pub url: String,
    pub title: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub attr: Attributes,
    pub inlines: DocumentInlines,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Math {
    pub math_type: MathType,
    pub content: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RawInline {
    pub format: Format,
    pub content: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Format(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MathType {
    DisplayMath,
    InlineMath,
}

pub fn link(line: &str, target: &str) -> DocumentInlines {
    vec![DocumentInline::Link(Link {
        target: Target {
            url: target.to_string(),
            title: "".to_string(),
        },
        attr: Attributes::default(),
        inlines: DocumentInline::from_string(line),
    })]
}

pub type DocumentInlines = Vec<DocumentInline>;
pub type DocumentBlocks = Vec<DocumentBlock>;
