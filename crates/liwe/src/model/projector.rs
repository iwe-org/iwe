use crate::model::graph::{GraphBlock, GraphInline};
use crate::model::node::{Node, NodeIter, ReferenceType};

pub struct Projector {
    header_level: usize,
    parent: String,
}

impl Projector {
    pub fn project<'a>(iter: impl NodeIter<'a>, parent: &str) -> Vec<GraphBlock> {
        Projector {
            header_level: 0,
            parent: parent.to_string(),
        }
        .project_node(iter)
    }

    fn with(&self, header_level: usize) -> Projector {
        Projector {
            header_level,
            parent: self.parent.clone(),
        }
    }

    fn project_node<'a>(&self, iter: impl NodeIter<'a>) -> Vec<GraphBlock> {
        let mut blocks = vec![];

        if iter.node().is_none() {
            return blocks;
        }

        match iter.node().unwrap() {
            Node::Document(_) => {
                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level).project_node(child));
                }
            }
            Node::Section(_) => {
                blocks.push(GraphBlock::Header(
                    self.header_level as u8 + 1,
                    iter.inlines(),
                ));

                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level + 1).project_node(child));
                }
            }
            Node::Quote() => {
                if let Some(child) = iter.child() {
                    blocks.push(GraphBlock::BlockQuote(self.with(0).project_node(child)));
                }
            }
            Node::BulletList() => {
                if let Some(child) = iter.child() {
                    blocks.push(GraphBlock::BulletList(
                        self.with(0).project_list_item(child),
                    ));
                }
            }
            Node::OrderedList() => {
                if let Some(child) = iter.child() {
                    blocks.push(GraphBlock::OrderedList(
                        self.with(0).project_list_item(child),
                    ));
                }
            }
            Node::Leaf(_) => {
                blocks.push(GraphBlock::Para(iter.inlines()));
            }
            Node::Raw(_, _) => {
                blocks.push(GraphBlock::CodeBlock(
                    iter.lang(),
                    iter.content().unwrap_or_default(),
                ));
            }
            Node::HorizontalRule() => {
                blocks.push(GraphBlock::HorizontalRule);
            }
            Node::Reference(_) => {
                let inlines = match iter.ref_type().unwrap() {
                    ReferenceType::Regular => iter.inlines(),
                    ReferenceType::WikiLink => vec![],
                    ReferenceType::WikiLinkPiped => {
                        vec![GraphInline::Str(iter.ref_text().unwrap_or_default())]
                    }
                };

                let link = GraphInline::Link(
                    iter.ref_key2().unwrap().to_rel_link_url(&self.parent),
                    String::default(),
                    iter.ref_type().unwrap().to_link_type(),
                    inlines,
                );

                blocks.push(GraphBlock::Para(vec![link]));
            }
            Node::Table(_) => {
                blocks.push(GraphBlock::Table(
                    iter.table_header()
                        .unwrap_or_default()
                        .iter()
                        .cloned()
                        .collect(),
                    iter.table_alignment().unwrap_or_default(),
                    iter.table_rows()
                        .unwrap_or_default()
                        .iter()
                        .map(|row| row.iter().cloned().collect())
                        .collect(),
                ));
            }
        }
        if let Some(next) = iter.next() {
            blocks.extend(self.with(self.header_level).project_node(next));
        }
        blocks
    }

    fn project_list_item<'a>(&self, iter: impl NodeIter<'a>) -> Vec<Vec<GraphBlock>> {
        let mut items: Vec<Vec<GraphBlock>> = vec![];

        if iter.node().is_none() {
            return items;
        }

        if iter.child().map(|n| n.is_leaf()).unwrap_or(false) {
            items.push(vec![GraphBlock::Para(iter.inlines())]);
        } else {
            items.push(vec![GraphBlock::Plain(iter.inlines())]);
        }

        iter.child()
            .map(|child| self.with(0).project_node(child))
            .map(|sub_list| {
                sub_list
                    .iter()
                    .for_each(|item| items.last_mut().unwrap().push(item.clone()))
            });

        iter.next()
            .map(|next| self.with(self.header_level).project_list_item(next))
            .map(|blocks| items.append(blocks.clone().as_mut()));

        items
    }
}
