use liwe::markdown::MarkdownReader;
use liwe::model::config::{Context, Model};
use liwe::model::node::{NodeIter, NodePointer};
use liwe::model::NodeId;

use super::super::llm::templates;
use super::{Action, ActionContext, ActionProvider, BlockAction, Change, Changes, Update};

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

    fn action(&self, target_id: NodeId, _: impl ActionContext) -> Option<Action> {
        Some(Action::BlockAction(BlockAction {
            title: self.title.clone(),
            identifier: self.identifier(),
            target_id,
        }))
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

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
