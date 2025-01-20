use std::collections::HashMap;

use itertools::Itertools;

use crate::{
    graph::{GraphContext, GraphPatch},
    model::{
        graph::{GraphNodeIter, Node, TreeIter, TreeNode},
        Key, Markdown, NodeId,
    },
};

pub enum ActionType {
    ListChangeType,            // action for nearest list surround cursor
    ListToSections,            // action for top-level list surrounding cursor
    ReferenceInlineSection,    // action for reference under cursor
    ReferenceInlineQuote,      // action for reference under cursor
    SectionExtract,            // acton for section under cursor
    SectionExtractSubsections, // action for section under cursor
    SectionToList,             // action for section under cursor

    // unstable
    ReferenceInlineList, // action for reference under cursor
    ListDetach,          // action for top-level list surrounding cursor
}

impl ActionType {
    pub fn identifier(&self) -> &'static str {
        match self {
            ActionType::ListChangeType => "refactor.rewrite.list.type",
            ActionType::ListDetach => "refactor.extract.list",
            ActionType::ListToSections => "refactor.rewrite.list.section",
            ActionType::ReferenceInlineSection => "refactor.inline.reference.section",
            ActionType::ReferenceInlineList => "refactor.inline.reference.list",
            ActionType::ReferenceInlineQuote => "refactor.inline.reference.quote",
            ActionType::SectionExtractSubsections => "refactor.extract.subsections",
            ActionType::SectionToList => "refactor.rewrite.secton.list",
            ActionType::SectionExtract => "refactor.extract.section",
        }
    }

    pub fn apply(&self, target_id: NodeId, context: impl GraphContext) -> Option<Action> {
        match self {
            ActionType::ListChangeType => change_list_type(target_id, context),
            ActionType::ListDetach => extract_list(target_id, context),
            ActionType::ListToSections => list_to_sections(target_id, context),
            ActionType::ReferenceInlineSection => inline_section(target_id, context),
            ActionType::ReferenceInlineList => inline_list(target_id, context),
            ActionType::ReferenceInlineQuote => inline_quote(target_id, context),
            ActionType::SectionExtractSubsections => extract_sub_sections(target_id, context),
            ActionType::SectionToList => section_to_list(target_id, context),
            ActionType::SectionExtract => extract_section(target_id, context),
        }
    }
}

pub enum Change {
    Remove(Remove),
    Create(Create),
    Update(Update),
}

pub struct Create {
    pub key: Key,
}
pub struct Update {
    pub key: Key,
    pub markdown: Markdown,
}
pub struct Remove {
    pub key: Key,
}

pub struct Action {
    pub title: String,
    pub changes: Vec<Change>,
    pub action_type: ActionType,
}

pub fn change_list_type(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    context.get_surrounding_list_id(target_id).map(|scope_id| {
        let mut patch = context.patch();
        let key = context.get_key(target_id);

        patch.add_key(&key, context.change_list_type_visitor(&key, scope_id));
        let update = patch.markdown(&key).unwrap();

        Action {
            title: match context.is_bullet_list(scope_id) {
                true => "Change to ordered list".to_string(),
                false => "Change to bullet list".to_string(),
            },
            changes: vec![Change::Update(Update {
                key: key,
                markdown: update,
            })],
            action_type: ActionType::ListChangeType,
        }
    })
}

pub fn list_to_sections(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    context
        .get_top_level_surrounding_list_id(target_id)
        .map(|scope_id| {
            let mut patch = context.patch();
            let key = context.get_key(scope_id);

            patch.add_key(&key, context.unwrap_vistior(&key, scope_id));
            let update = patch.markdown(&key).unwrap();

            Action {
                title: "List to sections".to_string(),
                changes: vec![Change::Update(Update {
                    key: key,
                    markdown: update,
                })],
                action_type: ActionType::ListToSections,
            }
        })
}

pub fn extract_list(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    context.get_surrounding_list_id(target_id).map(|scope_id| {
        let key = context.get_key(scope_id);
        let new_key = context.random_key();

        let mut patch = context.patch();
        patch.add_key(
            &key,
            context.extract_vistior(&key, HashMap::from([(scope_id, new_key.clone())])),
        );

        patch.add_key(&new_key, context.node_visitor(target_id));

        let markdown = patch.markdown(&key).unwrap();
        let new_markdown = patch.markdown(&new_key).unwrap();

        Action {
            title: "Extract list".to_string(),
            changes: vec![
                Change::Create(Create {
                    key: new_key.clone(),
                }),
                Change::Update(Update {
                    key: new_key,
                    markdown: new_markdown,
                }),
                Change::Update(Update {
                    key: key,
                    markdown: markdown,
                }),
            ],
            action_type: ActionType::ListDetach,
        }
    })
}

pub fn extract(node: &TreeNode, extract_id: NodeId, parent_id: NodeId, new_key: &Key) -> TreeNode {
    extract_rec(node, extract_id, parent_id, new_key)
        .first()
        .unwrap()
        .clone()
}

