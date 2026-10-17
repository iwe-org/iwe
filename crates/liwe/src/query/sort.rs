use std::borrow::Cow;
use std::cmp::Ordering;

use serde_yaml::{Mapping, Value};

use crate::collation::{apply_order, SortKey, TitleCollator};
use crate::model::Key;
use crate::query::document::{is_operator_segment, FieldPath, Sort, SortDir};
use crate::query::filter::cmp_ordered;

pub fn sort_in_place(rows: &mut [(Key, Mapping)], sort: &Sort, collator: &TitleCollator) {
    let collator = match &sort.collation {
        None => Cow::Borrowed(collator),
        Some(collation) => Cow::Owned(TitleCollator::for_collation(collation)),
    };
    let collator = collator.as_ref();
    let values: Vec<Option<&Value>> = rows
        .iter()
        .map(|(_, mapping)| lookup(mapping, &sort.key))
        .collect();
    let keys: Vec<Option<SortKey<'_>>> = values
        .iter()
        .map(|value| match value {
            Some(Value::String(text)) => Some(collator.sort_key(text)),
            _ => None,
        })
        .collect();
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| {
        let primary = match (&keys[a], &keys[b]) {
            (Some(key_a), Some(key_b)) => directed(collator.compare_keys(key_a, key_b), sort.dir),
            _ => compare_values(values[a], values[b], sort.dir),
        };
        primary.then_with(|| rows[a].0.cmp(&rows[b].0))
    });
    drop(keys);
    drop(values);
    apply_order(rows, &order);
}

fn directed(ordering: Ordering, dir: SortDir) -> Ordering {
    match dir {
        SortDir::Asc => ordering,
        SortDir::Desc => ordering.reverse(),
    }
}

fn lookup<'a>(doc: &'a Mapping, path: &FieldPath) -> Option<&'a Value> {
    let segments = path.segments();
    if segments.is_empty() {
        return None;
    }
    if segments.iter().any(|s| is_operator_segment(s)) {
        return None;
    }
    let mut current = doc.get(Value::String(segments[0].clone()))?;
    for seg in &segments[1..] {
        current = match current {
            Value::Mapping(m) => m.get(Value::String(seg.clone()))?,
            _ => return None,
        };
    }
    Some(current)
}

fn compare_values(a: Option<&Value>, b: Option<&Value>, dir: SortDir) -> Ordering {
    let a_null = is_null_or_missing(a);
    let b_null = is_null_or_missing(b);
    let raw = match (a_null, b_null) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => total_cmp(a.unwrap(), b.unwrap()),
    };
    directed(raw, dir)
}

fn total_cmp(a: &Value, b: &Value) -> Ordering {
    if let Some(ord) = cmp_ordered(a, b) {
        return ord;
    }
    let (ra, rb) = (type_rank(a), type_rank(b));
    if ra != rb {
        return ra.cmp(&rb);
    }
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .unwrap_or(f64::NAN)
            .total_cmp(&y.as_f64().unwrap_or(f64::NAN)),
        _ => Ordering::Equal,
    }
}

fn type_rank(v: &Value) -> u8 {
    match v {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Sequence(_) => 4,
        Value::Mapping(_) => 5,
        Value::Tagged(t) => type_rank(&t.value),
    }
}

fn is_null_or_missing(v: Option<&Value>) -> bool {
    matches!(v, None | Some(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(pairs: Vec<(&str, Value)>) -> Mapping {
        let mut m = Mapping::new();
        for (k, v) in pairs {
            m.insert(Value::String(k.to_string()), v);
        }
        m
    }

    fn k(s: &str) -> Key {
        Key::name(s)
    }

    fn sort_with(field: &str, dir: SortDir) -> Sort {
        let path = if field.contains('.') {
            FieldPath::from_dotted(field)
        } else {
            FieldPath(vec![field.to_string()])
        };
        Sort {
            key: path,
            dir,
            collation: None,
        }
    }

    fn english() -> TitleCollator {
        TitleCollator::for_locale(Some("en"))
    }

    fn key_order(rows: &[(Key, Mapping)]) -> Vec<String> {
        rows.iter().map(|(k, _)| k.to_string()).collect()
    }

    #[test]
    fn ascending_orders_low_to_high() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", 5i64.into())]);
        let c = doc(vec![("priority", 3i64.into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc), &english());
        assert_eq!(key_order(&rows), vec!["a", "c", "b"]);
    }

    #[test]
    fn descending_orders_high_to_low() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", 5i64.into())]);
        let c = doc(vec![("priority", 3i64.into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Desc), &english());
        assert_eq!(key_order(&rows), vec!["b", "c", "a"]);
    }

    #[test]
    fn null_first_ascending() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", Value::Null)]);
        let c = Mapping::new();
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc), &english());
        let order = key_order(&rows);
        assert_eq!(order[2], "a");
        assert!(order[0..2].contains(&"b".to_string()));
        assert!(order[0..2].contains(&"c".to_string()));
    }

    #[test]
    fn null_last_descending() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", Value::Null)]);
        let c = Mapping::new();
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Desc), &english());
        let order = key_order(&rows);
        assert_eq!(order[0], "a");
        assert!(order[1..3].contains(&"b".to_string()));
        assert!(order[1..3].contains(&"c".to_string()));
    }

    #[test]
    fn mixed_type_field_is_total_order() {
        let a = doc(vec![("priority", 3i64.into())]);
        let b = doc(vec![("priority", "high".into())]);
        let c = doc(vec![("priority", 5i64.into())]);
        let d = doc(vec![("priority", "low".into())]);
        let e = doc(vec![("priority", 1i64.into())]);
        let mut rows = vec![
            (k("a"), a),
            (k("b"), b),
            (k("c"), c),
            (k("d"), d),
            (k("e"), e),
        ];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc), &english());
        assert_eq!(key_order(&rows), vec!["e", "a", "c", "b", "d"]);
    }

    #[test]
    fn binary_collator_orders_strings_by_codepoint() {
        let a = doc(vec![("name", "apple".into())]);
        let b = doc(vec![("name", "Zebra".into())]);
        let c = doc(vec![("name", "Émile".into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(
            &mut rows,
            &sort_with("name", SortDir::Asc),
            &TitleCollator::binary(),
        );
        assert_eq!(key_order(&rows), vec!["b", "a", "c"]);
    }

    #[test]
    fn locale_collator_orders_strings_naturally() {
        let a = doc(vec![("name", "apple".into())]);
        let b = doc(vec![("name", "Zebra".into())]);
        let c = doc(vec![("name", "Émile".into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("name", SortDir::Asc), &english());
        assert_eq!(key_order(&rows), vec!["a", "c", "b"]);
    }

    #[test]
    fn stable_sort_preserves_input_order_on_ties() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", 1i64.into())]);
        let c = doc(vec![("priority", 1i64.into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc), &english());
        assert_eq!(key_order(&rows), vec!["a", "b", "c"]);
    }
}
