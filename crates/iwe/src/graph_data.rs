use itertools::Itertools;
use liwe::{
    graph::{Graph, GraphContext},
    model::{node::NodePointer, tree::Tree, Key, NodeId},
};
use std::collections::{HashMap, HashSet};

#[derive(Default, PartialEq, Debug, Clone)]
pub struct GraphData {
    pub sections: HashMap<NodeId, Section>,
    pub documents: HashMap<NodeId, Document>,
    pub section_to_section: Vec<(NodeId, NodeId)>,
    pub section_to_document: Vec<(NodeId, NodeId)>,
    pub document_to_document: Vec<(NodeId, NodeId)>,
}

impl GraphData {
    pub fn merge(&mut self, other: GraphData) {
        self.sections.extend(other.sections);
        self.documents.extend(other.documents);
        self.section_to_section.extend(other.section_to_section);
        self.section_to_document.extend(other.section_to_document);
        self.document_to_document.extend(other.document_to_document);
    }
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub id: NodeId,
    pub title: String,
    pub key: String,
    pub depth: u8,
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: NodeId,
    pub title: String,
    pub key: String,
    pub depth: u8,
}

#[derive(Default, Debug, PartialEq)]
struct GraphCache {
    sections: HashMap<NodeId, Section>,
    documents: HashMap<NodeId, Document>,
    section_to_section: HashSet<(NodeId, NodeId)>,
    section_to_document: HashSet<(NodeId, Key)>,
    document_to_document: HashSet<(Key, Key)>,
}

pub fn graph_data(key_filter: Option<Key>, depth: u8, graph: &Graph) -> GraphData {
    let keys = filter_keys(graph, key_filter.clone(), depth);

    keys.iter()
        .map(|pair| {
            let graph_data = build_graph_data(graph, pair.0, *pair.1);
            graph_data
        })
        .fold(
            GraphData {
                sections: HashMap::new(),
                documents: HashMap::new(),
                section_to_section: Vec::new(),
                section_to_document: Vec::new(),
                document_to_document: Vec::new(),
            },
            |mut acc, data| {
                acc.merge(data);
                acc
            },
        )
}

fn build_graph_data(graph: &Graph, key: &Key, key_depth: u8) -> GraphData {
    let tree = graph.collect(key);

    let mut cache = GraphCache {
        documents: HashMap::new(),
        sections: HashMap::new(),
        section_to_section: HashSet::new(),
        section_to_document: HashSet::new(),
        document_to_document: HashSet::new(),
    };

    build_sections(&key.to_string(), &mut cache, key_depth, 0, &tree);

    GraphData {
        sections: cache.sections.clone(),
        documents: cache.documents.clone(),
        section_to_section: cache.section_to_section.into_iter().collect_vec(),
        section_to_document: cache
            .section_to_document
            .into_iter()
            .map(|r| (r.0, resolve_key(graph, &r.1)))
            .collect_vec(),
        document_to_document: cache
            .document_to_document
            .into_iter()
            .map(|r| (resolve_key(graph, &r.0), resolve_key(graph, &r.1)))
            .collect_vec(),
    }
}

fn build_sections(key: &str, cache: &mut GraphCache, key_depth: u8, depth: u8, tree: &Tree) {
    tree.children.iter().for_each(|child| {
        if child.is_list() {
            return;
        } else if depth == 0 && child.is_section() {
            cache.documents.insert(
                child.id.unwrap(),
                Document {
                    depth: key_depth,
                    id: child.id.unwrap(),
                    title: child.node.plain_text(),
                    key: key.to_string(),
                },
            );
            build_sections(key, cache, key_depth, depth + 1, child);
        } else if child.is_section() {
            cache.sections.insert(
                child.id.unwrap(),
                Section {
                    depth: depth,
                    id: child.id.unwrap(),
                    title: child.node.plain_text(),
                    key: key.to_string(),
                },
            );
            if tree.is_section() {
                cache
                    .section_to_section
                    .insert((tree.id.unwrap(), child.id.unwrap()));
            }
            build_sections(key, cache, key_depth, depth + 1, child);
        } else if child.is_reference() {
            if let Some(ref_key) = child.node.reference_key() {
                cache
                    .section_to_document
                    .insert((tree.id.unwrap(), ref_key.clone()));

                cache.document_to_document.insert((key.into(), ref_key));
            }
        }
    })
}

