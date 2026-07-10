use std::collections::HashMap;

use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::model::Key;
use liwe::query::{execute_with_scores, FindOp, Operation, Outcome, QueryScores, SearchSpec, Sort};
use liwe::state::from_indoc;

const CORPUS: &str = indoc! {"
    ---
    rank: 1
    ---
    # Alpha
    _
    ---
    rank: 2
    ---
    # Beta
    _
    ---
    rank: 3
    ---
    # Gamma
"};

fn graph(docs: &str) -> Graph {
    Graph::import(&from_indoc(docs), MarkdownOptions::default(), None)
}

fn scores(pairs: &[(&str, f64)]) -> QueryScores {
    let fused: HashMap<Key, f64> = pairs.iter().map(|(k, s)| (Key::name(k), *s)).collect();
    QueryScores::from_fused(fused)
}

fn run(g: &Graph, op: FindOp, scores: &QueryScores) -> Vec<String> {
    match execute_with_scores(&Operation::Find(op), g, scores).expect("query succeeds") {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key.to_string()).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn search() -> SearchSpec {
    SearchSpec::new(Some("q".into()), None)
}

#[test]
fn search_clause_restricts_to_scored_keys_and_orders_by_score() {
    let g = graph(CORPUS);
    let s = scores(&[("1", 0.5), ("2", 0.9)]);
    let keys = run(&g, FindOp::new().search(search()), &s);
    assert_eq!(keys, vec!["2".to_string(), "1".to_string()]);
}

#[test]
fn score_ties_break_by_key_ascending() {
    let g = graph(CORPUS);
    let s = scores(&[("1", 0.5), ("2", 0.5), ("3", 0.5)]);
    let keys = run(&g, FindOp::new().search(search()), &s);
    assert_eq!(
        keys,
        vec!["1".to_string(), "2".to_string(), "3".to_string()]
    );
}

#[test]
fn search_clause_with_sort_keeps_membership_and_uses_sort() {
    let g = graph(CORPUS);
    let s = scores(&[("1", 0.5), ("2", 0.9)]);
    let keys = run(
        &g,
        FindOp::new().search(search()).sort(Sort::desc("rank")),
        &s,
    );
    assert_eq!(keys, vec!["2".to_string(), "1".to_string()]);
}

#[test]
fn empty_scores_yield_no_matches() {
    let g = graph(CORPUS);
    let keys = run(&g, FindOp::new().search(search()), &QueryScores::default());
    assert_eq!(keys, Vec::<String>::new());
}

#[test]
fn no_search_clause_ignores_scores() {
    let g = graph(CORPUS);
    let s = scores(&[("2", 0.9)]);
    let keys = run(&g, FindOp::new(), &s);
    assert_eq!(
        keys,
        vec!["1".to_string(), "2".to_string(), "3".to_string()]
    );
}
