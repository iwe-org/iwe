use serde_json::{Map, Value};

use crate::graph::basic_iter::GraphNodePointer;
use crate::graph::{Graph, GraphContext};
use crate::model::node::{NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::query::frontmatter::is_reserved_segment;

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub frontmatter: Value,
    pub body_tokens: usize,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub header: String,
    pub level: usize,
    pub header_tokens: usize,
    pub subtree_tokens: usize,
    pub sections: Vec<Section>,
}

pub fn build_document(graph: &Graph, key: &Key, count: impl Fn(&str) -> usize + Copy) -> Document {
    let frontmatter = graph
        .frontmatter(key)
        .map(yaml_mapping_to_object)
        .map(Value::Object)
        .unwrap_or_else(|| Value::Object(Map::new()));

    let body_tokens = count(&graph.to_markdown_skip_frontmatter(key));

    let sections = graph
        .maybe_key(key)
        .and_then(|document| document.to_child())
        .map(|child| child.get_next_sections())
        .unwrap_or_default()
        .into_iter()
        .map(|id| build_section(graph, key, id, 1, count))
        .collect();

    Document {
        frontmatter,
        body_tokens,
        sections,
    }
}

fn build_section(
    graph: &Graph,
    key: &Key,
    id: NodeId,
    level: usize,
    count: impl Fn(&str) -> usize + Copy,
) -> Section {
    let header = graph.get_text(id);
    let header_tokens = count(&header);

    let subtree = GraphNodePointer::new(graph, id)
        .collect_tree()
        .iter()
        .to_text(&key.parent(), graph.format_options());
    let subtree_tokens = count(&subtree);

    let sections = GraphNodePointer::new(graph, id)
        .get_sub_sections()
        .into_iter()
        .map(|child| build_section(graph, key, child, level + 1, count))
        .collect();

    Section {
        header,
        level,
        header_tokens,
        subtree_tokens,
        sections,
    }
}

fn yaml_mapping_to_object(mapping: &serde_yaml::Mapping) -> Map<String, Value> {
    let mut object = Map::new();
    for (key, value) in mapping {
        if let Some(name) = key.as_str() {
            if is_reserved_segment(name) {
                continue;
            }
            object.insert(name.to_string(), yaml_to_json(value));
        }
    }
    object
}

fn yaml_to_json(value: &serde_yaml::Value) -> Value {
    match value {
        serde_yaml::Value::Null => Value::Null,
        serde_yaml::Value::Bool(boolean) => Value::Bool(*boolean),
        serde_yaml::Value::Number(number) => yaml_number_to_json(number),
        serde_yaml::Value::String(text) => Value::String(text.clone()),
        serde_yaml::Value::Sequence(items) => {
            Value::Array(items.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(nested) => Value::Object(yaml_mapping_to_object(nested)),
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

fn yaml_number_to_json(number: &serde_yaml::Number) -> Value {
    if let Some(integer) = number.as_i64() {
        Value::Number(integer.into())
    } else if let Some(unsigned) = number.as_u64() {
        Value::Number(unsigned.into())
    } else if let Some(float) = number.as_f64() {
        serde_json::Number::from_f64(float)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::markdown::MarkdownReader;

    fn graph_from(content: &str) -> Graph {
        let mut graph = Graph::new();
        graph.from_markdown(Key::name("doc"), content, MarkdownReader::new());
        graph
    }

    #[test]
    fn builds_section_tree_with_levels_and_strips_reserved_frontmatter() {
        let graph = graph_from(
            "---\nstatus: draft\n_internal: secret\n---\n# Summary\n\ntext\n\n## Details\n\n# Tasks\n",
        );
        let document = build_document(&graph, &Key::name("doc"), |_| 0);
        assert_eq!(
            document,
            Document {
                frontmatter: json!({ "status": "draft" }),
                body_tokens: 0,
                sections: vec![
                    Section {
                        header: "Summary".to_string(),
                        level: 1,
                        header_tokens: 0,
                        subtree_tokens: 0,
                        sections: vec![Section {
                            header: "Details".to_string(),
                            level: 2,
                            header_tokens: 0,
                            subtree_tokens: 0,
                            sections: vec![],
                        }],
                    },
                    Section {
                        header: "Tasks".to_string(),
                        level: 1,
                        header_tokens: 0,
                        subtree_tokens: 0,
                        sections: vec![],
                    },
                ],
            }
        );
    }

    #[test]
    fn absent_frontmatter_is_empty_object() {
        let graph = graph_from("# Title\n");
        let document = build_document(&graph, &Key::name("doc"), |_| 0);
        assert_eq!(document.frontmatter, json!({}));
    }

    #[test]
    fn counts_header_tokens_with_injected_counter() {
        let graph = graph_from("# Two Words\n\nbody text here\n");
        let count = |text: &str| text.split_whitespace().count();
        let document = build_document(&graph, &Key::name("doc"), count);
        assert_eq!(document.sections[0].header, "Two Words");
        assert_eq!(document.sections[0].header_tokens, 2);
    }

    #[test]
    fn validates_a_real_document_against_a_schema() {
        use crate::schema::compile::compile_schema;
        use crate::schema::violation::{Crumb, Violation};

        let schema = "\
sections:
  - header: { const: Summary }
  - header: { const: Tasks }
additionalSections: false
";
        let graph = graph_from("# Summary\n\n# Extra\n");
        let document = build_document(&graph, &Key::name("doc"), |_| 0);
        let violations = compile_schema(schema).unwrap().validate(&document);
        assert_eq!(
            violations,
            vec![
                Violation {
                    breadcrumb: vec![],
                    message: "required section 'Tasks' missing".to_string(),
                    hint: None,
                    schema_pointer: "/sections/1/minContains".to_string(),
                    keyword: "minContains".to_string(),
                },
                Violation {
                    breadcrumb: vec![Crumb::Header("Extra".to_string())],
                    message: "unexpected section".to_string(),
                    hint: None,
                    schema_pointer: "/additionalSections".to_string(),
                    keyword: "additionalSections".to_string(),
                },
            ]
        );
    }
}
