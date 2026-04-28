use serde_yaml::{Mapping, Value};

use crate::query::document::Projection;
use crate::query::frontmatter::is_reserved_segment;


pub fn shape(projection: &Projection, doc: &Mapping) -> Mapping {
    let mut out = Mapping::new();
    for path in &projection.fields {
        copy_path(doc, &path.0, &mut out);
    }
    out
}

fn copy_path(src: &Mapping, segments: &[String], dst: &mut Mapping) {
    if segments.is_empty() {
        return;
    }
    if is_reserved_segment(&segments[0]) {
        return;
    }
    let head_key = Value::String(segments[0].clone());
    let value = match src.get(&head_key) {
        Some(v) => v,
        None => return,
    };
    if segments.len() == 1 {
        dst.insert(head_key, value.clone());
        return;
    }
    let inner_src = match value {
        Value::Mapping(m) => m,
        _ => return,
    };
    let entry = dst
        .entry(head_key)
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let inner_dst = match entry {
        Value::Mapping(m) => m,
        _ => return,
    };
    copy_path(inner_src, &segments[1..], inner_dst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::document::FieldPath;

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

    fn project(paths: &[&[&str]]) -> Projection {
        Projection {
            fields: paths
                .iter()
                .map(|p| FieldPath(p.iter().map(|s| s.to_string()).collect()))
                .collect(),
        }
    }

    fn key(s: &str) -> Value {
        Value::String(s.into())
    }

    #[test]
    fn included_field_kept() {
        let d = doc(vec![
            ("title", "Foo".into()),
            ("author", "dmytro".into()),
            ("status", "draft".into()),
        ]);
        let p = project(&[&["title"]]);
        let out = shape(&p, &d);
        assert_eq!(out.get(key("title")), Some(&Value::String("Foo".into())));
        assert!(!out.contains_key(key("author")));
        assert!(!out.contains_key(key("status")));
    }

    #[test]
    fn missing_field_stays_missing() {
        let d = doc(vec![("title", "Foo".into())]);
        let p = project(&[&["author"]]);
        let out = shape(&p, &d);
        assert!(out.is_empty());
    }

    #[test]
    fn nested_field_keeps_parent_structure() {
        let d = doc(vec![(
            "author",
            nested(vec![("name", "dmytro".into()), ("email", "a@b.c".into())]),
        )]);
        let p = project(&[&["author", "name"]]);
        let out = shape(&p, &d);
        let author = out
            .get(key("author"))
            .expect("author kept")
            .as_mapping()
            .expect("author is a mapping");
        assert_eq!(author.get(key("name")), Some(&Value::String("dmytro".into())));
        assert!(!author.contains_key(key("email")));
    }

    #[test]
    fn empty_projection_produces_empty_mapping() {
        let d = doc(vec![("title", "Foo".into())]);
        let p = project(&[]);
        let out = shape(&p, &d);
        assert!(out.is_empty());
    }

    #[test]
    fn nested_path_through_non_mapping_is_dropped() {
        let d = doc(vec![("author", "dmytro".into())]);
        let p = project(&[&["author", "name"]]);
        let out = shape(&p, &d);
        assert!(out.is_empty());
    }
}
