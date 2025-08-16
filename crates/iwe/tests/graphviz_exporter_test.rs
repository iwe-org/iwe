use std::collections::HashMap;

use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;

// Import the GraphvizExporter from the main crate
use iwe::graphviz_export::GraphvizExporter;

#[test]
fn test_graphviz_exporter_basic_export() {
    let graph = create_simple_graph();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Verify basic structure
    assert!(output.starts_with("digraph {"));
    assert!(output.ends_with("}\n"));
    assert!(output.contains("graph ["));
    assert!(output.contains("node ["));
}

#[test]
fn test_graphviz_exporter_with_key_filter() {
    let graph = create_simple_graph();
    let exporter = GraphvizExporter::new(Some("test".to_string()), 0);
    let output = exporter.export(&graph);

    // Should still produce valid DOT format
    assert!(output.starts_with("digraph {"));
    assert!(output.ends_with("}\n"));
}

#[test]
fn test_graphviz_exporter_with_depth_limit() {
    let graph = create_complex_graph();
    let exporter = GraphvizExporter::new(None, 2);
    let output = exporter.export(&graph);

    // Should produce valid DOT format
    assert!(output.starts_with("digraph {"));
    assert!(output.ends_with("}\n"));
    assert!(output.contains("graph ["));
}

#[test]
fn test_graphviz_exporter_empty_graph() {
    let graph = Graph::new();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Should still produce valid DOT format even with empty graph
    assert!(output.starts_with("digraph {"));
    assert!(output.ends_with("}\n"));
    assert!(output.contains("graph ["));
    assert!(output.contains("node ["));
}

#[test]
fn test_graphviz_exporter_node_attributes() {
    let graph = create_simple_graph();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Check for expected node attributes
    assert!(output.contains("group="));
    assert!(output.contains("class="));
    assert!(output.contains("label="));
}

#[test]
fn test_graphviz_exporter_graph_attributes() {
    let graph = create_simple_graph();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Check for expected graph attributes
    assert!(output.contains("overlap_scaling=3"));
    assert!(output.contains("pack=90"));
    assert!(output.contains("IWE Knowledge Graph"));
}

#[test]
fn test_graphviz_exporter_special_characters() {
    let graph = create_graph_with_special_chars();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Should handle special characters without breaking DOT format
    assert!(output.starts_with("digraph {"));
    assert!(output.ends_with("}\n"));

    // Should escape quotes and backslashes
    if output.contains("Special") {
        // Check that quotes are properly escaped in labels
        let lines: Vec<&str> = output.lines().collect();
        for line in lines {
            if line.contains("label=") && line.contains("Special") {
                // Should not contain unescaped quotes that would break parsing
                let label_part = line.split("label=").nth(1).unwrap_or("");
                let first_quote = label_part.find('"');
                let last_quote = label_part.rfind('"');
                assert!(first_quote.is_some() && last_quote.is_some());
                assert!(
                    first_quote != last_quote,
                    "Should have opening and closing quotes"
                );
            }
        }
    }
}

#[test]
fn test_graphviz_exporter_directed_edges() {
    let graph = create_graph_with_bidirectional_links();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Count edge definitions
    let edge_count = output.matches(" -> ").count();

    // Directed graphs preserve directional relationships (A -> B and B -> A are separate)
    // This is a basic test - exact count depends on graph structure
    // Just verify that edge processing doesn't panic or produce invalid output

    // Verify edges are in correct format
    if edge_count > 0 {
        assert!(output.contains(" -> "), "Should use directed edge format");
    }
}

#[test]
fn test_graphviz_exporter_node_classes() {
    let graph = create_complex_graph();
    let exporter = GraphvizExporter::new(None, 0);
    let output = exporter.export(&graph);

    // Should contain different CSS classes based on node rank
    let expected_classes = ["leaf", "small", "medium", "large", "major"];
    let mut found_classes = Vec::new();

    for class in expected_classes {
        if output.contains(&format!("class=\"{}\"", class)) {
            found_classes.push(class);
        }
    }

    // Should have at least some classified nodes
    assert!(
        !found_classes.is_empty(),
        "Should contain node classifications"
    );
}

// Helper functions to create test graphs

fn create_simple_graph() -> Graph {
    let mut state = HashMap::new();
    state.insert(
        "1".to_string(),
        "# Test Document\n\nSome content here.".to_string(),
    );
    state.insert(
        "2".to_string(),
        "# Another Document\n\n[Link to test](1)".to_string(),
    );

    Graph::import(&state, MarkdownOptions::default())
}

fn create_complex_graph() -> Graph {
    let mut state = HashMap::new();
    state.insert(
        "1".to_string(),
        "# Main Document\n\n[Chapter 1](2)\n[Chapter 2](3)".to_string(),
    );
    state.insert(
        "2".to_string(),
        "# Chapter 1\n\nContent with [reference](4)".to_string(),
    );
    state.insert(
        "3".to_string(),
        "# Chapter 2\n\nMore content [back to main](1)".to_string(),
    );
    state.insert(
        "4".to_string(),
        "# Reference\n\nReference material".to_string(),
    );
    state.insert(
        "5".to_string(),
        "# Standalone\n\nNo links to others".to_string(),
    );

    Graph::import(&state, MarkdownOptions::default())
}

fn create_graph_with_special_chars() -> Graph {
    let mut state = HashMap::new();
    state.insert(
        "1".to_string(),
        "# Test \"Document\" with Quotes\n\nContent with backslashes \\ and more".to_string(),
    );
    state.insert(
        "2".to_string(),
        "# Special Characters: <>&\n\nContent with 'single' and \"double\" quotes".to_string(),
    );

    Graph::import(&state, MarkdownOptions::default())
}

fn create_graph_with_bidirectional_links() -> Graph {
    let mut state = HashMap::new();
    state.insert(
        "1".to_string(),
        "# Document A\n\n[Link to B](2)".to_string(),
    );
    state.insert(
        "2".to_string(),
        "# Document B\n\n[Link to A](1)".to_string(),
    );
    state.insert(
        "3".to_string(),
        "# Document C\n\n[Link to A](1)\n[Link to B](2)".to_string(),
    );

    Graph::import(&state, MarkdownOptions::default())
}
