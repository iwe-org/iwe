use crate::model::NodeId;
use crate::model::graph::Node;
use super::{Graph, NodeIter};

pub struct NodeVisitor<'a> {
    id: NodeId,
    cut_at: Option<NodeId>,
    graph: &'a Graph,
}

impl<'a> NodeIter<'a> for NodeVisitor<'a> {
    fn next(&self) -> Option<impl NodeIter> {
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

    fn child(&self) -> Option<impl NodeIter> {
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

impl<'a> NodeVisitor<'a> {
    pub fn new(graph: &'a Graph, id: NodeId) -> Self {
        NodeVisitor {
            id,
            cut_at: None,
            graph,
        }
    }

    pub fn new_cut(graph: &'a Graph, id: NodeId) -> Self {
        NodeVisitor {
            id,
            cut_at: Some(id),
            graph,
        }
    }
}
