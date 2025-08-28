use crate::model::document::LinkType;
use crate::model::graph::GraphInlines;
use crate::model::{Key, NodeId};

use super::config::MarkdownOptions;
use super::graph::{blocks_to_markdown_sparce, GraphInline};
use super::projector::Projector;
use super::tree::Tree;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Document(Key, Option<String>),
    Section(GraphInlines),
    Quote(),
    BulletList(),
    OrderedList(),
    Leaf(GraphInlines),
    Raw(Option<String>, String),
    HorizontalRule(),
    Reference(Reference),
    Table(Table),
}

impl Node {
    pub fn plain_text(&self) -> String {
        match self {
            Node::Section(inlines) => inlines.iter().map(|i| i.plain_text()).collect(),
            Node::Leaf(inlines) => inlines.iter().map(|i| i.plain_text()).collect(),
            Node::Reference(reference) => reference.text.clone(),
            Node::Raw(_, content) => content.clone(),
            _ => "".to_string(),
        }
    }

    pub fn reference_key(&self) -> Option<Key> {
        match self {
            Node::Reference(reference) => Some(reference.key.clone()),
            _ => None,
        }
    }

    pub fn is_reference(&self) -> bool {
        match self {
            Node::Reference(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReferenceType {
    Regular,
    WikiLink,
    WikiLinkPiped,
}

impl ReferenceType {
    pub fn to_link_type(&self) -> LinkType {
        match self {
            ReferenceType::Regular => LinkType::Regular,
            ReferenceType::WikiLink => LinkType::WikiLink,
            ReferenceType::WikiLinkPiped => LinkType::WikiLinkPiped,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reference {
    pub key: Key,
    pub text: String,
    pub reference_type: ReferenceType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub header: Vec<GraphInlines>,
    pub alignment: Vec<ColumnAlignment>,
    pub rows: Vec<Vec<GraphInlines>>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ColumnAlignment {
    None,
    Left,
    Center,
    Right,
}

pub trait NodeIter<'a>: Sized {
    fn next(&self) -> Option<Self>;
    fn child(&self) -> Option<Self>;
    fn node(&self) -> Option<Node>;

    fn to_markdown(self, parent: &str, options: &MarkdownOptions) -> String {
        let blocks = Projector::project(self, parent);
        blocks_to_markdown_sparce(&blocks, options)
    }

    fn plain_text(&self) -> String {
        self.inlines().iter().map(|i| i.plain_text()).collect()
    }

    fn to_default_markdown(self) -> String {
        self.to_markdown("", &MarkdownOptions::default())
    }

    fn ref_type(&self) -> Option<ReferenceType> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.reference_type)
            } else {
                None
            }
        })
    }

    fn lang(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Raw(lang, _) = node {
                lang.clone()
            } else {
                None
            }
        })
    }

    fn table_header(&self) -> Option<Vec<GraphInlines>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.header.clone())
            } else {
                None
            }
        })
    }

    fn table_alignment(&self) -> Option<Vec<ColumnAlignment>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.alignment.clone())
            } else {
                None
            }
        })
    }

    fn table_rows(&self) -> Option<Vec<Vec<GraphInlines>>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.rows.clone())
            } else {
                None
            }
        })
    }

    fn content(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Raw(_, content) = node {
                Some(content.clone())
            } else {
                None
            }
        })
    }

    fn ref_text(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.text.clone())
            } else {
                None
            }
        })
    }

    fn ref_key2(&self) -> Option<Key> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.key.clone())
            } else {
                None
            }
        })
    }

    fn inlines(&self) -> GraphInlines {
        self.node()
            .map(|node| match node {
                Node::Section(inlines) => inlines.clone(),
                Node::Leaf(inlines) => inlines.clone(),
                Node::Reference(reference) => vec![GraphInline::Str(reference.text)],
                _ => vec![],
            })
            .unwrap_or_default()
    }

    fn is_list(&self) -> bool {
        self.is_ordered_list() || self.is_bullet_list()
    }

    fn is_document(&self) -> bool {
        match self.node() {
            Some(Node::Document(_, _)) => true,
            _ => false,
        }
    }

    fn is_section(&self) -> bool {
        match self.node() {
            Some(Node::Section(_)) => true,
            _ => false,
        }
    }

    fn is_ordered_list(&self) -> bool {
        match self.node() {
            Some(Node::OrderedList()) => true,
            _ => false,
        }
    }

    fn is_bullet_list(&self) -> bool {
        match self.node() {
            Some(Node::BulletList()) => true,
            _ => false,
        }
    }

    fn is_reference(&self) -> bool {
        match self.node() {
            Some(Node::Reference(_)) => true,
            _ => false,
        }
    }

    fn is_horizontal_rule(&self) -> bool {
        match self.node() {
            Some(Node::HorizontalRule()) => true,
            _ => false,
        }
    }

    fn is_raw(&self) -> bool {
        match self.node() {
            Some(Node::Raw(_, _)) => true,
            _ => false,
        }
    }

    fn is_leaf(&self) -> bool {
        match self.node() {
            Some(Node::Leaf(_)) => true,
            _ => false,
        }
    }

    fn is_quote(&self) -> bool {
        match self.node() {
            Some(Node::Quote()) => true,
            _ => false,
        }
    }
}

pub trait NodePointer<'a>: NodeIter<'a> {
    fn id(&self) -> Option<NodeId>;
    fn next_id(&self) -> Option<NodeId>;
    fn child_id(&self) -> Option<NodeId>;
    fn prev_id(&self) -> Option<NodeId>;
    fn to_node(&self, id: NodeId) -> Self;
    fn to_key(&self, key: Key) -> Option<Self>;

