use std::cmp::Ordering;

use serde_yaml::{Mapping, Value};

use crate::model::Key;
use crate::query::document::{FieldPath, Sort, SortDir};
use crate::query::filter::cmp_ordered;
use crate::query::frontmatter::is_reserved_segment;


pub fn sort_in_place(rows: &mut [(Key, Mapping)], sort: &Sort) {
    rows.sort_by(|a, b| {
        let primary = compare_values(lookup(&a.1, &sort.key), lookup(&b.1, &sort.key), sort.dir);
        primary.then_with(|| a.0.cmp(&b.0))
    });
}

fn lookup<'a>(doc: &'a Mapping, path: &FieldPath) -> Option<&'a Value> {
    let segments = path.segments();
    if segments.is_empty() {
        return None;
    }
    if segments.iter().any(|s| is_reserved_segment(s)) {
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
        (false, false) => cmp_ordered(a.unwrap(), b.unwrap()).unwrap_or(Ordering::Equal),
    };
    match dir {
        SortDir::Asc => raw,
        SortDir::Desc => raw.reverse(),
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
        Sort { key: path, dir }
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
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc));
        assert_eq!(key_order(&rows), vec!["a", "c", "b"]);
    }

    #[test]
    fn descending_orders_high_to_low() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", 5i64.into())]);
        let c = doc(vec![("priority", 3i64.into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Desc));
        assert_eq!(key_order(&rows), vec!["b", "c", "a"]);
    }

    #[test]
    fn null_first_ascending() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", Value::Null)]);
        let c = Mapping::new();
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc));
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
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Desc));
        let order = key_order(&rows);
        assert_eq!(order[0], "a");
        assert!(order[1..3].contains(&"b".to_string()));
        assert!(order[1..3].contains(&"c".to_string()));
    }

    #[test]
    fn stable_sort_preserves_input_order_on_ties() {
        let a = doc(vec![("priority", 1i64.into())]);
        let b = doc(vec![("priority", 1i64.into())]);
        let c = doc(vec![("priority", 1i64.into())]);
        let mut rows = vec![(k("a"), a), (k("b"), b), (k("c"), c)];
        sort_in_place(&mut rows, &sort_with("priority", SortDir::Asc));
        assert_eq!(key_order(&rows), vec!["a", "b", "c"]);
    }
}
