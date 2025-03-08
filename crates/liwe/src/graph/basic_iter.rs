use super::Graph;
use crate::model::node::Node;
use crate::model::node::NodeIter;
use crate::model::node::NodePointer;
use crate::model::NodeId;

pub struct GraphNodePointer<'a> {
    id: NodeId,
    graph: &'a Graph,
}

impl<'a> GraphNodePointer<'a> {
    pub fn new(graph: &'a Graph, id: NodeId) -> Self {
        GraphNodePointer { id, graph }
    }
}

impl<'a> NodePointer<'a> for GraphNodePointer<'a> {
    fn id(&self) -> Option<NodeId> {
        Some(self.id)
    }

    fn next_id(&self) -> Option<NodeId> {
        self.graph.graph_node(self.id).next_id()
    }

    fn child_id(&self) -> Option<NodeId> {
        self.graph.graph_node(self.id).child_id()
    }

    fn prev_id(&self) -> Option<NodeId> {
        self.graph.graph_node(self.id).prev_id()
    }

    fn to(&self, id: NodeId) -> Self {
        GraphNodePointer {
            id,
            graph: self.graph,
        }
    }
}

impl<'a> NodeIter<'a> for GraphNodePointer<'a> {
    fn next(&self) -> Option<Self> {
        self.graph
            .graph_node(self.id)
            .next_id()
            .map(|id| GraphNodePointer {
                graph: self.graph,
                id,
            })
    }

    fn child(&self) -> Option<Self> {
        self.graph
            .graph_node(self.id)
            .child_id()
            .map(|id| GraphNodePointer {
                graph: self.graph,
                id,
            })
    }

    fn node(&self) -> Option<Node> {
        self.graph.node(self.id)
    }
}
