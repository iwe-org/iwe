use liwe::model::node::NodeIter;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Update};

pub struct SectionToList {}

impl ActionProvider for SectionToList {
    fn identifier(&self) -> String {
        return "refactor.rewrite.section.list".to_string();
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
            .filter(|node_id| tree.is_header(*node_id))
            .map(|_| Action {
                title: "Section to list".to_string(),
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
