use liwe::graph::Graph;
use liwe::model::Key;
use liwe::query::execute;
use liwe::query::prelude::{count, filter, find, includes, key_eq, or, references};
use liwe::query::{FindMatch, FindOp, Filter, InclusionAnchor, Outcome, ReferenceAnchor};

pub fn all_keys(graph: &Graph) -> Vec<FindMatch> {
    match execute(&find(FindOp::new()), graph) {
        Outcome::Find { matches } => matches,
        _ => unreachable!("Find returns Find"),
    }
}

pub fn key_exists(graph: &Graph, key: &Key) -> bool {
    match execute(&count(filter(key_eq(key.to_string()))), graph) {
        Outcome::Count(n) => n > 0,
        _ => unreachable!("Count returns Count"),
    }
}

pub fn inclusion_backlinks(graph: &Graph, key: &Key) -> Vec<Key> {
    keys_of(execute(&find(filter(includes_direct(key))), graph))
}

pub fn reference_backlinks(graph: &Graph, key: &Key) -> Vec<Key> {
    keys_of(execute(&find(filter(references_direct(key))), graph))
}

pub fn all_backlinks(graph: &Graph, key: &Key) -> Vec<Key> {
    let f = or(vec![includes_direct(key), references_direct(key)]);
    keys_of(execute(&find(filter(f)), graph))
}

pub fn inclusion_count(graph: &Graph, key: &Key) -> usize {
    count_of(execute(&count(filter(includes_direct(key))), graph))
}

pub fn reference_count(graph: &Graph, key: &Key) -> usize {
    count_of(execute(&count(filter(references_direct(key))), graph))
}

fn includes_direct(key: &Key) -> Filter {
    includes(InclusionAnchor::new(key.to_string(), 1, 1))
}

fn references_direct(key: &Key) -> Filter {
    references(ReferenceAnchor::new(key.to_string(), 1, 1))
}

fn keys_of(outcome: Outcome) -> Vec<Key> {
    match outcome {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.key).collect(),
        _ => unreachable!("Find returns Find"),
    }
}

fn count_of(outcome: Outcome) -> usize {
    match outcome {
        Outcome::Count(n) => n,
        _ => unreachable!("Count returns Count"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;
    use itertools::Itertools;
    use liwe::graph::{Graph, GraphContext};
    use liwe::model::config::MarkdownOptions;
    use liwe::model::node::NodePointer;
    use liwe::state::from_indoc;

    fn make_graph(docs: &str) -> Graph {
        Graph::import(&from_indoc(docs), MarkdownOptions::default(), None)
    }

    fn sorted(mut keys: Vec<Key>) -> Vec<String> {
        keys.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
        keys.into_iter().map(|k| k.to_string()).collect()
    }

    fn graph_inclusion_keys(graph: &Graph, key: &Key) -> Vec<String> {
        graph
            .get_inclusion_edges_to(key)
            .into_iter()
            .map(|id| graph.node(id).node_key())
            .filter(|k| k != key)
            .unique()
            .map(|k| k.to_string())
            .sorted()
            .collect()
    }

    fn graph_reference_keys(graph: &Graph, key: &Key) -> Vec<String> {
        graph
            .get_reference_edges_to(key)
            .into_iter()
            .map(|id| graph.node(id).node_key())
            .filter(|k| k != key)
            .unique()
            .map(|k| k.to_string())
            .sorted()
            .collect()
    }

    const FIXTURE: &str = indoc! {"
        [b](2)
        _
        [c](3)
        _
        # C

        See [other](2) for details.
    "};

    #[test]
    fn all_keys_matches_graph_keys() {
        let graph = make_graph(FIXTURE);
        let helper: Vec<String> = all_keys(&graph)
            .into_iter()
            .map(|m| m.key.to_string())
            .sorted()
            .collect();
        let direct: Vec<String> = graph
            .keys()
            .into_iter()
            .map(|k| k.to_string())
            .sorted()
            .collect();
        assert_eq!(helper, direct);
    }

    #[test]
    fn key_exists_true_for_present_key() {
        let graph = make_graph(FIXTURE);
        assert!(key_exists(&graph, &Key::name("1")));
        assert!(key_exists(&graph, &Key::name("3")));
    }

    #[test]
    fn key_exists_false_for_missing_key() {
        let graph = make_graph(FIXTURE);
        assert!(!key_exists(&graph, &Key::name("does-not-exist")));
    }

    #[test]
    fn inclusion_backlinks_match_graph_edges() {
        let graph = make_graph(FIXTURE);
        let target = Key::name("2");
        assert_eq!(
            sorted(inclusion_backlinks(&graph, &target)),
            graph_inclusion_keys(&graph, &target),
        );
    }

    #[test]
    fn reference_backlinks_match_graph_edges() {
        let graph = make_graph(FIXTURE);
        let target = Key::name("2");
        assert_eq!(
            sorted(reference_backlinks(&graph, &target)),
            graph_reference_keys(&graph, &target),
        );
    }

    #[test]
    fn all_backlinks_unions_both_kinds() {
        let graph = make_graph(FIXTURE);
        let target = Key::name("2");
        let helper = sorted(all_backlinks(&graph, &target));
        let mut combined: Vec<String> = graph_inclusion_keys(&graph, &target)
            .into_iter()
            .chain(graph_reference_keys(&graph, &target))
            .unique()
            .collect();
        combined.sort();
        assert_eq!(helper, combined);
    }

    #[test]
    fn inclusion_count_matches_unique_sources() {
        let graph = make_graph(FIXTURE);
        let target = Key::name("3");
        assert_eq!(
            inclusion_count(&graph, &target),
            graph_inclusion_keys(&graph, &target).len(),
        );
    }

    #[test]
    fn reference_count_matches_unique_sources() {
        let graph = make_graph(FIXTURE);
        let target = Key::name("2");
        assert_eq!(
            reference_count(&graph, &target),
            graph_reference_keys(&graph, &target).len(),
        );
    }
}
