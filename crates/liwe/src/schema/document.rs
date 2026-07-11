use serde_json::{Map, Value};

use crate::graph::basic_iter::GraphNodePointer;
use crate::graph::{Graph, GraphContext};
use crate::model::node::{Node, NodeIter, NodePointer};
use crate::model::{Key, NodeId};
use crate::query::frontmatter::is_reserved_segment;

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub frontmatter: Value,
    pub body_tokens: usize,
    pub blocks: Vec<Block>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub header: String,
    pub level: usize,
    pub header_tokens: usize,
    pub subtree_tokens: usize,
    pub blocks: Vec<Block>,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
    pub text_tokens: usize,
    pub subtree_tokens: usize,
    pub lang: Option<String>,
    pub items: Vec<Item>,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub text: String,
    pub text_tokens: usize,
    pub subtree_tokens: usize,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    BulletList,
    OrderedList,
    Code,
    Quote,
    Table,
    Rule,
}

pub fn build_document(graph: &Graph, key: &Key, count: impl Fn(&str) -> usize + Copy) -> Document {
    let frontmatter = graph
        .frontmatter(key)
        .map(yaml_mapping_to_object)
        .map(Value::Object)
        .unwrap_or_else(|| Value::Object(Map::new()));

    let body_tokens = count(&graph.to_markdown_skip_frontmatter(key));

    let first_child = graph
        .maybe_key(key)
        .and_then(|document| document.child_id());
    let blocks = build_blocks(graph, key, first_child, count);

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
        blocks,
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

    let subtree_tokens = count(&subtree_text(graph, key, id));

    let blocks = build_blocks(
        graph,
        key,
        GraphNodePointer::new(graph, id).child_id(),
        count,
    );

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
        blocks,
        sections,
    }
}

fn build_blocks(
    graph: &Graph,
    key: &Key,
    first_child: Option<NodeId>,
    count: impl Fn(&str) -> usize + Copy,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut cursor = first_child;
    while let Some(id) = cursor {
        let pointer = GraphNodePointer::new(graph, id);
        if pointer.is_section() {
            break;
        }
        blocks.push(build_block(graph, key, id, count));
        cursor = pointer.next_id();
    }
    blocks
}

fn build_block(
    graph: &Graph,
    key: &Key,
    id: NodeId,
    count: impl Fn(&str) -> usize + Copy,
) -> Block {
    let pointer = GraphNodePointer::new(graph, id);
    let node = pointer.node().expect("block node to exist");

    let text = node.plain_text();
    let text_tokens = count(&text);
    let subtree_tokens = count(&subtree_text(graph, key, id));

    let kind = block_kind(&node);

    let lang = match &node {
        Node::Raw(lang, _) => lang.clone(),
        _ => None,
    };

    let items = match kind {
        BlockKind::BulletList | BlockKind::OrderedList => build_items(graph, key, id, count),
        _ => Vec::new(),
    };
    let blocks = match kind {
        BlockKind::Quote => build_blocks(graph, key, pointer.child_id(), count),
        _ => Vec::new(),
    };

    Block {
        kind,
        text,
        text_tokens,
        subtree_tokens,
        lang,
        items,
        blocks,
    }
}

fn build_items(
    graph: &Graph,
    key: &Key,
    list_id: NodeId,
    count: impl Fn(&str) -> usize + Copy,
) -> Vec<Item> {
    let mut items = Vec::new();
    let mut cursor = GraphNodePointer::new(graph, list_id).child_id();
    while let Some(id) = cursor {
        let pointer = GraphNodePointer::new(graph, id);
        let text = pointer
            .node()
            .map(|node| node.plain_text())
            .unwrap_or_default();
        let text_tokens = count(&text);
        let subtree_tokens = count(&subtree_text(graph, key, id));
        let blocks = build_blocks(graph, key, pointer.child_id(), count);
        items.push(Item {
            text,
            text_tokens,
            subtree_tokens,
            blocks,
        });
        cursor = pointer.next_id();
    }
    items
}

fn block_kind(node: &Node) -> BlockKind {
    match node {
        Node::Leaf(_) => BlockKind::Paragraph,
        Node::BulletList() => BlockKind::BulletList,
        Node::OrderedList() => BlockKind::OrderedList,
        Node::Raw(_, _) => BlockKind::Code,
        Node::Quote() => BlockKind::Quote,
        Node::Table(_) => BlockKind::Table,
        Node::Reference(_) => BlockKind::Paragraph,
        Node::HorizontalRule() => BlockKind::Rule,
        Node::Document(_, _) | Node::Section(_) | Node::Item(_, _) => BlockKind::Paragraph,
    }
}

