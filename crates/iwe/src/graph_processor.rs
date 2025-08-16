use liwe::graph::{Graph, GraphContext};
use std::collections::HashMap;

use crate::graphviz_export::GraphNode;

pub struct GraphProcessor {
    key_filter: Option<String>,
    depth_limit: u8,
}

impl GraphProcessor {
    pub fn new(key_filter: Option<String>, depth_limit: u8) -> Self {
        Self {
            key_filter,
            depth_limit,
        }
    }

    pub fn process_graph(&self, graph: &Graph) -> Vec<GraphNode> {
        let filtered_paths = self.filter_paths(graph);
        let mut nodes = self.build_nodes(graph, &filtered_paths);
        self.establish_relationships(&filtered_paths, &mut nodes);
        self.calculate_ranks(&mut nodes);
        self.sort_nodes(nodes)
    }

    fn filter_paths(&self, graph: &Graph) -> Vec<liwe::graph::path::NodePath> {
        let all_paths = graph.paths();

        if let Some(key) = &self.key_filter {
            all_paths
                .iter()
                .filter(|path| {
                    // Check if any node in the path matches the key (search in both key and title)
                    path.ids().iter().any(|&id| {
                        let node_key = graph.key_of(id);
                        let node_title = graph.get_text(id);
                        node_key.to_string().contains(key)
                            || node_title.to_lowercase().contains(&key.to_lowercase())
                    })
                })
                .filter(|path| {
                    // Apply depth filter if specified (0 means no limit)
                    self.depth_limit == 0 || path.ids().len() <= self.depth_limit as usize
                })
                .cloned()
                .collect()
        } else {
            // Apply only depth filter
            all_paths
                .iter()
                .filter(|path| {
                    self.depth_limit == 0 || path.ids().len() <= self.depth_limit as usize
                })
                .cloned()
                .collect()
        }
    }

    fn build_nodes(
        &self,
        graph: &Graph,
        paths: &[liwe::graph::path::NodePath],
    ) -> HashMap<u64, GraphNode> {
        let mut nodes: HashMap<u64, GraphNode> = HashMap::new();

        // Create all nodes
        for path in paths {
            for &node_id in path.ids().iter() {
                if !nodes.contains_key(&node_id) {
                    let title = graph.get_text(node_id).trim().to_string();
                    let rank = 0; // Will be calculated later
                    nodes.insert(node_id, GraphNode::new(node_id as i64, &title, rank));
                }
            }
        }

        nodes
    }

    fn establish_relationships(
        &self,
        paths: &[liwe::graph::path::NodePath],
        nodes: &mut HashMap<u64, GraphNode>,
    ) {
        // Establish parent-child relationships
        for path in paths {
            let ids = path.ids();
            for i in 0..ids.len() - 1 {
                let parent_id = ids[i];
                let child_id = ids[i + 1];

                if let Some(parent_node) = nodes.get_mut(&parent_id) {
                    if !parent_node.links.contains(&(child_id as i64)) {
                        parent_node.add_link(child_id as i64);
                    }
                }
            }
        }
    }

    fn calculate_ranks(&self, nodes: &mut HashMap<u64, GraphNode>) {
        // Calculate and update ranks for all nodes
        let calculated_ranks: HashMap<i64, usize> = nodes
            .iter()
            .map(|(_, node)| (node.id, self.count_descendants(node.id, nodes)))
            .collect();

        // Update the rank field in each node
        for (_, node) in nodes.iter_mut() {
            node.rank = *calculated_ranks.get(&node.id).unwrap_or(&0);
        }
    }

    fn count_descendants(&self, node_id: i64, nodes: &HashMap<u64, GraphNode>) -> usize {
        let mut count = 0;
        if let Some(node) = nodes.get(&(node_id as u64)) {
            for &child_id in &node.links {
                count += 1; // Count the child itself
                count += self.count_descendants(child_id, nodes); // Count its descendants
            }
        }
        count
    }

    fn sort_nodes(&self, nodes: HashMap<u64, GraphNode>) -> Vec<GraphNode> {
        // Convert to vector and sort by id for consistent output
        let mut node_list: Vec<GraphNode> = nodes.into_values().collect();
        node_list.sort_by_key(|node| node.id);
        node_list
    }
}
