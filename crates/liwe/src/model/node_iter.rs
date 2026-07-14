use crate::model::{Key, LineRange, NodeId};

use super::config::FormatOptions;
use super::ids::alloc_node_id;
use super::inline::{Inline, Inlines};
use super::node::{ColumnAlignment, Node, ReferenceType};
use super::projector::Projector;

pub trait NodeIter<'a>: Sized {
    fn next(&self) -> Option<Self>;
    fn child(&self) -> Option<Self>;
    fn node(&self) -> Option<Node>;

    fn iter_id(&self) -> NodeId {
        alloc_node_id()
    }

    fn line_range(&self) -> Option<LineRange> {
        None
    }

    fn to_text(self, parent: &str, format: &FormatOptions) -> String {
        let blocks = Projector::project(self, parent, format.refs_path());
        crate::format::write_document(&blocks, format)
    }

    fn to_text_skip_frontmatter(self, parent: &str, format: &FormatOptions) -> String {
        let blocks = Projector::project(self, parent, format.refs_path());
        crate::format::write_document_skip_frontmatter(&blocks, format)
    }

    fn plain_text(&self) -> String {
        self.inlines().iter().map(|i| i.plain_text()).collect()
    }

    fn to_default_text(self) -> String {
        self.to_text("", &FormatOptions::default())
    }

    fn ref_type(&self) -> Option<ReferenceType> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.reference_type)
            } else {
                None
            }
        })
    }

    fn lang(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Raw(lang, _) = node {
                lang.clone()
            } else {
                None
            }
        })
    }

    fn table_header(&self) -> Option<Vec<Inlines>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.header.clone())
            } else {
                None
            }
        })
    }

    fn table_alignment(&self) -> Option<Vec<ColumnAlignment>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.alignment.clone())
            } else {
                None
            }
        })
    }

    fn table_rows(&self) -> Option<Vec<Vec<Inlines>>> {
        self.node().and_then(|node| {
            if let Node::Table(table) = node {
                Some(table.rows.clone())
            } else {
                None
            }
        })
    }

    fn content(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Raw(_, content) = node {
                Some(content.clone())
            } else {
                None
            }
        })
    }

    fn ref_text(&self) -> Option<String> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.text.clone())
            } else {
                None
            }
        })
    }

    fn ref_key2(&self) -> Option<Key> {
        self.node().and_then(|node| {
            if let Node::Reference(reference) = node {
                Some(reference.key.clone())
            } else {
                None
            }
        })
    }

    fn inlines(&self) -> Inlines {
        self.node()
            .map(|node| match node {
                Node::Section(inlines) => inlines.clone(),
                Node::Leaf(inlines) => inlines.clone(),
                Node::Item(_, inlines) => inlines.clone(),
                Node::Reference(reference) => vec![Inline::Str(reference.text)],
                _ => vec![],
            })
            .unwrap_or_default()
    }

    fn is_item(&self) -> bool {
        matches!(self.node(), Some(Node::Item(_, _)))
    }

    fn item_checked(&self) -> Option<bool> {
        self.node().and_then(|node| {
            if let Node::Item(checked, _) = node {
                checked
            } else {
                None
            }
        })
    }

    fn is_list(&self) -> bool {
        self.is_ordered_list() || self.is_bullet_list()
    }

    fn is_document(&self) -> bool {
        matches!(self.node(), Some(Node::Document(_, _)))
    }

    fn is_section(&self) -> bool {
        matches!(self.node(), Some(Node::Section(_)))
    }

    fn is_ordered_list(&self) -> bool {
        matches!(self.node(), Some(Node::OrderedList()))
    }

    fn is_bullet_list(&self) -> bool {
        matches!(self.node(), Some(Node::BulletList()))
    }

    fn is_reference(&self) -> bool {
        matches!(self.node(), Some(Node::Reference(_)))
    }

    fn is_horizontal_rule(&self) -> bool {
        matches!(self.node(), Some(Node::HorizontalRule()))
    }

    fn is_raw(&self) -> bool {
        matches!(self.node(), Some(Node::Raw(_, _)))
    }

    fn is_leaf(&self) -> bool {
        matches!(self.node(), Some(Node::Leaf(_)))
    }

    fn is_quote(&self) -> bool {
        matches!(self.node(), Some(Node::Quote()))
    }
}
