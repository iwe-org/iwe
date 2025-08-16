use liwe::graph::Graph;

use crate::graph_processor::GraphProcessor;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: i64,
    pub title: String,
    pub rank: usize,
    pub links: Vec<i64>,
}

impl GraphNode {
    pub fn new(id: i64, title: &str, rank: usize) -> Self {
        GraphNode {
            id,
            title: title.to_string(),
            rank,
            links: Vec::new(),
        }
    }

    pub fn add_link(&mut self, link_id: i64) {
        self.links.push(link_id);
    }
}

pub struct GraphvizExporter {
    key_filter: Option<String>,
    depth_limit: u8,
}

impl GraphvizExporter {
    pub fn new(key_filter: Option<String>, depth_limit: u8) -> Self {
        Self {
            key_filter,
            depth_limit,
        }
    }

    pub fn export(&self, graph: &Graph) -> String {
        let mut output = String::new();

        // Use the shared graph processor to build the node list
        let processor = GraphProcessor::new(self.key_filter.clone(), self.depth_limit);
        let node_list = processor.process_graph(graph);

        // Generate the GraphViz output
        output.push_str(&self.generate_graph_opening());
        output.push_str(&self.generate_nodes(&node_list));
        output.push_str(&self.generate_edges(&node_list));
        output.push_str(&self.generate_graph_closing());

        output
    }

    fn generate_graph_opening(&self) -> String {
        let mut opening = String::new();
        opening.push_str("digraph {\n");
        opening.push_str("    graph [overlap_scaling=3, pack=90, label=\"IWE Knowledge Graph - Self-contained with embedded CSS\"];\n");
        opening.push_str("    node [label=\"\\N\"];\n\n");
        opening
    }

    fn generate_nodes(&self, node_list: &[GraphNode]) -> String {
        let mut nodes_output = String::new();

        for node in node_list {
            let escaped_title = node
                .title
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", " ")
                .replace("\r", " ")
                .replace("\t", " ");

            let rank = node.rank;

            // Determine group and CSS class based on rank
            let (group, css_class) = match rank {
                0 => (1, "leaf"),       // Leaf nodes
                1..=2 => (2, "small"),  // Small branches
                3..=5 => (3, "medium"), // Medium branches
                6..=10 => (4, "large"), // Large branches
                _ => (5, "major"),      // Major nodes
            };

            nodes_output.push_str(&format!(
                "    {} [label=\"{}\", group={}, class=\"{}\"];\n",
                node.id, escaped_title, group, css_class
            ));
        }

        nodes_output.push_str("\n");
        nodes_output
    }

    fn generate_edges(&self, node_list: &[GraphNode]) -> String {
        let mut edges_output = String::new();

        for node in node_list {
            for &link_id in &node.links {
                edges_output.push_str(&format!("    {} -> {};\n", node.id, link_id));
            }
        }

        edges_output
    }

    fn generate_graph_closing(&self) -> String {
        "}\n".to_string()
    }
}
