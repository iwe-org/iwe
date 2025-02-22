use super::*;
use crate::model::graph::Node;

#[derive(Debug, Clone)]
pub struct GraphNodeVisitor<'a> {
    id: NodeId,
    graph: &'a Graph,
}

impl<'a> GraphNodeVisitor<'a> {
    pub fn new(graph: &'a Graph, id: NodeId) -> GraphNodeVisitor<'a> {
        GraphNodeVisitor { id, graph }
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn key(&self) -> Option<Key> {
        self.node().key()
    }

    pub fn to_parent(&self) -> Option<Self> {
        if self
            .prev()
            .map(|p| p.node().is_parent_of(self.id))
            .unwrap_or(false)
        {
            self.prev()
        } else {
            self.prev().and_then(|p| p.to_parent())
        }
    }

    pub fn to_document(&self) -> Option<Self> {
        if self.is_document() {
            return Some(self.clone());
        } else {
            return self.prev().and_then(|prev| prev.to_document());
        }
    }

    pub fn get_sub_nodes(&self) -> Vec<NodeId> {
        self.to_child()
            .map_or(Vec::new(), |child| child.get_next_nodes())
    }

    pub fn get_all_sub_nodes(&self) -> Vec<NodeId> {
        let mut nodes = vec![self.id];
        if let Some(child) = self.to_child() {
            nodes.append(&mut child.get_all_sub_nodes());
        }
        nodes.append(
            &mut self
                .to_next()
                .map(|n| n.get_all_sub_nodes())
                .unwrap_or(vec![]),
        );
        nodes
    }

    fn get_next_nodes(&self) -> Vec<NodeId> {
        let mut nodes = vec![];
        nodes.push(self.id);
        nodes.append(&mut self.to_next().map(|n| n.get_next_nodes()).unwrap_or(vec![]));
        nodes
    }

    pub fn get_sub_sections(&self) -> Vec<NodeId> {
        if !self.is_section() {
            panic!("get_sub_sections called on non-section node")
        }
        self.to_child()
            .map(|n| n.get_next_sections())
            .unwrap_or(vec![])
    }

    fn get_next_sections(&self) -> Vec<NodeId> {
        let mut sections = vec![];
        if self.is_section() {
            sections.push(self.id);
        }
        sections.append(
            &mut self
                .to_next()
                .map(|n| n.get_next_sections())
                .unwrap_or(vec![]),
        );
        sections
    }

    pub fn to_first_section_at_the_same_level(&self) -> Self {
        self.to_prev()
            .filter(|p| p.node().is_section() && p.node().is_prev_of(self.id))
            .map(|p| p.to_first_section_at_the_same_level())
            .unwrap_or(self.clone())
    }

    pub fn is_in_list(&self) -> bool {
        if self.node().is_list() {
            return true;
        }
        if self.node().is_document() {
            return false;
        }
        self.to_parent().map(|p| p.is_in_list()).unwrap_or(false)
    }

    pub fn get_list(&self) -> Option<Self> {
        if self.node().is_list() {
            return Some(Self::new(self.graph, self.id));
        }
        if self.node().is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_list())
    }

    pub fn get_top_level_list(&self) -> Option<Self> {
        if self.node().is_list() && !self.to_parent().map(|p| p.is_in_list()).unwrap_or(false) {
            return Some(Self::new(self.graph, self.id));
        }
        if self.node().is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_top_level_list())
    }

    pub fn get_section(&self) -> Option<Self> {
        if self.node().is_section() && !self.is_in_list() {
            return Some(Self::new(self.graph, self.id));
        }
        if self.node().is_document() {
            return None;
        }
        self.to_parent().and_then(|p| p.get_section())
    }

    pub fn is_primary_section(&self) -> bool {
        self.node().is_section() && self.prev().map(|p| p.node().is_document()).unwrap_or(false)
    }

    pub fn is_document(&self) -> bool {
        self.node().is_document()
    }

    pub fn is_section(&self) -> bool {
        self.node().is_section()
    }

    pub fn is_reference(&self) -> bool {
        self.node().is_reference()
    }

    pub fn is_horizontal_rule(&self) -> bool {
        self.node().is_horizontal_rule()
    }

    pub fn is_raw(&self) -> bool {
        self.node().is_raw()
    }

    pub fn is_leaf(&self) -> bool {
        self.node().is_leaf()
    }

    pub fn is_quote(&self) -> bool {
        self.node().is_quote()
    }

    pub fn is_ordered_list(&self) -> bool {
        self.node().is_ordered_list()
    }

    pub fn is_bullet_list(&self) -> bool {
        self.node().is_bullet_list()
    }

    pub fn ref_key(&self) -> Option<Key> {
        self.node().ref_key()
    }

    pub fn document_key(&self) -> Option<Key> {
        self.node().key()
    }

    pub fn to_next(&self) -> Option<Self> {
        self.node().next_id().map(|id| Self::new(self.graph, id))
    }

    pub fn next_id(&self) -> Option<NodeId> {
        self.node().next_id()
    }

    pub fn to_child(&self) -> Option<Self> {
        self.node().child_id().map(|id| Self::new(self.graph, id))
    }

    pub fn child_id(&self) -> Option<NodeId> {
        self.node().child_id()
    }

    pub fn to_prev(&self) -> Option<Self> {
        self.node().prev_id().map(|id| Self::new(self.graph, id))
    }

    fn node(&self) -> GraphNode {
        self.graph.graph_node(self.id)
    }

    fn prev(&self) -> Option<Self> {
        self.node().prev_id().map(|id| Self::new(self.graph, id))
    }

    pub fn child(&self) -> Option<Self> {
        self.node().child_id().map(|id| Self::new(self.graph, id))
    }

    pub fn to_node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
