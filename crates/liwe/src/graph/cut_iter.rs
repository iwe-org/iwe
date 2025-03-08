use super::Graph;
use crate::model::node::Node;
use crate::model::node::NodeIter;
use crate::model::NodeId;

pub struct CutIter<'a> {
    id: NodeId,
    cut_at: Option<NodeId>,
    graph: &'a Graph,
}

impl<'a> CutIter<'a> {
    pub fn children_of(graph: &'a Graph, id: NodeId) -> Self {
        CutIter {
            id,
            cut_at: Some(id),
            graph,
        }
    }
}

impl<'a> NodeIter<'a> for CutIter<'a> {
    fn next(&self) -> Option<Self> {
        if self.cut_at == Some(self.id) {
            return None;
        }
        self.graph.graph_node(self.id).next_id().map(|id| CutIter {
            graph: self.graph,
            id,
            cut_at: self.cut_at,
        })
    }

    fn child(&self) -> Option<Self> {
        self.graph.graph_node(self.id).child_id().map(|id| CutIter {
            graph: self.graph,
            id,
            cut_at: self.cut_at,
        })
    }

    fn node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
