use crate::model::{
    Key, LineId, MaybeLineId, MaybeNodeId, NodeId,
};
use crate::model::node::{ColumnAlignment, ReferenceType};

#[derive(Clone, Debug, PartialEq)]
pub enum GraphNode {
    Empty,
    Document(Document),
    Section(Section),
    Quote(Quote),
    BulletList(BulletList),
    OrderedList(OrderedList),
    Leaf(Leaf),
    Raw(RawLeaf),
    HorizontalRule(HorizontalRule),
    Reference(Reference),
    Table(Table),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    id: NodeId,

    child: MaybeNodeId,

    key: Key,
    metadata: Option<String>,
}

impl Document {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn metadata(&self) -> Option<String> {
        self.metadata.clone()
    }

    pub fn child_id(&self) -> MaybeNodeId {
        self.child
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,
    child: MaybeNodeId,

    line: LineId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quote {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,
    child: MaybeNodeId,
}

impl Quote {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn child_id(&self) -> MaybeNodeId {
        self.child
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

impl Section {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn line_id(&self) -> LineId {
        self.line
    }

    pub fn child_id(&self) -> MaybeNodeId {
        self.child
    }
    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BulletList {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,
    child: MaybeNodeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderedList {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,
    child: MaybeNodeId,
}

impl BulletList {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn child_id(&self) -> MaybeNodeId {
        self.child
    }
    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

impl OrderedList {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn child_id(&self) -> MaybeNodeId {
        self.child
    }
    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Leaf {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,

    line: LineId,
}

impl Leaf {
    pub fn line_id(&self) -> LineId {
        self.line
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawLeaf {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,

    lang: Option<String>,
    content: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,

    header: Vec<LineId>,
    alignment: Vec<ColumnAlignment>,
    rows: Vec<Vec<LineId>>,
}

impl Table {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn header(&self) -> &Vec<LineId> {
        &self.header
    }

    pub fn alignment(&self) -> &Vec<ColumnAlignment> {
        &self.alignment
    }

    pub fn rows(&self) -> &Vec<Vec<LineId>> {
        &self.rows
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

impl RawLeaf {
    pub fn lang(&self) -> Option<String> {
        self.lang.clone()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalRule {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,
}
impl HorizontalRule {
    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    id: NodeId,

    prev: NodeId,
    next: MaybeNodeId,

    key: Key,
    text: String,
    reference_type: ReferenceType,
}

impl Reference {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> &Key {
        &self.key
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
    }

    pub fn reference_type(&self) -> ReferenceType {
        self.reference_type
    }
}

impl GraphNode {
    pub fn prev_id(&self) -> MaybeNodeId {
        match self {
            GraphNode::Section(section) => Some(section.prev),
            GraphNode::Quote(quote) => Some(quote.prev),
            GraphNode::BulletList(list) => Some(list.prev),
            GraphNode::OrderedList(list) => Some(list.prev),
            GraphNode::Leaf(leaf) => Some(leaf.prev),
            GraphNode::Reference(reference) => Some(reference.prev),
            GraphNode::HorizontalRule(rule) => Some(rule.prev),
            GraphNode::Raw(raw) => Some(raw.prev),
            GraphNode::Table(table) => Some(table.prev),
            GraphNode::Document(_) => None,
            GraphNode::Empty => None,
        }
    }

    pub fn id(&self) -> NodeId {
        match self {
            GraphNode::Document(document) => document.id,
            GraphNode::Section(section) => section.id,
            GraphNode::Quote(quote) => quote.id,
            GraphNode::HorizontalRule(rule) => rule.id,
            GraphNode::BulletList(list) => list.id,
            GraphNode::OrderedList(list) => list.id,
            GraphNode::Leaf(leaf) => leaf.id,
            GraphNode::Raw(leaf) => leaf.id,
            GraphNode::Table(table) => table.id,
            GraphNode::Reference(reference) => reference.id,
            GraphNode::Empty => panic!(),
        }
    }

    pub fn is_ref(&self) -> bool {
        matches!(self, GraphNode::Reference(_))
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, GraphNode::Empty)
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self,
            GraphNode::Leaf(_) |
            GraphNode::Raw(_) |
            GraphNode::Reference(_) |
            GraphNode::HorizontalRule(_)
        )
    }

    pub fn is_table(&self) -> bool {
        matches!(self, GraphNode::Table(_))
    }

    pub fn is_ordered_list(&self) -> bool {
        matches!(self, GraphNode::OrderedList(_))
    }

    pub fn is_bullet_list(&self) -> bool {
        matches!(self, GraphNode::BulletList(_))
    }

    pub fn is_document(&self) -> bool {
        matches!(self, GraphNode::Document(_))
    }

    pub fn is_raw_leaf(&self) -> bool {
        matches!(self, GraphNode::Raw(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, GraphNode::BulletList(_) | GraphNode::OrderedList(_))
    }

    pub fn is_quote(&self) -> bool {
        matches!(self, GraphNode::Quote(_))
    }

    pub fn is_rule(&self) -> bool {
        matches!(self, GraphNode::HorizontalRule(_))
    }

    pub fn is_section(&self) -> bool {
        matches!(self, GraphNode::Section(_))
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, GraphNode::Reference(_))
    }

    pub fn is_horizontal_rule(&self) -> bool {
        matches!(self, GraphNode::HorizontalRule(_))
    }

    pub fn is_raw(&self) -> bool {
        matches!(self, GraphNode::Raw(_))
    }

    pub fn is_root(&self) -> bool {
        matches!(self, GraphNode::Document(_))
    }

    pub fn is_reference_to(&self, key: &Key) -> bool {
        match self {
            GraphNode::Reference(reference) => reference.key.eq(key),
            _ => false,
        }
    }

    pub fn line_id(&self) -> MaybeLineId {
        match self {
            GraphNode::Section(section) => Some(section.line),
            GraphNode::Leaf(leaf) => Some(leaf.line),
            _ => None,
        }
    }

    pub fn next_id(&self) -> MaybeNodeId {
        match self {
            GraphNode::Section(section) => section.next,
            GraphNode::Quote(quote) => quote.next,
            GraphNode::HorizontalRule(quote) => quote.next,
            GraphNode::BulletList(list) => list.next,
            GraphNode::OrderedList(list) => list.next,
            GraphNode::Leaf(leaf) => leaf.next,
            GraphNode::Raw(leaf) => leaf.next,
            GraphNode::Table(table) => table.next,
            GraphNode::Reference(reference) => reference.next,
            GraphNode::Document(_) => None,
            GraphNode::Empty => panic!(),
        }
    }

    pub fn to_symbol(&self) -> String {
        match self {
            GraphNode::Section(_) => "S",
            GraphNode::Quote(_) => "Q",
            GraphNode::HorizontalRule(_) => "R",
            GraphNode::BulletList(_) => "L",
            GraphNode::OrderedList(_) => "L",
            GraphNode::Leaf(_) => "F",
            GraphNode::Raw(_) => "C",
            GraphNode::Reference(_) => "R",
            GraphNode::Document(_) => "D",
            GraphNode::Table(_) => "T",
            GraphNode::Empty => "-",
        }
        .to_string()
    }

    pub fn child_id(&self) -> MaybeNodeId {
        match self {
            GraphNode::Document(document) => document.child,
            GraphNode::Section(section) => section.child,
            GraphNode::Quote(quote) => quote.child,
            GraphNode::BulletList(list) => list.child,
            GraphNode::OrderedList(list) => list.child,
            _ => None,
        }
    }

    pub fn is_parent_of(&self, other: NodeId) -> bool {
        self.child_id().is_some() && self.child_id().unwrap() == other
    }

    pub fn is_prev_of(&self, other: NodeId) -> bool {
        self.next_id().is_some() && self.next_id().unwrap() == other
    }

    pub fn set_next_id(&mut self, next: NodeId) {
        match self {
            GraphNode::Section(section) => section.next = Some(next),
            GraphNode::Quote(quote) => quote.next = Some(next),
            GraphNode::BulletList(list) => list.next = Some(next),
            GraphNode::OrderedList(list) => list.next = Some(next),
            GraphNode::Leaf(leaf) => leaf.next = Some(next),
            GraphNode::HorizontalRule(rule) => rule.next = Some(next),
            GraphNode::Raw(leaf) => leaf.next = Some(next),
            GraphNode::Table(table) => table.next = Some(next),
            GraphNode::Reference(reference) => reference.next = Some(next),
            GraphNode::Document(_) => panic!("cant set next for document"),
            GraphNode::Empty => panic!(),
        }
    }

    pub fn set_child_id(&mut self, child: NodeId) {
        match self {
            GraphNode::Document(document) => document.child = Some(child),
            GraphNode::Section(section) => section.child = Some(child),
            GraphNode::Quote(quote) => quote.child = Some(child),
            GraphNode::BulletList(list) => list.child = Some(child),
            GraphNode::OrderedList(list) => list.child = Some(child),
            GraphNode::Leaf(_) => panic!("cant set child for leaf"),
            GraphNode::Raw(_) => panic!("cant set child for raw"),
            GraphNode::Table(_) => panic!("cant set child for table"),
            GraphNode::HorizontalRule(_) => panic!("cant set child for rule"),
            GraphNode::Reference(_) => panic!("cant set child for reference"),
            GraphNode::Empty => panic!(),
        }
    }

    pub fn insertable(&self) -> bool {
        match self {
            GraphNode::Document(_) => true,
            GraphNode::Section(_) => true,
            GraphNode::Quote(_) => true,
            GraphNode::BulletList(_) => true,
            GraphNode::OrderedList(_) => true,
            GraphNode::Leaf(_) => false,
            GraphNode::Raw(_) => false,
            GraphNode::HorizontalRule(_) => false,
            GraphNode::Reference(_) => false,
            GraphNode::Table(_) => false,
            GraphNode::Empty => false,
        }
    }

    pub fn new_leaf(prev: NodeId, id: NodeId, line: LineId) -> GraphNode {
        GraphNode::Leaf(Leaf {
            id,
            prev,
            next: None,
            line,
        })
    }

    pub fn new_raw_leaf(
        prev: NodeId,
        id: NodeId,
        content: String,
        lang: Option<String>,
    ) -> GraphNode {
        GraphNode::Raw(RawLeaf {
            id,
            prev,
            next: None,
            lang,
            content,
        })
    }

    pub fn new_table(
        prev: NodeId,
        id: NodeId,
        header: Vec<LineId>,
        alignment: Vec<ColumnAlignment>,
        rows: Vec<Vec<LineId>>,
    ) -> GraphNode {
        GraphNode::Table(Table {
            id,
            prev,
            next: None,
            header,
            alignment,
            rows,
        })
    }

    pub fn new_ref(
        prev: NodeId,
        id: NodeId,
        key: Key,
        text: String,
        reference_type: ReferenceType,
    ) -> GraphNode {
        GraphNode::Reference(Reference {
            id,
            prev,
            next: None,
            key,
            text,
            reference_type,
        })
    }

    pub fn new_bullet_list(prev: NodeId, id: NodeId) -> GraphNode {
        GraphNode::BulletList(BulletList {
            id,
            prev,
            next: None,
            child: None,
        })
    }

    pub fn new_ordered_list(prev: NodeId, id: NodeId) -> GraphNode {
        GraphNode::OrderedList(OrderedList {
            id,
            prev,
            next: None,
            child: None,
        })
    }

    pub fn new_quote(prev: NodeId, id: NodeId) -> GraphNode {
        GraphNode::Quote(Quote {
            id,
            prev,
            next: None,
            child: None,
        })
    }

    pub fn new_rule(prev: NodeId, id: NodeId) -> GraphNode {
        GraphNode::HorizontalRule(HorizontalRule {
            id,
            prev,
            next: None,
        })
    }

    pub fn new_section(prev: NodeId, id: NodeId, line: LineId) -> GraphNode {
        GraphNode::Section(Section {
            id,
            prev,
            next: None,
            child: None,
            line,
        })
    }
    pub fn new_root(key: Key, id: NodeId, metadata: Option<String>) -> GraphNode {
        GraphNode::Document(Document {
            id,
            child: None,
            key,
            metadata,
        })
    }

    pub fn is_ordered(&self) -> bool {
        match self {
            GraphNode::BulletList(_) => false,
            GraphNode::OrderedList(_) => true,
            _ => false,
        }
    }

    pub fn key(&self) -> Option<Key> {
        match self {
            GraphNode::Document(document) => Some(document.key.clone()),
            _ => None,
        }
    }

    pub fn content(&self) -> Option<String> {
        match self {
            GraphNode::Raw(document) => Some(document.content.clone()),
            _ => None,
        }
    }

    pub fn lang(&self) -> Option<String> {
        match self {
            GraphNode::Raw(document) => document.lang.clone(),
            _ => None,
        }
    }

    pub fn table_header(&self) -> Option<Vec<LineId>> {
        match self {
            GraphNode::Table(table) => Some(table.header.clone()),
            _ => None,
        }
    }

    pub fn table_rows(&self) -> Option<Vec<Vec<LineId>>> {
        match self {
            GraphNode::Table(table) => Some(table.rows.clone()),
            _ => None,
        }
    }

    pub fn table_alignment(&self) -> Option<Vec<ColumnAlignment>> {
        match self {
            GraphNode::Table(table) => Some(table.alignment.clone()),
            _ => None,
        }
    }

    pub fn ref_key(&self) -> Option<Key> {
        match self {
            GraphNode::Reference(reference) => Some(reference.key.clone()),
            _ => None,
        }
    }

    pub fn ref_type(&self) -> Option<ReferenceType> {
        match self {
            GraphNode::Reference(reference) => Some(reference.reference_type),
            _ => None,
        }
    }

    pub fn ref_text(&self) -> String {
        match self {
            GraphNode::Reference(reference) => reference.text.clone(),
            _ => panic!(),
        }
    }
}
