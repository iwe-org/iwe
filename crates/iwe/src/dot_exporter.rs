use std::cmp::max;

use crate::{graph_colors::key_colors, graph_data::GraphData};

pub fn export_dot(graph_data: &GraphData) -> String {
    let mut output = String::new();

    output.push_str(&generate_graph_opening());
    output.push_str(&generate_nodes(&graph_data));
    output.push_str(&generate_edges(&graph_data));
    output.push_str(&generate_graph_closing());

    output
}

fn generate_graph_opening() -> String {
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

fn generate_nodes(graph_data: &GraphData) -> String {
    let mut nodes_output = String::new();

    for document in graph_data.documents.values() {
        let font_size = max(12, 16 + document.depth * 8);
        let colors = key_colors(&document.key);

        nodes_output.push_str(&format!(
                r###"
                {} [label="{}", fillcolor="{}", fontsize="{}", shape="note", style="filled,rounded"];
                "###,
                document.id, document.title, colors.node_background, font_size,
            ));
    }

    nodes_output.push_str("\n");
    nodes_output
}

fn generate_edges(graph_data: &GraphData) -> String {
    let mut edges_output = String::new();

    for (from_id, to_id) in &graph_data.document_to_document {
        if graph_data.documents.contains_key(to_id) {
            edges_output.push_str(&format!("  {} -> {};\n", from_id, to_id));
        }
    }

    edges_output
}

fn generate_graph_closing() -> String {
    "}\n".to_string()
}
