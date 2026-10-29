use pretty_assertions::assert_eq;

use liwe::graph::Graph;
use liwe::model::Key;

#[test]
fn root_keys_include_documents_without_heading() {
    let mut graph = Graph::new();
    graph.insert_document("parent".into(), "# Parent\n\n[Child](child)\n".to_string());
    graph.insert_document("child".into(), "# Child\n\nText\n".to_string());
    graph.insert_document("plain".into(), "Text\n".to_string());

    assert_eq!(
        vec![Key::name("parent"), Key::name("plain")],
        graph.root_keys()
    );
}

#[test]
fn root_keys_exclude_circular_inclusions() {
    let mut graph = Graph::new();
    graph.insert_document("first".into(), "# First\n\n[Second](second)\n".to_string());
    graph.insert_document("second".into(), "# Second\n\n[First](first)\n".to_string());

    assert_eq!(Vec::<Key>::new(), graph.root_keys());
}
