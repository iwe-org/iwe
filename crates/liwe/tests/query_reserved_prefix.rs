use indoc::indoc;
use liwe::graph::Graph;
use liwe::model::config::MarkdownOptions;
use liwe::query::execute;
use liwe::query::prelude::{filter, find, update, update_op};
use liwe::query::{Filter, FindOp, Outcome, Update, UpdateOp, UpdateOperator};
use liwe::state::{from_indoc, to_indoc};
use pretty_assertions::assert_str_eq;
use serde_yaml::{Mapping, Value};

fn run_find(docs: &str, op: FindOp) -> Vec<Mapping> {
    let graph = Graph::import(&from_indoc(docs), MarkdownOptions::default(), None);
    match execute(&find(op), &graph) {
        Outcome::Find { matches } => matches.into_iter().map(|m| m.document).collect(),
        other => panic!("expected Find, got {:?}", other),
    }
}

fn assert_update(docs: &str, op: UpdateOp, expected: &str) {
    let state = from_indoc(docs);
    let graph = Graph::import(&state, MarkdownOptions::default(), None);
    match execute(&update(op), &graph) {
        Outcome::Update { changes } => {
            let mut new_state = graph.export();
            for (key, markdown) in changes {
                new_state.insert(key.to_string(), markdown);
            }
            assert_str_eq!(expected, to_indoc(&new_state));
        }
        other => panic!("expected Update, got {:?}", other),
    }
}

#[test]
fn reserved_prefix_keys_invisible_in_find_output() {
    let matches = run_find(
        indoc! {"
            ---
            _internal: 1
            $weird: 2
            name: ok
            ---
            # A
        "},
        filter(Filter::all()),
    );
    assert_eq!(matches.len(), 1);
    let m = &matches[0];
    assert!(!m.contains_key(Value::String("_internal".into())));
    assert!(!m.contains_key(Value::String("$weird".into())));
    assert!(m.contains_key(Value::String("name".into())));
}

#[test]
fn update_round_trip_strips_pre_existing_reserved_keys() {
    assert_update(
        indoc! {"
            ---
            _internal: 1
            name: original
            ---
            # A
        "},
        update_op(
            Filter::all(),
            Update::new(vec![UpdateOperator::set("name", "updated")]),
        ),
        indoc! {"
            ---
            name: updated
            ---

            # A
        "},
    );
}
