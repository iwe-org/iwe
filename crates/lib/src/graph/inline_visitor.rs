use super::{graph_node_visitor::GraphNodeVisitor, Graph, NodeIter};
use crate::model::graph::Node;
use crate::model::{Key, NodeId};

pub struct InlineVisitor<'a> {
    id: NodeId,
    inline_id: NodeId,
    graph: &'a Graph,
}

impl<'a> InlineVisitor<'a> {
    pub fn new(graph: &'a Graph, id: NodeId, inline_id: NodeId) -> Self {
        Self {
            id,
            inline_id,
            graph,
        }
    }

    fn ref_key(&self) -> Key {
        self.graph
            .graph_node(self.inline_id)
            .ref_key()
            .expect("Inline node should have ref key")
    }

    fn target(&self) -> GraphNodeVisitor {
        self.graph.visit_key(&self.ref_key()).expect("to have key")
    }

    fn is_on_target(&self) -> bool {
        self.id == self.inline_id
    }
}

impl<'a> NodeIter<'a> for InlineVisitor<'a> {
    fn next(&self) -> Option<impl NodeIter> {
        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| InlineVisitor {
                id,
                inline_id: self.inline_id,
                graph: self.graph,
            })
    }

    fn child(&self) -> Option<impl NodeIter> {
        if self.is_on_target() {
            return self.target().to_child().map(|child| InlineVisitor {
                id: child.id(),
                inline_id: self.inline_id,
                graph: self.graph,
            });
        }

        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| InlineVisitor {
                id,
                inline_id: self.inline_id,
                graph: self.graph,
            })
    }

    fn node(&self) -> Option<Node> {
        if self.is_on_target() {
            return Some(Node::BulletList());
        }
        self.graph.node(self.id)
    }
}
