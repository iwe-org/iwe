use indoc::indoc;
use liwe::query::{
    parse_operation, CountArg, CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, GraphOp,
    InclusionAnchor, KeyOp, Limit, MaxDepth, NumExpr, Operation, OperationKind,
    Projection, ReferenceAnchor, Sort, Update, UpdateOp, UpdateOperator, YamlType,
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

#[test]
fn key_scalar_parses_to_eq() {
    assert_parse(
        indoc! {"
            filter:
              $key: notes/foo
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::key(KeyOp::eq("notes/foo")))),
    );
}

#[test]
fn key_explicit_eq() {
    assert_parse(
        indoc! {"
            filter:
              $key: { $eq: notes/foo }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::key(KeyOp::eq("notes/foo")))),
    );
}

#[test]
fn key_in_list() {
    assert_parse(
        indoc! {"
            filter:
              $key: { $in: [a, b, c] }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::key(KeyOp::in_(&["a", "b", "c"])))),
    );
}

#[test]
fn key_gt_rejected() {
    assert_parse_error(
        "filter:\n  $key: { $gt: foo }\n",
        OperationKind::Find,
        "KeyOpForbidden",
    );
}

#[test]
fn key_in_with_non_string_rejected() {
    assert_parse_error(
        "filter:\n  $key: { $in: [1, 2] }\n",
        OperationKind::Find,
        "OperatorExpectedString",
    );
}

#[test]
fn includes_count_bare_int() {
    assert_parse(
        "filter:\n  $includesCount: 0\n",
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludesCount(
            CountArg::direct(NumExpr::eq(0)),
        )))),
    );
}

#[test]
fn includes_count_bare_expr() {
    assert_parse(
        "filter:\n  $includesCount: { $gte: 3 }\n",
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludesCount(
            CountArg::direct(NumExpr::gte(3)),
        )))),
    );
}

#[test]
fn includes_count_full_form_with_any() {
    assert_parse(
        indoc! {"
            filter:
              $includesCount:
                $count: { $gte: 10 }
                $maxDepth: any
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludesCount(
            CountArg {
                count: NumExpr::gte(10),
                min_depth: 1,
                max_depth: MaxDepth::Any,
            },
        )))),
    );
}

#[test]
fn includes_count_range() {
    assert_parse(
        indoc! {"
            filter:
              $includesCount:
                $count: { $gte: 1 }
                $minDepth: 2
                $maxDepth: 4
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludesCount(
            CountArg {
                count: NumExpr::gte(1),
                min_depth: 2,
                max_depth: MaxDepth::Bounded(4),
            },
        )))),
    );
}

#[test]
fn included_by_count_zero() {
    assert_parse(
        "filter:\n  $includedByCount: 0\n",
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludedByCount(
            CountArg::direct(NumExpr::eq(0)),
        )))),
    );
}

#[test]
fn count_missing_count_field_rejected() {
    assert_parse_error(
        "filter:\n  $includesCount: { $maxDepth: 3 }\n",
        OperationKind::Find,
        "MissingCountField",
    );
}

#[test]
fn count_zero_max_depth_rejected() {
    assert_parse_error(
        "filter:\n  $includesCount: { $count: 1, $maxDepth: 0 }\n",
        OperationKind::Find,
        "InvalidDepthValue",
    );
}

#[test]
fn count_inverted_range_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includesCount:
                $count: 1
                $minDepth: 5
                $maxDepth: 2
        "},
        OperationKind::Find,
        "DepthRangeInverted",
    );
}

#[test]
fn count_distance_modifier_rejected() {
    assert_parse_error(
        "filter:\n  $includesCount: { $count: 1, $maxDistance: 1 }\n",
        OperationKind::Find,
        "WrongBoundFamily",
    );
}

#[test]
fn includes_single_anchor() {
    assert_parse(
        "filter:\n  $includes: { $key: roadmap/q2, $maxDepth: 2 }\n",
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::Includes(vec![
            InclusionAnchor::with_max("roadmap/q2", 2),
        ])))),
    );
}

#[test]
fn included_by_range() {
    assert_parse(
        indoc! {"
            filter:
              $includedBy:
                $key: projects/alpha
                $minDepth: 2
                $maxDepth: 5
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludedBy(vec![
            InclusionAnchor::new("projects/alpha", 2, 5),
        ])))),
    );
}

#[test]
fn includes_multi_anchor_array() {
    assert_parse(
        indoc! {"
            filter:
              $includedBy:
                - { $key: projects/alpha, $maxDepth: 5 }
                - { $key: research/q2, $maxDepth: 2 }
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::IncludedBy(vec![
            InclusionAnchor::with_max("projects/alpha", 5),
            InclusionAnchor::with_max("research/q2", 2),
        ])))),
    );
}

#[test]
fn anchor_missing_bound_rejected() {
    assert_parse_error(
        "filter:\n  $includes: { $key: K }\n",
        OperationKind::Find,
        "AnchorMissingBound",
    );
}

#[test]
fn anchor_wrong_bound_family_rejected() {
    assert_parse_error(
        "filter:\n  $includes: { $key: K, $maxDistance: 1 }\n",
        OperationKind::Find,
        "WrongBoundFamily",
    );
}

#[test]
fn walk_key_op_expression_rejected() {
    assert_parse_error(
        "filter:\n  $includes: { $key: { $in: [a, b] }, $maxDepth: 1 }\n",
        OperationKind::Find,
        "WalkKeyNotScalar",
    );
}

#[test]
fn empty_anchor_list_rejected() {
    assert_parse_error(
        "filter:\n  $includes: []\n",
        OperationKind::Find,
        "EmptyAnchorList",
    );
}

#[test]
fn empty_anchor_mapping_rejected() {
    assert_parse_error(
        "filter:\n  $includes: {}\n",
        OperationKind::Find,
        "EmptyAnchorList",
    );
}

#[test]
fn references_with_distance() {
    assert_parse(
        "filter:\n  $references: { $key: people/dmytro, $maxDistance: 1 }\n",
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::References(vec![
            ReferenceAnchor::with_max("people/dmytro", 1),
        ])))),
    );
}

#[test]
fn referenced_by_range() {
    assert_parse(
        indoc! {"
            filter:
              $referencedBy:
                $key: archive/index
                $minDistance: 1
                $maxDistance: 3
        "},
        OperationKind::Find,
        Operation::Find(FindOp::new().filter(Filter::graph(GraphOp::ReferencedBy(vec![
            ReferenceAnchor::new("archive/index", 1, 3),
        ])))),
    );
}

#[test]
fn references_with_depth_modifier_rejected() {
    assert_parse_error(
        "filter:\n  $references: { $key: K, $maxDepth: 1 }\n",
        OperationKind::Find,
        "WrongBoundFamily",
    );
}

#[test]
fn references_zero_distance_rejected() {
    assert_parse_error(
        "filter:\n  $references: { $key: K, $maxDistance: 0 }\n",
        OperationKind::Find,
        "InvalidDepthValue",
    );
}
