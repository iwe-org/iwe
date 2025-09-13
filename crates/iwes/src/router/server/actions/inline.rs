use itertools::Itertools;
use liwe::model::config::InlineType;
use liwe::model::node::Node;
use liwe::model::node::NodeIter;
use liwe::model::tree::Tree;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Remove, Update};

pub struct InlineAction {
    pub title: String,
    pub identifier: String,
    pub inline_type: InlineType,
    pub keep_target: bool,
}

impl InlineAction {
    fn add_additional_reference_cleanup(
        &self,
        changes: &mut Vec<Change>,
        inline_key: &liwe::model::Key,
        current_key: &liwe::model::Key,
        context: &impl ActionContext,
    ) {
        context
            .get_block_references_to(inline_key)
            .into_iter()
            .map(|node_id| context.key_of(node_id))
            .chain(
                context
                    .get_inline_references_to(inline_key)
                    .into_iter()
                    .map(|node_id| context.key_of(node_id)),
            )
            .unique()
            .sorted()
            .filter(|ref_key| ref_key != current_key)
            .for_each(|ref_key| {
                changes.push(Change::Update(Update {
                    key: ref_key.clone(),
                    markdown: context
                        .collect(&ref_key)
                        .remove_block_references_to(inline_key)
                        .remove_inline_links_to(inline_key)
                        .iter()
                        .to_markdown(&ref_key.parent(), context.markdown_options()),
                }));
            });
    }
}

impl ActionProvider for InlineAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .and_then(|target_id| {
                let inline_key = context.collect(&key).find_reference_key(target_id);
                let mut changes = vec![];

                let markdown = match self.inline_type {
                    InlineType::Section => context
                        .collect(&key)
                        .get_surrounding_section_id(target_id)
                        .map(|section_id| {
                            context
                                .collect(&key)
                                .remove_node(target_id)
                                .append_pre_header(section_id, context.collect(&inline_key))
                                .iter()
                                .to_markdown(&key.parent(), context.markdown_options())
                        })?,
                    InlineType::Quote => {
                        let quote = Tree {
                            id: None,
                            node: Node::Quote(),
                            children: context.collect(&inline_key).children.clone(),
                        };

                        context
                            .collect(&key)
                            .replace(target_id, &quote)
                            .iter()
                            .to_markdown(&key.parent(), context.markdown_options())
                    }
                };

                if !self.keep_target {
                    changes.push(Change::Remove(Remove {
                        key: inline_key.clone(),
                    }));
                }

                changes.push(Change::Update(Update {
                    key: key.clone(),
                    markdown,
                }));

                if !self.keep_target {
                    self.add_additional_reference_cleanup(
                        &mut changes,
                        &inline_key,
                        &key,
                        &context,
                    );
                }

                Some(changes)
            })
    }
}
