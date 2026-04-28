use indoc::indoc;
use liwe::query::{
    parse_operation, CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, Limit, Operation,
    OperationKind, Projection, Sort, Update, UpdateOp, UpdateOperator, YamlType,
};
use serde_yaml::Value;


fn assert_parse(yaml: &str, kind: OperationKind, expected: Operation) {
    let actual = parse_operation(yaml, kind).expect("parse");
    assert_eq!(actual, expected);
}


fn assert_parse_error(yaml: &str, kind: OperationKind, needle: &str) {
    let err = parse_operation(yaml, kind).expect_err("parse must fail");
    let s = format!("{:?}", err);
    assert!(s.contains(needle), "{} not in {}", needle, s);
}

#[test]
fn find_round_trips_filter_project_sort_limit() {
    assert_parse(
        indoc! {"
            filter:
              status: draft
            project:
              title: 1
            sort:
              modified: -1
            limit: 5
        "},
        OperationKind::Find,
        Operation::Find(
            FindOp::new()
                .filter(Filter::eq("status", "draft"))
                .project(Projection::fields(&["title"]))
                .sort(Sort::desc("modified"))
                .limit(5),
        ),
    );
}

#[test]
fn count_round_trips_filter_and_limit() {
    assert_parse(
        indoc! {"
            filter:
              status: draft
            limit: 0
        "},
        OperationKind::Count,
        Operation::Count(CountOp::new().filter(Filter::eq("status", "draft")).limit(0)),
    );
}

#[test]
fn update_round_trips_set_and_unset() {
    assert_parse(
        indoc! {r#"
            filter:
              status: draft
            update:
              $set:
                reviewed: true
              $unset:
                stale: ""
        "#},
        OperationKind::Update,
        Operation::Update(UpdateOp::new(
            Filter::eq("status", "draft"),
            Update::new(vec![
                UpdateOperator::set("reviewed", true),
                UpdateOperator::unset("stale"),
            ]),
        )),
    );
}

#[test]
fn delete_round_trips_filter() {
    assert_parse(
        indoc! {"
            filter:
              status: archived
        "},
        OperationKind::Delete,
        Operation::Delete(DeleteOp::new(Filter::eq("status", "archived"))),
    );
}

#[test]
fn empty_yaml_parses_to_default_find() {
    assert_parse(
        "",
        OperationKind::Find,
        Operation::Find(FindOp::new()),
    );
}

#[test]
fn sort_positive_one_means_ascending() {
    assert_parse(
        indoc! {"
            sort:
              modified: 1
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().sort(Sort::asc("modified"))),
    );
}

#[test]
fn filter_eq_bare() {
    assert_parse(
        indoc! {"
            filter:
              x: 5
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::eq("x", 5i64))),
    );
}

#[test]
fn filter_eq_explicit_operator() {
    assert_parse(
        indoc! {"
            filter:
              x: { $eq: 5 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::eq("x", 5i64))),
    );
}

#[test]
fn filter_ne_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              status: { $ne: draft }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::ne("status", "draft"))),
    );
}

#[test]
fn filter_comparison_operators_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              a: { $gt: 1 }
              b: { $gte: 2 }
              c: { $lt: 3 }
              d: { $lte: 4 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::and(vec![
            Filter::gt("a", 1i64),
            Filter::gte("b", 2i64),
            Filter::lt("c", 3i64),
            Filter::lte("d", 4i64),
        ]))),
    );
}

#[test]
fn filter_in_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              tags: { $in: [rust, go] }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("tags"),
            op: FieldOp::In(vec![
                Value::String("rust".into()),
                Value::String("go".into()),
            ]),
        })),
    );
}

#[test]
fn filter_nin_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              tags: { $nin: [rust] }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("tags"),
            op: FieldOp::Nin(vec![Value::String("rust".into())]),
        })),
    );
}

#[test]
fn filter_exists_true() {
    assert_parse(
        indoc! {"
            filter:
              x: { $exists: true }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::exists("x", true))),
    );
}

#[test]
fn filter_exists_false() {
    assert_parse(
        indoc! {"
            filter:
              x: { $exists: false }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::exists("x", false))),
    );
}

#[test]
fn filter_type_single() {
    assert_parse(
        indoc! {"
            filter:
              x: { $type: string }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("x"),
            op: FieldOp::Type(vec![YamlType::String]),
        })),
    );
}

#[test]
fn filter_type_multiple() {
    assert_parse(
        indoc! {"
            filter:
              x: { $type: [string, number] }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("x"),
            op: FieldOp::Type(vec![YamlType::String, YamlType::Number]),
        })),
    );
}

#[test]
fn filter_all_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              tags: { $all: [rust, async] }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("tags"),
            op: FieldOp::All(vec![
                Value::String("rust".into()),
                Value::String("async".into()),
            ]),
        })),
    );
}

#[test]
fn filter_size_round_trip() {
    assert_parse(
        indoc! {"
            filter:
              tags: { $size: 3 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("tags"),
            op: FieldOp::Size(3),
        })),
    );
}

