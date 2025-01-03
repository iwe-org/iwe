use std::collections::HashMap;

use lsp_types::*;

use lib::graph::{Graph, GraphContext};

use super::{extensions::*, BasePath};

pub fn extract_list_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }

    if !params.only_includes(&CodeActionKind::REFACTOR_EXTRACT) {
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

    let new_key = context.random_key();
    let new_url = new_key.to_url(base_path);

    let mut patch = context.patch();

    // create new version of the original key where section is replaced with ref
    patch.build_key_and(&key, |doc| {
        doc.insert_from_iter(context.extract_vistior(
            &key,
            HashMap::from([(maybe_list_id.unwrap(), new_key.clone())]),
        ))
    });

    // create new key with the extracted part
    patch.build_key_and(&new_key, |doc| {
        doc.insert_from_iter(context.node_visitor(node_id))
    });

    Some(
        vec![
            new_url.to_create_file_op(),
            new_url.to_override_file_op(base_path, patch.export_key(&new_key).unwrap()),
            params
                .text_document
                .uri
                .to_override_file_op(base_path, patch.export_key(&key).unwrap()),
        ]
        .to_code_action("Extract list".to_string(), CodeActionKind::REFACTOR_EXTRACT),
    )
}
