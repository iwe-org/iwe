use std::collections::HashMap;

use itertools::Itertools;

use liwe::graph::Graph;
use liwe::markdown::MarkdownReader;
use liwe::model::config::{BlockAction, Configuration, Context, MarkdownOptions, Model};
use liwe::model::node::{Node, NodeIter, NodePointer, Reference, ReferenceType};
use liwe::model::tree::Tree;
use liwe::model::{Key, Markdown, NodeId};

use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, DocumentChanges, WorkspaceEdit};
use once_cell::sync::Lazy;
use serde_json::Value;

use minijinja::{context, Environment};
use std::sync::Mutex;

use chrono::Local;

use super::llm::templates;
use super::{BasePath, ChangeExt};

pub trait ActionContext {
    fn key_of(&self, node_id: NodeId) -> Key;
    fn key_exists(&self, key: &Key) -> bool;
    fn collect(&self, key: &Key) -> Tree;
    fn squash(&self, key: &Key, depth: u8) -> Tree;
    fn random_key(&self, parent: &str) -> Key;
    fn markdown_options(&self) -> &MarkdownOptions;
    fn llm_query(&self, prompt: String, model: &Model) -> String;
    fn default_model(&self) -> &Model;
    fn patch(&self) -> Graph;
}

pub fn all_action_types(configuration: &Configuration) -> Vec<ActionEnum> {
    let mut actions = vec![
        ActionEnum::ListChangeType(ListChangeType {}),
        ActionEnum::ListToSections(ListToSections {}),
        ActionEnum::ReferenceInlineSection(ReferenceInlineSection {}),
        ActionEnum::ReferenceInlineQuote(ReferenceInlineQuote {}),
        ActionEnum::SectionToList(SectionToList {}),
        ActionEnum::SectionExtract(SectionExtract {}),
        ActionEnum::SubSectionsExtract(SubSectionsExtract {}),
    ];

    actions.extend(
        configuration
            .actions
            .iter()
            .map(|(identifier, action)| match action {
                BlockAction::Transform(transform) => {
                    let action = ActionEnum::TransformBlockAction(TransformBlockAction {
                        title: transform.title.clone(),
                        identifier: identifier.clone(),
                        model_parameters: configuration
                            .models
                            .get(&transform.model)
                            .expect(
                                format!("Model {} not found in configuration", transform.model)
                                    .as_str(),
                            )
                            .clone(),
                        prompt_template: transform.prompt_template.clone(),
                        context: transform.context.clone(),
                    });
                    action
                }
                BlockAction::Attach(attach) => {
                    let action = ActionEnum::AttachAction(AttachAction {
                        title: attach.title.clone(),
                        identifier: identifier.clone(),
                        document_template: attach.document_template.clone(),
                        key_template: attach.key_template.clone(),
                        markdown_date_format: configuration
                            .clone()
                            .markdown
                            .date_format
                            .unwrap_or("%b %d, %Y".into()),
                        key_date_format: configuration
                            .clone()
                            .library
                            .date_format
                            .unwrap_or("%Y-%m-%d".into()),
                    });
                    action
                }
                BlockAction::Sort(sort) => {
                    let action = ActionEnum::SortAction(SortAction {
                        title: sort.title.clone(),
                        identifier: identifier.clone(),
                        reverse: sort.reverse.unwrap_or(false),
                    });
                    action
                }
            }),
    );

    actions
}

pub enum ActionEnum {
    ListChangeType(ListChangeType),
    ListToSections(ListToSections),
    ReferenceInlineSection(ReferenceInlineSection),
    ReferenceInlineQuote(ReferenceInlineQuote),
    SectionToList(SectionToList),
    SectionExtract(SectionExtract),
    SubSectionsExtract(SubSectionsExtract),
    TransformBlockAction(TransformBlockAction),
    AttachAction(AttachAction),
    SortAction(SortAction),
}

impl ActionEnum {}

pub fn all_actions() -> Vec<ActionEnum> {
    vec![
        ActionEnum::ListChangeType(ListChangeType {}),
        ActionEnum::ListToSections(ListToSections {}),
        ActionEnum::ReferenceInlineSection(ReferenceInlineSection {}),
        ActionEnum::ReferenceInlineQuote(ReferenceInlineQuote {}),
        ActionEnum::SectionToList(SectionToList {}),
        ActionEnum::SectionExtract(SectionExtract {}),
        ActionEnum::SubSectionsExtract(SubSectionsExtract {}),
    ]
}

