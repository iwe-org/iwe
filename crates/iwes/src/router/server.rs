use std::fs;
use std::path::Path;

use action::all_action_types;
use action::ActionContext;
use action::ActionProvider;

use command::CommandType;
use command::GenerateCommand;
use itertools::Itertools;
use liwe::graph::path::NodePath;

use liwe::graph::Graph;
use liwe::model::config::Configuration;
use liwe::model::config::MarkdownOptions;
use liwe::model::config::Model;
use liwe::model::node::NodePointer;
use liwe::model::tree::Tree;
use liwe::model::NodeId;
use lsp_server::ResponseError;
use lsp_types::*;

use liwe::graph::GraphContext;
use liwe::graph::SearchPath;
use liwe::model::Key;
use liwe::model::{self, InlineRange};

use liwe::parser::Parser;
use relative_path::RelativePath;

use super::LspClient;
use super::ServerConfig;
use liwe::database::Database;
use liwe::database::DatabaseContext;

use self::extensions::*;

pub mod action;
pub mod command;
mod extensions;
mod llm;

pub struct Server {
    base_path: BasePath,
    database: Database,
    lsp_client: LspClient,
    configuration: Configuration,
}

pub struct BasePath {
    base_path: String,
}

impl BasePath {
    fn key_to_url(&self, key: &Key) -> Url {
        Url::parse(&self.base_path)
            .unwrap()
            .join(&key.to_path())
            .expect("to work")
    }

    fn relative_to_full_path(&self, url: &str) -> Url {
        Url::parse(&self.base_path)
            .unwrap()
            .join(&format!("{}.md", url.trim_end_matches(".md")))
            .expect("to work")
    }

    fn name_to_url(&self, key: &str) -> Url {
        Url::parse(&format!("{}{}.md", self.base_path, key)).unwrap()
    }

    fn url_to_key(&self, url: &Url) -> Key {
        Key::from_file_name(
            &url.to_string()
                .trim_start_matches(&self.base_path)
                .to_string(),
        )
    }
}

impl DatabaseContext for &Server {
    fn parser(&self, id: &Key) -> Option<Parser> {
        self.database().parser(id)
    }

    fn lines(&self, key: &Key) -> u32 {
        self.database().lines(key)
    }
}

