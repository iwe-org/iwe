use std::cmp::Ordering;

use serde_yaml::{Mapping, Value};

use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::{FieldOp, FieldPath, Filter, YamlType};
use crate::query::frontmatter::is_reserved_segment;
use crate::query::graph_match::{
    match_inclusion_count, match_inclusion_walk, match_key_op, match_reference_walk,
};

pub fn matches(filter: &Filter, doc: &Mapping, key: &Key, graph: &Graph) -> bool {
    match filter {
        Filter::And(children) => children.iter().all(|c| matches(c, doc, key, graph)),
        Filter::Or(children) => children.iter().any(|c| matches(c, doc, key, graph)),
        Filter::Not(child) => !matches(child, doc, key, graph),
        Filter::Field { path, op } => match resolve_path(doc, path) {
            Resolution::Present(value) => match_field_op(op, Some(value)),
            Resolution::Missing => match_field_op(op, None),
        },
        Filter::Key(op) => match_key_op(op, key),
        Filter::IncludesCount(arg) => match_inclusion_count(arg, key, graph, true),
        Filter::IncludedByCount(arg) => match_inclusion_count(arg, key, graph, false),
        Filter::Includes(anchors) => match_inclusion_walk(anchors, key, graph, true),
        Filter::IncludedBy(anchors) => match_inclusion_walk(anchors, key, graph, false),
        Filter::References(anchors) => match_reference_walk(anchors, key, graph, true),
        Filter::ReferencedBy(anchors) => match_reference_walk(anchors, key, graph, false),
    }
}

#[derive(Debug)]
enum Resolution<'a> {
    Present(&'a Value),
    Missing,
}

fn resolve_path<'a>(doc: &'a Mapping, path: &FieldPath) -> Resolution<'a> {
    let segments = path.segments();
    if segments.is_empty() {
        return Resolution::Missing;
    }
    if segments.iter().any(|s| is_reserved_segment(s)) {
        return Resolution::Missing;
    }
    let mut current: &Value = match doc.get(Value::String(segments[0].clone())) {
        Some(v) => v,
        None => return Resolution::Missing,
    };
    for seg in &segments[1..] {
        let map = match current {
            Value::Mapping(m) => m,
            _ => return Resolution::Missing,
        };
        current = match map.get(Value::String(seg.clone())) {
            Some(v) => v,
            None => return Resolution::Missing,
        };
    }
    Resolution::Present(current)
}

fn match_field_op(op: &FieldOp, value: Option<&Value>) -> bool {
    match op {
        FieldOp::Eq(target) => match value {
            Some(v) => eq_with_membership(v, target),
            None => false,
        },
        FieldOp::Ne(target) => match value {
            Some(v) => !eq_with_membership(v, target),
            None => true,
        },
        FieldOp::Gt(target) => cmp_with_membership(value, target, |o| o == Some(Ordering::Greater)),
        FieldOp::Gte(target) => cmp_with_membership(value, target, |o| {
            matches!(o, Some(Ordering::Greater) | Some(Ordering::Equal))
        }),
        FieldOp::Lt(target) => cmp_with_membership(value, target, |o| o == Some(Ordering::Less)),
        FieldOp::Lte(target) => cmp_with_membership(value, target, |o| {
            matches!(o, Some(Ordering::Less) | Some(Ordering::Equal))
        }),
        FieldOp::In(list) => match value {
            Some(v) => list.iter().any(|elem| eq_with_membership(v, elem)),
            None => false,
        },
        FieldOp::Nin(list) => match value {
            Some(v) => !list.iter().any(|elem| eq_with_membership(v, elem)),
            None => true,
        },
        FieldOp::Exists(want_present) => value.is_some() == *want_present,
        FieldOp::Type(types) => match value {
            Some(v) => types.iter().any(|t| value_type_matches(v, *t)),
            None => false,
        },
        FieldOp::All(list) => match value {
            Some(Value::Sequence(seq)) => list
                .iter()
                .all(|target| seq.iter().any(|elem| deep_eq(elem, target))),
            _ => false,
        },
        FieldOp::Size(n) => match value {
            Some(Value::Sequence(seq)) => seq.len() as u64 == *n,
            _ => false,
        },
        FieldOp::Not(inner) => !match_field_op(inner, value),
    }
}


