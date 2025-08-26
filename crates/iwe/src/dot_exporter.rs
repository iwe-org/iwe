use std::{cmp::max, hash::Hash};

use crate::graph_data::GraphData;
use std::hash::{DefaultHasher, Hasher};

pub struct DotExporter {}

impl DotExporter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn export(&self, graph_data: &GraphData) -> String {
        let mut output = String::new();

        output.push_str(&self.generate_graph_opening());
        output.push_str(&self.generate_nodes(&graph_data));
        output.push_str(&self.generate_subgraphs(&graph_data));
        output.push_str(&self.generate_edges(&graph_data));
        output.push_str(&self.generate_graph_closing());

        output
    }

    fn generate_graph_opening(&self) -> String {
        r###"digraph G {
            graph [
              rankdir=LR
              fontname="Verdana"
              fontsize=13
              nodesep=0.7
              splines=polyline
              pad="0.5,0.2"
              ranksep=1.2
              overlap=false

            ];
            node [
              fontname="Verdana"
              fontsize=11
              color="#b3b3b3"
              penwidth=1.5
            ];
            edge [
              color="#38546c66"
              arrowhead=normal
              penwidth=1.2
            ];

            "###
        .to_string()
    }

    fn generate_nodes(&self, graph_data: &GraphData) -> String {
        let mut nodes_output = String::new();

        for section in graph_data.sections.values() {
            let escaped_title = section
                .title
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", " ")
                .replace("\r", " ")
                .replace("\t", " ");

            let font_size = max(12, 16 + section.key_depth * 8 - max(0, 8 * section.depth));
            let colors = key_colors(&section.key);

            if section.depth == 0 {
                nodes_output.push_str(&format!(
                    r###"
                    {} [label="{}", fillcolor="{}", fontsize="{}", shape="note", style="filled,rounded"];
                    "###,
                    section.id, escaped_title, colors.node_background, font_size,
                ));
            } else {
                nodes_output.push_str(&format!(
                    r###"
                    {} [label="{}", fontsize="{}", shape="plain"];
                    "###,
                    section.id,
                    escaped_title,
                    font_size - 2,
                ));
            }
        }

        nodes_output.push_str("\n");
        nodes_output
    }

    fn generate_subgraphs(&self, graph_data: &GraphData) -> String {
        let mut subgraphs_output = String::new();
        for (i, subgraph) in graph_data.documents.values().enumerate() {
            let colors = key_colors(&subgraph.key);
            subgraphs_output.push_str(&format!("  subgraph cluster_{} {{\n", i));
            subgraphs_output.push_str(&format!(
                r###"
                labeljust="l"
                style="filled,rounded"
                color="{}"
                fillcolor="{}"
                fontcolor="{}"
                penwidth=40
                "###,
                colors.subgraph_fill, colors.subgraph_fill, colors.subgraph_text
            ));

            subgraphs_output.push_str("");
            for &node_id in &subgraph.nodes {
                subgraphs_output.push_str(&format!("    {};\n", node_id));
            }
            subgraphs_output.push_str("  }\n");
        }

        subgraphs_output.push_str("\n");
        subgraphs_output
    }

    fn generate_edges(&self, graph_data: &GraphData) -> String {
        let mut edges_output = String::new();

        for (from_id, to_id) in &graph_data.section_to_key {
            if graph_data.sections.contains_key(to_id) {
                edges_output.push_str(&format!(
                    "  {} -> {} [arrowsize=1.5, arrowhead=\"empty\", style=\"dashed\"]; \n",
                    from_id, to_id
                ));
            }
        }

        for (from_id, to_id) in &graph_data.section_to_section {
            edges_output.push_str(&format!("  {} -> {};\n", from_id, to_id));
        }
        edges_output
    }

    fn generate_graph_closing(&self) -> String {
        "}\n".to_string()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SubgraphColor {
    pub subgraph_fill: &'static str,   // fillcolor
    pub subgraph_text: &'static str,   // fontcolor
    pub node_background: &'static str, // fillcolor for node
}

pub const SUBGRAPH_COLORS: [SubgraphColor; 14] = [
    SubgraphColor {
        // Pastel Blue
        subgraph_fill: "#eff8fd",
        subgraph_text: "#283747",
        node_background: "#e1f5fe",
    },
    SubgraphColor {
        // Pastel Green
        subgraph_fill: "#f6fcf5",
        subgraph_text: "#185c37",
        node_background: "#e9f9ef",
    },
    SubgraphColor {
        // Pastel Pink
        subgraph_fill: "#fff4fa",
        subgraph_text: "#a7475a",
        node_background: "#fae1ee",
    },
    SubgraphColor {
        // Pastel Yellow
        subgraph_fill: "#fffbea",
        subgraph_text: "#a67c00",
        node_background: "#fff9de",
    },
    SubgraphColor {
        // Pastel Lavender
        subgraph_fill: "#f8f8ff",
        subgraph_text: "#442b7e",
        node_background: "#eeebfa",
    },
    SubgraphColor {
        // Pastel Mint
        subgraph_fill: "#f3fcf7",
        subgraph_text: "#257257",
        node_background: "#d9fae7",
    },
    SubgraphColor {
        // Pastel Peach
        subgraph_fill: "#fff6eb",
        subgraph_text: "#b36a36",
        node_background: "#ffeee0",
    },
    SubgraphColor {
        // Pastel Aqua
        subgraph_fill: "#f0fbfc",
        subgraph_text: "#1d4c56",
        node_background: "#dcf7f7",
    },
    SubgraphColor {
        // Pastel Lilac
        subgraph_fill: "#fdf7fa",
        subgraph_text: "#764470",
        node_background: "#f6e5ee",
    },
    SubgraphColor {
        // Pastel Lemon
        subgraph_fill: "#fefeec",
        subgraph_text: "#96902d",
        node_background: "#fbfbda",
    },
    SubgraphColor {
        // Pastel Coral
        subgraph_fill: "#fef7f7",
        subgraph_text: "#c35151",
        node_background: "#ffeaea",
    },
    SubgraphColor {
        // Pastel Teal
        subgraph_fill: "#f0fafb",
        subgraph_text: "#225c5a",
        node_background: "#e2f6f6",
    },
    SubgraphColor {
        // Pastel Grey
        subgraph_fill: "#f7f9fa",
        subgraph_text: "#4a525a",
        node_background: "#eef2f5",
    },
    SubgraphColor {
        // Pastel Olive
        subgraph_fill: "#faffef",
        subgraph_text: "#847c36",
        node_background: "#f2fadf",
    },
];

pub fn key_colors(key: &str) -> SubgraphColor {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    SUBGRAPH_COLORS[(hasher.finish() as usize) % 14]
}
