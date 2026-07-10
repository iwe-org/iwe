use diwe::config::MarkdownOptions;
use diwe::search::Language;
use diwe::search_query::build_index;
use indoc::indoc;
use liwe::graph::Graph;
use liwe::state::from_indoc;

fn indexed_graph() -> Graph {
    let state = from_indoc(indoc! {"
        # Alpha

        The quick brown fox.
        _
        # Beta

        Lazy dog sleeps.
    "});
    Graph::import(&state, MarkdownOptions::default(), None)
}

fn found(graph: &Graph, query: &str) -> Vec<String> {
    build_index(graph, Language::English)
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
fn rebuild_reflects_updated_document() {
    let mut graph = indexed_graph();
    graph.update_document(
        "1".into(),
        "# Alpha\n\nPenguins waddle south.\n".to_string(),
    );

    assert_eq!(found(&graph, "fox"), Vec::<String>::new());
    assert_eq!(found(&graph, "penguins"), vec!["1".to_string()]);
}

#[test]
fn rebuild_clears_removed_document() {
    let mut graph = indexed_graph();
    graph.remove_document("2".into());

    assert_eq!(found(&graph, "dog"), Vec::<String>::new());
}

#[test]
fn rebuild_includes_inserted_document() {
    let mut graph = indexed_graph();
    graph.insert_document("3".into(), "# Gamma\n\nElephants roam.\n".to_string());

    assert_eq!(found(&graph, "elephants"), vec!["3".to_string()]);
}