fn filter_keys(graph: &Graph, key_filter: Option<Key>, depth_limit: u8) -> HashMap<Key, u8> {
    key_filter
        .clone()
        .map(|key| {
            if graph.maybe_key(&key).is_none() {
                return HashMap::new();
            }
            let mut keys = HashMap::new();
            get_keys_for_depth(graph, &key, depth_limit, &mut keys);
            keys
        })
        .unwrap_or_else(|| {
            graph
                .keys()
                .into_iter()
                .filter_map(|key| {
                    if graph.maybe_key(&key).is_some() {
                        Some((key, 0))
                    } else {
                        None
                    }
                })
                .collect()
        })
}

fn get_keys_for_depth(graph: &Graph, key: &Key, depth: u8, collected_keys: &mut HashMap<Key, u8>) {
    collected_keys.insert(key.clone(), depth);

    if depth == 0 {
        return;
    }
    let keys = graph.collect(key).get_all_block_reference_keys();
    for k in keys {
        if !collected_keys.contains_key(&k) && graph.maybe_key(&k).is_some() {
            get_keys_for_depth(graph, &k, depth - 1, collected_keys);
        }
    }
}

fn resolve_key(graph: &Graph, key: &Key) -> NodeId {
    graph
        .get_node_id(&key.clone())
        .map(|doc_id| graph.node(doc_id).child_id().unwrap_or_default())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use liwe::model::node::{Node, Reference, ReferenceType};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn build_sections_one_doc() {
        let doc1 = Tree::new(
            Some(1),
            Node::Document(Key::name("1")),
            vec![Tree::new(
                Some(2),
                Node::Section(vec!["title".to_string().into()]),
                vec![
                    Tree::new(
                        Some(3),
                        Node::Section(vec!["1.1".to_string().into()]),
                        vec![Tree::new(
                            Some(4),
                            Node::Reference(Reference {
                                key: Key::name("2"),
                                text: "".into(),
                                reference_type: ReferenceType::Regular,
                            }),
                            vec![],
                        )],
                    ),
                    Tree::new(
                        Some(5),
                        Node::Section(vec!["1.2".to_string().into()]),
                        vec![Tree::new(
                            Some(6),
                            Node::Reference(Reference {
                                key: Key::name("3"),
                                text: "".into(),
                                reference_type: ReferenceType::Regular,
                            }),
                            vec![],
                        )],
                    ),
                ],
            )],
        );

        let mut cache = GraphCache::default();

        build_sections("1", &mut cache, 2, 0, &doc1);

        assert_eq!(
            GraphCache {
                sections: vec![
                    (
                        3,
                        Section {
                            id: 3,
                            title: "1.1".into(),
                            key: "1".into(),
                            depth: 1,
                        }
                    ),
                    (
                        5,
                        Section {
                            id: 5,
                            title: "1.2".into(),
                            key: "1".into(),
                            depth: 1,
                        }
                    )
                ]
                .into_iter()
                .collect(),
                documents: vec![(
                    2,
                    Document {
                        id: 2,
                        title: "title".into(),
                        key: "1".into(),
                        depth: 2,
                    }
                )]
                .into_iter()
                .collect(),
                section_to_section: vec![(2, 5), (2, 3)].into_iter().collect(),
                section_to_document: vec![(3, Key::name("2")), (5, Key::name("3"))]
                    .into_iter()
                    .collect(),
                document_to_document: vec![
                    (Key::name("1"), Key::name("2")),
                    (Key::name("1"), Key::name("3"))
                ]
                .into_iter()
                .collect(),
            },
            cache
        );
    }
}
