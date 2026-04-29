use serde_yaml::{Mapping, Value};

pub fn strip_reserved(mapping: &mut Mapping) {
    let keys_to_remove: Vec<Value> = mapping
        .iter()
        .filter_map(|(k, _)| match k.as_str() {
            Some(s) if is_reserved_segment(s) => Some(k.clone()),
            _ => None,
        })
        .collect();
    for k in keys_to_remove {
        mapping.remove(&k);
    }
}

pub fn is_reserved_segment(s: &str) -> bool {
    matches!(s.chars().next(), Some('_' | '$' | '.' | '#' | '@'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(yaml: &str) -> Mapping {
        match serde_yaml::from_str::<Value>(yaml).unwrap() {
            Value::Mapping(m) => m,
            Value::Null => Mapping::new(),
            _ => panic!(),
        }
    }

    #[test]
    fn strip_removes_reserved_prefixes() {
        let mut m = map("_internal: 1\n$weird: 2\n.dot: 3\n\"#hash\": 4\n\"@user\": 5\nname: ok\n");
        strip_reserved(&mut m);
        assert!(!m.contains_key(Value::String("_internal".into())));
        assert!(!m.contains_key(Value::String("$weird".into())));
        assert!(!m.contains_key(Value::String(".dot".into())));
        assert!(!m.contains_key(Value::String("#hash".into())));
        assert!(!m.contains_key(Value::String("@user".into())));
        assert!(m.contains_key(Value::String("name".into())));
    }

    #[test]
    fn strip_keeps_unreserved_prefixes() {
        let mut m = map(
            "foo: 1\nfoo_bar: 2\nfoo123: 3\n\"2024\": 4\n\"-hyphen\": 5\n\"with/slash\": 6\n",
        );
        strip_reserved(&mut m);
        assert!(m.contains_key(Value::String("foo".into())));
        assert!(m.contains_key(Value::String("foo_bar".into())));
        assert!(m.contains_key(Value::String("foo123".into())));
        assert!(m.contains_key(Value::String("2024".into())));
        assert!(m.contains_key(Value::String("-hyphen".into())));
        assert!(m.contains_key(Value::String("with/slash".into())));
    }

    #[test]
    fn reserved_segment_classification() {
        assert!(is_reserved_segment("_x"));
        assert!(is_reserved_segment("$x"));
        assert!(is_reserved_segment(".x"));
        assert!(is_reserved_segment("#x"));
        assert!(is_reserved_segment("@x"));
        assert!(!is_reserved_segment("foo"));
        assert!(!is_reserved_segment("2024"));
        assert!(!is_reserved_segment("-foo"));
        assert!(!is_reserved_segment("/foo"));
        assert!(!is_reserved_segment(""));
    }
}
