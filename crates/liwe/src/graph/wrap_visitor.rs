use super::{Graph, NodeIter};
use crate::model::graph::Node;
use crate::model::{MaybeNodeId, NodeId};

pub struct WrapVisitor<'a> {
    id: NodeId,
    target_id: MaybeNodeId,
    new_node: bool,
    graph: &'a Graph,
}

impl<'a> WrapVisitor<'a> {
    pub fn new(graph: &'a Graph, target_id: NodeId) -> Self {
        Self {
            id: graph
                .visit_node(target_id)
                .to_document()
                .expect("to have document")
                .id(),
            target_id: Some(
                graph
                    .visit_node(target_id)
                    .to_first_section_at_the_same_level()
                    .id(),
            ),
            graph,
            new_node: false,
        }
    }

    fn is_on_target(&self) -> bool {
        self.target_id.is_some() && self.id == self.target_id.unwrap()
    }

    fn next_is_target(&self) -> bool {
        self.graph
            .visit_node(self.id)
            .to_next()
            .map(|next| self.target_id.is_some() && next.id() == self.target_id.unwrap())
            .unwrap_or(false)
    }

    fn child_is_target(&self) -> bool {
        self.graph
            .visit_node(self.id)
            .to_child()
            .map(|child| self.target_id.is_some() && child.id() == self.target_id.unwrap())
            .unwrap_or(false)
    }
}

impl<'a> NodeIter<'a> for WrapVisitor<'a> {
    fn next(&self) -> Option<Self> {
        if self.next_is_target() && !self.new_node {
            return Some(WrapVisitor {
                id: self.id,
                target_id: self.target_id,
                graph: self.graph,
                new_node: true,
            });
        }

        if self.new_node {
            return None;
        }

        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| WrapVisitor {
                id,
                target_id: self.target_id,
                graph: self.graph,
                new_node: false,
            })
    }

    fn child(&self) -> Option<Self> {
        if self.child_is_target() && !self.new_node {
            return Some(WrapVisitor {
                id: self.id,
                target_id: self.target_id,
                graph: self.graph,
                new_node: true,
            });
        }

        if self.new_node {
            return Some(WrapVisitor {
                id: self.target_id.unwrap(),
                target_id: None,
                graph: self.graph,
                new_node: false,
            });
        }

        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| WrapVisitor {
                id,
                target_id: self.target_id,
                graph: self.graph,
                new_node: false,
            })
    }

    fn node(&self) -> Option<Node> {
        if self.new_node {
            return Some(Node::BulletList());
        }

        self.graph.node(self.id)
    }
}
