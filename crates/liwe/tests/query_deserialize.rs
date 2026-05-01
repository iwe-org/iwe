use indoc::indoc;
use liwe::query::prelude::{
    all, and, count, delete, eq, exists, filter, find, gt, gte, in_, included_by, includes,
    key_eq, key_in, lt, lte, ne, nin, nor, not, or, referenced_by, references, size, type_of,
    update, update_op,
};
use liwe::query::{
    parse_operation, CountOp, DeleteOp, FieldOp, FieldPath, Filter, FindOp, InclusionAnchor, Limit,
    Operation, OperationKind, Projection, ReferenceAnchor, Sort, Update, UpdateOperator, YamlType,
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
        find(filter::<FindOp>(eq("status", "draft"))
            .project(Projection::fields(&["title"]))
            .sort(Sort::desc("modified"))
            .limit(5)),
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
        count(filter::<CountOp>(eq("status", "draft")).limit(0)),
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
        update(update_op(
            eq("status", "draft"),
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
        delete(filter(eq("status", "archived"))),
    );
}

#[test]
fn empty_yaml_parses_to_default_find() {
    assert_parse(
        "",
        OperationKind::Find,
        find(FindOp::new()),
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
        find(FindOp::new().sort(Sort::asc("modified"))),
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
        find(filter(eq("x", 5i64))),
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
        find(filter(eq("x", 5i64))),
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
        find(filter(ne("status", "draft"))),
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
        find(filter(and(vec![
            gt("a", 1i64),
            gte("b", 2i64),
            lt("c", 3i64),
            lte("d", 4i64),
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
        find(filter(in_("tags", ["rust", "go"]))),
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
        find(filter(nin("tags", ["rust"]))),
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
        find(filter(exists("x", true))),
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
        find(filter(exists("x", false))),
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
        find(filter(type_of("x", [YamlType::String]))),
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
        find(filter(type_of("x", [YamlType::String, YamlType::Number]))),
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
        find(filter(all("tags", ["rust", "async"]))),
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
        find(filter(size("tags", 3))),
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
        find(filter(Filter::Field {
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
        find(filter(and(vec![
            eq("x", 1i64),
            eq("y", 2i64),
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
        find(filter(or(vec![
            eq("x", 1i64),
            eq("y", 2i64),
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
        find(filter(not(eq("x", 1i64)))),
    );
}

#[test]
fn filter_nor() {
    assert_parse(
        indoc! {"
            filter:
              $nor:
                - { status: archived }
                - { status: deleted }
        "},
        OperationKind::Find,
        find(filter(nor(vec![
            eq("status", "archived"),
            eq("status", "deleted"),
        ]))),
    );
}

#[test]
fn filter_nor_empty_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $nor: []
        "},
        OperationKind::Find,
        "EmptyOperatorList",
    );
}

#[test]
fn filter_nested_not_top_level_parses() {
    assert_parse(
        indoc! {"
            filter:
              $not:
                $not:
                  x: 1
        "},
        OperationKind::Find,
        find(filter(not(not(eq("x", 1i64))))),
    );
}

#[test]
fn filter_per_field_nested_not_parses() {
    assert_parse(
        indoc! {"
            filter:
              priority: { $not: { $not: { $gt: 5 } } }
        "},
        OperationKind::Find,
        find(filter(Filter::Field {
            path: FieldPath::from_dotted("priority"),
            op: FieldOp::Not(Box::new(FieldOp::Not(Box::new(FieldOp::Gt(
                Value::from(5i64),
            ))))),
        })),
    );
}

#[test]
fn filter_per_field_not_wraps_multi_operator_range() {
    assert_parse(
        indoc! {"
            filter:
              priority: { $not: { $gt: 5, $lt: 10 } }
        "},
        OperationKind::Find,
        find(filter(Filter::Field {
            path: FieldPath::from_dotted("priority"),
            op: FieldOp::Not(Box::new(FieldOp::And(vec![
                FieldOp::Gt(Value::from(5i64)),
                FieldOp::Lt(Value::from(10i64)),
            ]))),
        })),
    );
}

#[test]
fn filter_dotted_and_nested_paths_produce_same_path() {
    let dotted = parse_operation("filter:\n  a.b: 1\n", OperationKind::Find).unwrap();
    let nested = parse_operation("filter:\n  a:\n    b: 1\n", OperationKind::Find).unwrap();
    assert_eq!(dotted, nested);
    assert_eq!(
        dotted,
        find(filter(eq("a.b", 1i64))),
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
        find(filter(eq(
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
        update(update_op(
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
        delete(filter::<DeleteOp>(eq("status", "archived")).limit(0)),
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
        find(filter(key_eq("notes/foo"))),
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
        find(filter(key_eq("notes/foo"))),
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
        find(filter(key_in(&["a", "b", "c"]))),
    );
}

#[test]
fn key_gt_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $key: { $gt: foo }
        "},
        OperationKind::Find,
        "KeyOpForbidden",
    );
}

#[test]
fn key_in_with_non_string_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $key: { $in: [1, 2] }
        "},
        OperationKind::Find,
        "OperatorExpectedString",
    );
}

#[test]
fn includes_count_operator_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includesCount: 0
        "},
        OperationKind::Find,
        "UnknownOperator",
    );
}

#[test]
fn included_by_count_operator_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includedByCount: 0
        "},
        OperationKind::Find,
        "UnknownOperator",
    );
}

#[test]
fn includes_scalar_shorthand() {
    assert_parse(
        indoc! {"
            filter:
              $includes: roadmap/q2
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::new("roadmap/q2", 1, 1)))),
    );
}

#[test]
fn includes_single_anchor() {
    assert_parse(
        indoc! {"
            filter:
              $includes: { match: { $key: roadmap/q2 }, maxDepth: 2 }
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::with_max("roadmap/q2", 2)))),
    );
}

#[test]
fn included_by_range() {
    assert_parse(
        indoc! {"
            filter:
              $includedBy:
                match: { $key: projects/alpha }
                minDepth: 2
                maxDepth: 5
        "},
        OperationKind::Find,
        find(filter(included_by(InclusionAnchor::new("projects/alpha", 2, 5)))),
    );
}

#[test]
fn includes_array_form_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includedBy:
                - { match: { $key: projects/alpha }, maxDepth: 5 }
                - { match: { $key: research/q2 }, maxDepth: 2 }
        "},
        OperationKind::Find,
        "ArrayFormRemoved",
    );
}

#[test]
fn anchor_full_form_omitted_bounds_unbounded() {
    assert_parse(
        indoc! {"
            filter:
              $includes: { match: { $key: K } }
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::new("K", 1, u32::MAX)))),
    );
}

#[test]
fn anchor_wrong_bound_family_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includes: { match: { $key: K }, maxDistance: 1 }
        "},
        OperationKind::Find,
        "WrongBoundFamily",
    );
}

#[test]
fn walk_key_in_expression_anchors_set() {
    assert_parse(
        indoc! {"
            filter:
              $includes: { match: { $key: { $in: [a, b] } }, maxDepth: 1 }
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::with_match(
            key_in(&["a", "b"]),
            1,
            1,
        )))),
    );
}

#[test]
fn empty_anchor_list_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includes: []
        "},
        OperationKind::Find,
        "ArrayFormRemoved",
    );
}

#[test]
fn empty_anchor_mapping_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $includes: {}
        "},
        OperationKind::Find,
        "EmptyAnchorMapping",
    );
}