pub fn extract_rec(
    node: &TreeNode,
    extract_id: NodeId,
    parent_id: NodeId,
    new_key: &Key,
) -> Vec<TreeNode> {
    if node.id_eq(parent_id) {
        let mut children = node
            .clone()
            .children
            .into_iter()
            .filter(|child| !child.id_eq(extract_id))
            .collect_vec();

        children.insert(
            node.pre_sub_header_position(),
            TreeNode {
                id: None,
                payload: Node::Reference(new_key.clone(), "".to_string()),
                children: vec![],
            },
        );

        return vec![TreeNode {
            id: node.id,
            payload: node.payload.clone(),
            children,
        }];
    }

    return vec![TreeNode {
        id: node.id,
        payload: node.payload.clone(),
        children: node
            .children
            .iter()
            .map(|child| extract_rec(child, extract_id, parent_id, new_key))
            .flatten()
            .collect(),
    }];
}

pub fn extract_section(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    context
        .get_surrounding_section_id(target_id)
        .filter(|_| context.is_header(target_id))
        .map(|parent_id| {
            let key = context.get_key(target_id);
            let new_key = context.random_key();

            let mut patch = context.patch();

            let tree = TreeNode::from_iter(context.visit(&key)).unwrap();

            let updated_tree = extract(&tree, target_id, parent_id, &new_key);
            patch.add_key(&key, TreeIter::new(&updated_tree));

            patch.add_key(&new_key, context.node_visit_children_of(target_id));

            let markdown = patch.markdown(&key).unwrap();
            let new_markdown = patch.markdown(&new_key).unwrap();

            Action {
                title: "Extract section".to_string(),
                changes: vec![
                    Change::Create(Create {
                        key: new_key.clone(),
                    }),
                    Change::Update(Update {
                        key: new_key,
                        markdown: new_markdown,
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ],
                action_type: ActionType::SectionExtract,
            }
        })
}

pub fn extract_sub_sections(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    Some(target_id)
        .filter(|node_id| context.is_header(*node_id))
        .filter(|node_id| context.get_sub_sections(*node_id).len() > 0)
        .map(|header_id| {
            let key = context.get_key(target_id);
            let sub_sections = context.get_sub_sections(header_id);

            let mut patch = context.patch();
            let mut extracted = HashMap::new();

            for section_id in sub_sections {
                let new_key = context.random_key();
                extracted.insert(section_id, new_key.clone());
                patch.add_key(&new_key, context.node_visit_children_of(section_id));
            }

            patch.add_key(&key, context.extract_vistior(&key, extracted.clone()));

            let mut changes = vec![];

            for new_key in extracted.values() {
                changes.push(Change::Create(Create {
                    key: new_key.clone(),
                }));
                changes.push(Change::Update(Update {
                    key: new_key.clone(),
                    markdown: patch.markdown(&new_key).unwrap(),
                }));
            }

            changes.push(Change::Update(Update {
                key: key.clone(),
                markdown: patch.markdown(&key).unwrap(),
            }));

            Action {
                title: "Extract sub-sections".to_string(),
                changes,
                action_type: ActionType::SectionExtractSubsections,
            }
        })
}

pub fn inline_list(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    Some(target_id)
        .filter(|node_id| context.is_reference(*node_id))
        .map(|reference_id| {
            let key = context.get_key(target_id);
            let mut patch = context.patch();
            patch.add_key(&key, context.inline_vistior(&key, reference_id));
            let markdown = patch.markdown(&key).unwrap();

            Action {
                title: "Inline list".to_string(),
                changes: vec![
                    Change::Remove(Remove {
                        key: context.get_reference_key(reference_id),
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ],
                action_type: ActionType::ReferenceInlineList,
            }
        })
}

pub fn inline_quote(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    Some(target_id)
        .filter(|node_id| context.is_reference(*node_id))
        .map(|reference_id| {
            let key = context.get_key(target_id);
            let mut patch = context.patch();
            patch.add_key(&key, context.inline_quote_vistior(&key, reference_id));
            let markdown = patch.markdown(&key).unwrap();

            Action {
                title: "Inline quote".to_string(),
                changes: vec![
                    Change::Remove(Remove {
                        key: context.get_reference_key(reference_id),
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ],
                action_type: ActionType::ReferenceInlineQuote,
            }
        })
}

pub fn inline_section(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    Some(target_id)
        .filter(|target_id| context.is_reference(*target_id))
        .map(|target_id| {
            let key = context.get_key(target_id);
            let inline_key = context.get_reference_key(target_id);
            let mut patch = context.patch();

            patch.add_key(
                &key,
                context
                    .visit(&key)
                    .collect_tree()
                    .remove_node(target_id)
                    .append_pre_header(
                        context.get_surrounding_section_id(target_id).unwrap(),
                        context.visit(&inline_key).collect_tree(),
                    )
                    .iter(),
            );

            let markdown = patch.markdown(&key).unwrap();

            Action {
                title: "Inline section".to_string(),
                changes: vec![
                    Change::Remove(Remove {
                        key: context.get_reference_key(target_id),
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ],
                action_type: ActionType::ReferenceInlineSection,
            }
        })
}

pub fn section_to_list(target_id: NodeId, context: impl GraphContext) -> Option<Action> {
    Some(target_id)
        .filter(|node_id| context.is_header(*node_id))
        .map(|scope_id| {
            let mut patch = context.patch();
            let key = context.get_key(target_id);

            patch.add_key(&key, context.wrap_vistior(scope_id));
            let update = patch.markdown(&key).unwrap();

            Action {
                title: "Section to list".to_string(),
                changes: vec![Change::Update(Update {
                    key: key,
                    markdown: update,
                })],
                action_type: ActionType::SectionToList,
            }
        })
}
