use std::collections::HashMap;

use lsp_types::*;

use lib::graph::{Graph, GraphContext};

use super::{extensions::*, BasePath};

pub fn extract_sub_sections_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }

    if !params.only_includes(&CodeActionKind::REFACTOR) {
        return None;
    }

    let line = params.range.start.line;
    let target_key = params.text_document.uri.to_key(base_path);

    let maybe_node_id = context.get_node_id_at(&target_key, line as usize);

    if maybe_node_id.is_none() {
        return None;
    }

    let node_id = maybe_node_id.unwrap();

    if !context.is_header(node_id) {
        return None;
    }

    let sub_sections = context.get_sub_sections(node_id);

    let mut patch = context.patch();
    let mut extracted = HashMap::new();

    for section_id in sub_sections {
        let new_key = context.random_key();
        patch.build_key_and(&new_key, |doc| {
            doc.insert_from_iter(context.node_visitor_cut(section_id));
            extracted.insert(section_id, new_key.clone());
        });
    }

    patch.build_key_and(&target_key, |doc| {
        doc.insert_from_iter(context.extract_vistior(&target_key, extracted.clone()))
    });

    let mut ops = vec![];

    for new_key in extracted.values() {
        let new_url = new_key.to_url(base_path);
        ops.push(new_url.to_create_file_op());
        ops.push(new_url.to_override_file_op(base_path, patch.export_key(&new_key).unwrap()));
    }

    ops.push(
        params
            .text_document
            .uri
            .to_override_file_op(base_path, patch.export_key(&target_key).unwrap()),
    );

    Some(ops.to_code_action("Extract sub-sections".to_string(), CodeActionKind::REFACTOR))
}
