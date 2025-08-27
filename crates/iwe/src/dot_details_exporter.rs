use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use std::cmp::max;

use crate::{graph_colors::key_colors, graph_data::GraphData};

pub fn export_dot_with_headers(graph_data: &GraphData) -> String {
    let mut statements = Vec::new();

    // Add document nodes
    for document in graph_data.documents.values() {
        let font_size = max(12, 16 + document.depth * 8);
        let colors = key_colors(&document.key);

        let node = node!(
            document.id.to_string();
            attr!("label", &quoted(&document.title)),
            attr!("fillcolor", &quoted(colors.node_background)),
            attr!("fontsize", font_size),
            attr!("fontname", "Verdana"),
            attr!("color", &quoted("#b3b3b3")),
            attr!("penwidth", "1.5"),
            attr!("shape", "note"),
            attr!("style", "filled")
        );
        statements.push(Stmt::Node(node));
    }

    // Add section nodes
    for section in graph_data.sections.values() {
        let font_size = max(12, 16 + section.depth * 8 - max(0, 8 * section.depth));
        let section_font_size = font_size - 2;

        let node = node!(
            section.id.to_string();
            attr!("label", &quoted(&section.title)),
            attr!("fontsize", section_font_size),
            attr!("fontname", "Verdana"),
            attr!("color", &quoted("#b3b3b3")),
            attr!("penwidth", "1.5"),
            attr!("shape", "plain")
        );
        statements.push(Stmt::Node(node));
    }

    // Add subgraphs for each document
    for (i, subgraph_doc) in graph_data.documents.values().enumerate() {
        let colors = key_colors(&subgraph_doc.key);

        // Collect section nodes that belong to this document
        let section_nodes: Vec<Stmt> = graph_data
            .sections
            .values()
            .filter(|s| s.key == subgraph_doc.key)
            .map(|s| Stmt::Node(node!(s.id.to_string())))
            .collect();

        if !section_nodes.is_empty() {
            let subgraph_name = format!("cluster_{}", i);
            let subgraph = subgraph!(
                subgraph_name;
                attr!("labeljust", &quoted("l")),
                attr!("style", "filled"),
                attr!("color", &quoted(colors.subgraph_fill)),
                attr!("fillcolor", &quoted(colors.subgraph_fill)),
                attr!("fontcolor", &quoted(colors.subgraph_text)),
                attr!("penwidth", "40")
            );

            let mut subgraph_with_nodes = subgraph;
            for node_stmt in section_nodes {
                subgraph_with_nodes.add_stmt(node_stmt);
            }

            statements.push(Stmt::Subgraph(subgraph_with_nodes));
        }
    }

    // Add section_to_document edges
    for (from_id, to_id) in &graph_data.section_to_document {
        if graph_data.sections.contains_key(to_id) || graph_data.documents.contains_key(to_id) {
            let edge = edge!(
                node_id!(from_id.to_string()) => node_id!(to_id.to_string());
                attr!("arrowsize", "1.5"),
                attr!("arrowhead", &quoted("empty")),
                attr!("style", &quoted("dashed")),
                attr!("color", &quoted("#38546c66")),
                attr!("penwidth", "1.2")
            );
            statements.push(Stmt::Edge(edge));
        }
    }

    // Add section_to_section edges
    for (from_id, to_id) in &graph_data.section_to_section {
        if graph_data.sections.contains_key(to_id) || graph_data.documents.contains_key(to_id) {
            let edge = edge!(
                node_id!(from_id.to_string()) => node_id!(to_id.to_string());
                attr!("color", &quoted("#38546c66")),
                attr!("arrowhead", "normal"),
                attr!("penwidth", "1.2")
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

    let output = final_graph.print(&mut PrinterContext::default());
    format!("{}\n", output)
}

fn quoted(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}
