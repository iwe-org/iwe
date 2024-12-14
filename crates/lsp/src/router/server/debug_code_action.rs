use lsp_types::*;

use lib::graph::GraphContext;

use super::{extensions::*, BasePath};

pub fn code_action(
    context: impl GraphContext,
    base_path: &BasePath,
    params: &CodeActionParams,
) -> Option<CodeActionOrCommand> {
    if !params.range.empty() {
        return None;
    }
    let line = params.range.start.line;
    let key = params.text_document.uri.to_key(base_path);

    let node_id = context.get_node_id_at(&key, line as usize);

    node_id.map(|id| {
        vec![].to_code_action(
            (format!("{}", context.get_text(id))).to_string(),
            CodeActionKind::REFACTOR_EXTRACT,
        )
    })
}
