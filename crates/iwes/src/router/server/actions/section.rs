use liwe::model::node::NodeIter;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Update};

pub struct SectionToList {}

impl ActionProvider for SectionToList {
    fn identifier(&self) -> String {
        return "refactor.rewrite.section.list".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|node_id| tree.is_header(*node_id))
            .map(|_| Action {
                title: "Section to list".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|node_id| tree.is_header(*node_id))
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .wrap_into_list(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}
