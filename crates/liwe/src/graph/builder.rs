use super::*;
use crate::model::graph::GraphInline;
use crate::model::node::{ColumnAlignment, Node, Reference, ReferenceType};

pub struct GraphBuilder<'a> {
    id: NodeId,
    graph: &'a mut Graph,
    insert: bool,
}

impl<'a> GraphBuilder<'a> {
    pub fn node(&self) -> GraphNode {
        self.graph.graph_node(self.id)
    }

    pub fn insert(&self) -> bool {
        self.insert
    }

    pub fn new(graph: &'a mut Graph, id: NodeId) -> GraphBuilder<'a> {
        GraphBuilder {
            id,
            graph,
            insert: true,
        }
    }

    pub fn to_parent(&mut self) -> &mut Self {
        let id = self.id;
        self.id = self.node().prev_id().unwrap();

        if self.node().is_parent_of(id) {
            self
        } else {
            self.to_parent()
        }
    }

    pub fn prev_is_root(&self) -> bool {
        self.node()
            .prev_id()
            .map_or(false, |id| self.graph.graph_node(id).is_root())
    }

    pub fn set_insert(&mut self, insert: bool) -> &mut Self {
        self.insert = insert;
        self
    }

    pub fn graph(&mut self) -> &mut Graph {
        self.graph
    }

    pub fn quote(&mut self) {
        self.quote_and(|_| {})
    }

    pub fn table(
        &mut self,
        header: Vec<LineId>,
        alignment: Vec<ColumnAlignment>,
        rows: Vec<Vec<LineId>>,
    ) {
        self.table_and(header, alignment, rows, |_| {});
    }

    pub fn quote_and<F>(&mut self, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_quote(self.id, new_id), f);
    }

