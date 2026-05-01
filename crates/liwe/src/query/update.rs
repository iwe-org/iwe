use serde_yaml::{Mapping, Value};

use crate::query::document::{Update, UpdateOperator};

pub fn apply(update: &Update, doc: &mut Mapping) {
    for op in &update.operators {
        match op {
            UpdateOperator::Set { path, value } => set_path(doc, &path.0, value.clone()),
            UpdateOperator::Unset { path } => {
                unset_path(doc, &path.0);
            }
        }
    }
}

fn set_path(doc: &mut Mapping, segments: &[String], value: Value) {
    if segments.is_empty() {
        return;
    }
    if segments.len() == 1 {
        doc.insert(Value::String(segments[0].clone()), value);
        return;
    }
    let head_key = Value::String(segments[0].clone());
    let needs_init = !matches!(doc.get(&head_key), Some(Value::Mapping(_)));
    if needs_init {
        doc.insert(head_key.clone(), Value::Mapping(Mapping::new()));
    }
    let inner = match doc.get_mut(&head_key).unwrap() {
        Value::Mapping(m) => m,
        _ => unreachable!(),
    };
    set_path(inner, &segments[1..], value)
}

fn unset_path(doc: &mut Mapping, segments: &[String]) {
    if segments.is_empty() {
        return;
    }
    if segments.len() == 1 {
        doc.remove(Value::String(segments[0].clone()));
        return;
    }
    let head_key = Value::String(segments[0].clone());
    let inner = match doc.get_mut(&head_key) {
        Some(Value::Mapping(m)) => m,
        _ => return,
    };
    unset_path(inner, &segments[1..]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::document::FieldPath;

    fn build_set(path: &[&str], value: Value) -> UpdateOperator {
        UpdateOperator::Set {
            path: FieldPath(path.iter().map(|s| s.to_string()).collect()),
            value,
        }
    }

    fn build_unset(path: &[&str]) -> UpdateOperator {
        UpdateOperator::Unset {
            path: FieldPath(path.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn doc(pairs: Vec<(&str, Value)>) -> Mapping {
        let mut m = Mapping::new();
        for (k, v) in pairs {
            m.insert(Value::String(k.to_string()), v);
        }
        m
    }

    fn nested(pairs: Vec<(&str, Value)>) -> Value {
        Value::Mapping(doc(pairs))
    }

    fn key(s: &str) -> Value {
        Value::String(s.into())
    }

    #[test]
    fn set_top_level_field() {
        let mut d = doc(vec![("status", "draft".into())]);
        let u = Update {
            operators: vec![build_set(&["reviewed"], Value::Bool(true))],
        };
        apply(&u, &mut d);
        assert_eq!(d.get(key("reviewed")), Some(&Value::Bool(true)));
    }

    #[test]
    fn set_replaces_existing_field() {
        let mut d = doc(vec![("status", "draft".into())]);
        let u = Update {
            operators: vec![build_set(&["status"], Value::String("published".to_string()))],
        };
        apply(&u, &mut d);
        assert_eq!(d.get(key("status")), Some(&Value::String("published".into())));
    }

    #[test]
    fn set_dotted_path_auto_creates_intermediates() {
        let mut d = Mapping::new();
        let u = Update {
            operators: vec![build_set(&["a", "b", "c"], Value::Number(1.into()))],
        };
        apply(&u, &mut d);
        let a = d.get(key("a")).expect("a present").as_mapping().unwrap();
        let b = a.get(key("b")).expect("b present").as_mapping().unwrap();
        assert_eq!(b.get(key("c")), Some(&Value::Number(1.into())));
    }

    #[test]
    fn set_dotted_path_extends_existing_mapping() {
        let mut d = doc(vec![("a", nested(vec![("x", 1i64.into())]))]);
        let u = Update {
            operators: vec![build_set(&["a", "y"], Value::Number(2.into()))],
        };
        apply(&u, &mut d);
        let a = d.get(key("a")).unwrap().as_mapping().unwrap();
        assert_eq!(a.get(key("x")), Some(&Value::Number(1.into())));
        assert_eq!(a.get(key("y")), Some(&Value::Number(2.into())));
    }

    #[test]
    fn set_through_scalar_replaces_with_mapping() {
        let mut d = doc(vec![("a", "scalar".into())]);
        let u = Update {
            operators: vec![build_set(&["a", "b"], Value::Number(1.into()))],
        };
        apply(&u, &mut d);
        let a = d.get(key("a")).expect("a present").as_mapping().unwrap();
        assert_eq!(a.get(key("b")), Some(&Value::Number(1.into())));
    }

    #[test]
    fn unset_existing_field() {
        let mut d = doc(vec![
            ("status", "draft".into()),
            ("reviewed", true.into()),
        ]);
        let u = Update {
            operators: vec![build_unset(&["reviewed"])],
        };
        apply(&u, &mut d);
        assert!(!d.contains_key(key("reviewed")));
    }

    #[test]
    fn unset_missing_is_noop() {
        let mut d = doc(vec![("status", "draft".into())]);
        let u = Update {
            operators: vec![build_unset(&["never_existed"])],
        };
        apply(&u, &mut d);
        assert_eq!(d.get(key("status")), Some(&Value::String("draft".into())));
    }

    #[test]
    fn unset_through_non_mapping_is_noop() {
        let mut d = doc(vec![("a", "scalar".into())]);
        let u = Update {
            operators: vec![build_unset(&["a", "b"])],
        };
        apply(&u, &mut d);
        assert_eq!(d.get(key("a")), Some(&Value::String("scalar".into())));
    }

    #[test]
    fn multiple_operators_apply_in_order() {
        let mut d = doc(vec![("a", 1i64.into()), ("b", 2i64.into())]);
        let u = Update {
            operators: vec![
                build_set(&["c"], Value::Number(3.into())),
                build_unset(&["a"]),
            ],
        };
        apply(&u, &mut d);
        assert!(!d.contains_key(key("a")));
        assert_eq!(d.get(key("c")), Some(&Value::Number(3.into())));
    }
}
