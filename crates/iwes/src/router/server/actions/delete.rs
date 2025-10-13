use itertools::Itertools;

use liwe::model::node::NodeIter;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Remove, Update};

pub struct DeleteAction {}

impl ActionProvider for DeleteAction {
    fn identifier(&self) -> String {
        "refactor.delete".to_string()
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| Action {
                title: "Delete".to_string(),
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
        let tree = context.collect(&key);
        let target_key = tree.find_reference_key(target_id);

        let mut changes = vec![];

        changes.push(Change::Remove(Remove {
            key: target_key.clone(),
        }));

        context
            .get_block_references_to(&target_key)
            .into_iter()
            .map(|node_id| context.key_of(node_id))
            .chain(
                context
                    .get_inline_references_to(&target_key)
                    .into_iter()
                    .map(|node_id| context.key_of(node_id)),
            )
            .unique()
            .sorted()
            .for_each(|ref_key| {
                changes.push(Change::Update(Update {
                    key: ref_key.clone(),
                    markdown: context
                        .collect(&ref_key)
                        .remove_block_references_to(&target_key)
                        .remove_inline_links_to(&target_key)
                        .iter()
                        .to_markdown(&ref_key.parent(), context.markdown_options()),
                }));
            });

        Some(changes)
    }
}
