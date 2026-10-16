use std::collections::{HashMap, HashSet};

use crate::model::key_index::KeyIndex;
use crate::model::reference::Target;
use crate::model::{Key, NodeId};

use super::Graph;

type Names = HashMap<String, HashSet<NodeId>>;

#[derive(Default, Clone)]
struct Edges {
    by_key: HashMap<Key, HashSet<NodeId>>,
    by_name: HashMap<String, Names>,
}

impl Edges {
    fn merge(&mut self, other: Edges) {
        for (key, set) in other.by_key {
            self.by_key.entry(key).or_default().extend(set);
        }
        for (bucket, names) in other.by_name {
            let entry = self.by_name.entry(bucket).or_default();
            for (written, set) in names {
                entry.entry(written).or_default().extend(set);
            }
        }
    }

    fn retain_sources(&mut self, keep: impl Fn(NodeId) -> bool) {
        for set in self.by_key.values_mut() {
            set.retain(|id| keep(*id));
        }
        self.by_key.retain(|_, set| !set.is_empty());
        for names in self.by_name.values_mut() {
            for set in names.values_mut() {
                set.retain(|id| keep(*id));
            }
            names.retain(|_, set| !set.is_empty());
        }
        self.by_name.retain(|_, names| !names.is_empty());
    }

    fn insert(&mut self, target: &Target, node_id: NodeId) {
        match target {
            Target::Path { key, .. } => {
                self.by_key.entry(key.clone()).or_default().insert(node_id);
            }
            Target::Name(written) => {
                self.by_name
                    .entry(KeyIndex::name_bucket(written))
                    .or_default()
                    .entry(written.clone())
                    .or_default()
                    .insert(node_id);
            }
        }
    }

    fn remove(&mut self, target: &Target, node_id: NodeId) {
        match target {
            Target::Path { key, .. } => remove_from(&mut self.by_key, key, node_id),
            Target::Name(written) => {
                let bucket = KeyIndex::name_bucket(written);
                if let Some(names) = self.by_name.get_mut(&bucket) {
                    remove_from(names, written, node_id);
                    if names.is_empty() {
                        self.by_name.remove(&bucket);
                    }
                }
            }
        }
    }

    fn to_key(&self, key: &Key) -> Vec<NodeId> {
        self.by_key
            .get(key)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn names_in(&self, bucket: &str) -> impl Iterator<Item = (&String, &HashSet<NodeId>)> {
        self.by_name.get(bucket).into_iter().flatten()
    }

    fn names(&self) -> impl Iterator<Item = &String> {
        self.by_name.values().flat_map(|names| names.keys())
    }

    #[cfg(test)]
    fn edge_count(&self) -> usize {
        self.by_key.values().map(|set| set.len()).sum::<usize>()
            + self
                .by_name
                .values()
                .flat_map(|names| names.values())
                .map(|set| set.len())
                .sum::<usize>()
    }

    #[cfg(test)]
    fn key_count(&self) -> usize {
        self.by_key.len()
            + self
                .by_name
                .values()
                .map(|names| names.len())
                .sum::<usize>()
    }
}

fn remove_from<K: std::hash::Hash + Eq>(
    map: &mut HashMap<K, HashSet<NodeId>>,
    key: &K,
    node_id: NodeId,
) {
    if let Some(set) = map.get_mut(key) {
        set.remove(&node_id);
        if set.is_empty() {
            map.remove(key);
        }
    }
}

#[derive(Default, Clone)]
pub struct RefIndex {
    inclusion: Edges,
    reference: Edges,
}

impl RefIndex {
    pub fn new() -> RefIndex {
        RefIndex::default()
    }

    pub fn merge(&mut self, other: RefIndex) {
        self.inclusion.merge(other.inclusion);
        self.reference.merge(other.reference);
    }

    pub fn remove_edges_from_sources(&mut self, graph: &Graph, sources: &HashSet<Key>) {
        let keep = |id: NodeId| !sources.contains(&graph.node_key(id));
        self.inclusion.retain_sources(keep);
        self.reference.retain_sources(keep);
    }

    pub fn get_inclusion_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.inclusion.to_key(key)
    }

    pub fn get_reference_edges_to(&self, key: &Key) -> Vec<NodeId> {
        self.reference.to_key(key)
    }

    pub fn inclusion_names_in(
        &self,
        bucket: &str,
    ) -> impl Iterator<Item = (&String, &HashSet<NodeId>)> {
        self.inclusion.names_in(bucket)
    }

    pub fn reference_names_in(
        &self,
        bucket: &str,
    ) -> impl Iterator<Item = (&String, &HashSet<NodeId>)> {
        self.reference.names_in(bucket)
    }

    pub fn inclusion_edge_target_keys(&self) -> impl Iterator<Item = &Key> {
        self.inclusion.by_key.keys()
    }

    pub fn reference_edge_target_keys(&self) -> impl Iterator<Item = &Key> {
        self.reference.by_key.keys()
    }

    pub fn inclusion_edge_names(&self) -> impl Iterator<Item = &String> {
        self.inclusion.names()
    }

    pub fn reference_edge_names(&self) -> impl Iterator<Item = &String> {
        self.reference.names()
    }

    pub fn index_node(&mut self, graph: &Graph, root_id: NodeId) {
        Self::walk_edges(graph, root_id, |inclusion, target, node_id| {
            let edges = if inclusion {
                &mut self.inclusion
            } else {
                &mut self.reference
            };
            edges.insert(target, node_id);
        });
    }

    pub fn unindex_node(&mut self, graph: &Graph, root_id: NodeId) {
        Self::walk_edges(graph, root_id, |inclusion, target, node_id| {
            let edges = if inclusion {
                &mut self.inclusion
            } else {
                &mut self.reference
            };
            edges.remove(target, node_id);
        });
    }

    fn walk_edges<F>(graph: &Graph, root_id: NodeId, mut visit: F)
    where
        F: FnMut(bool, &Target, NodeId),
    {
        let mut stack: Vec<NodeId> = vec![root_id];
        while let Some(node_id) = stack.pop() {
            let node = graph.graph_node(node_id);
            let inclusion = node.is_ref();
            for target in node.targets(graph) {
                visit(inclusion, &target, node_id);
            }
            stack.extend(node.child_id());
            stack.extend(node.next_id());
        }
    }

    #[cfg(test)]
    pub fn edge_counts(&self) -> (usize, usize) {
        (self.inclusion.edge_count(), self.reference.edge_count())
    }

    #[cfg(test)]
    pub fn key_counts(&self) -> (usize, usize) {
        (self.inclusion.key_count(), self.reference.key_count())
    }
}
