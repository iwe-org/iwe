use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::search::Language;
use liwe::state::from_indoc;

fn indexed_graph() -> Graph {
    let state = from_indoc(indoc! {"
        # Alpha

        The quick brown fox.
        _
        # Beta

        Lazy dog sleeps.
    "});
    Graph::from_state(
        &state,
        false,
        MarkdownOptions::default(),
        None,
        Some(Language::English),
    )
}

fn found(graph: &Graph, query: &str) -> Vec<String> {
    graph
        .search(query)
        .into_iter()
        .map(|scored| scored.id.to_string())
        .collect()
}

#[test]
fn body_terms_are_searchable_after_build() {
    let graph = indexed_graph();
    assert_eq!(found(&graph, "fox"), vec!["1".to_string()]);
    assert_eq!(found(&graph, "dog"), vec!["2".to_string()]);
}

#[test]
fn update_document_replaces_index_vector() {
    let mut graph = indexed_graph();
    graph.update_document(
        "1".into(),
        "# Alpha\n\nPenguins waddle south.\n".to_string(),
    );

    assert_eq!(found(&graph, "fox"), Vec::<String>::new());
    assert_eq!(found(&graph, "penguins"), vec!["1".to_string()]);
}

#[test]
fn remove_document_clears_from_search() {
    let mut graph = indexed_graph();
    graph.remove_document("2".into());

    assert_eq!(found(&graph, "dog"), Vec::<String>::new());
}

#[test]
fn insert_document_is_searchable_immediately() {
    let mut graph = indexed_graph();
    graph.insert_document("3".into(), "# Gamma\n\nElephants roam.\n".to_string());

    assert_eq!(found(&graph, "elephants"), vec!["3".to_string()]);
}

#[test]
fn search_disabled_returns_empty() {
    let state = from_indoc(indoc! {"
        # Alpha

        The quick brown fox.
    "});
    let graph = Graph::from_state(&state, false, MarkdownOptions::default(), None, None);

    assert_eq!(found(&graph, "fox"), Vec::<String>::new());
}