impl ActionProvider for ActionEnum {
    fn identifier(&self) -> String {
        match self {
            ActionEnum::ListChangeType(inner) => inner.identifier(),
            ActionEnum::ListToSections(inner) => inner.identifier(),
            ActionEnum::ReferenceInlineSection(inner) => inner.identifier(),
            ActionEnum::ReferenceInlineQuote(inner) => inner.identifier(),
            ActionEnum::SectionToList(inner) => inner.identifier(),
            ActionEnum::SectionExtract(inner) => inner.identifier(),
            ActionEnum::SubSectionsExtract(inner) => inner.identifier(),
            ActionEnum::TransformBlockAction(inner) => inner.identifier(),
            ActionEnum::AttachAction(inner) => inner.identifier(),
            ActionEnum::SortAction(inner) => inner.identifier(),
        }
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        match self {
            ActionEnum::ListChangeType(inner) => inner.action(target_id, context),
            ActionEnum::ListToSections(inner) => inner.action(target_id, context),
            ActionEnum::ReferenceInlineSection(inner) => inner.action(target_id, context),
            ActionEnum::ReferenceInlineQuote(inner) => inner.action(target_id, context),
            ActionEnum::SectionToList(inner) => inner.action(target_id, context),
            ActionEnum::SectionExtract(inner) => inner.action(target_id, context),
            ActionEnum::SubSectionsExtract(inner) => inner.action(target_id, context),
            ActionEnum::TransformBlockAction(inner) => inner.action(target_id, context),
            ActionEnum::AttachAction(inner) => inner.action(target_id, context),
            ActionEnum::SortAction(inner) => inner.action(target_id, context),
        }
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        match self {
            ActionEnum::ListChangeType(inner) => inner.changes(target_id, context),
            ActionEnum::ListToSections(inner) => inner.changes(target_id, context),
            ActionEnum::ReferenceInlineSection(inner) => inner.changes(target_id, context),
            ActionEnum::ReferenceInlineQuote(inner) => inner.changes(target_id, context),
            ActionEnum::SectionToList(inner) => inner.changes(target_id, context),
            ActionEnum::SectionExtract(inner) => inner.changes(target_id, context),
            ActionEnum::SubSectionsExtract(inner) => inner.changes(target_id, context),
            ActionEnum::TransformBlockAction(inner) => inner.changes(target_id, context),
            ActionEnum::AttachAction(inner) => inner.changes(target_id, context),
            ActionEnum::SortAction(inner) => inner.changes(target_id, context),
        }
    }
}

pub struct Action {
    pub title: String,
    pub identifier: String,
    pub target_id: NodeId,
}

pub enum Change {
    Remove(Remove),
    Create(Create),
    Update(Update),
}

type Changes = Vec<Change>;

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

impl Action {
    pub fn to_code_action(&self) -> CodeActionOrCommand {
        CodeActionOrCommand::CodeAction(CodeAction {
            title: self.title.to_string(),
            kind: Some(identifier_to_action_kind(self.identifier.to_string())),
            data: Some(Value::Number(self.target_id.into())),
            ..Default::default()
        })
    }

    pub fn resolve_code_action(
        &self,
        base_path: &BasePath,
        changes: Changes,
    ) -> CodeActionOrCommand {
        CodeActionOrCommand::CodeAction(CodeAction {
            title: self.title.to_string(),
            kind: Some(identifier_to_action_kind(self.identifier.to_string())),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(
                    changes
                        .iter()
                        .map(|change| change.to_document_change(base_path))
                        .collect_vec(),
                )),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

pub trait ActionProvider {
    fn identifier(&self) -> String;
    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action>;
    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes>;

    fn action_kind(&self) -> CodeActionKind {
        identifier_to_action_kind(self.identifier())
    }
}

pub struct AttachAction {
    pub title: String,
    pub identifier: String,

    pub key_template: String,
    pub document_template: String,
    pub markdown_date_format: String,
    pub key_date_format: String,
}

impl AttachAction {
    fn format_target_key(&self) -> Key {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.key_date_format).to_string();

        Key::name(
            &Environment::new()
                .template_from_str(&self.key_template)
                .expect("correct template")
                .render(context! {
                today => formatted,
                })
                .expect("template to work"),
        )
    }

    fn format_target_document(&self, content: String) -> String {
        let date = Local::now().date_naive();
        let formatted = date.format(&self.markdown_date_format).to_string();
        Environment::new()
            .template_from_str(&self.document_template)
            .expect("correct template")
            .render(context! {
            today => formatted,
            content => content
            })
            .expect("template to work")
    }
}

impl ActionProvider for AttachAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let reference_key = context.collect(&key).reference_key(target_id);
        let attach_to_key = self.format_target_key();

        if context.key_exists(&attach_to_key) {
            if context
                .collect(&attach_to_key)
                .get_all_block_reference_keys()
                .contains(&reference_key)
            {
                return None;
            }
        }

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|target| target.is_reference())
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let reference_key = context.collect(&key).reference_key(target_id);
        let attach_to_key = self.format_target_key();
        let reference = Tree {
            id: None,
            node: Node::Reference(Reference {
                key: reference_key,
                text: context.collect(&key).reference_text(target_id),
                reference_type: ReferenceType::Regular,
            }),
            children: vec![],
        };

