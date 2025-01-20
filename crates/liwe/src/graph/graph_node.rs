use crate::model::{Key, LineId, MaybeLineId, MaybeNodeId, NodeId};
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

    pub fn key(&self) -> &str {
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
    title: String,
}

impl Reference {
    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn next_id(&self) -> MaybeNodeId {
        self.next
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
            GraphNode::Reference(reference) => reference.id,
            GraphNode::Empty => panic!(),
        }
    }

    pub fn is_ref(&self) -> bool {
        match self {
            GraphNode::Reference(_) => true,
            _ => false,
        }
    }
    pub fn is_empty(&self) -> bool {
        match self {
            GraphNode::Empty => true,
            _ => false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            GraphNode::Leaf(_) => true,
            GraphNode::Raw(_) => true,
            GraphNode::Reference(_) => true,
            GraphNode::HorizontalRule(_) => true,
            _ => false,
        }
    }

    pub fn is_ordered_list(&self) -> bool {
        match self {
            GraphNode::OrderedList(_) => true,
            _ => false,
        }
    }

    pub fn is_bullet_list(&self) -> bool {
        match self {
            GraphNode::BulletList(_) => true,
            _ => false,
        }
    }

    pub fn is_document(&self) -> bool {
        match self {
            GraphNode::Document(_) => true,
            _ => false,
        }
    }

    pub fn is_raw_leaf(&self) -> bool {
        match self {
            GraphNode::Raw(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            GraphNode::BulletList(_) => true,
            GraphNode::OrderedList(_) => true,
            _ => false,
        }
    }

    pub fn is_quote(&self) -> bool {
        match self {
            GraphNode::Quote(_) => true,
            _ => false,
        }
    }

    pub fn is_rule(&self) -> bool {
        match self {
            GraphNode::HorizontalRule(_) => true,
            _ => false,
        }
    }

    pub fn is_section(&self) -> bool {
        match self {
            GraphNode::Section(_) => true,
            _ => false,
        }
    }

    pub fn is_reference(&self) -> bool {
        match self {
            GraphNode::Reference(_) => true,
            _ => false,
        }
    }

    pub fn is_horizontal_rule(&self) -> bool {
        match self {
            GraphNode::HorizontalRule(_) => true,
            _ => false,
        }
    }

    pub fn is_raw(&self) -> bool {
        match self {
            GraphNode::Raw(_) => true,
            _ => false,
        }
    }

    pub fn is_root(&self) -> bool {
        match self {
            GraphNode::Document(_) => true,
            _ => false,
        }
    }

    pub fn is_reference_to(&self, key: &str) -> bool {
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

    pub fn new_ref(prev: NodeId, id: NodeId, key: Key, title: String) -> GraphNode {
        GraphNode::Reference(Reference {
            id,
            prev,
            next: None,
            key,
            title,
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

    pub fn key(&self) -> Option<String> {
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

    pub fn ref_key(&self) -> Option<String> {
        match self {
            GraphNode::Reference(reference) => Some(reference.key.clone()),
            _ => None,
        }
    }

    pub fn ref_title(&self) -> String {
        match self {
            GraphNode::Reference(reference) => reference.title.clone(),
            _ => panic!(),
        }
    }
}