fn subtree_text(graph: &Graph, key: &Key, id: NodeId) -> String {
    GraphNodePointer::new(graph, id)
        .collect_tree()
        .iter()
        .to_text(&key.parent(), graph.format_options())
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
                blocks: vec![],
                sections: vec![
                    Section {
                        header: "Summary".to_string(),
                        level: 1,
                        header_tokens: 0,
                        subtree_tokens: 0,
                        blocks: vec![Block {
                            kind: BlockKind::Paragraph,
                            text: "text".to_string(),
                            text_tokens: 0,
                            subtree_tokens: 0,
                            lang: None,
                            items: vec![],
                            blocks: vec![],
                        }],
                        sections: vec![Section {
                            header: "Details".to_string(),
                            level: 2,
                            header_tokens: 0,
                            subtree_tokens: 0,
                            blocks: vec![],
                            sections: vec![],
                        }],
                    },
                    Section {
                        header: "Tasks".to_string(),
                        level: 1,
                        header_tokens: 0,
                        subtree_tokens: 0,
                        blocks: vec![],
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

    fn block(kind: BlockKind, text: &str) -> Block {
        Block {
            kind,
            text: text.to_string(),
            text_tokens: 0,
            subtree_tokens: 0,
            lang: None,
            items: vec![],
            blocks: vec![],
        }
    }

    fn item(text: &str, blocks: Vec<Block>) -> Item {
        Item {
            text: text.to_string(),
            text_tokens: 0,
            subtree_tokens: 0,
            blocks,
        }
    }

    fn section(header: &str, level: usize, blocks: Vec<Block>, sections: Vec<Section>) -> Section {
        Section {
            header: header.to_string(),
            level,
            header_tokens: 0,
            subtree_tokens: 0,
            blocks,
            sections,
        }
    }

    #[test]
    fn extracts_every_block_kind_and_splits_subsections() {
        let graph = graph_from(
            "# Section\n\npara one\n\n- a\n- b\n\n```rust\nfn x() {}\n```\n\n> quoted\n\n| H |\n| - |\n| c |\n\n[link](other)\n\n---\n\n## Sub\n\nsub para\n",
        );
        let document = build_document(&graph, &Key::name("doc"), |_| 0);
        assert_eq!(
            document,
            Document {
                frontmatter: json!({}),
                body_tokens: 0,
                blocks: vec![],
                sections: vec![section(
                    "Section",
                    1,
                    vec![
                        block(BlockKind::Paragraph, "para one"),
                        Block {
                            items: vec![item("a", vec![]), item("b", vec![])],
                            ..block(BlockKind::BulletList, "")
                        },
                        Block {
                            lang: Some("rust".to_string()),
                            ..block(BlockKind::Code, "fn x() {}\n")
                        },
                        Block {
                            blocks: vec![block(BlockKind::Paragraph, "quoted")],
                            ..block(BlockKind::Quote, "")
                        },
                        block(BlockKind::Table, "H c"),
                        block(BlockKind::Paragraph, "link"),
                        block(BlockKind::Rule, ""),
                    ],
                    vec![section(
                        "Sub",
                        2,
                        vec![block(BlockKind::Paragraph, "sub para")],
                        vec![]
                    )],
                )],
            }
        );
    }

    #[test]
    fn extracts_document_blocks_above_the_first_heading() {
        let graph = graph_from("lead\n\n- a\n\n# Section\n");
        let document = build_document(&graph, &Key::name("doc"), |_| 0);
        assert_eq!(
            document.blocks,
            vec![
                block(BlockKind::Paragraph, "lead"),
                Block {
                    items: vec![item("a", vec![])],
                    ..block(BlockKind::BulletList, "")
                },
            ]
        );
        assert_eq!(
            document.sections,
            vec![section("Section", 1, vec![], vec![])]
        );
    }

    #[test]
    fn extracts_nested_items_and_counts_subtree_tokens() {
        let graph = graph_from(
            "# Section\n\nlead para\n\n- outer one\n\n    nested para\n\n    - inner one\n    - inner two\n- outer two\n",
        );
        let count = |text: &str| text.split_whitespace().count();
        let document = build_document(&graph, &Key::name("doc"), count);
        assert_eq!(
            document,
            Document {
                frontmatter: json!({}),
                body_tokens: 18,
                blocks: vec![],
                sections: vec![Section {
                    header: "Section".to_string(),
                    level: 1,
                    header_tokens: 1,
                    subtree_tokens: 18,
                    blocks: vec![
                        Block {
                            text_tokens: 2,
                            subtree_tokens: 2,
                            ..block(BlockKind::Paragraph, "lead para")
                        },
                        Block {
                            subtree_tokens: 14,
                            items: vec![
                                Item {
                                    text: "outer one".to_string(),
                                    text_tokens: 2,
                                    subtree_tokens: 10,
                                    blocks: vec![
                                        Block {
                                            text_tokens: 2,
                                            subtree_tokens: 2,
                                            ..block(BlockKind::Paragraph, "nested para")
                                        },
                                        Block {
                                            subtree_tokens: 6,
                                            items: vec![
                                                Item {
                                                    text: "inner one".to_string(),
                                                    text_tokens: 2,
                                                    subtree_tokens: 2,
                                                    blocks: vec![],
                                                },
                                                Item {
                                                    text: "inner two".to_string(),
                                                    text_tokens: 2,
                                                    subtree_tokens: 2,
                                                    blocks: vec![],
                                                },
                                            ],
                                            ..block(BlockKind::BulletList, "")
                                        },
                                    ],
                                },
                                Item {
                                    text: "outer two".to_string(),
                                    text_tokens: 2,
                                    subtree_tokens: 2,
                                    blocks: vec![],
                                },
                            ],
                            ..block(BlockKind::BulletList, "")
                        },
                    ],
                    sections: vec![],
                }],
            }
        );
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
