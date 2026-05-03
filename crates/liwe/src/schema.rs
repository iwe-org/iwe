use std::collections::{BTreeMap, HashMap};

use serde::Serialize;
use serde_yaml::Value;

use crate::graph::Graph;
use crate::model::Key;
use crate::query::document::YamlType;
use crate::query::filter::detect_type;
use crate::query::frontmatter::is_reserved_segment;

const MAX_DISTINCT_VALUES: usize = 100;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeCount {
    #[serde(rename = "type")]
    pub yaml_type: String,
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueCount {
    pub value: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub count: usize,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSchema {
    #[serde(rename = "field")]
    pub name: String,
    pub types: Vec<TypeCount>,
    pub coverage: Coverage,
    pub distinct: usize,
    pub values: Vec<ValueCount>,
}

struct FieldAccumulator {
    type_counts: HashMap<YamlType, usize>,
    coverage: usize,
    value_counts: HashMap<String, usize>,
}

impl FieldAccumulator {
    fn new() -> Self {
        Self {
            type_counts: HashMap::new(),
            coverage: 0,
            value_counts: HashMap::new(),
        }
    }

    fn record(&mut self, value: &Value) {
        self.coverage += 1;
        let t = detect_type(value);
        *self.type_counts.entry(t).or_insert(0) += 1;

        if let Some(s) = scalar_to_string(value) {
            *self.value_counts.entry(s).or_insert(0) += 1;
        }
    }
}

fn is_enum_like(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
}

fn scalar_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Null => Some("null".to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) if is_enum_like(s) => Some(s.clone()),
        Value::Tagged(t) => scalar_to_string(&t.value),
        _ => None,
    }
}

fn walk_mapping(
    mapping: &serde_yaml::Mapping,
    prefix: &str,
    accumulators: &mut BTreeMap<String, FieldAccumulator>,
) {
    for (key, value) in mapping {
        let field_name = match key.as_str() {
            Some(s) => s,
            None => continue,
        };

        if is_reserved_segment(field_name) {
            continue;
        }

        let path = if prefix.is_empty() {
            field_name.to_string()
        } else {
            format!("{}.{}", prefix, field_name)
        };

        let acc = accumulators
            .entry(path.clone())
            .or_insert_with(FieldAccumulator::new);
        acc.record(value);

        if let Value::Mapping(nested) = value {
            walk_mapping(nested, &path, accumulators);
        }
    }
}

