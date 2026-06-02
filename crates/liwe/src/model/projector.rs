use crate::model::graph::{GraphBlock, GraphInline, GraphInlines};
use crate::model::key_index::KeyIndex;
use crate::model::node::{Node, NodeIter, ReferenceType};
use crate::model::Key;

pub struct Projector<'i> {
    header_level: usize,
    parent: String,
    key_index: &'i KeyIndex,
}

impl<'i> Projector<'i> {
    pub fn project<'a>(
        iter: impl NodeIter<'a>,
        parent: &str,
        key_index: &'i KeyIndex,
    ) -> Vec<GraphBlock> {
        Projector {
            header_level: 0,
            parent: parent.to_string(),
            key_index,
        }
        .project_node(iter)
    }

    fn with(&self, header_level: usize) -> Projector<'i> {
        Projector {
            header_level,
            parent: self.parent.clone(),
            key_index: self.key_index,
        }
    }

    fn wiki_url(&self, key: &Key) -> String {
        self.key_index.shorten_wiki(key)
    }

    fn resolve_inlines(&self, inlines: GraphInlines) -> GraphInlines {
        inlines
            .into_iter()
            .map(|i| self.resolve_inline(i))
            .collect()
    }

    fn resolve_inline(&self, inline: GraphInline) -> GraphInline {
        match inline {
            GraphInline::Reference(reference) => {
                let url = match reference.reference_type {
                    ReferenceType::Regular => reference.key.to_rel_link_url(&self.parent),
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => {
                        self.wiki_url(&reference.key)
                    }
                };
                let inlines = match reference.reference_type {
                    ReferenceType::WikiLink => vec![],
                    _ => vec![GraphInline::Str(reference.text)],
                };
                GraphInline::Link(
                    url,
                    String::default(),
                    reference.reference_type.to_link_type(),
                    inlines,
                )
            }
            GraphInline::Emph(v) => GraphInline::Emph(self.resolve_inlines(v)),
            GraphInline::Strong(v) => GraphInline::Strong(self.resolve_inlines(v)),
            GraphInline::Strikeout(v) => GraphInline::Strikeout(self.resolve_inlines(v)),
            GraphInline::Underline(v) => GraphInline::Underline(self.resolve_inlines(v)),
            GraphInline::Superscript(v) => GraphInline::Superscript(self.resolve_inlines(v)),
            GraphInline::Subscript(v) => GraphInline::Subscript(self.resolve_inlines(v)),
            GraphInline::SmallCaps(v) => GraphInline::SmallCaps(self.resolve_inlines(v)),
            GraphInline::Image(url, title, v) => {
                GraphInline::Image(url, title, self.resolve_inlines(v))
            }
            GraphInline::Link(url, title, lt, v) => {
                GraphInline::Link(url, title, lt, self.resolve_inlines(v))
            }
            other => other,
        }
    }

    fn project_node<'a>(&self, iter: impl NodeIter<'a>) -> Vec<GraphBlock> {
        let mut blocks = vec![];

        if iter.node().is_none() {
            return blocks;
        }

        match iter.node().unwrap() {
            Node::Document(_, frontmatter) => {
                if let Some(mapping) = frontmatter {
                    blocks.push(GraphBlock::Frontmatter(mapping.clone()));
                }
                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level).project_node(child));
                }
            }
            Node::Section(_) => {
                blocks.push(GraphBlock::Header(
                    self.header_level as u8 + 1,
                    self.resolve_inlines(iter.inlines()),
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
                blocks.push(GraphBlock::Para(self.resolve_inlines(iter.inlines())));
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
                let reference_type = iter.ref_type().unwrap();
                let inlines = match reference_type {
                    ReferenceType::Regular => self.resolve_inlines(iter.inlines()),
                    ReferenceType::WikiLink => vec![],
                    ReferenceType::WikiLinkPiped => {
                        vec![GraphInline::Str(iter.ref_text().unwrap_or_default())]
                    }
                };

                let key = iter.ref_key2().unwrap();
                let url = match reference_type {
                    ReferenceType::Regular => key.to_rel_link_url(&self.parent),
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => self.wiki_url(&key),
                };

                let link = GraphInline::Link(
                    url,
                    String::default(),
                    reference_type.to_link_type(),
                    inlines,
                );

                blocks.push(GraphBlock::Para(vec![link]));
            }
            Node::Table(_) => {
                blocks.push(GraphBlock::Table(
                    iter.table_header()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|cell| self.resolve_inlines(cell))
                        .collect(),
                    iter.table_alignment().unwrap_or_default(),
                    iter.table_rows()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|row| {
                            row.into_iter()
                                .map(|cell| self.resolve_inlines(cell))
                                .collect()
                        })
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
            items.push(vec![GraphBlock::Para(self.resolve_inlines(iter.inlines()))]);
        } else {
            items.push(vec![GraphBlock::Plain(
                self.resolve_inlines(iter.inlines()),
            )]);
        }

        if let Some(sub_list) = iter.child().map(|child| self.with(0).project_node(child)) {
            sub_list
                .iter()
                .for_each(|item| items.last_mut().unwrap().push(item.clone()))
        }

        if let Some(blocks) = iter
            .next()
            .map(|next| self.with(self.header_level).project_list_item(next))
        {
            items.append(blocks.clone().as_mut())
        }

        items
    }
}