    pub fn table_and<F>(
        &mut self,
        header: Vec<LineId>,
        alignment: Vec<ColumnAlignment>,
        rows: Vec<Vec<LineId>>,
        f: F,
    ) where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let new_id = self.graph.new_node_id();
        self.add_node_and(
            GraphNode::new_table(self.id, new_id, header, alignment, rows),
            f,
        );
    }

    pub fn horizontal_rule(&mut self) {
        let new_id = self.graph.new_node_id();
        self.add_node(GraphNode::new_rule(self.id, new_id));
    }

    pub fn bullet_list(&mut self) {
        self.bullet_list_and(|_| {});
    }

    pub fn ordered_list(&mut self) {
        self.ordered_list_and(|_| {});
    }

    pub fn bullet_list_and<F>(&mut self, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_bullet_list(self.id, new_id), f);
    }

    pub fn ordered_list_and<F>(&mut self, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_ordered_list(self.id, new_id), f);
    }

    pub fn section_text(&mut self, text: &str) -> &mut Self {
        let line_id = self.graph.add_line(GraphInline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_section(self.id, new_id, line_id), |_| {});
        self
    }

    pub fn section_text_and<F>(&mut self, text: &str, f: F) -> &mut Self
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let line_id = self.graph.add_line(GraphInline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_section(self.id, new_id, line_id), f);
        self
    }

    pub fn section(&mut self, inlines: GraphInlines) {
        self.section_and(inlines, |_| {})
    }

    pub fn section_and<F>(&mut self, inlines: GraphInlines, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let line_id = self.graph.add_line(inlines);
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_section(self.id, new_id, line_id), f);
    }

    pub fn leaf_text(&mut self, text: &str) -> &mut Self {
        let line_id = self.graph.add_line(GraphInline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node(GraphNode::new_leaf(self.id, new_id, line_id));
        self
    }

    pub fn leaf(&mut self, block: GraphInlines) {
        let line_id = self.graph.add_line(block);
        let new_id = self.graph.new_node_id();
        self.add_node(GraphNode::new_leaf(self.id, new_id, line_id));
    }

    pub fn raw(&mut self, block: &str, lang: Option<String>) {
        let new_id = self.graph.new_node_id();
        self.add_node(GraphNode::new_raw_leaf(
            self.id,
            new_id,
            block.to_string(),
            lang,
        ));
    }

    pub fn reference(&mut self, key: &Key) {
        let new_id = self.graph().new_node_id();
        self.add_node(GraphNode::new_ref(
            self.id,
            new_id,
            key.clone(),
            String::default(),
            ReferenceType::Regular,
        ));
    }

    pub fn reference_with_text(&mut self, key: &Key, text: &str, reference_type: ReferenceType) {
        let new_id = self.graph().new_node_id();
        self.add_node(GraphNode::new_ref(
            self.id,
            new_id,
            key.clone(),
            text.to_string(),
            reference_type,
        ));
    }

    fn add_node(&mut self, node: GraphNode) {
        self.add_node_and(node, |_| {});
    }

    fn add_node_and<F>(&mut self, node: GraphNode, f: F)
    where
        F: FnOnce(&mut GraphBuilder<'_>) -> (),
    {
        if self.insert {
            self.graph.node_mut(self.id).set_child_id(node.id());
            self.insert = false;
        } else {
            self.graph.node_mut(self.id).set_next_id(node.id());
        }

        self.id = node.id();
        self.graph.add_graph_node(node.clone());

        f(&mut GraphBuilder {
            id: self.id,
            graph: self.graph,
            insert: node.insertable(),
        });
    }

    fn add_node_and2<F>(&mut self, node: GraphNode, f: F)
    where
        F: FnOnce(&mut GraphBuilder<'_>) -> (),
    {
        if self.insert {
            self.graph.node_mut(self.id).set_child_id(node.id());
            self.insert = false;
        } else {
            self.graph.node_mut(self.id).set_next_id(node.id());
        }

        self.graph.add_graph_node(node.clone());

        f(&mut GraphBuilder {
            id: node.id(),
            graph: self.graph,
            insert: node.insertable(),
        });
    }

    fn add_new_node_and<F>(&mut self, node: Node, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        match node {
            Node::Document(_) => panic!("Document node is not allowed"),
            Node::Section(inlines) => {
                let line_id = self.graph.add_line(inlines);
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_section(self.id, new_id, line_id), f);
            }
            Node::Quote() => {
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_quote(self.id, new_id), f);
            }
            Node::BulletList() => {
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_bullet_list(self.id, new_id), f);
            }
            Node::OrderedList() => {
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_ordered_list(self.id, new_id), f);
            }
            Node::Leaf(inlines) => {
                let line_id = self.graph.add_line(inlines);
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_leaf(self.id, new_id, line_id), f);
            }
            Node::Raw(lang, content) => {
                let new_id = self.graph.new_node_id();
                self.add_node_and2(
                    GraphNode::new_raw_leaf(self.id, new_id, content.to_string(), lang),
                    f,
                );
            }
            Node::HorizontalRule() => {
                let new_id = self.graph.new_node_id();
                self.add_node_and2(GraphNode::new_rule(self.id, new_id), f);
            }
            Node::Reference(Reference {
                key,
                text: title,
                reference_type,
            }) => {
                let new_id = self.graph().new_node_id();
                self.add_node_and2(
                    GraphNode::new_ref(
                        self.id,
                        new_id,
                        key.clone(),
                        title.to_string(),
                        reference_type,
                    ),
                    f,
                );
            }
            Node::Table(table) => {
                let new_id = self.graph().new_node_id();

                let header_line_ids = table
                    .header
                    .iter()
                    .map(|inlines| self.graph.add_line(inlines.clone()))
                    .collect();

                let rows = table
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|inlines| self.graph.add_line(inlines.clone()))
                            .collect()
                    })
                    .collect();

                self.add_node_and2(
                    GraphNode::new_table(
                        self.id,
                        new_id,
                        header_line_ids,
                        table.alignment.clone(),
                        rows,
                    ),
                    f,
                );
            }
        }
    }

    fn append_from_visitor<'b>(&mut self, iter: impl NodeIter<'b>) {
        self.insert = false;

        if iter.is_document() {
            self.append_from_visitor(iter.child().unwrap());
            return;
        }

        iter.node().map(|node| {
            self.add_new_node_and(node, |builder| {
                iter.child().map(|child| {
                    builder.insert_from_iter(child);
                });
                iter.next().map(|next| {
                    builder.append_from_visitor(next);
                });
            });
        });
    }

    pub fn insert_from_iter<'b>(&mut self, iter: impl NodeIter<'b>) {
        self.insert = true;

        if iter.is_document() {
            self.insert_from_iter(iter.child().unwrap());
            return;
        }

        iter.node().map(|node| {
            self.add_new_node_and(node, |builder| {
                iter.child().map(|child| {
                    builder.insert_from_iter(child);
                });
                iter.next().map(|next| {
                    builder.append_from_visitor(next);
                });
            });
        });
    }

    pub fn link_node_id(&mut self, node_id: NodeId) {
        if self.insert {
            self.graph.node_mut(self.id).set_child_id(node_id);
            self.insert = false;
        } else {
            self.graph.node_mut(self.id).set_next_id(node_id);
        }

        self.id = node_id;
    }

    pub fn id(&self) -> NodeId {
        self.id
    }

    pub fn set_id(&mut self, id: NodeId) {
        self.id = id;
    }
}