#[test]
fn match_predicate_form_anchors_by_frontmatter() {
    assert_parse(
        indoc! {"
            filter:
              $includes: { match: { status: draft }, maxDepth: 2 }
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::with_match(
            eq("status", "draft"),
            1,
            2,
        )))),
    );
}

#[test]
fn match_compound_predicate_anchors_with_full_filter() {
    assert_parse(
        indoc! {"
            filter:
              $includedBy:
                match:
                  type: project
                  status: active
                maxDepth: 5
        "},
        OperationKind::Find,
        find(filter(included_by(InclusionAnchor::with_match(
            and(vec![eq("type", "project"), eq("status", "active")]),
            1,
            5,
        )))),
    );
}

#[test]
fn match_or_expression_anchors_with_filter() {
    assert_parse(
        indoc! {"
            filter:
              $includes:
                match:
                  $or:
                    - status: draft
                    - tag: important
                maxDepth: 2
        "},
        OperationKind::Find,
        find(filter(includes(InclusionAnchor::with_match(
            or(vec![eq("status", "draft"), eq("tag", "important")]),
            1,
            2,
        )))),
    );
}

#[test]
fn match_nested_relational_anchor() {
    assert_parse(
        indoc! {"
            filter:
              $includedBy:
                match:
                  $includedBy: { match: { $key: projects/alpha }, maxDepth: 5 }
                maxDepth: 5
        "},
        OperationKind::Find,
        find(filter(included_by(InclusionAnchor::with_match(
            included_by(InclusionAnchor::with_max("projects/alpha", 5)),
            1,
            5,
        )))),
    );
}

#[test]
fn references_scalar_shorthand() {
    assert_parse(
        indoc! {"
            filter:
              $references: people/alice
        "},
        OperationKind::Find,
        find(filter(references(ReferenceAnchor::new("people/alice", 1, 1)))),
    );
}

#[test]
fn references_with_distance() {
    assert_parse(
        indoc! {"
            filter:
              $references: { match: { $key: people/dmytro }, maxDistance: 1 }
        "},
        OperationKind::Find,
        find(filter(references(ReferenceAnchor::with_max("people/dmytro", 1)))),
    );
}

#[test]
fn referenced_by_range() {
    assert_parse(
        indoc! {"
            filter:
              $referencedBy:
                match: { $key: archive/index }
                minDistance: 1
                maxDistance: 3
        "},
        OperationKind::Find,
        find(filter(referenced_by(ReferenceAnchor::new("archive/index", 1, 3)))),
    );
}

#[test]
fn references_with_depth_modifier_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $references: { match: { $key: K }, maxDepth: 1 }
        "},
        OperationKind::Find,
        "WrongBoundFamily",
    );
}

#[test]
fn references_zero_distance_rejected() {
    assert_parse_error(
        indoc! {"
            filter:
              $references: { match: { $key: K }, maxDistance: 0 }
        "},
        OperationKind::Find,
        "InvalidDepthValue",
    );
}
