use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::block_update::EvalError;
use liwe::query::search::{lexical_has_no_terms, no_terms_warning};
use liwe::query::{execute, FindOp, Operation, Outcome, SearchSpec, Sort};
use liwe::search::Language;
use liwe::state::from_indoc;

const CORPUS: &str = indoc! {"
    ---
    rank: 1
    ---
    # Alpha

    apple apple apple orchard
    _
    ---
    rank: 2
    ---
    # Beta

    apple and orange basket
    _
    ---
    rank: 3
    ---
    # Gamma

    orange citrus grove
"};

fn indexed_graph(docs: &str) -> Graph {
    Graph::from_state(
        &from_indoc(docs),
        false,
        MarkdownOptions::default(),
        None,
        Some(Language::English),
    )
}

fn plain_graph(docs: &str) -> Graph {
    Graph::import(&from_indoc(docs), MarkdownOptions::default(), None)
}

fn run(graph: &Graph, op: FindOp) -> Vec<String> {
    match execute(&Operation::Find(op), graph).expect("query succeeds") {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key.to_string()).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn lexical(query: &str) -> SearchSpec {
    SearchSpec::new(Some(query.to_string()), None)
}

fn fuzzy(query: &str) -> SearchSpec {
    SearchSpec::new(None, Some(query.to_string()))
}

#[test]
fn lexical_restricts_and_orders_by_relevance() {
    let graph = indexed_graph(CORPUS);
    let keys = run(&graph, FindOp::new().search(lexical("apple")));
    assert_eq!(keys, vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn lexical_drops_unmatched_documents() {
    let graph = indexed_graph(CORPUS);
    let keys = run(&graph, FindOp::new().search(lexical("orchard")));
    assert_eq!(keys, vec!["1".to_string()]);
}

#[test]
fn fuzzy_matches_against_title() {
    let graph = indexed_graph(CORPUS);
    let keys = run(&graph, FindOp::new().search(fuzzy("beta")));
    assert_eq!(keys, vec!["2".to_string()]);
}

#[test]
fn search_with_sort_restricts_by_search_and_orders_by_sort() {
    let graph = indexed_graph(CORPUS);
    let keys = run(
        &graph,
        FindOp::new()
            .search(lexical("apple"))
            .sort(Sort::desc("rank")),
    );
    assert_eq!(keys, vec!["2".to_string(), "1".to_string()]);
}

#[test]
fn limit_takes_top_n_of_the_search_ranking() {
    let graph = indexed_graph(CORPUS);
    let keys = run(&graph, FindOp::new().search(lexical("apple")).limit(1));
    assert_eq!(keys, vec!["1".to_string()]);
}

#[test]
fn no_searchable_terms_yields_empty_and_the_warning_helper_fires() {
    let graph = indexed_graph(CORPUS);
    let spec = lexical("the and of");
    let keys = run(&graph, FindOp::new().search(spec.clone()));
    assert_eq!(keys, Vec::<String>::new());
    assert!(lexical_has_no_terms(&graph, &spec));
    assert_eq!(
        no_terms_warning(&spec),
        "lexical query 'the and of' has no searchable terms after stop-word removal and stemming; it matches nothing"
    );
}

#[test]
fn search_without_index_is_an_error() {
    let graph = plain_graph(CORPUS);
    let err = execute(
        &Operation::Find(FindOp::new().search(lexical("apple"))),
        &graph,
    )
    .expect_err("search without an index fails");
    assert!(matches!(err, EvalError::SearchIndexMissing));
    assert_eq!(
        err.to_string(),
        "'search' requires the search-indexed graph, which is not built for this command"
    );
}
