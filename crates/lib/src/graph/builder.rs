use crate::model::graph::{Node, Inline};
use super::*;

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

    pub fn quote_and<F>(&mut self, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_quote(self.id, new_id), f);
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
        let line_id = self.graph.add_line(Inline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node_and(
            GraphNode::new_section(self.id, new_id, line_id),
            |builder| {},
        );
        self
    }

    pub fn section_text_and<F>(&mut self, text: &str, f: F) -> &mut Self
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let line_id = self.graph.add_line(Inline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_section(self.id, new_id, line_id), f);
        self
    }

    pub fn section(&mut self, inlines: Inlines) {
        self.section_and(inlines, |builder| {})
    }

    pub fn section_and<F>(&mut self, inlines: Inlines, f: F)
    where
        F: FnOnce(&mut GraphBuilder) -> (),
    {
        let line_id = self.graph.add_line(inlines);
        let new_id = self.graph.new_node_id();
        self.add_node_and(GraphNode::new_section(self.id, new_id, line_id), f);
    }

    pub fn leaf_text(&mut self, text: &str) -> &mut Self {
        let line_id = self.graph.add_line(Inline::from_string(text));
        let new_id = self.graph.new_node_id();
        self.add_node(GraphNode::new_leaf(self.id, new_id, line_id));
        self
    }

    pub fn leaf(&mut self, block: Inlines) {
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

    pub fn reference(&mut self, key: &str) {
        let new_id = self.graph().new_node_id();
        self.add_node(GraphNode::new_ref(
            self.id,
            new_id,
            key.to_string(),
            "".to_string(),
        ));
    }

    pub fn reference_with_title(&mut self, key: &str, title: &str) {
        let new_id = self.graph().new_node_id();
        self.add_node(GraphNode::new_ref(
            self.id,
            new_id,
            key.to_string(),
            title.to_string(),
        ));
    }

    fn add_node(&mut self, node: GraphNode) {
        self.add_node_and(node, |builder| {});
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
            Node::Reference(key, title) => {
                let new_id = self.graph().new_node_id();
                self.add_node_and2(
                    GraphNode::new_ref(self.id, new_id, key.to_string(), title.to_string()),
                    f,
                );
            }
        }
    }

    fn append_from_visitor<'b>(&mut self, visitor: impl NodeIter<'b>) {
        self.insert = false;
        visitor.node().map(|node| {
            self.add_new_node_and(node, |builder| {
                visitor.child().map(|child| {
                    builder.insert_from_iter(child);
                });
                visitor.next().map(|next| {
                    builder.append_from_visitor(next);
                });
            });
        });
    }

    pub fn insert_from_iter<'b>(&mut self, visitor: impl NodeIter<'b>) {
        self.insert = true;
        visitor.node().map(|node| {
            self.add_new_node_and(node, |builder| {
                visitor.child().map(|child| {
                    builder.insert_from_iter(child);
                });
                visitor.next().map(|next| {
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
    use super::Graph;
    use crate::markdown::MarkdownReader;
    use indoc::indoc;
    use crate::model::graph::{Node, Inline};

    #[test]
    pub fn add_new_node_leaf() {
        assert_eq(
            Graph::with(|graph| {
                graph.build_key("key").add_new_node_and(
                    Node::Leaf(vec![Inline::Str("item".to_string())]),
                    |f| {},
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
                    .build_key("key")
                    .add_new_node_and(Node::BulletList(), |f| {
                        f.add_new_node_and(
                            Node::Section(vec![Inline::Str("item".to_string())]),
                            |f| {},
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
                    .build_key("key")
                    .add_new_node_and(Node::BulletList(), |list| {
                        list.add_new_node_and(
                            Node::Section(vec![Inline::Str("item".to_string())]),
                            |section| {
                                section.add_new_node_and(Node::BulletList(), |list| {
                                    list.add_new_node_and(
                                        Node::Section(vec![Inline::Str("item2".to_string())]),
                                        |f| {},
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
        actual_graph.from_markdown("key", actual, MarkdownReader::new());

        assert_eq!(expected, actual_graph);
    }
}
