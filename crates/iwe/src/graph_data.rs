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
    pub section_to_section: Vec<(NodeId, NodeId)>,
    pub section_to_key: Vec<(NodeId, NodeId)>,
    pub key_to_key: Vec<(NodeId, NodeId)>,
}

impl GraphData {
    pub fn merge(&mut self, other: GraphData) {
        self.sections.extend(other.sections);
        self.documents.extend(other.documents);
        self.section_to_section.extend(other.section_to_section);
        self.section_to_key.extend(other.section_to_key);
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

struct GraphCache {
    sections: HashMap<NodeId, Section>,
    section_to_sections: HashSet<(NodeId, NodeId)>,
    sections_to_keys: HashSet<(NodeId, Key)>,
    keys_to_keys: HashSet<(Key, Key)>,
}

pub fn build_graph_data(graph: &Graph, key: &Key, key_depth: u8) -> GraphData {
    let tree = graph.collect(key);

    let mut cache = GraphCache {
        sections: HashMap::new(),
        section_to_sections: HashSet::new(),
        sections_to_keys: HashSet::new(),
        keys_to_keys: HashSet::new(),
    };

    build_sections(&key.to_string(), &mut cache, key_depth, 0, &tree);

    GraphData {
        sections: cache.sections.clone(),
        documents: [(
            key.to_string(),
            Document {
                key: key.to_string(),
                nodes: cache.sections.into_keys().collect_vec(),
            },
        )]
        .into_iter()
        .collect(),
        section_to_section: cache.section_to_sections.into_iter().collect_vec(),
        section_to_key: cache
            .sections_to_keys
            .into_iter()
            .map(|r| (r.0, resolve_key(graph, &r.1)))
            .collect_vec(),
        key_to_key: cache
            .keys_to_keys
            .into_iter()
            .map(|r| (resolve_key(graph, &r.0), resolve_key(graph, &r.1)))
            .collect_vec(),
    }
}

fn build_sections(key: &str, cache: &mut GraphCache, key_depth: u8, depth: u8, tree: &Tree) {
    tree.children.iter().for_each(|child| {
        if child.is_list() {
            return;
        }

        if child.is_section() {
            cache.sections.insert(
                child.id.unwrap(),
                Section {
                    key_depth,
                    depth: depth,
                    id: child.id.unwrap(),
                    title: child.node.plain_text(),
                    key: key.to_string(),
                },
            );
            if tree.is_section() {
                cache
                    .section_to_sections
                    .insert((tree.id.unwrap(), child.id.unwrap()));
            }
            build_sections(key, cache, key_depth, depth + 1, child);
        }

        if child.is_reference() {
            if let Some(ref_key) = child.node.reference_key() {
                cache
                    .sections_to_keys
                    .insert((tree.id.unwrap(), ref_key.clone()));

                cache.keys_to_keys.insert((key.into(), ref_key));
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
