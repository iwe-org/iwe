use chrono::Local;
use minijinja::{context, Environment};

use liwe::model::node::NodeIter;
use liwe::model::node::{Node, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::Key;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Create, Update};

pub struct AttachAction {
    pub title: String,
    pub identifier: String,

    pub key_template: String,
    pub document_template: String,
    pub markdown_date_format: String,
    pub key_date_format: String,
}

impl AttachAction {
    fn format_target_key(&self) -> Key {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.key_date_format).to_string();

        Key::name(
            &Environment::new()
                .template_from_str(&self.key_template)
                .expect("correct template")
                .render(context! {
                today => formatted,
                })
                .expect("template to work"),
        )
    }

    fn format_target_document(&self, content: String) -> String {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.markdown_date_format).to_string();
        Environment::new()
            .template_from_str(&self.document_template)
            .expect("correct template")
            .render(context! {
            today => formatted,
            content => content
            })
            .expect("template to work")
    }
}

impl ActionProvider for AttachAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(
        &self,
        key: Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let reference_key = context.collect(&key).find_reference_key(target_id);
        let attach_to_key = self.format_target_key();

        if context.key_exists(&attach_to_key) {
            if context
                .collect(&attach_to_key)
                .get_all_block_reference_keys()
                .contains(&reference_key)
            {
                return None;
            }
        }

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|target| target.is_reference())
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                key: key.clone(),
                range: selection.clone(),
            })
    }

    fn changes(
        &self,
        key: Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Changes> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let reference_key = context.collect(&key).find_reference_key(target_id);
        let attach_to_key = self.format_target_key();
        let reference = Tree {
            id: None,
            node: Node::Reference(Reference {
                key: reference_key,
                text: {
                    context
                        .collect(&key)
                        .find_id(target_id)
                        .and_then(|tree| tree.node.reference_text())
                        .unwrap_or_default()
                },
                reference_type: ReferenceType::Regular,
            }),
            children: vec![],
        };

        if context.key_exists(&attach_to_key) {
            let tree = context.collect(&attach_to_key);

            let updated = tree.attach(reference);

            Some(vec![Change::Update(Update {
                key: attach_to_key.clone(),
                markdown: updated
                    .iter()
                    .to_markdown(&attach_to_key.parent(), &context.markdown_options()),
            })])
        } else {
            Some(vec![
                Change::Create(Create {
                    key: attach_to_key.clone(),
                }),
                Change::Update(Update {
                    key: attach_to_key.clone(),
                    markdown: self.format_target_document(
                        reference
                            .iter()
                            .to_markdown(&attach_to_key.parent(), &context.markdown_options()),
                    ),
                }),
            ])
        }
    }
}
