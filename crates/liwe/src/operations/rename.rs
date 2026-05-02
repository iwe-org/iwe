use std::collections::HashSet;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::node::NodeIter;
use crate::model::Key;

use super::changes::{Changes, OperationError};

pub fn rename(graph: &Graph, old_key: &Key, new_key: &Key) -> Result<Changes, OperationError> {
    if new_key.relative_path.is_empty() {
        return Err(OperationError::InvalidTarget(
            "Key cannot be empty".to_string(),
        ));
    }
    if graph.get_node_id(old_key).is_none() {
        return Err(OperationError::NotFound(old_key.clone()));
    }
    if graph.get_node_id(new_key).is_some() {
        return Err(OperationError::AlreadyExists(new_key.clone()));
    }

    let mut result = Changes::default();
    let options = graph.markdown_options();

    let block_refs = graph.get_inclusion_edges_to(old_key);
    let inline_refs = graph.get_reference_edges_to(old_key);

    let affected: HashSet<Key> = block_refs
        .into_iter()
        .chain(inline_refs)
        .map(|node_id| graph.key_of(node_id))
        .filter(|k| k != old_key)
        .collect();

    for affected_key in affected.iter().sorted() {
        let tree = graph.collect(affected_key);
        let updated = tree.change_key(old_key, new_key);
        let markdown = updated.iter().to_markdown(&affected_key.parent(), &options);
        result.add_update(affected_key.clone(), markdown);
    }

    let tree = graph.collect(old_key);
    let updated_tree = tree.change_key(old_key, new_key);
    let markdown = updated_tree.iter().to_markdown(&new_key.parent(), &options);
    result.add_create(new_key.clone(), markdown);
    result.add_remove(old_key.clone());

    Ok(result)
}
