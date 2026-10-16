use std::collections::HashSet;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::key_index::KeyRename;
use crate::model::node::NodeIter;
use crate::model::node::Target;
use crate::model::Key;

use super::changes::{Changes, OperationError};

pub fn rename(graph: &Graph, old_key: &Key, new_key: &Key) -> Result<Changes, OperationError> {
    if new_key.as_str().is_empty() {
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
    let format = graph.format_options().clone();

    let rename = KeyRename::new(graph.key_index(), old_key, new_key);
    let rewrite = |target: &Target| rename.rewrite(target);

    let block_refs = graph.get_inclusion_edges_to(old_key);
    let inline_refs = graph.get_reference_edges_to(old_key);

    let affected: HashSet<Key> = block_refs
        .into_iter()
        .chain(inline_refs)
        .chain(graph.get_wiki_edges_named_like(new_key))
        .map(|node_id| graph.key_of(node_id))
        .filter(|k| k != old_key)
        .collect();

    for affected_key in affected.iter().sorted() {
        let tree = graph.collect(affected_key);
        let updated = tree.rewrite_targets(&rewrite);
        if updated == tree {
            continue;
        }
        let markdown = updated.iter().to_text(&affected_key.parent(), &format);
        result.add_update(affected_key.clone(), markdown);
    }

    let tree = graph.collect(old_key);
    let updated_tree = tree.rewrite_targets(&rewrite);
    let markdown = updated_tree.iter().to_text(&new_key.parent(), &format);
    result.add_create(new_key.clone(), markdown);
    result.add_remove(old_key.clone());

    Ok(result)
}
