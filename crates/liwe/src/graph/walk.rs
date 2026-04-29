use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::{Graph, GraphContext};
use crate::model::Key;

pub fn descendants_inclusion(graph: &Graph, anchor: &Key, max_depth: u32) -> HashMap<Key, u32> {
    bfs_inclusion(graph, anchor, max_depth, true)
}

pub fn ancestors_inclusion(graph: &Graph, anchor: &Key, max_depth: u32) -> HashMap<Key, u32> {
    bfs_inclusion(graph, anchor, max_depth, false)
}

pub fn outbound_reference(graph: &Graph, anchor: &Key, max_distance: u32) -> HashMap<Key, u32> {
    bfs_reference(graph, anchor, max_distance, true)
}

pub fn inbound_reference(graph: &Graph, anchor: &Key, max_distance: u32) -> HashMap<Key, u32> {
    bfs_reference(graph, anchor, max_distance, false)
}

fn bfs_inclusion(
    graph: &Graph,
    anchor: &Key,
    max_depth: u32,
    outbound: bool,
) -> HashMap<Key, u32> {
    let mut out: HashMap<Key, u32> = HashMap::new();
    let mut queue: VecDeque<(Key, u32)> = VecDeque::new();
    queue.push_back((anchor.clone(), 0));
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let next_depth = depth + 1;
        let neighbors: Vec<Key> = if outbound {
            graph
                .get_inclusion_edges_in(&current)
                .into_iter()
                .filter_map(|node_id| graph.graph_node(node_id).ref_key())
                .collect()
        } else {
            graph
                .get_inclusion_edges_to(&current)
                .into_iter()
                .map(|node_id| graph.key_of(node_id))
                .collect()
        };
        for neighbor in neighbors {
            if neighbor == *anchor {
                continue;
            }
            if out.contains_key(&neighbor) {
                continue;
            }
            out.insert(neighbor.clone(), next_depth);
            queue.push_back((neighbor, next_depth));
        }
    }
    out
}

fn bfs_reference(
    graph: &Graph,
    anchor: &Key,
    max_distance: u32,
    outbound: bool,
) -> HashMap<Key, u32> {
    let mut out: HashMap<Key, u32> = HashMap::new();
    let mut visited: HashSet<Key> = HashSet::new();
    visited.insert(anchor.clone());
    let mut queue: VecDeque<(Key, u32)> = VecDeque::new();
    queue.push_back((anchor.clone(), 0));
    while let Some((current, distance)) = queue.pop_front() {
        if distance >= max_distance {
            continue;
        }
        let next_distance = distance + 1;
        let neighbors: Vec<Key> = if outbound {
            graph.get_reference_edges_in(&current)
        } else {
            graph
                .get_reference_edges_to(&current)
                .into_iter()
                .map(|node_id| graph.key_of(node_id))
                .collect()
        };
        for neighbor in neighbors {
            if !visited.insert(neighbor.clone()) {
                continue;
            }
            if neighbor == *anchor {
                continue;
            }
            out.entry(neighbor.clone()).or_insert(next_distance);
            queue.push_back((neighbor, next_distance));
        }
    }
    out
}
