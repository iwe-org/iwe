use chrono::Local;
use itertools::Itertools;
use liwe::model::config::LinkType;
use minijinja::{context, Environment};
use sanitize_filename::sanitize;
use std::collections::HashMap;

use liwe::model::node::NodeIter;
use liwe::model::node::{Node, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::{Key, NodeId};

use super::{
    string_to_slug, Action, ActionContext, ActionProvider, Change, Changes, Create, Update,
};

pub struct ExtractAll {
    pub title: String,
    pub identifier: String,
    pub link_type: Option<LinkType>,
    pub key_template: String,
    pub key_date_format: String,
}

impl ExtractAll {
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

        let slug = string_to_slug(&title);

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
                slug => slug,
                parent => context! {
                      title => sanitize(&parent_title),
                      slug => string_to_slug(&parent_title),
                      key => parent_key
                },
                source => context! {
                    key => key.to_string(),
                    file => key.source(),
                    title => context.get_ref_text(&key).unwrap_or_default(),
                    slug => string_to_slug(&context.get_ref_text(&key).unwrap_or_default()),
                    path => key.path().unwrap_or_default(),
                }
            })
            .expect("template to work");

        let base_key = Key::combine(&key.parent(), &relative_key);

        base_key
    }

    fn config_to_reference_type(link_type: Option<&LinkType>) -> ReferenceType {
        match link_type {
            Some(LinkType::WikiLink) => ReferenceType::WikiLink,
            Some(LinkType::Markdown) | None => ReferenceType::Regular,
        }
    }

    fn extract_sections(
        tree: &Tree,
        sub_sections: &[NodeId],
        extracted: &HashMap<NodeId, (Key, String)>,
        link_type: Option<&LinkType>,
    ) -> Tree {
        Self::extract_sections_rec(tree, sub_sections, extracted, link_type)
            .first()
            .unwrap()
            .clone()
    }

    fn extract_sections_rec(
        tree: &Tree,
        sub_sections: &[NodeId],
        extracted: &HashMap<NodeId, (Key, String)>,
        link_type: Option<&LinkType>,
    ) -> Vec<Tree> {
        if let Some(tree_id) = tree.id {
            if sub_sections.contains(&tree_id) {
                if let Some((new_key, text)) = extracted.get(&tree_id) {
                    return vec![Tree {
                        id: None,
                        node: Node::Reference(Reference {
                            key: new_key.clone(),
                            text: text.clone(),
                            reference_type: Self::config_to_reference_type(link_type),
                        }),
                        children: vec![],
                    }];
                }
            }
        }

        vec![Tree {
            id: tree.id,
            node: tree.node.clone(),
            children: tree
                .children
                .iter()
                .map(|child| Self::extract_sections_rec(child, sub_sections, extracted, link_type))
                .flatten()
                .collect(),
        }]
    }
}

impl ActionProvider for ExtractAll {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                key: key.clone(),
                range: selection.clone(),
            })
    }

    fn changes(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Changes> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;

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
                let mut generated_keys = vec![];
                let ids = context.unique_ids(&key.parent(), sub_sections.len());

                for (i, section_id) in sub_sections.iter().enumerate() {
                    let base_key =
                        self.format_target_key(&context, &ids[i], &key.parent(), *section_id);

                    let mut new_key = base_key.clone();
                    let mut counter = 1;

                    while context.key_exists(&new_key) || generated_keys.contains(&new_key) {
                        let suffixed_name = format!("{}-{}", base_key.to_string(), counter);
                        new_key = Key::name(&suffixed_name);
                        counter += 1;
                    }

                    generated_keys.push(new_key.clone());

                    extracted.insert(
                        *section_id,
                        (
                            new_key.clone(),
                            tree.find_id(*section_id)
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
                            .find_id(*section_id)
                            .expect("to have section")
                            .iter()
                            .to_markdown(&new_key.parent(), context.markdown_options()),
                    }));
                }

                let tree = context.collect(&key);
                let updated_tree = Self::extract_sections(
                    &tree,
                    &sub_sections,
                    &extracted,
                    self.link_type.as_ref(),
                );

                changes.push(Change::Update(Update {
                    key: key.clone(),
                    markdown: updated_tree
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                }));

                changes
            })
    }
}
