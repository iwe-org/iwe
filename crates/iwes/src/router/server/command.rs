use std::fmt;
use itertools::Itertools;
use liwe::{
    markdown::MarkdownReader,
    model::{node::NodeIter, Key},
};
use lsp_types::{ApplyWorkspaceEditParams, DocumentChanges, ExecuteCommandParams, WorkspaceEdit};
use serde::{Deserialize, Serialize};

use super::{
    actions::{ActionContext, Change, Create, Update},
    BasePath, ChangeExt,
};

pub struct CommandResult {
    pub edits: Vec<Change>,
}

impl CommandResult {
    pub fn to_workspace_edit(&self, base_path: &BasePath) -> ApplyWorkspaceEditParams {
        ApplyWorkspaceEditParams {
            label: None,
            edit: WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(
                    self.edits
                        .iter()
                        .map(|change| change.to_document_change(base_path))
                        .collect_vec(),
                )),
                ..Default::default()
            },
        }
    }
}

pub enum CommandType {
    Generate,
}

impl fmt::Display for CommandType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandType::Generate => write!(f, "generate")
        }
    }
}

impl CommandType {
    pub fn from_string(s: &str) -> CommandType {
        match s {
            "generate" => CommandType::Generate,
            _ => panic!("Unknown command type"),
        }
    }
}


#[derive(Serialize, Deserialize, Clone)]
pub struct GenerateCommand {
    pub new_key: String,
    pub prompt_key: String,
    pub target_key: String,
}

impl GenerateCommand {
    fn execute(&self, context: impl ActionContext) -> CommandResult {
        let target_key: Key = self.target_key.clone().into();
        let prompt_key: Key = self.prompt_key.clone().into();
        let new_key: Key = self.new_key.clone().into();

        let combined_prompt = format!(
            "{}\n\n{}",
            context
                .squash(&prompt_key, 1)
                .iter()
                .to_markdown(&prompt_key.parent(), context.markdown_options()),
            context
                .squash(&target_key, 1)
                .iter()
                .to_markdown(&target_key.parent(), context.markdown_options())
        );
        let new_content = context.llm_query(combined_prompt, context.default_model());

        let mut patch = context.patch();
        patch.from_markdown(new_key.clone(), &new_content, MarkdownReader::new());
        patch.build_key_from_iter(&target_key, context.collect(&target_key).iter());

        CommandResult {
            edits: vec![
                Change::Create(Create {
                    key: new_key.clone(),
                }),
                Change::Update(Update {
                    key: new_key.clone(),
                    markdown: patch.to_markdown(&new_key),
                }),
                Change::Update(Update {
                    key: target_key.clone(),
                    markdown: patch
                        .to_markdown(&target_key.clone())
                        .replace("[⏳](", "[Generated content]("),
                }),
            ],
        }
    }
}

pub enum Command {
    Generate(GenerateCommand),
}

impl Command {
    pub fn execute(&self, graph: impl ActionContext) -> CommandResult {
        match self {
            Command::Generate(args) => args.execute(graph),
        }
    }
}

impl Command {
    pub fn from_params(params: ExecuteCommandParams) -> Command {
        match params.command.as_str() {
            "generate" => Command::Generate(
                params
                    .arguments
                    .first()
                    .and_then(|arg| serde_json::from_value::<GenerateCommand>(arg.clone()).ok())
                    .unwrap(),
            ),
            _ => panic!("Unknown command type"),
        }
    }
}
