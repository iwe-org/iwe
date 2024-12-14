use lsp_types::*;

use lib::graph::{Graph, GraphContext, NodeIter};

use super::{extensions::*, BasePath};

pub fn section_to_list_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }

    if !params.only_includes(&CodeActionKind::REFACTOR_REWRITE) {
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

    if maybe_list_id.is_some() {
        return None;
    }

    if !context.is_header(node_id) {
        return None;
    }

    let mut patch = Graph::new();

    patch.build_key_and(&key, |doc| {
        doc.insert_from_iter(
            context
                .wrap_vistior(node_id)
                .child()
                .expect("to have child"),
        )
    });

    Some(
        vec![params
            .text_document
            .uri
            .to_override_file_op(base_path, patch.export_key(&key).unwrap())]
        .to_code_action(
            "Sections to list".to_string(),
            CodeActionKind::REFACTOR_REWRITE,
        ),
    )
}
