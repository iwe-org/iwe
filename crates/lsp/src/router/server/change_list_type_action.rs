use lsp_types::*;

use lib::graph::{Graph, GraphContext, NodeIter};

use super::{extensions::*, BasePath};

pub fn change_list_type_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }

    if !params.only_includes(&CodeActionKind::QUICKFIX) {
        return None;
    }

    let line = params.range.start.line;
    let key = params.text_document.uri.to_key(base_path);

    let maybe_node_id = context.get_node_id_at(&key, line as usize);

    if maybe_node_id.is_none() {
        return None;
    }

    let node_id = maybe_node_id.unwrap();

    let maybe_list_id = context.get_surrounding_list_id(node_id);

    if maybe_list_id.is_none() {
        return None;
    }

    let target_id = maybe_list_id.unwrap();

    let action_title = if context.is_bullet_list(target_id) {
        "Change to ordered list"
    } else {
        "Change to bullet list"
    };

    let mut patch = context.patch();

    patch.build_key_and(&key, |doc| {
        doc.insert_from_iter(
            context
                .change_list_type_visiton(&key, target_id)
                .child()
                .expect("to have child"),
        )
    });

    Some(
        vec![params
            .text_document
            .uri
            .to_override_file_op(base_path, patch.export_key(&key).unwrap())]
        .to_code_action(action_title.to_string(), CodeActionKind::QUICKFIX),
    )
}
