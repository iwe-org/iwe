use liwe::graph::Graph;

use crate::graph_processor::GraphProcessor;

pub struct JsonExporter {
    key_filter: Option<String>,
    depth_limit: u8,
}

impl JsonExporter {
    pub fn new(key_filter: Option<String>, depth_limit: u8) -> Self {
        Self {
            key_filter,
            depth_limit,
        }
    }

    pub fn export(&self, graph: &Graph) -> String {
        // Use the shared graph processor to build the node list
        let processor = GraphProcessor::new(self.key_filter.clone(), self.depth_limit);
        let node_list = processor.process_graph(graph);

        // Serialize to JSON
        serde_json::to_string_pretty(&node_list).expect("Failed to serialize to JSON")
    }
}
