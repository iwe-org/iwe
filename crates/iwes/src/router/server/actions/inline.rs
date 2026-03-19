use liwe::model::config::InlineType;
use liwe::operations::{inline, Changes, InlineConfig};

use super::{Action, ActionContext, ActionProvider};

pub struct InlineAction {
    pub title: String,
    pub identifier: String,
    pub inline_type: InlineType,
    pub keep_target: bool,
}

impl ActionProvider for InlineAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier)
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
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .and_then(|target_id| {
                let graph = context.graph();
                let config = InlineConfig {
                    inline_type: self.inline_type.clone(),
                    keep_target: self.keep_target,
                };

                inline(graph, &key, target_id, &config).ok()
            })
    }
}
