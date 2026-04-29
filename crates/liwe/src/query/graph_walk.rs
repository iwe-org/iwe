use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::{Graph, GraphContext};
use crate::model::Key;

pub(crate) fn descendants_inclusion(
    graph: &Graph,
    anchor: &Key,
    max_depth: u32,
) -> HashMap<Key, u32> {
    bfs_inclusion_outbound(graph, anchor, max_depth)
}

pub(crate) fn ancestors_inclusion(
    graph: &Graph,
    anchor: &Key,
    max_depth: u32,
) -> HashMap<Key, u32> {
    bfs_inclusion_inbound(graph, anchor, max_depth)
}

pub(crate) fn outbound_reference(
    graph: &Graph,
    anchor: &Key,
    max_distance: u32,
) -> HashMap<Key, u32> {
    bfs_reference(graph, anchor, max_distance, Direction::Outbound)
}

pub(crate) fn inbound_reference(
    graph: &Graph,
    anchor: &Key,
    max_distance: u32,
) -> HashMap<Key, u32> {
    bfs_reference(graph, anchor, max_distance, Direction::Inbound)
}

#[derive(Clone, Copy)]
enum Direction {
    Outbound,
    Inbound,
}

fn bfs_inclusion_outbound(graph: &Graph, anchor: &Key, max_depth: u32) -> HashMap<Key, u32> {
    let mut out: HashMap<Key, u32> = HashMap::new();
    let mut queue: VecDeque<(Key, u32)> = VecDeque::new();
    queue.push_back((anchor.clone(), 0));
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let next_depth = depth + 1;
        for node_id in graph.get_inclusion_edges_in(&current) {
            let target = match graph.graph_node(node_id).ref_key() {
                Some(k) => k,
                None => continue,
            };
            if target == *anchor {
                continue;
            }
            if out.contains_key(&target) {
                continue;
            }
            out.insert(target.clone(), next_depth);
            queue.push_back((target, next_depth));
        }
    }
    out
}

fn bfs_inclusion_inbound(graph: &Graph, anchor: &Key, max_depth: u32) -> HashMap<Key, u32> {
    let mut out: HashMap<Key, u32> = HashMap::new();
    let mut queue: VecDeque<(Key, u32)> = VecDeque::new();
    queue.push_back((anchor.clone(), 0));
    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let next_depth = depth + 1;
        for node_id in graph.get_inclusion_edges_to(&current) {
            let parent = graph.key_of(node_id);
            if parent == *anchor {
                continue;
            }
            if out.contains_key(&parent) {
                continue;
            }
            out.insert(parent.clone(), next_depth);
            queue.push_back((parent, next_depth));
        }
    }
    out
}

fn bfs_reference(
    graph: &Graph,
    anchor: &Key,
    max_distance: u32,
    direction: Direction,
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
        let neighbors: Vec<Key> = match direction {
            Direction::Outbound => graph.get_reference_edges_in(&current),
            Direction::Inbound => graph
                .get_reference_edges_to(&current)
                .into_iter()
                .map(|node_id| graph.key_of(node_id))
                .collect(),
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
