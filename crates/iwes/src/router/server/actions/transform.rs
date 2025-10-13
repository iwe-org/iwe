use liwe::markdown::MarkdownReader;
use liwe::model::config::{Context, Model};
use liwe::model::node::{NodeIter, NodePointer};

use super::super::llm::templates;
use super::{Action, ActionContext, ActionProvider, Change, Changes, Update};

pub struct TransformBlockAction {
    pub title: String,
    pub identifier: String,

    pub model_parameters: Model,

    pub prompt_template: String,
    pub context: Context,
}

impl ActionProvider for TransformBlockAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(
        &self,
        key: super::Key,
        selection: super::TextRange,
        context: impl ActionContext,
    ) -> Option<Action> {
        let _target_id = context.get_node_id_at(&key, selection.start.line as usize)?;
        Some(Action {
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

        let tree = &context.collect(&key);

        let target_id = tree
            .get_surrounding_top_level_block(target_id)
            .unwrap_or(target_id);

        let prompt = templates::block_action_prompt(&self.prompt_template, target_id, tree);

        let generated = context.llm_query(prompt, &self.model_parameters);

        let mut patch = context.patch();

        patch.from_markdown("new".into(), &generated, MarkdownReader::new());
        let tree = patch.maybe_key(&"new".into()).unwrap().collect_tree();

        let markdown = context
            .collect(&key)
            .replace(target_id, &tree)
            .iter()
            .to_markdown(&key.parent(), &context.markdown_options());

        Some(vec![Change::Update(Update { key, markdown })])
    }
}
