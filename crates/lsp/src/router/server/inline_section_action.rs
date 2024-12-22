use lsp_types::*;

use lib::{
    graph::{Graph, GraphContext},
    model::graph::MarkdownOptions,
};

use super::{extensions::*, BasePath};

pub fn inline_section_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }

    if !params.only_includes(&CodeActionKind::REFACTOR_INLINE) {
        return None;
    }

    let line = params.range.start.line;
    let key = params.text_document.uri.to_key(base_path);

    let maybe_node_id = context.get_node_id_at(&key, line as usize);

    if maybe_node_id.is_none() {
        return None;
    }

    let node_id = maybe_node_id.unwrap();

    if !context.is_reference(node_id) {
        return None;
    }

    let mut patch = Graph::new();

    // create new version of the original key where reference is inlined
    patch
        .build_key(&key)
        .insert_from_iter(context.inline_vistior(&key, node_id));

    Some(
        vec![
            context
                .get_reference_key(node_id)
                .to_url(base_path)
                .to_delete_file_op(),
            params
                .text_document
                .uri
                .to_override_file_op(base_path, patch.export_key(&key).unwrap()),
        ]
        .to_code_action(
            "Inline reference".to_string(),
            CodeActionKind::REFACTOR_INLINE,
        ),
    )
}