impl Server {
    pub fn new(config: ServerConfig) -> Server {
        Server {
            base_path: BasePath {
                base_path: format!("file://{}/", config.base_path),
            },
            database: Database::new(
                config.state,
                config.sequential_ids.unwrap_or(false),
                config.configuration.markdown.clone(),
            ),
            lsp_client: config.lsp_client,
            configuration: config.configuration,
        }
    }
    pub fn database(&self) -> impl DatabaseContext + '_ {
        &self.database
    }

    pub fn handle_did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        params.text.map(|text| {
            self.database.update_document(
                self.base_path.url_to_key(&params.text_document.uri.clone()),
                text,
            )
        });
    }

    pub fn handle_did_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
        self.database.update_document(
            self.base_path.url_to_key(&params.text_document.uri.clone()),
            params.content_changes.first().unwrap().text.clone(),
        );
    }

    fn handle_plus_completions(&self, params: CompletionParams) -> Vec<CompletionItem> {
        let current_key = params
            .text_document_position
            .text_document
            .uri
            .to_key(&self.base_path);

        let new_key = self.database.graph().random_key(&current_key.parent());
        let keys = self.database.graph().keys();
        keys.iter()
            .filter(|key| {
                self.configuration
                    .prompt_key_prefix
                    .clone()
                    .map(|prefix| key.relative_path.starts_with(&prefix))
                    .unwrap_or(false)
            })
            .map(|key| {
                let command = GenerateCommand {
                    new_key: new_key.to_string(),
                    prompt_key: key.to_string(),
                    target_key: current_key.to_string(),
                };

                CompletionItem {
                    preselect: Some(true),
                    label: format!(
                        "🤖 {}",
                        self.database.graph().get_ref_text(key).unwrap_or_default()
                    ),
                    insert_text: Some(format!("[⏳]({})", new_key)),
                    filter_text: Some(format!(
                        "_{}",
                        self.database.graph().get_ref_text(key).unwrap_or_default()
                    )),
                    sort_text: Some(self.database.graph().get_ref_text(key).unwrap_or_default()),
                    command: Some(Command {
                        title: "generate".into(),
                        command: CommandType::Generate.to_string().into(),
                        arguments: Some(vec![serde_json::to_value(command).unwrap()]),
                    }),
                    documentation: None,
                    ..Default::default()
                }
            })
            .sorted_by(|a, b| a.label.cmp(&b.label))
            .collect_vec()
    }

    pub fn handle_link_completion(&self, params: CompletionParams) -> Vec<CompletionItem> {
        let current_key = params
            .text_document_position
            .text_document
            .uri
            .to_key(&self.base_path);

        self.database
            .graph()
            .keys()
            .iter()
            .map(|key| {
                key.to_completion(
                    &current_key.parent(),
                    self.database.graph(),
                    &self.base_path,
                )
            })
            .sorted_by(|a, b| a.label.cmp(&b.label))
            .collect_vec()
    }

    pub fn handle_completion(&self, params: CompletionParams) -> CompletionResponse {
        CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: self
                .handle_plus_completions(params.clone())
                .into_iter()
                .chain(self.handle_link_completion(params).into_iter())
                .collect_vec(),
        })
    }

    pub fn resolve_completion(&self, completion: CompletionItem) -> CompletionItem {
        completion
    }

    pub fn handle_workspace_command(
        &self,
        params: ExecuteCommandParams,
    ) -> ApplyWorkspaceEditParams {
        command::Command::from_params(params)
            .execute(self)
            .to_workspace_edit(&self.base_path)
    }

    pub fn handle_workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> WorkspaceSymbolResponse {
        self.database
            .global_search(&params.query)
            .iter()
            .map(|p| path_to_symbol(p, self.database.graph(), &self.base_path))
            .filter(|p| !p.name.is_empty())
            .collect_vec()
            .to_response()
    }

    pub fn handle_goto_definition(&self, params: GotoDefinitionParams) -> GotoDefinitionResponse {
        let relative_to = params
            .text_document_position_params
            .text_document
            .uri
            .to_key(&self.base_path)
            .parent();

        self.parser(
            &params
                .text_document_position_params
                .text_document
                .uri
                .to_key(&self.base_path),
        )
        .and_then(|parser| {
            parser.url_at(to_position(
                (&params).text_document_position_params.position,
            ))
        })
        .map(|url| {
            let relative_url = RelativePath::new(&relative_to).join(url).to_string();
            GotoDefinitionResponse::Scalar(Location::new(
                self.base_path.relative_to_full_path(&relative_url),
                Range::default(),
            ))
        })
        .unwrap_or(GotoDefinitionResponse::Array(vec![]))
    }

    pub fn handle_document_formatting(&self, params: DocumentFormattingParams) -> Vec<TextEdit> {
        let key = params.text_document.uri.to_key(&self.base_path);

        let mut patch = self.database.graph().new_patch();
        patch
            .build_key(&key)
            .insert_from_iter(self.database.graph().collect(&key).iter());

        vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: patch.export_key(&key).unwrap(),
        }]
    }

    pub fn handle_inlay_hints(&self, params: InlayHintParams) -> Vec<InlayHint> {
        let key = params.text_document.uri.to_key(&self.base_path);

        self.container_hint(&key)
            .into_iter()
            .chain(self.refs_counter_hints(&key).into_iter())
            .chain(self.block_reference_hints(&key))
            .collect_vec()
    }

    pub fn block_reference_hints(&self, key: &Key) -> Vec<InlayHint> {
        self.database
            .graph()
            .get_block_references_in(key)
            .into_iter()
            .filter_map(|id| {
                self.database
                    .graph()
                    .node_line_range(id)
                    .map(|range| (id, range.start))
            })
            .map(|(id, line)| {
                (
                    self.database
                        .graph()
                        .node(id)
                        .ref_key()
                        .map(|key| self.database.graph().get_block_references_to(&key).len())
                        .unwrap_or_default(),
                    line,
                )
            })
            .map(|(count, line)| hint_at(&format!("⎘{}", number_substr(count)), line as u32))
            .collect_vec()
    }

    pub fn container_hint(&self, key: &Key) -> Vec<InlayHint> {
        self.database
            .graph()
            .get_block_references_to(key)
            .iter()
            .map(|id| self.database.graph().get_container_document_ref_text(*id))
            .sorted()
            .dedup()
            .map(|text| hint_at(&format!("↖{}", text), 0))
            .collect_vec()
    }

    pub fn refs_counter_hints(&self, key: &Key) -> Vec<InlayHint> {
        let inline_refs = self.database.graph().get_inline_references_to(key).len();

        if inline_refs > 0 {
            vec![hint_at(&format!("‹{}›", inline_refs), 0)]
        } else {
            vec![]
        }
    }

    pub fn handle_inline_values(&self, _: InlineValueParams) -> Vec<InlineValue> {
        vec![]
    }

    pub fn handle_document_symbols(&self, params: DocumentSymbolParams) -> Vec<SymbolInformation> {
        let key = params.text_document.uri.to_key(&self.base_path);
        let id_opt = self
            .database
            .graph()
            .maybe_key(&key)
            .and_then(|key_node| key_node.to_child().and_then(|child| child.id()));

        let id2_opt = self
            .database
            .graph()
            .maybe_key(&key)
            .and_then(|key_node| key_node.id());

        if id_opt.is_none() || id2_opt.is_none() {
            return vec![];
        }

        let id = id_opt.unwrap();
        let id2 = id2_opt.unwrap();

        let paths = self.database.graph().paths();

        paths
            .iter()
            .filter(|p| p.contains(id) || p.contains(id2))
            .filter(|p| p.ids().len() > 1)
            .sorted_by(|a, b| {
                for (x, y) in a.ids().iter().zip(b.ids().iter()) {
                    if x != y {
                        return y.cmp(x); // For descending order
                    }
                }
                b.ids().len().cmp(&a.ids().len()) // If all elements are equal, compare b
            })
            .map(|p| p.drop_first())
            .filter(|p| p.ids().len() < 4)
            .map(|p| p.to_nested_symbol(self.database.graph(), &self.base_path))
            .filter(|p| !p.name.is_empty())
            .collect_vec()
    }

    pub fn handle_prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        self.parser(&params.text_document.uri.to_key(&self.base_path))
            .and_then(|parser| parser.link_at(to_position(params.position)))
            .and_then(|link| {
                link.key_range()
                    .map(|range| PrepareRenameResponse::RangeWithPlaceholder {
                        range: to_range(range),
                        placeholder: link.url().unwrap_or("".to_string()),
                    })
            })
    }

    pub fn handle_rename(
        &self,
        params: RenameParams,
    ) -> Result<Option<WorkspaceEdit>, ResponseError> {
        if self
            .database
            .graph()
            .maybe_key(&params.new_name.clone().into())
            .is_some()
        {
            return Result::Err(ResponseError {
                code: 1,
                message: format!("The file name {} is already taken", params.new_name),
                data: None,
            });
        }

        let relative_to = &params
            .text_document_position
            .text_document
            .uri
            .to_key(&self.base_path)
            .parent();

        Result::Ok(
            self.parser(
                &params
                    .text_document_position
                    .text_document
                    .uri
                    .to_key(&self.base_path),
            )
            .and_then(|parser| parser.url_at(to_position(params.text_document_position.position)))
            .map(|url| {
                let key = Key::from_rel_link_url(&url, relative_to);

                let affected_keys = self
                    .database
                    .graph()
                    .get_block_references_to(&key.clone())
                    .into_iter()
                    .chain(self.database.graph().get_inline_references_to(&key.clone()))
                    .map(|node_id| self.database.graph().node(node_id).node_key())
                    .filter(|k| k != &key)
                    .unique()
                    .sorted()
                    .collect_vec();

                let mut patch = self.database.graph().new_patch();

                patch
                    .build_key(&params.new_name.clone().into())
                    .insert_from_iter(
                        self.database
                            .graph()
                            .collect(&key)
                            .change_key(&key, &params.new_name.clone().into())
                            .iter(),
                    );

                affected_keys.iter().for_each(|affected_key| {
                    patch.build_key(&affected_key).insert_from_iter(
                        self.database
                            .graph()
                            .collect(&affected_key)
                            .change_key(&key, &params.new_name.clone().into())
                            .iter(),
                    );
                });

                let new_key = Key::from_rel_link_url(&params.new_name, relative_to);

                let document_changes = affected_keys
                    .into_iter()
                    .map(|affected_key| {
                        self.base_path
                            .key_to_url(&affected_key)
                            .to_override_file_op(
                                &self.base_path,
                                patch.export_key(&affected_key).expect("to have key"),
                            )
                    })
                    .chain(vec![key
                        .clone()
                        .to_full_url(&self.base_path)
                        .to_delete_file_op()])
                    .chain(vec![
                        params.new_name.to_url(&self.base_path).to_create_file_op(),
                        params
                            .new_name
                            .to_url(&self.base_path)
                            .to_override_new_file_op(
                                &self.base_path,
                                patch.export_key(&new_key).expect("to have key"),
                            ),
                    ])
                    .collect();

                WorkspaceEdit {
                    changes: None,
                    document_changes: Some(DocumentChanges::Operations(document_changes)),
                    change_annotations: None,
                }
            }),
        )
    }

    pub fn handle_references(&self, params: ReferenceParams) -> Vec<Location> {
        let key = params
            .text_document_position
            .text_document
            .uri
            .to_key(&self.base_path);

        self.database
            .graph()
            .get_block_references_to(&key.clone())
            .iter()
            .chain(
                self.database
                    .graph()
                    .get_inline_references_to(&key.clone())
                    .iter(),
            )
            .map(|id| (id, self.database.graph().node(*id).node_key()))
            .dedup()
            .map(|(id, key)| {
                Location::new(
                    key.to_full_url(&self.base_path),
                    Range::new(
                        Position::new(
                            self.database
                                .graph()
                                .node_line_range(*id)
                                .map(|f| f.start as u32)
                                .unwrap_or(0),
                            0,
                        ),
                        Position::new(
                            self.database
                                .graph()
                                .node_line_range(*id)
                                .map(|f| f.end as u32)
                                .unwrap_or(0),
                            0,
                        ),
                    ),
                )
            })
            .sorted_by(|a, b| a.uri.cmp(&b.uri))
            .collect_vec()
    }

    pub fn handle_code_action(&self, params: &CodeActionParams) -> CodeActionResponse {
        let context = self.database.graph();
        let base_path: &BasePath = &self.base_path;

        context
            .get_node_id_at(
                &params.text_document.uri.to_key(base_path),
                params.range.start.line as usize,
            )
            .filter(|_| params.range.empty() || self.lsp_client == LspClient::Helix)
            .map(|node_id| {
                all_action_types(&self.configuration)
                    .into_iter()
                    .filter(|action_provider| params.only_includes(&action_provider.action_kind()))
                    .flat_map(|action_type| action_type.action(node_id, self))
                    .map(|action| action.to_code_action())
                    .collect_vec()
            })
            .unwrap_or_default()
    }

    pub fn handle_code_action_resolve(&self, code_action: &CodeAction) -> CodeAction {
        let base_path: &BasePath = &self.base_path;

        let target_node_id = code_action.clone().data.unwrap().as_u64().unwrap();

        let all_types = all_action_types(&self.configuration);

        let action_provider = all_types
            .iter()
            .find(|action_provider| {
                action_provider
                    .action_kind()
                    .eq(&code_action.clone().kind.unwrap())
            })
            .unwrap();

        let changes = action_provider.changes(target_node_id, self).unwrap();

        let mut action = code_action.clone();
        action.edit = Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(
                changes
                    .iter()
                    .map(|change| change.to_document_change(base_path))
                    .collect_vec(),
            )),
            ..Default::default()
        });

        action
    }
}

