use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use std::cmp::max;

use crate::{graph_colors::key_colors, graph_data::GraphData};

pub fn export_dot(graph_data: &GraphData) -> String {
    let mut statements = Vec::new();

    // Add nodes
    for document in graph_data.documents.values() {
        let font_size = max(12, 16 + document.depth * 8);
        let colors = key_colors(&document.key);

        let node = node!(
            document.id.to_string();
            attr!("label", &quoted(&document.title)),
            attr!("fillcolor", &quoted(colors.node_background)),
            attr!("fontsize", font_size),
            attr!("fontname", "Verdana"),
            attr!("shape", "note"),
            attr!("style", "filled")
        );
        statements.push(Stmt::Node(node));
    }

    // Add edges
    for (from_id, to_id) in &graph_data.document_to_document {
        if graph_data.documents.contains_key(to_id) {
            let edge = edge!(
                node_id!(from_id.to_string()) => node_id!(to_id.to_string());
                attr!("color", &quoted("#38546c66"))
            );
            statements.push(Stmt::Edge(edge));
        }
    }

    let g = graph!(
        di id!("G");
        attr!("rankdir", "LR"),
        attr!("fontname", "Verdana"),
        attr!("fontsize", "13"),
        attr!("nodesep", "0.7"),
        attr!("splines", "polyline"),
        attr!("pad", &quoted("0.5,0.2")),
        attr!("ranksep", "1.2"),
        attr!("overlap", "false")
    );

    // Add statements to graph
    let mut final_graph = g;
    for stmt in statements {
        final_graph.add_stmt(stmt);
    }

    format!("{}\n", final_graph.print(&mut PrinterContext::default()))
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}
