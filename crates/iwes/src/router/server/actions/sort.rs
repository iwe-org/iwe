use liwe::model::node::NodeIter;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Update};

pub struct SortAction {
    pub title: String,
    pub identifier: String,
    pub reverse: bool,
}

impl ActionProvider for SortAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .filter(|scope_id| {
                // Only offer the action if the list is not already sorted in the desired order
                !context.collect(&key).is_sorted(*scope_id, self.reverse)
            })
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .sort_children(scope_id, self.reverse)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}
