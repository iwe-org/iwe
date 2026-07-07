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
            Node::Table(table) => table.plain_text(),
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

impl Table {
    fn plain_text(&self) -> String {
        let cells = self.header.iter().chain(self.rows.iter().flatten());
        cells
            .map(|inlines| {
                inlines
                    .iter()
                    .map(|inline| inline.plain_text())
                    .collect::<String>()
            })
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ColumnAlignment {
    None,
    Left,
    Center,
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::inline::Inline;

    #[test]
    fn table_plain_text_concatenates_header_and_cells() {
        let node = Node::Table(Table {
            header: vec![
                vec![Inline::Str("Name".to_string())],
                vec![Inline::Str("Value".to_string())],
            ],
            alignment: vec![ColumnAlignment::None, ColumnAlignment::None],
            rows: vec![
                vec![
                    vec![Inline::Str("foo".to_string())],
                    vec![Inline::Str("bar".to_string())],
                ],
                vec![vec![], vec![Inline::Str("baz".to_string())]],
            ],
        });

        assert_eq!(node.plain_text(), "Name Value foo bar baz");
    }
}
