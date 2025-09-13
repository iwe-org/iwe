use std::collections::HashMap;

use liwe::graph::Graph;
use liwe::model::config::{BlockAction, Configuration, MarkdownOptions, Model};
use liwe::model::tree::Tree;
use liwe::model::{Key, Markdown, NodeId};

use lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Mutex;

use super::{BasePath, ChangeExt};

mod attach;
mod delete;
mod extract;
mod inline;
mod list;
mod section;
mod sort;
mod transform;

pub use attach::AttachAction;
pub use delete::DeleteAction;
pub use extract::{SectionExtract, SubSectionsExtract};
pub use inline::InlineAction;
pub use list::{ListChangeType, ListToSections};
pub use section::SectionToList;
pub use sort::SortAction;
pub use transform::TransformBlockAction;

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
    fn get_block_references_to(&self, key: &Key) -> Vec<NodeId>;
    fn get_inline_references_to(&self, key: &Key) -> Vec<NodeId>;
    fn get_ref_text(&self, key: &Key) -> Option<String>;
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

pub type Changes = Vec<Change>;

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
        use itertools::Itertools;
        use lsp_types::{DocumentChanges, WorkspaceEdit};

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

pub enum ActionEnum {
    ListChangeType(ListChangeType),
    ListToSections(ListToSections),
    SectionToList(SectionToList),
    SectionExtract(SectionExtract),
    SubSectionsExtract(SubSectionsExtract),
    TransformBlockAction(TransformBlockAction),
    AttachAction(AttachAction),
    SortAction(SortAction),
    InlineAction(InlineAction),
    DeleteAction(DeleteAction),
}

impl ActionProvider for ActionEnum {
    fn identifier(&self) -> String {
        match self {
            ActionEnum::ListChangeType(inner) => inner.identifier(),
            ActionEnum::ListToSections(inner) => inner.identifier(),
            ActionEnum::SectionToList(inner) => inner.identifier(),
            ActionEnum::SectionExtract(inner) => inner.identifier(),
            ActionEnum::SubSectionsExtract(inner) => inner.identifier(),
            ActionEnum::TransformBlockAction(inner) => inner.identifier(),
            ActionEnum::AttachAction(inner) => inner.identifier(),
            ActionEnum::SortAction(inner) => inner.identifier(),
            ActionEnum::InlineAction(inner) => inner.identifier(),
            ActionEnum::DeleteAction(inner) => inner.identifier(),
        }
    }

    fn action(&self, target_id: NodeId, context: impl ActionContext) -> Option<Action> {
        match self {
            ActionEnum::ListChangeType(inner) => inner.action(target_id, context),
            ActionEnum::ListToSections(inner) => inner.action(target_id, context),
            ActionEnum::SectionToList(inner) => inner.action(target_id, context),
            ActionEnum::SectionExtract(inner) => inner.action(target_id, context),
            ActionEnum::SubSectionsExtract(inner) => inner.action(target_id, context),
            ActionEnum::TransformBlockAction(inner) => inner.action(target_id, context),
            ActionEnum::AttachAction(inner) => inner.action(target_id, context),
            ActionEnum::SortAction(inner) => inner.action(target_id, context),
            ActionEnum::InlineAction(inner) => inner.action(target_id, context),
            ActionEnum::DeleteAction(inner) => inner.action(target_id, context),
        }
    }

    fn changes(&self, target_id: NodeId, context: impl ActionContext) -> Option<Changes> {
        match self {
            ActionEnum::ListChangeType(inner) => inner.changes(target_id, context),
            ActionEnum::ListToSections(inner) => inner.changes(target_id, context),
            ActionEnum::SectionToList(inner) => inner.changes(target_id, context),
            ActionEnum::SectionExtract(inner) => inner.changes(target_id, context),
            ActionEnum::SubSectionsExtract(inner) => inner.changes(target_id, context),
            ActionEnum::TransformBlockAction(inner) => inner.changes(target_id, context),
            ActionEnum::AttachAction(inner) => inner.changes(target_id, context),
            ActionEnum::SortAction(inner) => inner.changes(target_id, context),
            ActionEnum::InlineAction(inner) => inner.changes(target_id, context),
            ActionEnum::DeleteAction(inner) => inner.changes(target_id, context),
        }
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

pub fn all_actions() -> Vec<ActionEnum> {
    vec![
        ActionEnum::ListChangeType(ListChangeType {}),
        ActionEnum::ListToSections(ListToSections {}),
        ActionEnum::SectionToList(SectionToList {}),
        ActionEnum::SectionExtract(SectionExtract {}),
        ActionEnum::SubSectionsExtract(SubSectionsExtract {}),
    ]
}

pub fn all_action_types(configuration: &Configuration) -> Vec<ActionEnum> {
    let mut actions = vec![
        ActionEnum::ListChangeType(ListChangeType {}),
        ActionEnum::ListToSections(ListToSections {}),
        ActionEnum::SectionToList(SectionToList {}),
        ActionEnum::SectionExtract(SectionExtract {}),
        ActionEnum::SubSectionsExtract(SubSectionsExtract {}),
        ActionEnum::DeleteAction(DeleteAction {}),
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
                BlockAction::Inline(inline) => {
                    let action = ActionEnum::InlineAction(InlineAction {
                        title: inline.title.clone(),
                        identifier: identifier.clone(),
                        inline_type: inline.inline_type.clone(),
                        keep_target: inline.keep_target.unwrap_or(false),
                    });
                    action
                }
            }),
    );

    actions
}
