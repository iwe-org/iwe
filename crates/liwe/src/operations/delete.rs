use std::collections::HashSet;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::node::NodeIter;
use crate::model::Key;

use super::changes::{Changes, OperationError};

pub fn delete(graph: &Graph, target_key: &Key) -> Result<Changes, OperationError> {
    if graph.get_node_id(target_key).is_none() {
        return Err(OperationError::NotFound(target_key.clone()));
    }

    let mut result = Changes::default();
    let options = graph.markdown_options();

    let block_refs = graph.get_block_references_to(target_key);
    let inline_refs = graph.get_inline_references_to(target_key);

    let affected: HashSet<Key> = block_refs
        .into_iter()
        .chain(inline_refs)
        .map(|node_id| graph.key_of(node_id))
        .collect();

    for affected_key in affected.iter().sorted() {
        let tree = graph.collect(affected_key);
        let updated = tree
            .remove_block_references_to(target_key)
            .remove_inline_links_to(target_key);
        let markdown = updated.iter().to_markdown(&affected_key.parent(), &options);
        result.add_update(affected_key.clone(), markdown);
    }

    result.add_remove(target_key.clone());

    Ok(result)
}