#[cfg(test)]
mod test {
    use super::{Graph, GraphNodePointer, Tree};
    use crate::markdown::MarkdownReader;
    use crate::model::graph::GraphInline;
    use crate::model::node::{Node, NodePointer};
    use indoc::indoc;

    #[test]
    pub fn simple_tree() {
        let graph = Graph::with(|graph| {
            graph.build_key(&"key".into()).add_new_node_and(
                Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                |_| {},
            )
        });

        let visitor = GraphNodePointer::new(&graph, graph.get_document_id(&"key".into()));

        assert_eq!(
            Tree {
                id: Some(0),
                node: Node::Document("key".into()),
                children: vec![Tree {
                    id: Some(1),
                    node: Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                    children: vec![]
                }]
            },
            visitor.collect_tree()
        )
    }

    #[test]
    pub fn nested_tree() {
        let graph = Graph::with(|graph| {
            graph.build_key(&"key".into()).add_new_node_and(
                Node::Section(vec![GraphInline::Str("item".to_string())]),
                |f| {
                    f.add_new_node_and(
                        Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                        |_| {},
                    );
                },
            )
        });

        let visitor = GraphNodePointer::new(&graph, graph.get_document_id(&"key".into()));

        assert_eq!(
            Tree {
                id: Some(0),
                node: Node::Document("key".into()),
                children: vec![Tree {
                    id: Some(1),
                    node: Node::Section(vec![GraphInline::Str("item".to_string())]),
                    children: vec![Tree {
                        id: Some(2),
                        node: Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                        children: vec![]
                    }]
                }]
            },
            visitor.collect_tree()
        )
    }

    #[test]
    pub fn graph_form_tree() {
        let tree = Tree {
            id: Some(0),
            node: Node::Document("key".into()),
            children: vec![Tree {
                id: Some(1),
                node: Node::Section(vec![GraphInline::Str("section".to_string())]),
                children: vec![Tree {
                    id: Some(2),
                    node: Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                    children: vec![],
                }],
            }],
        };

        let mut graph = Graph::new();

        graph.build_key_from_iter(&"key".into(), tree.iter());

        assert_eq(
            graph,
            indoc! { "
                # section

                item
                "},
        );
    }

    #[test]
    pub fn add_new_node_leaf() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key(&"key".into()).add_new_node_and(
                    Node::Leaf(vec![GraphInline::Str("item".to_string())]),
                    |_| {},
                )
            }),
            indoc! {"
            item
            "},
        )
    }

    #[test]
    pub fn add_new_node_one_list() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .add_new_node_and(Node::BulletList(), |f| {
                        f.add_new_node_and(
                            Node::Section(vec![GraphInline::Str("item".to_string())]),
                            |_| {},
                        )
                    })
            }),
            indoc! {"
            - item
            "},
        )
    }

    #[test]
    pub fn add_new_node_list_list() {
        assert_eq(
            Graph::with(|graph| {
                graph
                    .build_key(&"key".into())
                    .add_new_node_and(Node::BulletList(), |list| {
                        list.add_new_node_and(
                            Node::Section(vec![GraphInline::Str("item".to_string())]),
                            |section| {
                                section.add_new_node_and(Node::BulletList(), |list| {
                                    list.add_new_node_and(
                                        Node::Section(vec![GraphInline::Str("item2".to_string())]),
                                        |_| {},
                                    );
                                });
                            },
                        )
                    })
            }),
            indoc! {"
            - item
              - item2
            "},
        )
    }

    fn assert_eq(expected: Graph, actual: &str) {
        let mut actual_graph = Graph::new();
        actual_graph.from_markdown("key".into(), actual, MarkdownReader::new());

        assert_eq!(expected, actual_graph);
    }
}
