use crate::model::document::LinkType;
use crate::model::graph::GraphInlines;
use crate::model::{Key, NodeId};
use itertools::Itertools;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Document(Key),
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

    fn is_list(&self) -> bool {
        self.is_ordered_list() || self.is_bullet_list()
    }

    fn is_document(&self) -> bool {
        match self.node() {
            Some(Node::Document(_)) => true,
            _ => false,
        }
    }

    fn is_section(&self) -> bool {
        match self.node() {
            Some(Node::Section(_)) => true,
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
}

impl<'a, 'b> NodeIter<'a> for TreeIter<'b> {
    fn next(&self) -> Option<Self> {
        self.path.last().map(|n| {
            let mut path = self.path.clone();
            path.pop();
            path.push(n + 1);

            TreeIter {
                tree_node: self.tree_node,
                path,
            }
        })
    }

    fn child(&self) -> Option<Self> {
        let mut path = self.path.clone();
        path.push(0);

        Some(TreeIter {
            tree_node: self.tree_node,
            path,
        })
    }

    fn node(&self) -> Option<Node> {
        let mut node = self.tree_node;

        for n in self.path.iter() {
            if let Some(n) = &node.children.get(*n) {
                node = n;
            } else {
                return None;
            }
        }

        Some(node.payload.clone())
    }
}

pub trait NodePointer<'a>: NodeIter<'a> {
    fn id(&self) -> Option<NodeId>;
    fn next_id(&self) -> Option<NodeId>;
    fn child_id(&self) -> Option<NodeId>;
    fn prev_id(&self) -> Option<NodeId>;
    fn to(&self, id: NodeId) -> Self;

    fn at(&self, id: NodeId) -> bool {
        self.id() == Some(id)
    }

    fn collect_tree(self) -> TreeNode {
        TreeNode::from_iter(self).expect("to have node")
    }

    fn to_prev(&self) -> Option<Self> {
        self.prev_id().map(|id| self.to(id))
    }

    fn to_next(&self) -> Option<Self> {
        self.next_id().map(|id| self.to(id))
    }

    fn to_child(&self) -> Option<Self> {
        self.child_id().map(|id| self.to(id))
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
            if let Node::Document(key) = node {
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
        self.id().map(|id| self.to(id))
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
            Some(self.to(self.id()?))
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

    fn to_first_section_at_the_same_level(&self) -> Self {
        self.to_prev()
            .filter(|p| p.is_section() && p.is_prev_of(self.id().expect("Expected node ID")))
            .map(|p| p.to_first_section_at_the_same_level())
            .unwrap_or_else(|| self.id().map(|id| self.to(id)).unwrap())
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
            return self.id().map(|id| self.to(id));
        }
        if self.is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_section())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub id: Option<NodeId>,
    pub payload: Node,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn iter(&self) -> TreeIter {
        TreeIter::new(self)
    }

    pub fn is_section(&self) -> bool {
        match self.payload {
            Node::Section(_) => true,
            _ => false,
        }
    }

    pub fn pre_sub_header_position(&self) -> usize {
        self.children
            .iter()
            .take_while(|child| !child.is_section())
            .count()
    }

    pub fn remove_node(&self, target_id: NodeId) -> TreeNode {
        TreeNode {
            id: self.id,
            payload: self.payload.clone(),
            children: self
                .clone()
                .children
                .iter()
                .filter(|child| !child.id_eq(target_id))
                .map(|child| child.remove_node(target_id))
                .collect(),
        }
    }

    pub fn append_pre_header(&self, target_id: NodeId, new: TreeNode) -> TreeNode {
        let mut children = self.children.clone();

        if self.id_eq(target_id) {
            children.insert(self.pre_sub_header_position(), new.clone());
        }

        TreeNode {
            id: self.id,
            payload: self.payload.clone(),
            children: children
                .into_iter()
                .map(|child| child.append_pre_header(target_id, new.clone()))
                .collect(),
        }
    }

    pub fn id_eq(&self, id: NodeId) -> bool {
        self.id == Some(id)
    }

    pub fn from_iter<'a>(iter: impl NodePointer<'a>) -> Option<TreeNode> {
        let id = iter.id();
        let payload = iter.node()?;
        let mut children = Vec::new();

        iter.child().map(|child| children.push(child));

        if let Some(child) = iter.child() {
            let mut i = child;
            while let Some(next) = i.next() {
                children.push(next);
                i = i.next().unwrap();
            }
        }

        Some(TreeNode {
            id,
            payload,
            children: children
                .into_iter()
                .map(|c| TreeNode::from_iter(c))
                .flatten()
                .collect_vec(),
        })
    }
}

pub struct TreeIter<'a> {
    tree_node: &'a TreeNode,
    path: Vec<usize>,
}

impl<'a> TreeIter<'a> {
    pub fn new(tree_node: &'a TreeNode) -> TreeIter<'a> {
        TreeIter {
            tree_node: &tree_node,
            path: vec![],
        }
    }
}
