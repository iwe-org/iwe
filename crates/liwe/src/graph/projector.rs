use crate::graph::graph_node::GraphNode;
use crate::model::NodeId;

use crate::graph::Graph;
use crate::model::graph::{Block, Inline, ReferenceType};

pub struct Projector<'a> {
    id: NodeId,
    graph: &'a Graph,
    header_level: usize,
    list_level: usize,
}

impl<'a> Projector<'a> {
    pub fn new(
        graph: &'a Graph,
        id: NodeId,
        header_level: usize,
        list_level: usize,
    ) -> Projector<'a> {
        Projector {
            id,
            graph,
            header_level,
            list_level,
        }
    }

    pub fn project(&self) -> Vec<Block> {
        if self.node().is_root() {
            self.project_root()
        } else if self.node().is_rule() {
            self.project_rule()
        } else if self.node().is_quote() {
            self.project_quote()
        } else if self.node().is_list() {
            self.project_list()
        } else if self.node().is_ref() {
            self.project_ref()
        } else if !self.node().is_leaf() {
            self.project_section()
        } else if self.node().is_leaf() {
            self.project_leaf()
        } else {
            vec![]
        }
    }

    fn project_root(&self) -> Vec<Block> {
        let mut blocks = vec![];

        self.child()
            .map(|child| Projector::new(self.graph, child.id(), 0, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn project_section(&self) -> Vec<Block> {
        let mut blocks = vec![];
        blocks.push(Block::Header(
            self.header_level as u8 + 1,
            self.project_line(),
        ));

        self.child()
            .map(|child| Projector::new(self.graph, child.id(), self.header_level + 1, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn project_list_item(&self) -> Vec<Vec<Block>> {
        let mut items: Vec<Vec<Block>> = vec![];

        if self.child().map(|n| n.is_leaf()).unwrap_or(false) {
            items.push(vec![Block::Para(self.project_line())]);
        } else {
            items.push(vec![Block::Plain(self.project_line())]);
        }

        self.child()
            .map(|child| Projector::new(self.graph, child.id(), 0, self.list_level + 1).project())
            .map(|sub_list| {
                sub_list
                    .iter()
                    .for_each(|item| items.last_mut().unwrap().push(item.clone()))
            });

        self.next()
            .map(|next| {
                Projector::new(self.graph, next.id(), self.header_level, 0).project_list_item()
            })
            .map(|blocks| items.append(blocks.clone().as_mut()));

        items
    }

    fn project_list(&self) -> Vec<Block> {
        let mut blocks = vec![];
        let mut items = vec![];

        self.child()
            .map(|child| {
                Projector::new(self.graph, child.id(), 0, self.list_level + 1).project_list_item()
            })
            .map(|item| {
                let mut cloned = item.clone();
                items.append(&mut cloned);
            });

        if self.node().is_ordered() {
            blocks.push(Block::OrderedList(items));
        } else {
            blocks.push(Block::BulletList(items));
        }

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn project_quote(&self) -> Vec<Block> {
        let mut blocks = vec![];
        let mut items = vec![];

        self.child()
            .map(|child| Projector::new(self.graph, child.id(), 0, 0).project())
            .map(|item| items.append(item.clone().as_mut()));

        blocks.push(Block::BlockQuote(items));

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn project_line(&self) -> Vec<Inline> {
        self.node()
            .line_id()
            .map(|id| self.graph.get_line(id))
            .map(|line| line.inlines().clone())
            .unwrap_or_default()
    }

    fn project_leaf(&self) -> Vec<Block> {
        let mut blocks = vec![];

        if self.node().is_raw_leaf() {
            blocks.push(Block::CodeBlock(
                self.node().lang(),
                self.node().content().unwrap_or_default(),
            ));
        } else {
            blocks.push(Block::Para(self.project_line()));
        }

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn node(&self) -> GraphNode {
        self.graph.graph_node(self.id)
    }

    fn child(&self) -> Option<GraphNode> {
        self.node().child_id().map(|id| self.graph.graph_node(id))
    }

    fn next(&self) -> Option<GraphNode> {
        self.node().next_id().map(|id| self.graph.graph_node(id))
    }

    fn project_ref(&self) -> Vec<Block> {
        let mut blocks: Vec<Block> = vec![];

        let inlines = match self.ref_type() {
            ReferenceType::Regular => vec![Inline::Str(
                self.graph
                    .get_key_title(&self.ref_key())
                    .filter(|title| !title.is_empty())
                    .unwrap_or(self.ref_text()),
            )],
            ReferenceType::WikiLink => vec![],
            ReferenceType::WikiLinkPiped => vec![Inline::Str(self.ref_text())],
        };

        let link = Inline::Link(
            self.ref_key(),
            String::default(),
            self.ref_type().to_link_type(),
            inlines,
        );

        blocks.push(Block::Para(vec![link]));

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }

    fn ref_key(&self) -> String {
        self.node().ref_key().unwrap()
    }

    fn ref_type(&self) -> ReferenceType {
        self.node().ref_type().unwrap()
    }

    fn ref_text(&self) -> String {
        self.node().ref_text()
    }

    fn project_rule(&self) -> Vec<Block> {
        let mut blocks = vec![];

        blocks.push(Block::HorizontalRule);

        self.next()
            .map(|next| Projector::new(self.graph, next.id(), self.header_level, 0).project())
            .unwrap_or_default()
            .iter()
            .for_each(|block| blocks.push(block.clone()));

        blocks
    }
}