        if context.key_exists(&attach_to_key) {
            let tree = context.collect(&attach_to_key);

            let updated = tree.attach(reference);

            Some(vec![Change::Update(Update {
                key: attach_to_key.clone(),
                markdown: updated
                    .iter()
                    .to_markdown(&attach_to_key.parent(), &context.markdown_options()),
            })])
        } else {
            Some(vec![
                Change::Create(Create {
                    key: attach_to_key.clone(),
                }),
                Change::Update(Update {
                    key: attach_to_key.clone(),
                    markdown: self.format_target_document(
                        reference
                            .iter()
                            .to_markdown(&attach_to_key.parent(), &context.markdown_options()),
                    ),
                }),
            ])
        }
    }
}

pub struct TransformBlockAction {
    pub title: String,
    pub identifier: String,

    pub model_parameters: Model,

    pub prompt_template: String,
    pub context: Context,
}

impl ActionProvider for TransformBlockAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, _: impl ActionContext) -> Option<Action> {
        Some(Action {
            title: self.title.clone(),
            identifier: self.identifier(),
            target_id,
        })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

        let tree = &context.collect(&key);

        let target_id = tree
            .get_surrounding_top_level_block(target_id)
            .unwrap_or(target_id);

        let prompt = templates::block_action_prompt(&self.prompt_template, target_id, tree);

        let generated = context.llm_query(prompt, &self.model_parameters);

        let mut patch = context.patch();

        patch.from_markdown("new".into(), &generated, MarkdownReader::new());
        let tree = patch.maybe_key(&"new".into()).unwrap().collect_tree();

        let markdown = context
            .collect(&key)
            .replace(target_id, &tree)
            .iter()
            .to_markdown(&key.parent(), &context.markdown_options());

        Some(vec![Change::Update(Update { key, markdown })])
    }
}

pub struct SortAction {
    pub title: String,
    pub identifier: String,
    pub reverse: bool,
}

