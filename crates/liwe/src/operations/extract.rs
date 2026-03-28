use itertools::Itertools;

use crate::graph::{Graph, GraphContext};
use crate::model::config::LinkType;
use crate::model::node::{Node, NodeIter, Reference, ReferenceType};
use crate::model::tree::Tree;
use crate::model::{Key, NodeId};

use super::changes::{Changes, OperationError};
use super::config::ExtractConfig;
use super::util::{format_target_key, KeyFormatContext};

pub fn extract(
    graph: &Graph,
    source_key: &Key,
    target_id: NodeId,
    config: &ExtractConfig,
) -> Result<Changes, OperationError> {
    if graph.get_node_id(source_key).is_none() {
        return Err(OperationError::NotFound(source_key.clone()));
    }

    let tree = graph.collect(source_key);

    let parent_id = tree
        .get_surrounding_section_id(target_id)
        .ok_or(OperationError::NoParentSection)?;

    if !tree.is_header(target_id) {
        return Err(OperationError::InvalidTarget(
            "Target must be a section header".to_string(),
        ));
    }

    let id = graph
        .unique_ids(&source_key.parent(), 1)
        .first()
        .expect("to have one")
        .to_string();

    let section_title = tree
        .find_id(target_id)
        .map(|t| t.node.plain_text())
        .unwrap_or_default();

    let parent_title = tree
        .get_surrounding_section_id(target_id)
        .and_then(|pid| tree.find_id(pid))
        .map(|t| t.node.plain_text());

    let fmt_ctx = KeyFormatContext {
        id: &id,
        title: section_title.clone(),
        parent_title,
        parent_key: Some(source_key.parent()),
        source_key,
        source_title: graph.get_ref_text(source_key),
    };

    let new_key = format_target_key(&config.key_template, &config.key_date_format, config.locale, &fmt_ctx, graph);

    let options = graph.markdown_options();

    let extracted = tree.get(target_id);
    let new_markdown = extracted.iter().to_markdown(&new_key.parent(), &options);

    let reference_type = match &config.link_type {
        Some(LinkType::WikiLink) => ReferenceType::WikiLink,
        Some(LinkType::Markdown) | None => ReferenceType::Regular,
    };

    let updated_tree =
        extract_section(&tree, target_id, parent_id, &new_key, &section_title, reference_type);

    let source_markdown = updated_tree
        .iter()
        .to_markdown(&source_key.parent(), &options);

    let mut result = Changes::default();
    result.add_create(new_key.clone(), new_markdown);
    result.add_update(source_key.clone(), source_markdown);

    Ok(result)
}

fn extract_section(
    tree: &Tree,
    extract_id: NodeId,
    parent_id: NodeId,
    new_key: &Key,
    title: &str,
    reference_type: ReferenceType,
) -> Tree {
    extract_section_rec(tree, extract_id, parent_id, new_key, title, reference_type)
        .first()
        .unwrap()
        .clone()
}

fn extract_section_rec(
    tree: &Tree,
    extract_id: NodeId,
    parent_id: NodeId,
    new_key: &Key,
    title: &str,
    reference_type: ReferenceType,
) -> Vec<Tree> {
    if tree.id_eq(parent_id) {
        let mut children = tree
            .clone()
            .children
            .into_iter()
            .filter(|child| !child.id_eq(extract_id))
            .collect_vec();

        children.insert(
            tree.pre_sub_header_position(),
            Tree {
                id: None,
                node: Node::Reference(Reference {
                    key: new_key.clone(),
                    text: title.to_string(),
                    reference_type,
                }),
                children: vec![],
            },
        );

        return vec![Tree {
            id: tree.id,
            node: tree.node.clone(),
            children,
        }];
    }

    vec![Tree {
        id: tree.id,
        node: tree.node.clone(),
        children: tree
            .children
            .iter()
            .flat_map(|child| {
                extract_section_rec(child, extract_id, parent_id, new_key, title, reference_type)
            })
            .collect(),
    }]
}

pub fn extract_all(
    graph: &Graph,
    source_key: &Key,
    parent_id: NodeId,
    config: &ExtractConfig,
) -> Result<Changes, OperationError> {
    if graph.get_node_id(source_key).is_none() {
        return Err(OperationError::NotFound(source_key.clone()));
    }

    let tree = graph.collect(source_key);

    if !tree.is_header(parent_id) {
        return Err(OperationError::InvalidTarget(
            "Target must be a section header".to_string(),
        ));
    }

    let parent_tree = tree.find_id(parent_id).ok_or(OperationError::InvalidTarget(
        "Parent section not found".to_string(),
    ))?;

    let subsection_ids: Vec<NodeId> = parent_tree
        .children
        .iter()
        .filter(|child| child.is_section())
        .filter_map(|child| child.id)
        .collect();

    if subsection_ids.is_empty() {
        return Err(OperationError::InvalidTarget(
            "No subsections to extract".to_string(),
        ));
    }

    let num_sections = subsection_ids.len();
    let ids = graph.unique_ids(&source_key.parent(), num_sections);
    let options = graph.markdown_options();

    let mut result = Changes::default();
    let mut current_tree = tree.clone();
    let mut generated_keys: Vec<Key> = Vec::new();

    for (idx, section_id) in subsection_ids.iter().enumerate() {
        let section_title = current_tree
            .find_id(*section_id)
            .map(|t| t.node.plain_text())
            .unwrap_or_default();

        let parent_title = current_tree
            .find_id(parent_id)
            .map(|t| t.node.plain_text());

        let fmt_ctx = KeyFormatContext {
            id: &ids[idx],
            title: section_title.clone(),
            parent_title,
            parent_key: Some(source_key.parent()),
            source_key,
            source_title: graph.get_ref_text(source_key),
        };

        let base_key =
            format_target_key(&config.key_template, &config.key_date_format, config.locale, &fmt_ctx, graph);

        let new_key = ensure_unique_key_in_batch(&base_key, graph, &generated_keys);
        generated_keys.push(new_key.clone());

        let extracted = current_tree.get(*section_id);
        let new_markdown = extracted.iter().to_markdown(&new_key.parent(), &options);

        let reference_type = match &config.link_type {
            Some(LinkType::WikiLink) => ReferenceType::WikiLink,
            Some(LinkType::Markdown) | None => ReferenceType::Regular,
        };

        current_tree = extract_section(
            &current_tree,
            *section_id,
            parent_id,
            &new_key,
            &section_title,
            reference_type,
        );

        result.add_create(new_key, new_markdown);
    }

    let source_markdown = current_tree
        .iter()
        .to_markdown(&source_key.parent(), &options);
    result.add_update(source_key.clone(), source_markdown);

    Ok(result)
}

fn ensure_unique_key_in_batch(base_key: &Key, graph: &Graph, generated_keys: &[Key]) -> Key {
    let mut candidate_key = base_key.clone();
    let mut counter = 1;

    while graph.get_node_id(&candidate_key).is_some() || generated_keys.contains(&candidate_key) {
        let suffixed_name = format!("{}-{}", base_key, counter);
        candidate_key = Key::name(&suffixed_name);
        counter += 1;
    }

    candidate_key
}
