use std::collections::HashSet;

use itertools::Itertools;
use rayon::iter::IntoParallelIterator;

use crate::graph::graph_node::GraphNode;
use crate::graph::Graph;
use crate::model::node::{NodeIter, NodePointer};
use crate::model::NodeId;
use rayon::prelude::*;

use super::GraphContext;

#[derive(Clone, Default, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub struct NodePath {
    // parent 1, parent 2, ..., parent n, target
    ids: Vec<NodeId>,
}

impl NodePath {
    pub fn target(&self) -> NodeId {
        *self.ids.last().unwrap()
    }

    pub fn new(parents: Vec<NodeId>) -> NodePath {
        NodePath { ids: parents }
    }

    pub fn from_id(id: NodeId) -> NodePath {
        NodePath {
            ids: vec![id],
        }
    }

    pub fn from_ids(id1: NodeId, id2: NodeId) -> NodePath {
        NodePath {
            ids: vec![id1, id2],
        }
    }

    pub fn ids(&self) -> Vec<NodeId> {
        self.ids.clone()
    }

    pub fn first_id(&self) -> NodeId {
        *self.ids.first().unwrap()
    }

    pub fn last_id(&self) -> NodeId {
        *self.ids.last().unwrap()
    }

    pub fn append(&self, key: NodeId) -> NodePath {
        let mut parents = self.ids.clone();
        parents.push(key);
        NodePath { ids: parents }
    }

    pub fn combine(&self, other: &NodePath) -> NodePath {
        let mut parents = self.ids.clone();
        parents.extend(other.ids.clone());
        NodePath { ids: parents }
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.ids.contains(&id)
    }

    pub fn drop_first(&self) -> NodePath {
        let mut parents = self.ids.clone();
        parents.remove(0);
        NodePath { ids: parents }
    }
}

pub fn graph_to_paths(graph: &Graph) -> Vec<NodePath> {
    let paths: Vec<NodePath> = graph
        .nodes()
        .into_par_iter()
        .filter(|node| !matches!(node, GraphNode::Empty))
        .filter(|node| !graph.node(node.id()).is_in_list())
        .flat_map(|node| paths_for_node(graph, node.id(), &mut HashSet::new()))
        .filter(|path| !path.ids.is_empty())
        .filter(|path| {
            graph
                .index
                .get_block_references_to(&graph.node_key(path.first_id()))
                .is_empty()
                && graph
                    .node(path.first_id())
                    .to_parent()
                    .unwrap()
                    .is_document()
        })
        .collect();

    paths.into_iter().sorted().dedup().collect_vec()
}

fn paths_for_node(graph: &Graph, id: NodeId, nodes: &mut HashSet<NodeId>) -> Vec<NodePath> {
    if nodes.contains(&id) {
        return vec![];
    }

    nodes.insert(id);

    let paths = match graph.graph_node(id) {
        GraphNode::Document(document) => graph
            .index
            .get_block_references_to(document.key())
            .iter()
            .map(|node_id| graph.node(*node_id))
            .flat_map(|reference| reference.to_parent())
            .flat_map(|parent| paths_for_node(graph, parent.id().unwrap(), nodes))
            .collect_vec(),
        GraphNode::Section(_) => graph
            .node(id)
            .to_parent()
            .map(|parent| paths_for_node(graph, parent.id().unwrap(), nodes))
            .unwrap_or_default()
            .iter()
            .map(|path| path.append(id))
            .chain(vec![NodePath::from_id(id)])
            .collect_vec(),
        _ => {
            vec![]
        }
    };

    nodes.remove(&id);

    paths
}

#[cfg(test)]
mod test {

    use crate::graph::path::{graph_to_paths, NodePath};
    use crate::graph::Graph;

    #[test]
    pub fn no_parents() {
        assert_eq!(
            vec![NodePath::from_id(1)],
            graph_to_paths(
                Graph::new()
                    .build_key(&"key".into())
                    .section_text("test")
                    .graph()
            )
        );
    }

