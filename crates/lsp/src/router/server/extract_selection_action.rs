use lsp_types::*;

use lib::graph::GraphContext;

use super::{extensions::*, BasePath};

pub fn code_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if params.range.empty() {
        return None;
    }

    None

    // let selection = context.selection(&params.text_document.uri, &params.range);

    // let new_key = context.random_key();
    // let new_url = new_key.to_url(context);

    // let title = Node::from_markdown(&new_key, &selection, context)
    //     .map(|n| n.title_or_empty())
    //     .unwrap_or_default();

    // Some(
    //     vec![
    //         new_url.to_create_file_op(),
    //         new_url.to_update_file_op(context, selection.clone()),
    //         params.text_document.uri.to_update_file_range_op(
    //             format!("{}\n", new_key.to_link(title)),
    //             params.range.just_lines(),
    //         ),
    //     ]
    //     .to_code_action(
    //         "Extract Selection".to_string(),
    //         CodeActionKind::REFACTOR_EXTRACT,
    //     ),
    // )
}
