use chrono::Local;
use itertools::Itertools;
use liwe::model::config::LinkType;
use minijinja::{context, Environment};
use sanitize_filename::sanitize;

use liwe::model::node::NodeIter;
use liwe::model::node::{Node, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::{Key, NodeId};

use super::{Action, ActionContext, ActionProvider, Change, Changes, Create, Update};
pub struct SectionExtract {
    pub title: String,
    pub identifier: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
    pub key_date_format: String,
}

impl SectionExtract {
    fn format_target_key(
        &self,
        context: &impl ActionContext,
        id: &str,
        parent_key: &str,
        target_id: NodeId,
    ) -> Key {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.key_date_format).to_string();

        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        let title = tree
            .find_id(target_id)
            .map(|tree| tree.node.plain_text())
            .unwrap_or_default();

        let parent_title = tree
            .get_surrounding_section_id(target_id)
            .and_then(|parent_id| tree.find_id(parent_id))
            .map(|tree| tree.node.plain_text())
            .unwrap_or_default();

        let relative_key = Environment::new()
            .template_from_str(&self.key_template)
            .expect("correct template")
            .render(context! {
                today => formatted,
                id => id.to_string(),
                title => sanitize(title),
                parent => context! {
                      title => sanitize(parent_title),
                      key => parent_key,
                },
                source => context! {
                    key => key.to_string(),
                    file => key.source(),
                    title => context.get_ref_text(&key).unwrap_or_default(),
                    path => key.path().unwrap_or_default(),
                }
            })
            .expect("template to work");

        let base_key = Key::combine(&key.parent(), &relative_key);

        let mut candidate_key = base_key.clone();
        let mut counter = 1;

        while context.key_exists(&candidate_key) {
            let suffixed_name = format!("{}-{}", base_key.to_string(), counter);
            candidate_key = Key::name(&suffixed_name);
            counter += 1;
        }

        candidate_key
    }

    fn config_to_reference_type(link_type: Option<&LinkType>) -> ReferenceType {
        match link_type {
            Some(LinkType::WikiLink) => ReferenceType::WikiLink,
            Some(LinkType::Markdown) | None => ReferenceType::Regular,
        }
    }
    fn extract(
        node: &Tree,
        extract_id: NodeId,
        parent_id: NodeId,
        new_key: &liwe::model::Key,
        link_type: Option<&LinkType>,
    ) -> Tree {
        Self::extract_rec(node, extract_id, parent_id, new_key, link_type)
            .first()
            .unwrap()
            .clone()
    }

    fn extract_rec(
        tree: &Tree,
        extract_id: NodeId,
        parent_id: NodeId,
        new_key: &liwe::model::Key,
        link_type: Option<&LinkType>,
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
                        reference_type: Self::config_to_reference_type(link_type),
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
                .map(|child| Self::extract_rec(child, extract_id, parent_id, new_key, link_type))
                .flatten()
                .collect(),
        }];
    }
}

impl ActionProvider for SectionExtract {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|_| Action {
                title: self.title.clone(),
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
                let id = context
                    .unique_ids(&key.parent(), 1)
                    .first()
                    .expect("to have one")
                    .to_string();
                let new_key = self.format_target_key(&context, &id, &key.parent(), target_id);

                let tree = context.collect(&key);

                let updated_tree = Self::extract(
                    &tree,
                    target_id,
                    parent_id,
                    &new_key,
                    self.link_type.as_ref(),
                );

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
