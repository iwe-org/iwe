use indoc::indoc;
use liwe::graph::{Graph, GraphContext, GraphPatch};
use liwe::model::tree::Tree;
use liwe::model::{LineRange, NodeId};

fn document() -> String {
    indoc! {"
        # Title

        A paragraph with a [link](target).

        ## Section

        Another paragraph.

        ### Sub

        - item one
        - item two

        ``` rust
        let answer = 42;
        ```
        "}
    .to_string()
}

fn identities(tree: &Tree) -> Vec<(NodeId, Option<LineRange>)> {
    let mut out = Vec::new();
    for child in &tree.children {
        collect(child, &mut out);
    }
    out
}

fn collect(tree: &Tree, out: &mut Vec<(NodeId, Option<LineRange>)>) {
    out.push((tree.id, tree.line_range.clone()));
    for child in &tree.children {
        collect(child, out);
    }
}

#[test]
fn export_import_roundtrip_preserves_ids_and_line_ranges() {
    let mut graph = Graph::new();
    graph.insert_document("doc".into(), document());

    let graph_ref: &Graph = &graph;
    let tree = graph_ref.collect(&"doc".into());

    let mut rebuilt = Graph::new();
    rebuilt.add_key(&"doc".into(), tree.iter());
    let rebuilt_ref: &Graph = &rebuilt;
    let rebuilt_tree = rebuilt_ref.collect(&"doc".into());

    assert_eq!(identities(&tree), identities(&rebuilt_tree));
}

#[test]
fn graph_rebuilt_from_export_is_structurally_equal() {
    let mut graph = Graph::new();
    graph.insert_document("doc".into(), document());

    let graph_ref: &Graph = &graph;
    let tree = graph_ref.collect(&"doc".into());

    let mut rebuilt = Graph::new();
    rebuilt.add_key(&"doc".into(), tree.iter());

    assert_eq!(graph, rebuilt);
}