impl ActionProvider for SortAction {
    fn identifier(&self) -> String {
        format!("custom.{}", self.identifier.to_string())
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .filter(|scope_id| {
                // Only offer the action if the list is not already sorted in the desired order
                !context.collect(&key).is_sorted(*scope_id, self.reverse)
            })
            .map(|_| Action {
                title: self.title.clone(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .sort_children(scope_id, self.reverse)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}

pub struct SectionExtract {}

impl SectionExtract {
    fn extract(node: &Tree, extract_id: NodeId, parent_id: NodeId, new_key: &Key) -> Tree {
        Self::extract_rec(node, extract_id, parent_id, new_key)
            .first()
            .unwrap()
            .clone()
    }

    fn extract_rec(tree: &Tree, extract_id: NodeId, parent_id: NodeId, new_key: &Key) -> Vec<Tree> {
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
                        text: tree
                            .find_id(extract_id)
                            .expect("to have node")
                            .node
                            .plain_text(),
                        reference_type: ReferenceType::Regular,
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

        return vec![Tree {
            id: tree.id,
            node: tree.node.clone(),
            children: tree
                .children
                .iter()
                .map(|child| Self::extract_rec(child, extract_id, parent_id, new_key))
                .flatten()
                .collect(),
        }];
    }
}

impl ActionProvider for SectionExtract {
    fn identifier(&self) -> String {
        return "refactor.extract.section".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|_| Action {
                title: "Extract section".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_section_id(target_id)
            .filter(|_| tree.is_header(target_id))
            .map(|parent_id| {
                let new_key = context.random_key(&key.parent());

                let tree = context.collect(&key);

                let updated_tree = Self::extract(&tree, target_id, parent_id, &new_key);

                let markdown = updated_tree
                    .iter()
                    .to_markdown(&key.parent(), context.markdown_options());
                let new_markdown = tree
                    .get(target_id)
                    .iter()
                    .to_markdown(&new_key.parent(), context.markdown_options());

                vec![
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
                ]
            })
    }
}

pub struct SubSectionsExtract {}

impl ActionProvider for SubSectionsExtract {
    fn identifier(&self) -> String {
        return "refactor.extract.subsections".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .map(|_| Action {
                title: "Extract sub-sections".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);

        context
            .collect(&key)
            .find_id(target_id)
            .filter(|tree| tree.is_section())
            .filter(|tree| tree.children.iter().any(|child| child.is_section()))
            .map(|tree| {
                let sub_sections = tree
                    .children
                    .iter()
                    .filter(|child| child.is_section())
                    .map(|child| child.id.unwrap())
                    .collect_vec();

                let mut extracted = HashMap::new();
                let mut changes = vec![];

                for section_id in sub_sections {
                    let new_key = context.random_key(&key.parent());

                    extracted.insert(
                        section_id,
                        (
                            new_key.clone(),
                            tree.find_id(section_id)
                                .map(|n| n.node.plain_text())
                                .unwrap_or_default(),
                        ),
                    );
                    changes.push(Change::Create(Create {
                        key: new_key.clone(),
                    }));
                    changes.push(Change::Update(Update {
                        key: new_key.clone(),
                        markdown: tree
                            .find_id(section_id)
                            .expect("to have section")
                            .iter()
                            .to_markdown(&new_key.parent(), context.markdown_options()),
                    }));
                }

                changes.push(Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .extract_sections(extracted.clone())
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                }));

                changes
            })
    }
}

pub struct ListChangeType {}

impl ActionProvider for ListChangeType {
    fn identifier(&self) -> String {
        return "refactor.rewrite.list.type".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| Action {
                title: match tree.find_id(scope_id).map(|n| n.is_bullet_list()).unwrap() {
                    true => "Change to ordered list".to_string(),
                    false => "Change to bullet list".to_string(),
                },
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        context
            .collect(&key)
            .get_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .change_list_type(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}

pub struct ListToSections {}
impl ActionProvider for ListToSections {
    fn identifier(&self) -> String {
        return "refactor.rewrite.list.section".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = &context.key_of(target_id);
        context
            .collect(&key)
            .get_top_level_surrounding_list_id(target_id)
            .map(|_| Action {
                title: "List to sections".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = &context.key_of(target_id);
        context
            .collect(&key)
            .get_top_level_surrounding_list_id(target_id)
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .unwrap_list(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
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
                let inline_key = context.collect(&key).reference_key(target_id);

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
                                key: context.collect(&key).reference_key(target_id),
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
                let inline_key = context.collect(&key).reference_key(reference_id);

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
                        key: context.collect(&key).reference_key(reference_id),
                    }),
                    Change::Update(Update {
                        key: key,
                        markdown: markdown,
                    }),
                ]
            })
    }
}

pub struct SectionToList {}
impl ActionProvider for SectionToList {
    fn identifier(&self) -> String {
        return "refactor.rewrite.section.list".to_string();
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|node_id| tree.is_header(*node_id))
            .map(|_| Action {
                title: "Section to list".to_string(),
                identifier: self.identifier(),
                target_id,
            })
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        let key = context.key_of(target_id);
        let tree = context.collect(&key);

        Some(target_id)
            .filter(|node_id| tree.is_header(*node_id))
            .map(|scope_id| {
                vec![Change::Update(Update {
                    key: key.clone(),
                    markdown: context
                        .collect(&key)
                        .wrap_into_list(scope_id)
                        .iter()
                        .to_markdown(&key.parent(), context.markdown_options()),
                })]
            })
    }
}

static CODE_ACTION_MAP: Lazy<Mutex<HashMap<String, CodeActionKind>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn identifier_to_action_kind(identifier: String) -> CodeActionKind {
    let mut map = CODE_ACTION_MAP.lock().unwrap();
    map.entry(identifier.clone())
        .or_insert_with(|| CodeActionKind::new(identifier.clone().leak()))
        .clone()
}
