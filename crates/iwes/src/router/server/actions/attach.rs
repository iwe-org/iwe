use std::time::SystemTime;

use chrono::{DateTime, Local, Locale};
use minijinja::{context, Environment};

use liwe::model::node::NodeIter;
use liwe::model::node::{Node, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::Key;
use liwe::operations::Changes;

use super::{Action, ActionContext, ActionProvider};

pub struct AttachAction {
    pub title: String,
    pub identifier: String,

    pub key_template: String,
    pub document_template: String,
    pub markdown_date_format: String,
    pub markdown_time_format: String,
    pub key_date_format: String,
    pub key_time_format: String,
    pub key_locale: Locale,
    pub markdown_locale: Locale,
}

impl AttachAction {
    fn format_target_key(&self, now: SystemTime) -> Key {
        let now: DateTime<Local> = now.into();
        let today_formatted = now
            .format_localized(&self.key_date_format, self.key_locale)
            .to_string();
        let now_formatted = now
            .format_localized(&self.key_time_format, self.key_locale)
            .to_string();

        Key::name(
            &Environment::new()
                .template_from_str(&self.key_template)
                .expect("correct template")
                .render(context! {
                today => today_formatted,
                now => now_formatted,
                })
                .expect("template to work"),
        )
    }

    fn format_target_document(&self, now: SystemTime, content: String) -> String {
        let now: DateTime<Local> = now.into();
        let today_formatted = now
            .format_localized(&self.markdown_date_format, self.markdown_locale)
            .to_string();
        let now_formatted = now
            .format_localized(&self.markdown_time_format, self.markdown_locale)
            .to_string();
        Environment::new()
            .template_from_str(&self.document_template)
            .expect("correct template")
            .render(context! {
            today => today_formatted,
            now => now_formatted,
            content => content
            })
            .expect("template to work")
    }
}

impl ActionProvider for AttachAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier)
    }

    fn action(
        &self,
        key: Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let tree = context.collect(&key);

        let (reference_key, _reference_text) = if tree.get(target_id).is_reference() {
            let ref_key = tree.find_reference_key(target_id);
            let ref_text = tree
                .find_id(target_id)
                .and_then(|t| t.node.reference_text())
                .unwrap_or_default();
            (ref_key, ref_text)
        } else {
            let ref_key = context.get_link_key_at(
                &key,
                selection.start.line as usize,
                selection.start.character as usize,
            )?;
            let ref_text = context
                .get_link_text_at(
                    &key,
                    selection.start.line as usize,
                    selection.start.character as usize,
                )
                .unwrap_or_default();
            (ref_key, ref_text)
        };

        let now = context.now();
        let attach_to_key = self.format_target_key(now);

        if context.key_exists(&attach_to_key)
            && context
                .collect(&attach_to_key)
                .get_all_block_reference_keys()
                .contains(&reference_key)
        {
            return None;
        }

        context.graph().maybe_key(&reference_key)?;

        Some(Action {
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
        let tree = context.collect(&key);

        let (reference_key, reference_text) = if tree.get(target_id).is_reference() {
            let ref_key = tree.find_reference_key(target_id);
            let ref_text = tree
                .find_id(target_id)
                .and_then(|t| t.node.reference_text())
                .unwrap_or_default();
            (ref_key, ref_text)
        } else {
            let ref_key = context.get_link_key_at(
                &key,
                selection.start.line as usize,
                selection.start.character as usize,
            )?;
            let ref_text = context
                .get_link_text_at(
                    &key,
                    selection.start.line as usize,
                    selection.start.character as usize,
                )
                .unwrap_or_default();
            (ref_key, ref_text)
        };

        let now = context.now();
        let attach_to_key = self.format_target_key(now);
        let reference = Tree {
            id: None,
            node: Node::Reference(Reference {
                key: reference_key,
                text: reference_text,
                reference_type: ReferenceType::Regular,
            }),
            children: vec![],
        };

        if context.key_exists(&attach_to_key) {
            let tree = context.collect(&attach_to_key);

            let updated = tree.attach(reference);

            Some(Changes::new().update(
                attach_to_key.clone(),
                updated
                    .iter()
                    .to_markdown(&attach_to_key.parent(), context.markdown_options()),
            ))
        } else {
            Some(Changes::new().create(
                attach_to_key.clone(),
                self.format_target_document(
                    now,
                    reference
                        .iter()
                        .to_markdown(&attach_to_key.parent(), context.markdown_options()),
                ),
            ))
        }
    }
}
