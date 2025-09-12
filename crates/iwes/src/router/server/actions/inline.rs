use liwe::model::config::InlineType;
use liwe::model::node::Node;
use liwe::model::node::NodeIter;
use liwe::model::tree::Tree;
use liwe::model::NodeId;

use super::{Action, ActionContext, ActionProvider, Change, Changes, Remove, Update};

pub struct InlineAction {
    pub title: String,
    pub identifier: String,
    pub inline_type: InlineType,
}

impl ActionProvider for InlineAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .and_then(|target_id| {
                let inline_key = context.collect(&key).find_reference_key(target_id);

                match self.inline_type {
                    InlineType::Section => context
                        .collect(&key)
                        .get_surrounding_section_id(target_id)
                        .map(|section_id| {
                            let markdown = context
                                .collect(&key)
                                .remove_node(target_id)
                                .append_pre_header(section_id, context.collect(&inline_key))
                                .iter()
                                .to_markdown(&key.parent(), context.markdown_options());

                            vec![
                                Change::Remove(Remove {
                                    key: context.collect(&key).find_reference_key(target_id),
                                }),
                                Change::Update(Update {
                                    key: key,
                                    markdown: markdown,
                                }),
                            ]
                        }),
                    InlineType::Quote => {
                        let quote = Tree {
                            id: None,
                            node: Node::Quote(),
                            children: context.collect(&inline_key).children.clone(),
                        };

                        let markdown = context
                            .collect(&key)
                            .replace(target_id, &quote)
                            .iter()
                            .to_markdown(&key.parent(), context.markdown_options());

                        Some(vec![
                            Change::Remove(Remove {
                                key: context.collect(&key).find_reference_key(target_id),
                            }),
                            Change::Update(Update {
                                key: key,
                                markdown: markdown,
                            }),
                        ])
                    }
                }
            })
    }
}

pub struct ReferenceInlineSection {}

impl ActionProvider for ReferenceInlineSection {
    fn identifier(&self) -> String {
        return "refactor.inline.reference.section".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| Action {
                title: "Inline section".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .and_then(|target_id| {
                let inline_key = context.collect(&key).find_reference_key(target_id);

                context
                    .collect(&key)
                    .get_surrounding_section_id(target_id)
                    .map(|section_id| {
                        let markdown = context
                            .collect(&key)
                            .remove_node(target_id)
                            .append_pre_header(section_id, context.collect(&inline_key))
                            .iter()
                            .to_markdown(&key.parent(), context.markdown_options());

                        vec![
                            Change::Remove(Remove {
                                key: context.collect(&key).find_reference_key(target_id),
                            }),
                            Change::Update(Update {
                                key: key,
                                markdown: markdown,
                            }),
                        ]
                    })
            })
    }
}

pub struct ReferenceInlineQuote {}

impl ActionProvider for ReferenceInlineQuote {
    fn identifier(&self) -> String {
        return "refactor.inline.reference.quote".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|_| Action {
                title: "Inline quote".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|target_id| tree.get(*target_id).is_reference())
            .map(|reference_id| {
                let inline_key = context.collect(&key).find_reference_key(reference_id);

                let quote = Tree {
                    id: None,
                    node: Node::Quote(),
                    children: context.collect(&inline_key).children.clone(),
                };

                let markdown = context
                    .collect(&key)
                    .replace(reference_id, &quote)
                    .iter()
                    .to_markdown(&key.parent(), context.markdown_options());

                vec![
                    Change::Remove(Remove {
                        key: context.collect(&key).find_reference_key(reference_id),
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ]
            })
    }
}
