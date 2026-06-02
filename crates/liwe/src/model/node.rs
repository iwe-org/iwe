use serde_yaml::Mapping;

use crate::model::inline::Inlines;
use crate::model::Key;

pub use crate::model::node_iter::NodeIter;
pub use crate::model::node_pointer::NodePointer;
pub use crate::model::reference::{Reference, ReferenceType};

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Document(Key, Option<Mapping>),
    Section(Inlines),
    Quote(),
    BulletList(),
    OrderedList(),
    Leaf(Inlines),
    Item(Option<bool>, Inlines),
    Raw(Option<String>, String),
    HorizontalRule(),
    Reference(Reference),
    Table(Table),
}

impl Node {
    pub fn plain_text(&self) -> String {
        match self {
            Node::Section(inlines) => inlines.iter().map(|i| i.plain_text()).collect(),
            Node::Leaf(inlines) => inlines.iter().map(|i| i.plain_text()).collect(),
            Node::Item(_, inlines) => inlines.iter().map(|i| i.plain_text()).collect(),
            Node::Reference(reference) => reference.text.clone(),
            Node::Raw(_, content) => content.clone(),
            _ => "".to_string(),
        }
    }

    pub fn reference_key(&self) -> Option<Key> {
        match self {
            Node::Reference(reference) => Some(reference.key.clone()),
            _ => None,
        }
    }

    pub fn reference_text(&self) -> Option<String> {
        match self {
            Node::Reference(reference) => Some(reference.text.clone()),
            _ => None,
        }
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Node::Reference(_))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub header: Vec<Inlines>,
    pub alignment: Vec<ColumnAlignment>,
    pub rows: Vec<Vec<Inlines>>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ColumnAlignment {
    None,
    Left,
    Center,
    Right,
}