    fn at(&self, id: NodeId) -> bool {
        self.id() == Some(id)
    }

    fn node_key(&self) -> Key {
        self.to_document().and_then(|v| v.document_key()).unwrap()
    }

    fn is_header(&self) -> bool {
        !self.is_in_list() && self.is_section()
    }

    fn collect_tree(self) -> Tree {
        Tree::from_pointer(self).expect("to have node")
    }

    fn squash_tree(self, depth: u8) -> Tree {
        Tree::squash_from_pointer(self, depth)
            .first()
            .cloned()
            .unwrap()
    }

    fn to_prev(&self) -> Option<Self> {
        self.prev_id().map(|id| self.to_node(id))
    }

    fn to_next(&self) -> Option<Self> {
        self.next_id().map(|id| self.to_node(id))
    }

    fn to_child(&self) -> Option<Self> {
        self.child_id().map(|id| self.to_node(id))
    }

    fn get_next_sections(&self) -> Vec<NodeId> {
        let mut sections = vec![];
        if self.is_section() {
            if let Some(id) = self.id() {
                sections.push(id);
            }
        }
        if let Some(next) = self.to_next() {
            sections.extend(next.get_next_sections());
        }
        sections
    }

    fn ref_key(&self) -> Option<Key> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.key.clone())
            } else {
                None
            }
        })
    }

    fn document_key(&self) -> Option<Key> {
        self.node().and_then(|node| {
            if let Node::Document(key, _) = node {
                Some(key.clone())
            } else {
                None
            }
        })
    }

    fn is_primary_section(&self) -> bool {
        self.is_section() && self.to_prev().map(|p| p.is_document()).unwrap_or(false)
    }

    fn to_parent(&self) -> Option<Self> {
        if let Some(prev) = self.to_prev() {
            if let Some(id) = self.id() {
                if prev.is_parent_of(id) {
                    return Some(prev);
                }
            }
            prev.to_parent()
        } else {
            None
        }
    }

    fn to_self(&self) -> Option<Self> {
        self.id().map(|id| self.to_node(id))
    }

    fn get_list(&self) -> Option<Self> {
        if self.is_ordered_list() || self.is_bullet_list() {
            return self.to_self();
        }
        if self.is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_list())
    }

    fn get_top_level_list(&self) -> Option<Self> {
        if self.is_list() && !self.to_parent().map(|p| p.is_in_list()).unwrap_or(false) {
            return self.to_self();
        }
        if self.is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_top_level_list())
    }

    fn to_document(&self) -> Option<Self> {
        if self.is_document() {
            Some(self.to_node(self.id()?))
        } else {
            self.to_prev().and_then(|prev| prev.to_document())
        }
    }

    fn is_parent_of(&self, other: NodeId) -> bool {
        self.child_id().is_some() && self.child_id().unwrap() == other
    }

    fn get_sub_nodes(&self) -> Vec<NodeId> {
        self.to_child()
            .map_or(Vec::new(), |child| child.get_next_nodes())
    }

    fn get_all_sub_nodes(&self) -> Vec<NodeId> {
        let mut nodes = vec![self.id().unwrap_or_default()];
        if let Some(child) = self.to_child() {
            nodes.extend(child.get_all_sub_nodes());
        }
        nodes.extend(
            self.to_next()
                .map(|n| n.get_all_sub_nodes())
                .unwrap_or_else(Vec::new),
        );
        nodes
    }

    fn get_next_nodes(&self) -> Vec<NodeId> {
        let mut nodes = vec![];
        if let Some(id) = self.id() {
            nodes.push(id);
        }
        if let Some(next) = self.to_next() {
            nodes.extend(next.get_next_nodes());
        }
        nodes
    }

    fn get_sub_sections(&self) -> Vec<NodeId> {
        if !self.is_section() {
            panic!("get_sub_sections called on non-section node")
        }
        self.to_child()
            .map(|n| n.get_next_sections())
            .unwrap_or(vec![])
    }

    fn get_all_sub_headers(&self) -> Vec<NodeId> {
        if !self.is_section() {
            panic!("get_all_sub_headers called on non-section node")
        }
        let mut headers = vec![];
        if let Some(id) = self.id() {
            headers.push(id);
        }
        if let Some(child) = self.to_child() {
            headers.extend(child.get_all_sub_headers());
        }
        headers.extend(
            self.to_next()
                .map(|n| n.get_all_sub_headers())
                .unwrap_or_else(Vec::new),
        );
        headers
    }

    fn to_first_section_at_the_same_level(&self) -> Self {
        self.to_prev()
            .filter(|p| p.is_section() && p.is_prev_of(self.id().expect("Expected node ID")))
            .map(|p| p.to_first_section_at_the_same_level())
            .unwrap_or_else(|| self.id().map(|id| self.to_node(id)).unwrap())
    }

    fn is_prev_of(&self, other: NodeId) -> bool {
        self.next_id().is_some() && self.next_id().unwrap() == other
    }

    fn is_in_list(&self) -> bool {
        if self.is_ordered_list() || self.is_bullet_list() {
            return true;
        }
        if self.is_document() {
            return false;
        }
        self.to_parent().map(|p| p.is_in_list()).unwrap_or(false)
    }

    fn get_section(&self) -> Option<Self> {
        if self.is_section() && !self.is_in_list() {
            return self.id().map(|id| self.to_node(id));
        }
        if self.is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_section())
    }
}