fn hint_at(text: &str, line: u32) -> InlayHint {
    InlayHint {
        label: InlayHintLabel::String(text.to_string()),
        position: Position::new(line, 120),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: None,
        data: None,
    }
}

fn to_position(value: lsp_types::Position) -> model::Position {
    model::Position {
        line: value.line as usize,
        character: value.character as usize,
    }
}

fn to_range(value: InlineRange) -> Range {
    Range::new(
        Position::new(value.start.line as u32, value.start.character as u32),
        Position::new(value.end.line as u32, value.end.character as u32),
    )
}

pub fn number_substr(num: usize) -> &'static str {
    match num {
        0 => "",
        1 => "",
        2 => "²",
        3 => "³",
        4 => "⁴",
        5 => "⁵",
        6 => "⁶",
        7 => "⁷",
        8 => "⁸",
        9 => "⁹",
        _ => "+",
    }
}

#[allow(deprecated)]
fn path_to_symbol(
    path: &SearchPath,
    context: impl GraphContext,
    base_path: &BasePath,
) -> SymbolInformation {
    let kind = if path.root {
        SymbolKind::NAMESPACE
    } else {
        SymbolKind::OBJECT
    };

    SymbolInformation {
        name: render_path(&path.path, context),
        kind,
        deprecated: None,
        tags: None,
        location: Location {
            uri: base_path.key_to_url(&path.key),
            range: Range::new(
                Position {
                    line: (path.line),
                    character: 0,
                },
                Position {
                    line: (path.line) + 1,
                    character: 0,
                },
            ),
        },
        container_name: None,
    }
}

