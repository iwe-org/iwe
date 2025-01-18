use super::Graph;
use crate::model::graph::{GraphNodeIter, Node, NodeIter, TreeNode};
use crate::model::NodeId;

pub struct NodeVisitor<'a> {
    id: NodeId,
    cut_at: Option<NodeId>,
    graph: &'a Graph,
}

impl<'a> NodeVisitor<'a> {
    pub fn new(graph: &'a Graph, id: NodeId) -> Self {
        NodeVisitor {
            id,
            cut_at: None,
            graph,
        }
    }

    pub fn children_of(graph: &'a Graph, id: NodeId) -> Self {
        NodeVisitor {
            id,
            cut_at: Some(id),
            graph,
        }
    }
}

impl<'a> GraphNodeIter<'a> for NodeVisitor<'a> {
    fn id(&self) -> Option<NodeId> {
        Some(self.id)
    }
}

impl<'a> NodeIter<'a> for NodeVisitor<'a> {
    fn next(&self) -> Option<Self> {
        if self.cut_at == Some(self.id) {
            return None;
        }
        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| NodeVisitor {
                graph: self.graph,
                id,
                cut_at: self.cut_at,
            })
    }

    fn child(&self) -> Option<Self> {
        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| NodeVisitor {
                graph: self.graph,
                id,
                cut_at: self.cut_at,
            })
    }

    fn node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
