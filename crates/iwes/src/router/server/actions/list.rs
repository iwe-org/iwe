use liwe::model::node::NodeIter;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Update};

pub struct ListChangeType {}

impl ActionProvider for ListChangeType {
    fn identifier(&self) -> String {
        return "refactor.rewrite.list.type".to_string();
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| Action {
                title: match tree.find_id(scope_id).map(|n| n.is_bullet_list()).unwrap() {
                    true => "Change to ordered list".to_string(),
                    false => "Change to bullet list".to_string(),
                },
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
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .change_list_type(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}

pub struct ListToSections {}

impl ActionProvider for ListToSections {
    fn identifier(&self) -> String {
        return "refactor.rewrite.list.section".to_string();
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        context
            .collect(&key)
            .get_top_level_surrounding_list_id(target_id)
            .map(|_| Action {
                title: "List to sections".to_string(),
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
        context
            .collect(&key)
            .get_top_level_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .unwrap_list(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}