#[test]
fn filter_field_level_not() {
    assert_parse(
        indoc! {"
            filter:
              x: { $not: { $eq: 1 } }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Field {
            path: FieldPath::from_dotted("x"),
            op: FieldOp::Not(Box::new(FieldOp::Eq(Value::Number(1.into())))),
        })),
    );
}

#[test]
fn filter_and() {
    assert_parse(
        indoc! {"
            filter:
              $and:
                - { x: 1 }
                - { y: 2 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::and(vec![
            Filter::eq("x", 1i64),
            Filter::eq("y", 2i64),
        ]))),
    );
}

#[test]
fn filter_or() {
    assert_parse(
        indoc! {"
            filter:
              $or:
                - { x: 1 }
                - { y: 2 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::or(vec![
            Filter::eq("x", 1i64),
            Filter::eq("y", 2i64),
        ]))),
    );
}

#[test]
fn filter_top_level_not() {
    assert_parse(
        indoc! {"
            filter:
              $not:
                x: 1
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::Not(Box::new(Filter::eq(
            "x",
            1i64,
        ))))),
    );
}

#[test]
fn filter_dotted_and_nested_paths_produce_same_path() {
    let dotted = parse_operation("filter:\n  a.b: 1\n", OperationKind::Find).unwrap();
    let nested = parse_operation("filter:\n  a:\n    b: 1\n", OperationKind::Find).unwrap();
    assert_eq!(dotted, nested);
    assert_eq!(
        dotted,
        Operation::Find(FindOp::new().filter(Filter::eq("a.b", 1i64))),
    );
}

#[test]
fn filter_array_literal_decodes_as_eq_with_sequence() {
    assert_parse(
        indoc! {"
            filter:
              tags: [rust, async]
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::eq(
            "tags",
            Value::Sequence(vec![
                Value::String("rust".into()),
                Value::String("async".into()),
            ]),
        ))),
    );
}

#[test]
fn update_set_dotted_path_round_trip() {
    assert_parse(
        indoc! {r#"
            filter: {}
            update:
              $set:
                "a.b.c": 1
        "#},
        OperationKind::Update,
        Operation::Update(UpdateOp::new(
            Filter::all(),
            Update::new(vec![UpdateOperator::set("a.b.c", 1i64)]),
        )),
    );
}

#[test]
fn find_rejects_update_field() {
    assert_parse_error(
        indoc! {"
            update:
              $set:
                x: 1
        "},
        OperationKind::Find,
        "OperationFieldNotAllowed",
    );
}

#[test]
fn count_rejects_project_field() {
    assert_parse_error(
        indoc! {"
            project:
              title: 1
        "},
        OperationKind::Count,
        "OperationFieldNotAllowed",
    );
}

#[test]
fn count_rejects_update_field() {
    assert_parse_error(
        indoc! {"
            update:
              $set:
                x: 1
        "},
        OperationKind::Count,
        "OperationFieldNotAllowed",
    );
}

#[test]
fn delete_rejects_update_field() {
    assert_parse_error(
        indoc! {"
            filter: {}
            update:
              $set:
                x: 1
        "},
        OperationKind::Delete,
        "OperationFieldNotAllowed",
    );
}

#[test]
fn delete_rejects_project_field() {
    assert_parse_error(
        indoc! {"
            filter: {}
            project:
              title: 1
        "},
        OperationKind::Delete,
        "OperationFieldNotAllowed",
    );
}

#[test]
fn update_set_unset_conflict_rejected() {
    assert_parse_error(
        indoc! {r#"
            filter: {}
            update:
              $set:
                x: 1
              $unset:
                x: ""
        "#},
        OperationKind::Update,
        "SetUnsetConflict",
    );
}

#[test]
fn update_empty_body_rejected() {
    assert_parse_error(
        indoc! {"
            filter: {}
            update: {}
        "},
        OperationKind::Update,
        "EmptyUpdate",
    );
}

#[test]
fn limit_string_rejected() {
    let err = parse_operation("limit: \"20\"\n", OperationKind::Find)
        .expect_err("string limit must be rejected");
    let s = format!("{:?}", err);
    assert!(
        s.contains("Wire") || s.to_lowercase().contains("invalid"),
        "{}",
        s
    );
}

#[test]
fn update_filter_required_at_parse() {
    assert_parse_error(
        indoc! {"
            update:
              $set:
                x: 1
        "},
        OperationKind::Update,
        "MissingRequiredField",
    );
}

#[test]
fn delete_filter_required_at_parse() {
    assert_parse_error(
        "limit: 10\n",
        OperationKind::Delete,
        "MissingRequiredField",
    );
}

#[test]
fn update_rejects_reserved_prefix_target() {
    assert_parse_error(
        indoc! {"
            filter: {}
            update:
              $set:
                _internal: 1
        "},
        OperationKind::Update,
        "ReservedPrefix",
    );
}

#[test]
fn delete_zero_limit_unbounded() {
    assert_parse(
        indoc! {"
            filter:
              status: archived
            limit: 0
        "},
        OperationKind::Delete,
        Operation::Delete(
            DeleteOp::new(Filter::eq("status", "archived")).limit(0),
        ),
    );
    assert!(Limit(0).is_unbounded());
}
