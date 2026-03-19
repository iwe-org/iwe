use liwe::operations::{delete, Changes};

use super::{Action, ActionContext, ActionProvider};

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
        let graph = context.graph();

        delete(graph, &target_key).ok()
    }
}