fn eq_with_membership(field: &Value, target: &Value) -> bool {
    if let Value::Sequence(seq) = field {
        if !matches!(target, Value::Sequence(_) | Value::Mapping(_)) {
            return seq.iter().any(|elem| deep_eq(elem, target));
        }
    }
    deep_eq(field, target)
}

fn cmp_with_membership(
    value: Option<&Value>,
    target: &Value,
    pred: impl Fn(Option<Ordering>) -> bool,
) -> bool {
    let v = match value {
        None => return false,
        Some(v) => v,
    };
    if let Value::Sequence(seq) = v {
        if !matches!(target, Value::Sequence(_) | Value::Mapping(_)) {
            return seq.iter().any(|elem| pred(cmp_ordered(elem, target)));
        }
    }
    pred(cmp_ordered(v, target))
}


pub fn deep_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(xf), Some(yf)) => xf == yf,
            _ => false,
        },
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Sequence(x), Value::Sequence(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| deep_eq(a, b))
        }
        (Value::Mapping(x), Value::Mapping(y)) => {
            if x.len() != y.len() {
                return false;
            }
            for (k, v) in x {
                match y.get(k) {
                    Some(other) => {
                        if !deep_eq(v, other) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        (Value::Tagged(x), Value::Tagged(y)) => x.tag == y.tag && deep_eq(&x.value, &y.value),
        _ => false,
    }
}


pub fn cmp_ordered(a: &Value, b: &Value) -> Option<Ordering> {
    use Value::*;
    match (a, b) {
        (Null, _) | (_, Null) => None,
        (Number(x), Number(y)) => x.as_f64().and_then(|xf| {
            y.as_f64().map(|yf| {
                if xf < yf {
                    Ordering::Less
                } else if xf > yf {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
        }),
        (String(x), String(y)) => Some(x.cmp(y)),
        (Bool(x), Bool(y)) => Some(x.cmp(y)),
        (Tagged(x), Tagged(y)) if x.tag == y.tag => cmp_ordered(&x.value, &y.value),
        _ => None,
    }
}

fn value_type_matches(v: &Value, t: YamlType) -> bool {


    match (v, t) {
        (Value::Null, YamlType::Null) => true,
        (Value::Bool(_), YamlType::Boolean) => true,
        (Value::Number(_), YamlType::Number) => true,
        (Value::String(_), YamlType::String) => true,
        (Value::Sequence(_), YamlType::Array) => true,
        (Value::Mapping(_), YamlType::Object) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::document::{FieldOp, FieldPath, Filter, YamlType};


    fn p(s: &str) -> FieldPath {
        if s.contains('.') {
            FieldPath::from_dotted(s)
        } else {
            FieldPath(vec![s.to_string()])
        }
    }

    fn eq(path: &str, v: impl Into<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Eq(v.into()) }
    }
    fn ne(path: &str, v: impl Into<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Ne(v.into()) }
    }
    fn gt(path: &str, v: impl Into<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Gt(v.into()) }
    }
    fn gte(path: &str, v: impl Into<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Gte(v.into()) }
    }
    fn lt(path: &str, v: impl Into<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Lt(v.into()) }
    }
    fn exists(path: &str, present: bool) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Exists(present) }
    }
    fn in_op(path: &str, values: Vec<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::In(values) }
    }
    fn nin(path: &str, values: Vec<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Nin(values) }
    }
    fn all(path: &str, values: Vec<Value>) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::All(values) }
    }
    fn size(path: &str, n: u64) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Size(n) }
    }
    fn type_of(path: &str, t: YamlType) -> Filter {
        Filter::Field { path: p(path), op: FieldOp::Type(vec![t]) }
    }
    fn and(filters: Vec<Filter>) -> Filter { Filter::And(filters) }
    fn or(filters: Vec<Filter>) -> Filter { Filter::Or(filters) }
    fn not(f: Filter) -> Filter { Filter::Not(Box::new(f)) }


    fn doc(pairs: Vec<(&str, Value)>) -> Mapping {
        let mut m = Mapping::new();
        for (k, v) in pairs {
            m.insert(Value::String(k.to_string()), v);
        }
        m
    }
    fn nested(pairs: Vec<(&str, Value)>) -> Value { Value::Mapping(doc(pairs)) }
    fn list(values: Vec<Value>) -> Value { Value::Sequence(values) }
    fn null() -> Value { Value::Null }

    fn check(filter: &Filter, doc: &Mapping, expected: bool) {
        let g = Graph::new();
        let k = Key::name("test");
        assert_eq!(
            matches(filter, doc, &k, &g),
            expected,
            "filter: {:?}\ndoc: {:?}",
            filter,
            doc
        );
    }

    #[test]
    fn drafts() {
        let f = eq("status", "draft");
        check(&f, &doc(vec![("status", "draft".into())]), true);
        check(&f, &doc(vec![("status", "published".into())]), false);
        check(&f, &Mapping::new(), false);
    }

    #[test]
    fn drafts_modified_this_year() {
        let f = and(vec![eq("status", "draft"), gte("modified_at", "2026-01-01")]);
        check(
            &f,
            &doc(vec![
                ("status", "draft".into()),
                ("modified_at", "2026-04-15".into()),
            ]),
            true,
        );
        check(
            &f,
            &doc(vec![
                ("status", "draft".into()),
                ("modified_at", "2025-12-15".into()),
            ]),
            false,
        );
    }

    #[test]
    fn tagged_either_rust_or_async() {
        let f = in_op("tags", vec!["rust".into(), "async".into()]);
        check(
            &f,
            &doc(vec![("tags", list(vec!["rust".into(), "go".into()]))]),
            true,
        );
        check(
            &f,
            &doc(vec![("tags", list(vec!["go".into(), "python".into()]))]),
            false,
        );
    }

    #[test]
    fn tagged_with_both_rust_and_async() {
        let f = all("tags", vec!["rust".into(), "async".into()]);
        check(
            &f,
            &doc(vec![(
                "tags",
                list(vec!["rust".into(), "async".into(), "go".into()]),
            )]),
            true,
        );
        check(
            &f,
            &doc(vec![("tags", list(vec!["rust".into(), "go".into()]))]),
            false,
        );
    }

    #[test]
    fn has_no_tags() {
        let f = or(vec![exists("tags", false), size("tags", 0)]);
        check(&f, &doc(vec![("tags", list(vec![]))]), true);
        check(&f, &Mapping::new(), true);
        check(&f, &doc(vec![("tags", list(vec!["rust".into()]))]), false);
    }

    #[test]
    fn reviewed_but_no_reviewer() {
        let f = and(vec![exists("reviewed_at", true), exists("reviewed_by", false)]);
        check(&f, &doc(vec![("reviewed_at", "2026-04-26".into())]), true);
        check(
            &f,
            &doc(vec![
                ("reviewed_at", "2026-04-26".into()),
                ("reviewed_by", "alice".into()),
            ]),
            false,
        );
        check(&f, &Mapping::new(), false);
    }

    #[test]
    fn drafts_not_by_dmytro() {
        let f = and(vec![eq("status", "draft"), ne("author", "dmytro")]);
        check(
            &f,
            &doc(vec![
                ("status", "draft".into()),
                ("author", "alice".into()),
            ]),
            true,
        );
        check(
            &f,
            &doc(vec![
                ("status", "draft".into()),
                ("author", "dmytro".into()),
            ]),
            false,
        );

        check(&f, &doc(vec![("status", "draft".into())]), true);
    }

    #[test]
    fn recent_high_priority() {
        let f = and(vec![
            gte("modified_at", "2026-04-01"),
            or(vec![gte("priority", 8i64), eq("tags", "urgent")]),
        ]);
        check(
            &f,
            &doc(vec![
                ("modified_at", "2026-04-15".into()),
                ("priority", 9i64.into()),
            ]),
            true,
        );
        check(
            &f,
            &doc(vec![
                ("modified_at", "2026-04-15".into()),
                ("tags", list(vec!["urgent".into()])),
            ]),
            true,
        );
        check(
            &f,
            &doc(vec![
                ("modified_at", "2026-04-15".into()),
                ("priority", 5i64.into()),
            ]),
            false,
        );
        check(
            &f,
            &doc(vec![
                ("modified_at", "2026-03-01".into()),
                ("priority", 9i64.into()),
            ]),
            false,
        );
    }

    #[test]
    fn array_membership_eq_bare_scalar() {
        let f = eq("tags", "rust");
        check(
            &f,
            &doc(vec![("tags", list(vec!["rust".into(), "async".into()]))]),
            true,
        );
        check(
            &f,
            &doc(vec![("tags", list(vec!["go".into(), "python".into()]))]),
            false,
        );
    }

    #[test]
    fn array_membership_ne() {
        let f = ne("tags", "rust");
        check(
            &f,
            &doc(vec![("tags", list(vec!["go".into(), "python".into()]))]),
            true,
        );
        check(
            &f,
            &doc(vec![("tags", list(vec!["rust".into(), "go".into()]))]),
            false,
        );
    }

    #[test]
    fn whole_array_equality_via_array_literal() {
        let f = eq("tags", list(vec!["rust".into(), "async".into()]));
        check(
            &f,
            &doc(vec![("tags", list(vec!["rust".into(), "async".into()]))]),
            true,
        );
        check(
            &f,
            &doc(vec![("tags", list(vec!["async".into(), "rust".into()]))]),
            false,
        );
    }

    #[test]
    fn missing_vs_explicit_null() {
        check(&exists("x", true), &doc(vec![("x", null())]), true);
        check(&exists("x", false), &doc(vec![("x", null())]), false);
        check(&eq("x", null()), &doc(vec![("x", null())]), true);
        check(&eq("x", null()), &Mapping::new(), false);
    }

    #[test]
    fn type_bracket_number_vs_string() {
        check(&gt("x", 3i64), &doc(vec![("x", 5i64.into())]), true);
        check(&gt("x", 3i64), &doc(vec![("x", "5".into())]), false);
    }

    #[test]
    fn boolean_ordering() {
        check(&gt("flag", false), &doc(vec![("flag", true.into())]), true);
        check(&lt("flag", true), &doc(vec![("flag", false.into())]), true);
    }

    #[test]
    fn nested_intermediate_non_mapping_makes_leaf_missing() {
        check(
            &eq("author.name", "dmytro"),
            &doc(vec![("author", "dmytro".into())]),
            false,
        );
        check(
            &exists("author.name", false),
            &doc(vec![("author", "dmytro".into())]),
            true,
        );
    }

    #[test]
    fn dotted_equivalent_to_nested() {
        let nested_filter = Filter::Field {
            path: FieldPath(vec!["author".to_string(), "name".to_string()]),
            op: FieldOp::Eq(Value::String("dmytro".into())),
        };
        let dotted_filter = eq("author.name", "dmytro");
        let d = doc(vec![("author", nested(vec![("name", "dmytro".into())]))]);
        let g = Graph::new();
        let k = Key::name("test");
        assert_eq!(
            matches(&nested_filter, &d, &k, &g),
            matches(&dotted_filter, &d, &k, &g)
        );
        assert!(matches(&nested_filter, &d, &k, &g));
    }

    #[test]
    fn not_matches_missing_field() {
        let f = not(eq("reviewed", true));
        check(&f, &Mapping::new(), true);
        check(&f, &doc(vec![("reviewed", false.into())]), true);
        check(&f, &doc(vec![("reviewed", true.into())]), false);
    }

    #[test]
    fn comparison_against_missing_is_false() {
        check(&gt("x", 3i64), &Mapping::new(), false);
        check(&lt("x", 3i64), &Mapping::new(), false);
    }

    #[test]
    fn nin_matches_missing() {
        check(
            &nin("x", vec!["a".into(), "b".into()]),
            &Mapping::new(),
            true,
        );
    }

    #[test]
    fn type_does_not_match_missing() {
        check(&type_of("x", YamlType::String), &Mapping::new(), false);
    }
}
