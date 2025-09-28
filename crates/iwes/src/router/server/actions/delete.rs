use itertools::Itertools;

use liwe::model::node::NodeIter;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, BlockAction, Change, Changes, Remove, Update};

pub struct DeleteAction {}

impl ActionProvider for DeleteAction {
    fn identifier(&self) -> String {
        "refactor.delete".to_string()
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| {
                Action::BlockAction(BlockAction {
                    title: "Delete".to_string(),
                    identifier: self.identifier(),
                    target_id,
                })
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
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
