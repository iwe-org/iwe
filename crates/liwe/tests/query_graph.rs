use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::prelude::{
    and, eq, filter, find, included_by, includes, key_eq, key_in, key_ne, key_nin, nor, not, or,
    referenced_by, references,
};
use liwe::query::{execute, FindOp, InclusionAnchor, Outcome, ReferenceAnchor};
use liwe::state::from_indoc;

fn run_find_keys(docs: &str, op: FindOp) -> Vec<String> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&find(op), &graph) {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key.to_string()).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn assert_keys(docs: &str, op: FindOp, expected: &[&str]) {
    let mut actual = run_find_keys(docs, op);
    actual.sort();
    let mut expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn key_eq_selects_one() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        filter(key_eq("2")),
        &["2"],
    );
}

#[test]
fn key_in_selects_subset() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        filter(key_in(&["1", "3"])),
        &["1", "3"],
    );
}

#[test]
fn key_ne_excludes() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        filter(key_ne("2")),
        &["1", "3"],
    );
}

#[test]
fn included_by_direct() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(included_by(InclusionAnchor::with_max("1", 1))),
        &["2"],
    );
}

#[test]
fn included_by_transitive() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(included_by(InclusionAnchor::with_max("1", 5))),
        &["2", "3"],
    );
}

#[test]
fn included_by_range_excludes_direct() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(included_by(InclusionAnchor::new("1", 2, 5))),
        &["3"],
    );
}

#[test]
fn includes_outbound() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(includes(InclusionAnchor::with_max("3", 5))),
        &["1", "2"],
    );
}

#[test]
fn anchor_excluded_from_walk_results() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(included_by(InclusionAnchor::with_max("1", 5))),
        &["2", "3"],
    );
}

#[test]
fn multi_anchor_intersects() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [c](3)
            _
            # C
        "},
        filter(and(vec![
            included_by(InclusionAnchor::with_max("1", 5)),
            included_by(InclusionAnchor::with_max("2", 5)),
        ])),
        &["3"],
    );
}

#[test]
fn references_by_inline_link_in_text() {
    assert_keys(
        indoc! {"
            # A

            See [other](2) for details.
            _
            # B
        "},
        filter(references(ReferenceAnchor::with_max("2", 1))),
        &["1"],
    );
}

#[test]
fn referenced_by_inline() {
    assert_keys(
        indoc! {"
            # A

            See [other](2) for details.
            _
            # B
        "},
        filter(referenced_by(ReferenceAnchor::with_max("1", 1))),
        &["2"],
    );
}

#[test]
fn missing_anchor_returns_empty() {
    assert_keys(
        indoc! {"
            # A
        "},
        filter(included_by(InclusionAnchor::with_max("does-not-exist", 5))),
        &[],
    );
}

#[test]
fn empty_corpus_returns_empty() {
    let graph = Graph::import(
        &std::collections::HashMap::new(),
        MarkdownOptions::default(),
        None,
    );
    let op = filter(key_eq("anything"));
    match execute(&find(op), &graph) {
        Outcome::Find { matches } => assert!(matches.is_empty()),
        other => panic!("{:?}", other),
    }
}

#[test]
fn graph_op_combines_with_frontmatter_predicate() {
    assert_keys(
        indoc! {"
            ---
            status: draft
            ---
            [b](2)
            _
            ---
            status: published
            ---
            # B
        "},
        filter(and(vec![
            included_by(InclusionAnchor::with_max("1", 5)),
            eq("status", "published"),
        ])),
        &["2"],
    );
}

#[test]
fn key_nin_excludes_listed() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
            _
            # D
        "},
        filter(key_nin(&["2", "4"])),
        &["1", "3"],
    );
}

#[test]
fn key_eq_against_missing_returns_empty() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
        "},
        filter(key_eq("missing")),
        &[],
    );
}

#[test]
fn included_by_chain_at_depth_3() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            [d](4)
            _
            # D
        "},
        filter(included_by(InclusionAnchor::new("1", 3, 3))),
        &["4"],
    );
}

#[test]
fn includes_outbound_chain() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            [d](4)
            _
            # D
        "},
        filter(includes(InclusionAnchor::with_max("4", 5))),
        &["1", "2", "3"],
    );
}

#[test]
fn included_by_polyhierarchy_set_semantics() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [c](3)
            _
            # C
        "},
        filter(and(vec![
            included_by(InclusionAnchor::with_max("1", 5)),
            included_by(InclusionAnchor::with_max("2", 5)),
        ])),
        &["3"],
    );
}

#[test]
fn references_multi_hop() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        filter(references(ReferenceAnchor::with_max("3", 2))),
        &["1", "2"],
    );
}

