use itertools::Itertools;
use std::collections::HashMap;

use liwe::model::node::NodeIter;
use liwe::model::node::{Node, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Create, Update};

pub struct SectionExtract {}

impl SectionExtract {
    fn extract(
        node: &Tree,
        extract_id: NodeId,
        parent_id: NodeId,
        new_key: &liwe::model::Key,
    ) -> Tree {
        Self::extract_rec(node, extract_id, parent_id, new_key)
            .first()
            .unwrap()
            .clone()
    }

    fn extract_rec(
        tree: &Tree,
        extract_id: NodeId,
        parent_id: NodeId,
        new_key: &liwe::model::Key,
    ) -> Vec<Tree> {
        if tree.id_eq(parent_id) {
            let mut children = tree
                .clone()
                .children
                .into_iter()
                .filter(|child| !child.id_eq(extract_id))
                .collect_vec();

            children.insert(
                tree.pre_sub_header_position(),
                Tree {
                    id: None,
                    node: Node::Reference(Reference {
                        key: new_key.clone(),
                        text: tree
                            .find_id(extract_id)
                            .expect("to have node")
                            .node
                            .plain_text(),
                        reference_type: ReferenceType::Regular,
                    }),
                    children: vec![],
                },
            );

            return vec![Tree {
                id: tree.id,
                node: tree.node.clone(),
                children,
            }];
        }

        return vec![Tree {
            id: tree.id,
            node: tree.node.clone(),
            children: tree
                .children
                .iter()
                .map(|child| Self::extract_rec(child, extract_id, parent_id, new_key))
                .flatten()
                .collect(),
        }];
    }
}

impl ActionProvider for SectionExtract {
    fn identifier(&self) -> String {
        return "refactor.extract.section".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|_| Action {
                title: "Extract section".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|parent_id| {
                let new_key = context.random_key(&key.parent());

                let tree = context.collect(&key);

                let updated_tree = Self::extract(&tree, target_id, parent_id, &new_key);

                let markdown = updated_tree
                    .iter()
                    .to_markdown(&key.parent(), context.markdown_options());
                let new_markdown = tree
                    .get(target_id)
                    .iter()
                    .to_markdown(&new_key.parent(), context.markdown_options());

                vec![
                    Change::Create(Create {
                        key: new_key.clone(),
                    }),
                    Change::Update(Update {
                        key: new_key,
                        markdown: new_markdown,
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ]
            })
    }
}

pub struct SubSectionsExtract {}

impl ActionProvider for SubSectionsExtract {
    fn identifier(&self) -> String {
        return "refactor.extract.subsections".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .map(|_| Action {
                title: "Extract sub-sections".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .map(|tree| {
                let sub_sections = tree
                    .children
                    .iter()
                    .filter(|child| child.is_section())
                    .map(|child| child.id.unwrap())
                    .collect_vec();

                let mut extracted = HashMap::new();
                let mut changes = vec![];

                for section_id in sub_sections {
                    let new_key = context.random_key(&key.parent());

                    extracted.insert(
                        section_id,
                        (
                            new_key.clone(),
                            tree.find_id(section_id)
                                .map(|n| n.node.plain_text())
                                .unwrap_or_default(),
                        ),
                    );
                    changes.push(Change::Create(Create {
                        key: new_key.clone(),
                    }));
                    changes.push(Change::Update(Update {
                        key: new_key.clone(),
                        markdown: tree
                            .find_id(section_id)
                            .expect("to have section")
                            .iter()
                            .to_markdown(&new_key.parent(), context.markdown_options()),
                    }));
                }

                changes.push(Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .extract_sections(extracted.clone())
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                }));

                changes
            })
    }
}
