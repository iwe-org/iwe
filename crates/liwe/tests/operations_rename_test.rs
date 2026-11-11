use pretty_assertions::assert_eq;

use liwe::graph::Graph;
use liwe::model::Key;
use liwe::operations::rename;

fn graph_with(documents: &[(&str, &str)]) -> Graph {
    let mut graph = Graph::new();
    for (key, content) in documents {
        graph.insert_document(Key::name(key), content.to_string());
    }
    graph
}

#[test]
fn rename_to_key_differing_only_in_case_from_another_document_is_rejected() {
    let graph = graph_with(&[("source", "# Source\n"), ("other", "# Other\n")]);

    let error = rename(&graph, &Key::name("source"), &Key::name("Other")).unwrap_err();

    assert_eq!("Document 'other' already exists", error.to_string());
}

#[test]
fn rename_changing_only_the_case_of_the_same_document_is_allowed() {
    let graph = graph_with(&[("source", "# Source\n")]);

    let changes = rename(&graph, &Key::name("source"), &Key::name("Source")).unwrap();

    assert_eq!(
        vec![(Key::name("Source"), "# Source\n".to_string())],
        changes.creates
    );
    assert_eq!(vec![Key::name("source")], changes.removes);
}