#[test]
fn referenced_by_multi_hop() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        filter(referenced_by(ReferenceAnchor::with_max("1", 2))),
        &["2", "3"],
    );
}

#[test]
fn reference_self_link_excluded_from_walk() {
    assert_keys(
        indoc! {"
            See [self](1).
        "},
        filter(references(ReferenceAnchor::with_max("1", 1))),
        &[],
    );
}

#[test]
fn referenced_by_range_excludes_direct() {
    assert_keys(
        indoc! {"
            See [b](2).
            _
            See [c](3).
            _
            # C
        "},
        filter(referenced_by(ReferenceAnchor::new("1", 2, 3))),
        &["3"],
    );
}

#[test]
fn or_of_two_graph_ops() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(or(vec![
            key_eq("1"),
            included_by(InclusionAnchor::with_max("2", 1)),
        ])),
        &["1", "3"],
    );
}

#[test]
fn not_wraps_walk() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            # B
            _
            # C
        "},
        filter(not(included_by(InclusionAnchor::with_max("1", 5)))),
        &["1", "3"],
    );
}

#[test]
fn nor_excludes_union_of_children() {
    assert_keys(
        indoc! {"
            # A
            _
            # B
            _
            # C
        "},
        filter(nor(vec![key_eq("1"), key_eq("3")])),
        &["2"],
    );
}

#[test]
fn nor_equivalent_to_not_or() {
    let docs = indoc! {"
        # A
        _
        # B
        _
        # C
    "};
    let nor_op = filter(nor(vec![key_eq("1"), key_eq("2")]));
    let not_or = filter(not(or(vec![key_eq("1"), key_eq("2")])));
    let mut a = run_find_keys(docs, nor_op);
    let mut b = run_find_keys(docs, not_or);
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn combined_three_predicates_hub_under_anchor() {
    assert_keys(
        indoc! {"
            ---
            status: draft
            ---
            [hub](2)
            _
            ---
            status: draft
            ---
            [a](3)

            [b](4)

            [c](5)
            _
            # A
            _
            # B
            _
            # C
        "},
        filter(and(vec![
            eq("status", "draft"),
            included_by(InclusionAnchor::with_max("1", 5)),
        ])),
        &["2"],
    );
}

#[test]
fn or_anchor_or_descendants() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            [c](3)
            _
            # C
        "},
        filter(or(vec![
            key_eq("1"),
            included_by(InclusionAnchor::with_max("1", 5)),
        ])),
        &["1", "2", "3"],
    );
}

#[test]
fn exclude_inside_descendants() {
    assert_keys(
        indoc! {"
            [b](2)

            [c](3)
            _
            # B
            _
            # C
        "},
        filter(and(vec![
            included_by(InclusionAnchor::with_max(
                "1", 5,
            )),
            key_ne("3"),
        ])),
        &["2"],
    );
}

#[test]
fn disconnected_components() {
    assert_keys(
        indoc! {"
            [b](2)
            _
            # B
            _
            [d](4)
            _
            # D
        "},
        filter(included_by(InclusionAnchor::with_max("1", 100))),
        &["2"],
    );
}

#[test]
fn sort_after_graph_filter() {
    use liwe::query::Sort;
    use liwe::query::prelude::exists;
    assert_keys(
        indoc! {"
            ---
            created: 2026-03-01
            ---
            [b](2)
            _
            ---
            created: 2026-01-01
            ---
            [d](4)
            _
            # B
            _
            # D
        "},
        FindOp::new()
            .filter(exists("created", true))
            .sort(Sort::asc("created"))
            .limit(2),
        &["1", "2"],
    );
}

#[test]
fn included_by_match_anchors_by_frontmatter() {
    assert_keys(
        indoc! {"
            ---
            kind: project
            ---
            [c](3)
            _
            ---
            kind: project
            ---
            [d](4)
            _
            # C
            _
            # D
        "},
        filter(included_by(InclusionAnchor::with_match(
            eq("kind", "project"),
            1,
            5,
        ))),
        &["3", "4"],
    );
}

#[test]
fn included_by_match_anchors_by_key_in() {
    assert_keys(
        indoc! {"
            [c](3)
            _
            [d](4)
            _
            [e](5)
            _
            # D
            _
            # E
        "},
        filter(included_by(InclusionAnchor::with_match(
            key_in(&["1", "2"]),
            1,
            5,
        ))),
        &["3", "4", "5"],
    );
}

#[test]
fn included_by_match_excludes_anchor_set() {
    assert_keys(
        indoc! {"
            ---
            kind: project
            ---
            [b](2)
            _
            ---
            kind: project
            ---
            [c](3)
            _
            # C
        "},
        filter(included_by(InclusionAnchor::with_match(
            eq("kind", "project"),
            1,
            5,
        ))),
        &["3"],
    );
}
