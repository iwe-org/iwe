use liwe::model::{node::NodeIter, tree::Tree, NodeId};
use minijinja::{context, Environment};

static UPDATE_START: &str = "<update_here>";
static UPDATE_END: &str = "</update_here>";

static CONTEXT_START: &str = "<context>";
static CONTEXT_END: &str = "</context>";

pub fn block_action_prompt(prompt_template: &str, node_id: NodeId, tree: &Tree) -> String {
    let marked = tree.mark_node(node_id, UPDATE_START, UPDATE_END);

    let context: &str = &marked.iter().to_default_markdown();

    Environment::new()
        .template_from_str(prompt_template)
        .expect("correct template")
        .render(context! {
        context => context,
        context_start => CONTEXT_START,
        context_end => CONTEXT_END,
        update_start => UPDATE_START,
        update_end => UPDATE_END
        })
        .unwrap()
}