pub fn infer_schema(graph: &Graph, keys: &[Key]) -> Vec<FieldSchema> {
    let total_documents = keys.len();
    let mut accumulators: BTreeMap<String, FieldAccumulator> = BTreeMap::new();

    for key in keys {
        if let Some(mapping) = graph.frontmatter(key) {
            walk_mapping(mapping, "", &mut accumulators);
        }
    }

    accumulators
        .into_iter()
        .map(|(name, acc)| {
            let coverage_count = acc.coverage;

            let mut types: Vec<TypeCount> = acc
                .type_counts
                .into_iter()
                .map(|(t, count)| TypeCount {
                    yaml_type: t.to_string(),
                    count,
                    percentage: if coverage_count > 0 {
                        (count as f64 / coverage_count as f64) * 100.0
                    } else {
                        0.0
                    },
                })
                .collect();
            types.sort_by(|a, b| b.count.cmp(&a.count).then(a.yaml_type.cmp(&b.yaml_type)));

            let distinct = acc.value_counts.len();
            let values = if distinct <= MAX_DISTINCT_VALUES {
                let mut vals: Vec<ValueCount> = acc
                    .value_counts
                    .into_iter()
                    .map(|(value, count)| ValueCount { value, count })
                    .collect();
                vals.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
                vals
            } else {
                Vec::new()
            };

            FieldSchema {
                name,
                types,
                coverage: Coverage {
                    count: coverage_count,
                    percentage: if total_documents > 0 {
                        (coverage_count as f64 / total_documents as f64) * 100.0
                    } else {
                        0.0
                    },
                },
                distinct,
                values,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::MarkdownReader;
    use crate::model::Key;

    fn build_graph(docs: &[(&str, &str)]) -> Graph {
        let mut graph = Graph::new();
        for (key, content) in docs {
            graph.from_markdown(Key::name(key), content, MarkdownReader::new());
        }
        graph
    }

    #[test]
    fn empty_graph() {
        let graph = build_graph(&[]);
        let fields = infer_schema(&graph, &[]);
        assert!(fields.is_empty());
    }

    #[test]
    fn single_doc_flat_fields() {
        let graph = build_graph(&[(
            "doc1",
            "---\ntype: post\nstatus: draft\n---\n# Title\n",
        )]);
        let keys = vec![Key::name("doc1")];
        let fields = infer_schema(&graph, &keys);

        assert_eq!(fields.len(), 2);

        let type_field = fields.iter().find(|f| f.name == "type").unwrap();
        assert_eq!(type_field.coverage.count, 1);
        assert_eq!(type_field.coverage.percentage, 100.0);
        assert_eq!(type_field.types.len(), 1);
        assert_eq!(type_field.types[0].yaml_type, "string");
        assert_eq!(type_field.values.len(), 1);
        assert_eq!(type_field.values[0].value, "post");
    }

    #[test]
    fn partial_coverage() {
        let graph = build_graph(&[
            ("doc1", "---\ntype: post\nstatus: draft\n---\n# A\n"),
            ("doc2", "---\ntype: external\n---\n# B\n"),
        ]);
        let keys = vec![Key::name("doc1"), Key::name("doc2")];
        let fields = infer_schema(&graph, &keys);

        let type_field = fields.iter().find(|f| f.name == "type").unwrap();
        assert_eq!(type_field.coverage.count, 2);
        assert_eq!(type_field.coverage.percentage, 100.0);

        let status_field = fields.iter().find(|f| f.name == "status").unwrap();
        assert_eq!(status_field.coverage.count, 1);
        assert_eq!(status_field.coverage.percentage, 50.0);
    }

    #[test]
    fn polymorphic_types() {
        let graph = build_graph(&[
            ("doc1", "---\nurl: https://example.com\n---\n# A\n"),
            ("doc2", "---\nurl: null\n---\n# B\n"),
        ]);
        let keys = vec![Key::name("doc1"), Key::name("doc2")];
        let fields = infer_schema(&graph, &keys);

        let url_field = fields.iter().find(|f| f.name == "url").unwrap();
        assert_eq!(url_field.coverage.count, 2);
        assert_eq!(url_field.types.len(), 2);

        let string_type = url_field.types.iter().find(|t| t.yaml_type == "string").unwrap();
        assert_eq!(string_type.count, 1);
        let null_type = url_field.types.iter().find(|t| t.yaml_type == "null").unwrap();
        assert_eq!(null_type.count, 1);
    }

    #[test]
    fn nested_objects() {
        let graph = build_graph(&[(
            "doc1",
            "---\nengagement:\n  upvotes: 10\n  comments: 5\n---\n# A\n",
        )]);
        let keys = vec![Key::name("doc1")];
        let fields = infer_schema(&graph, &keys);

        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"engagement"));
        assert!(names.contains(&"engagement.upvotes"));
        assert!(names.contains(&"engagement.comments"));

        let engagement = fields.iter().find(|f| f.name == "engagement").unwrap();
        assert_eq!(engagement.types[0].yaml_type, "object");
    }

    #[test]
    fn date_detection() {
        let graph = build_graph(&[(
            "doc1",
            "---\ncreated: 2026-04-25\n---\n# A\n",
        )]);
        let keys = vec![Key::name("doc1")];
        let fields = infer_schema(&graph, &keys);

        let created = fields.iter().find(|f| f.name == "created").unwrap();
        assert_eq!(created.types[0].yaml_type, "date");
    }

    #[test]
    fn value_counting() {
        let graph = build_graph(&[
            ("doc1", "---\nstatus: draft\n---\n# A\n"),
            ("doc2", "---\nstatus: published\n---\n# B\n"),
            ("doc3", "---\nstatus: draft\n---\n# C\n"),
        ]);
        let keys = vec![Key::name("doc1"), Key::name("doc2"), Key::name("doc3")];
        let fields = infer_schema(&graph, &keys);

        let status = fields.iter().find(|f| f.name == "status").unwrap();
        assert_eq!(status.values.len(), 2);
        assert_eq!(status.values[0].value, "draft");
        assert_eq!(status.values[0].count, 2);
        assert_eq!(status.values[1].value, "published");
        assert_eq!(status.values[1].count, 1);
    }

    #[test]
    fn reserved_fields_skipped() {
        let graph = build_graph(&[(
            "doc1",
            "---\ntype: post\n_internal: secret\n---\n# A\n",
        )]);
        let keys = vec![Key::name("doc1")];
        let fields = infer_schema(&graph, &keys);

        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"type"));
        assert!(!names.contains(&"_internal"));
    }

    #[test]
    fn filtered_keys_subset() {
        let graph = build_graph(&[
            ("doc1", "---\ntype: post\nstatus: draft\n---\n# A\n"),
            ("doc2", "---\ntype: external\nurl: https://x.com\n---\n# B\n"),
        ]);
        let keys = vec![Key::name("doc1")];
        let fields = infer_schema(&graph, &keys);

        assert_eq!(fields.len(), 2);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"status"));
        assert!(!names.contains(&"url"));
    }
}
