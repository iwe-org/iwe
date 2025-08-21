use itertools::Itertools;
use liwe::{
    graph::{Graph, GraphContext},
    model::{node::NodePointer, tree::Tree, Key, NodeId},
};
use std::collections::{HashMap, HashSet};

#[derive(Default, PartialEq, Debug, Clone)]
pub struct GraphData {
    pub sections: HashMap<NodeId, Section>,
    pub documents: HashMap<String, Document>,
    pub sub_sections: Vec<(NodeId, NodeId)>,
    pub references: Vec<(NodeId, NodeId)>,
}

impl GraphData {
    pub fn merge(&mut self, other: GraphData) {
        self.sections.extend(other.sections);
        self.documents.extend(other.documents);
        self.sub_sections.extend(other.sub_sections);
        self.references.extend(other.references);
    }
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Section {
    pub id: NodeId,
    pub title: String,
    pub key: String,
    pub key_depth: u8,
    pub depth: u8,
}

#[derive(PartialEq, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub key: String,
    pub nodes: Vec<NodeId>,
}

pub fn build_sections(
    key: &str,
    key_depth: u8,
    depth: u8,
    tree: &Tree,
    sections: &mut HashMap<NodeId, Section>,
    sub_sections: &mut HashSet<(NodeId, NodeId)>,
    references: &mut HashSet<(NodeId, Key)>,
) {
    tree.children.iter().for_each(|child| {
        if child.is_list() {
            return;
        }

        if child.is_section() {
            sections.insert(
                child.id.unwrap(),
                Section {
                    key_depth: key_depth,
                    depth: depth,
                    id: child.id.unwrap(),
                    title: child.node.plain_text(),
                    key: key.to_string(),
                },
            );
            if tree.is_section() {
                sub_sections.insert((tree.id.unwrap(), child.id.unwrap()));
            }
            build_sections(
                key,
                key_depth,
                depth + 1,
                child,
                sections,
                sub_sections,
                references,
            );
        }

        if child.is_reference() {
            if let Some(key) = child.node.reference_key() {
                references.insert((tree.id.unwrap(), key));
            }
        }
    })
}

pub fn filter_keys(graph: &Graph, key_filter: Option<Key>, depth_limit: u8) -> HashMap<Key, u8> {
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

pub fn get_keys_for_depth(
    graph: &Graph,
    key: &Key,
    depth: u8,
    collected_keys: &mut HashMap<Key, u8>,
) {
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

pub fn build_graph_data(graph: &Graph, key: &Key, key_depth: u8) -> GraphData {
    let mut sections = HashMap::<NodeId, Section>::new();
    let mut sub_sections = HashSet::<(NodeId, NodeId)>::new();
    let mut references = HashSet::<(NodeId, Key)>::new();
    let mut subgraphs = HashMap::new();

    let tree = graph.collect(key);

    build_sections(
        &key.to_string(),
        key_depth,
        0,
        &tree,
        &mut sections,
        &mut sub_sections,
        &mut references,
    );

    subgraphs.insert(
        key.to_string(),
        Document {
            key: key.to_string(),
            nodes: sections.clone().into_keys().collect_vec(),
        },
    );

    GraphData {
        sections,
        documents: subgraphs,
        sub_sections: sub_sections.into_iter().collect_vec(),
        references: references
            .into_iter()
            .map(|r| {
                (
                    r.0,
                    graph
                        .get_node_id(&r.1.clone())
                        .map(|doc_id| graph.node(doc_id).child_id().unwrap_or_default())
                        .unwrap_or_default(),
                )
            })
            .collect_vec(),
    }
}
