use liwe::graph::Graph;

use crate::graph_processor::GraphProcessor;

pub struct JsonExporter {}

impl JsonExporter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn export(&self, graph: &Graph) -> String {
        // Use the shared graph processor to build the node list
        let processor = GraphProcessor::new_unfiltered();
        let node_list = processor.process_graph(graph);

        // Serialize to JSON
        serde_json::to_string_pretty(&node_list).expect("Failed to serialize to JSON")
    }
}