    #[test]
    pub fn two_sections() {
        assert_eq!(
            vec![NodePath::from_id(1), NodePath::from_id(2)],
            graph_to_paths(
                Graph::new()
                    .build_key(&"key".into())
                    .section_text("test")
                    .section_text("test2")
                    .graph()
            )
        );
    }

    #[test]
    pub fn list() {
        assert_eq!(
            vec![NodePath::from_id(1)],
            graph_to_paths(&Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .section_text_and("test", |s| {
                        s.bullet_list_and(|l| {
                            l.section_text("test2");
                        });
                    });
            }))
        );
    }

    #[test]
    pub fn one_parent_two_sections() {
        assert_eq!(
            vec![NodePath::new(vec![1]), NodePath::new(vec![1, 2])],
            graph_to_paths(
                Graph::new()
                    .build_key(&"a".into())
                    .section_text_and("1", |s| {
                        s.section_text("2");
                    })
                    .graph()
            )
        );
    }

    #[test]
    pub fn two_parents() {
        assert_eq!(
            vec![
                NodePath::new(vec![1]),
                NodePath::new(vec![1, 2]),
                NodePath::new(vec![1, 3])
            ],
            graph_to_paths(Graph::new().build_key_and(&"a".into(), |a| {
                a.section_text_and("1", |s1| {
                    s1.section_text("2").section_text("3");
                });
            }))
        );
    }

    #[test]
    pub fn three_segments() {
        assert_eq!(
            vec![
                NodePath::new(vec![1]),
                NodePath::new(vec![1, 2]),
                NodePath::new(vec![1, 2, 3])
            ],
            graph_to_paths(Graph::new().build_key_and(&"a".into(), |a| {
                a.section_text_and("1", |s1| {
                    s1.section_text_and("2", |s2| {
                        s2.section_text("3");
                    });
                });
            }))
        );
    }

    #[test]
    pub fn reference_parent() {
        let graph = Graph::with(|graph| {
            graph
                .build_key_and(&"a".into(), |document| {
                    document.section_text("1");
                })
                .build_key_and(&"b".into(), |document| {
                    document.section_text_and("3", |s| {
                        s.reference(&"a".into());
                    });
                });
        });

        assert_eq!(
            vec![NodePath::new(vec![3]), NodePath::new(vec![3, 1])],
            graph_to_paths(&graph)
        );
    }

    #[test]
    pub fn two_level_references() {
        let graph = Graph::with(|graph| {
            graph
                .build_key_and(&"a".into(), |document| {
                    document.section_text("1");
                })
                .build_key_and(&"b".into(), |document| {
                    document.section_text_and("3", |s| {
                        s.reference(&"a".into());
                    });
                })
                .build_key_and(&"c".into(), |document| {
                    document.section_text_and("6", |s| {
                        s.reference(&"b".into());
                    });
                });
        });

        assert_eq!(
            vec![
                NodePath::new(vec![6]),
                NodePath::new(vec![6, 3]),
                NodePath::new(vec![6, 3, 1])
            ],
            graph_to_paths(&graph)
        );
    }

    #[test]
    pub fn two_level_infinite_recursion_references() {
        let graph = Graph::with(|graph| {
            graph
                .build_key_and(&"a".into(), |document| {
                    document.reference(&"c".into());
                })
                .build_key_and(&"b".into(), |document| {
                    document.section_text_and("3", |s| {
                        s.reference(&"a".into());
                    });
                })
                .build_key_and(&"c".into(), |document| {
                    document.section_text_and("6", |s| {
                        s.reference(&"b".into());
                    });
                });
        });

        assert_eq!(0, graph_to_paths(&graph).len());
    }

    #[test]
    pub fn infinite_recursion() {
        let a_key = "a".into();
        let b_key = "b".into();
        assert_eq!(
            Vec::<NodePath>::new(),
            graph_to_paths(
                Graph::new()
                    .build_key_and(&a_key, |document| {
                        document.section_text("1").reference(&b_key);
                    })
                    .build_key_and(&b_key, |document| {
                        document.section_text("4").reference(&a_key);
                    })
            )
        );
    }
}
