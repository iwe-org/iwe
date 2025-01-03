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
}
