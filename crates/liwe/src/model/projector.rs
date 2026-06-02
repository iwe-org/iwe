use crate::model::inline::{prepend_checkbox, Inline, Inlines};
use crate::model::node::{Node, NodeIter, ReferenceType};
use crate::model::writer::Block;

pub struct Projector {
    header_level: usize,
    parent: String,
}

impl Projector {
    pub fn project<'a>(iter: impl NodeIter<'a>, parent: &str) -> Vec<Block> {
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

    fn resolve_inlines(&self, inlines: Inlines) -> Inlines {
        inlines
            .into_iter()
            .map(|i| self.resolve_inline(i))
            .collect()
    }

    fn resolve_inline(&self, inline: Inline) -> Inline {
        match inline {
            Inline::Reference(reference) => {
                let url = match reference.reference_type {
                    ReferenceType::Regular => reference.key.to_rel_link_url(&self.parent),
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => reference
                        .display_url
                        .clone()
                        .unwrap_or_else(|| reference.key.to_library_url()),
                };
                let inlines = match reference.reference_type {
                    ReferenceType::WikiLink => vec![],
                    _ => vec![Inline::Str(reference.text)],
                };
                Inline::Link(
                    url,
                    String::default(),
                    reference.reference_type.to_link_type(),
                    inlines,
                )
            }
            Inline::Emph(v) => Inline::Emph(self.resolve_inlines(v)),
            Inline::Strong(v) => Inline::Strong(self.resolve_inlines(v)),
            Inline::Strikeout(v) => Inline::Strikeout(self.resolve_inlines(v)),
            Inline::Underline(v) => Inline::Underline(self.resolve_inlines(v)),
            Inline::Superscript(v) => Inline::Superscript(self.resolve_inlines(v)),
            Inline::Subscript(v) => Inline::Subscript(self.resolve_inlines(v)),
            Inline::SmallCaps(v) => Inline::SmallCaps(self.resolve_inlines(v)),
            Inline::Image(url, title, v) => Inline::Image(url, title, self.resolve_inlines(v)),
            Inline::Link(url, title, lt, v) => {
                Inline::Link(url, title, lt, self.resolve_inlines(v))
            }
            other => other,
        }
    }

    fn project_node<'a>(&self, iter: impl NodeIter<'a>) -> Vec<Block> {
        let mut blocks = vec![];

        if iter.node().is_none() {
            return blocks;
        }

        match iter.node().unwrap() {
            Node::Document(_, frontmatter) => {
                if let Some(mapping) = frontmatter {
                    blocks.push(Block::Frontmatter(mapping.clone()));
                }
                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level).project_node(child));
                }
            }
            Node::Section(_) => {
                blocks.push(Block::Header(
                    self.header_level as u8 + 1,
                    self.resolve_inlines(iter.inlines()),
                ));

                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level + 1).project_node(child));
                }
            }
            Node::Quote() => {
                if let Some(child) = iter.child() {
                    blocks.push(Block::BlockQuote(self.with(0).project_node(child)));
                }
            }
            Node::BulletList() => {
                if let Some(child) = iter.child() {
                    blocks.push(Block::BulletList(self.with(0).project_list_item(child)));
                }
            }
            Node::OrderedList() => {
                if let Some(child) = iter.child() {
                    blocks.push(Block::OrderedList(self.with(0).project_list_item(child)));
                }
            }
            Node::Leaf(_) => {
                blocks.push(Block::Para(self.resolve_inlines(iter.inlines())));
            }
            Node::Item(checked, _) => {
                let inlines = prepend_checkbox(checked, self.resolve_inlines(iter.inlines()));
                blocks.push(Block::Para(inlines));

                if let Some(child) = iter.child() {
                    blocks.extend(self.with(self.header_level).project_node(child));
                }
            }
            Node::Raw(_, _) => {
                blocks.push(Block::CodeBlock(
                    iter.lang(),
                    iter.content().unwrap_or_default(),
                ));
            }
            Node::HorizontalRule() => {
                blocks.push(Block::HorizontalRule);
            }
            Node::Reference(reference) => {
                let reference_type = reference.reference_type;
                let inlines = match reference_type {
                    ReferenceType::Regular => self.resolve_inlines(iter.inlines()),
                    ReferenceType::WikiLink => vec![],
                    ReferenceType::WikiLinkPiped => {
                        vec![Inline::Str(iter.ref_text().unwrap_or_default())]
                    }
                };

                let url = match reference_type {
                    ReferenceType::Regular => reference.key.to_rel_link_url(&self.parent),
                    ReferenceType::WikiLink | ReferenceType::WikiLinkPiped => reference
                        .display_url
                        .clone()
                        .unwrap_or_else(|| reference.key.to_library_url()),
                };

                let link = Inline::Link(
                    url,
                    String::default(),
                    reference_type.to_link_type(),
                    inlines,
                );

                blocks.push(Block::Para(vec![link]));
            }
            Node::Table(_) => {
                blocks.push(Block::Table(
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

    fn project_list_item<'a>(&self, iter: impl NodeIter<'a>) -> Vec<Vec<Block>> {
        let mut items: Vec<Vec<Block>> = vec![];

        if iter.node().is_none() {
            return items;
        }

        let inlines = if iter.is_item() {
            prepend_checkbox(iter.item_checked(), self.resolve_inlines(iter.inlines()))
        } else {
            self.resolve_inlines(iter.inlines())
        };

        if iter.child().map(|n| n.is_leaf()).unwrap_or(false) {
            items.push(vec![Block::Para(inlines)]);
        } else {
            items.push(vec![Block::Plain(inlines)]);
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