fn render_path(path: &NodePath, context: impl GraphContext) -> String {
    path.ids()
        .iter()
        .map(|id| context.get_text(*id).trim().to_string())
        .collect_vec()
        .join(" • ")
}

impl ActionContext for &Server {
    fn key_of(&self, node_id: NodeId) -> Key {
        self.database.graph().node(node_id).node_key()
    }

    fn collect(&self, key: &Key) -> Tree {
        self.database.graph().collect(key)
    }

    fn squash(&self, key: &Key, depth: u8) -> Tree {
        self.database.graph().squash(key, depth)
    }

    fn random_key(&self, parent: &str) -> Key {
        self.database.graph().random_key(parent)
    }

    fn markdown_options(&self) -> &MarkdownOptions {
        &self.configuration.markdown
    }

    fn default_model(&self) -> &Model {
        self.configuration.models.get("default").unwrap()
    }

    fn patch(&self) -> Graph {
        self.database.graph().new_patch()
    }

    fn llm_query(&self, prompt: String, model: &Model) -> String {
        if Path::new("./.iwe").exists() {
            fs::write("./.iwe/prompt.md", &prompt).expect("Unable to write file");
        }

        if self
            .configuration
            .models
            .iter()
            .all(|(_, model)| model.api_key_env.is_empty())
        {
            "".to_string()
        } else {
            let response = llm::apply_prompt(prompt, model);

            if Path::new("./.iwe").exists() {
                fs::write("./.iwe/generated.md", &response).expect("Unable to write file");
            }

            response
        }
    }
}
