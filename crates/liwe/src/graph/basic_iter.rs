use super::graph_node::GraphNode;
use super::Graph;
use super::GraphContext;
use crate::model::node::Node;
use crate::model::node::NodeIter;
use crate::model::node::NodePointer;
use crate::model::node::Reference;
use crate::model::node::ReferenceType;
use crate::model::node::Table;
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

    fn to_node(&self, id: NodeId) -> Self {
        GraphNodePointer {
            id,
            graph: self.graph,
        }
    }

    fn to_key(&self, key: crate::model::Key) -> Option<Self> {
        self.graph.get_node_id(&key).map(|id| GraphNodePointer {
            id,
            graph: self.graph,
        })
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
        match self.graph.graph_node(self.id) {
            GraphNode::Empty => None,
            GraphNode::Document(document) => Some(Node::Document(document.key().clone())),
            GraphNode::Section(section) => Some(Node::Section(
                self.graph.get_line(section.line_id()).normalize(self.graph),
            )),
            GraphNode::Quote(_) => Some(Node::Quote()),
            GraphNode::BulletList(_) => Some(Node::BulletList()),
            GraphNode::OrderedList(_) => Some(Node::OrderedList()),
            GraphNode::Leaf(leaf) => Some(Node::Leaf(
                self.graph.get_line(leaf.line_id()).normalize(self.graph),
            )),
            GraphNode::Raw(raw) => Some(Node::Raw(raw.lang(), raw.content().to_string())),
            GraphNode::HorizontalRule(_) => Some(Node::HorizontalRule()),
            GraphNode::Reference(reference) => {
                let text = match reference.reference_type() {
                    ReferenceType::Regular => self
                        .graph
                        .get_ref_text(reference.key())
                        .unwrap_or(reference.text().to_string()),
                    ReferenceType::WikiLink => String::default(),
                    ReferenceType::WikiLinkPiped => reference.text().to_string(),
                };

                Some(Node::Reference(Reference {
                    key: reference.key().clone(),
                    text,
                    reference_type: reference.reference_type(),
                }))
            }
            GraphNode::Table(table) => Some(Node::Table(Table {
                header: table
                    .header()
                    .iter()
                    .map(|id| self.graph.get_line(*id).normalize(self.graph))
                    .collect(),
                rows: table
                    .rows()
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|id| self.graph.get_line(*id).normalize(self.graph))
                            .collect()
                    })
                    .collect(),
                alignment: table.alignment().clone(),
            })),
        }
    }
}
