use std::collections::HashSet;

use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::config::InlineType;
use crate::model::node::{Node, NodeIter};
use crate::model::tree::Tree;
use crate::model::{Key, NodeId};

use super::changes::{Changes, OperationError};
use super::config::InlineConfig;

pub fn inline(
    graph: &Graph,
    source_key: &Key,
    ref_id: NodeId,
    config: &InlineConfig,
) -> Result<Changes, OperationError> {
    if graph.get_node_id(source_key).is_none() {
        return Err(OperationError::NotFound(source_key.clone()));
    }

    let tree = graph.collect(source_key);

    if !tree.get(ref_id).is_reference() {
        return Err(OperationError::InvalidTarget(
            "Target must be a reference".to_string(),
        ));
    }

    let inline_key = tree.find_reference_key(ref_id);

    if graph.get_node_id(&inline_key).is_none() {
        return Err(OperationError::TargetNotFound(inline_key));
    }

    let options = graph.markdown_options();
    let inline_tree = graph.collect(&inline_key);

    let mut result = Changes::default();

    let updated_tree = match config.inline_type {
        InlineType::Section => {
            let section_id = tree
                .get_surrounding_section_id(ref_id)
                .ok_or(OperationError::InvalidTarget(
                    "Reference must be within a section to inline as section".to_string(),
                ))?;
            tree.remove_node(ref_id)
                .append_pre_header(section_id, inline_tree.clone())
        }
        InlineType::Quote => {
            let quote_tree = Tree {
                id: None,
                node: Node::Quote(),
                children: inline_tree.children.clone(),
            };
            tree.replace(ref_id, &quote_tree)
        }
    };

    let source_markdown = updated_tree
        .iter()
        .to_markdown(&source_key.parent(), &options);
    result.add_update(source_key.clone(), source_markdown);

    if !config.keep_target {
        result.add_remove(inline_key.clone());

        let block_refs = graph.get_inclusion_edges_to(&inline_key);
        let inline_refs = graph.get_reference_edges_to(&inline_key);

        let additional_refs: HashSet<Key> = block_refs
            .into_iter()
            .chain(inline_refs)
            .map(|node_id| graph.key_of(node_id))
            .filter(|k| k != source_key && k != &inline_key)
            .collect();

        for ref_key in additional_refs.iter().sorted() {
            let tree = graph.collect(ref_key);
            let updated = tree
                .remove_inclusion_edges_to(&inline_key)
                .remove_inline_links_to(&inline_key);
            let markdown = updated.iter().to_markdown(&ref_key.parent(), &options);
            result.add_update(ref_key.clone(), markdown);
        }
    }

    Ok(result)
}
