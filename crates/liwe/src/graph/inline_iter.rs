use super::Graph;
use crate::model::node::Node;
use crate::model::{Key, NodeId};
use crate::model::node::{NodeIter, NodePointer};

pub struct InlineIter<'a> {
    id: NodeId,
    inline_id: NodeId,
    graph: &'a Graph,
}

impl<'a> InlineIter<'a> {
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

    fn target(&self) -> impl NodePointer {
        self.graph.visit_key(&self.ref_key()).expect("to have key")
    }

    fn is_on_target(&self) -> bool {
        self.id == self.inline_id
    }
}

impl<'a> NodeIter<'a> for InlineIter<'a> {
    fn next(&self) -> Option<Self> {
        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| InlineIter {
                id,
                inline_id: self.inline_id,
                graph: self.graph,
            })
    }

    fn child(&self) -> Option<Self> {
        if self.is_on_target() {
            return self.target().to_child().map(|child| InlineIter {
                id: child.id().unwrap(),
                inline_id: self.inline_id,
                graph: self.graph,
            });
        }

        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| InlineIter {
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
