use std::fs;
use std::path::Path;

use actions::{all_action_types, ActionContext, ActionProvider};
use command::{CommandType, GenerateCommand};
use itertools::Itertools;
use liwe::{
    graph::{DatabaseContext, Graph, GraphContext},
    model::{
        config::{Configuration, MarkdownOptions, Model},
        node::NodePointer,
        tree::Tree,
        Key, NodeId,
    },
};
use lsp_server::ResponseError;
use lsp_types::*;

use super::{LspClient, ServerConfig};

use self::base_path::BasePath;
use self::extensions::*;

pub mod actions;
pub mod base_path;
pub mod command;
mod extensions;
mod llm;

pub struct Server {
    base_path: BasePath,
    graph: Graph,
    lsp_client: LspClient,
    configuration: Configuration,
}

impl Server {
    pub fn new(config: ServerConfig) -> Server {
        Server {
            base_path: BasePath::new(format!("file://{}/", config.base_path)),
            graph: Graph::from_state(
                config.state,
                config.sequential_ids.unwrap_or(false),
                config.configuration.markdown.clone(),
            ),
            lsp_client: config.lsp_client,
            configuration: config.configuration,
        }
    }
    pub fn graph(&self) -> impl DatabaseContext + '_ {
        &self.graph
    }

    pub fn handle_did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        params.text.map(|text| {
            self.graph.update_document(
                self.base_path.url_to_key(&params.text_document.uri.clone()),
                text,
            )
        });
    }

    pub fn handle_did_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
        self.graph.update_document(
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

        let new_key = (&self.graph).random_key(&current_key.parent());
        let keys = self.graph.keys();
        keys.iter()
            .filter(|key| {
                self.configuration
                    .library
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
                    label: format!("🤖 {}", (&self.graph).get_ref_text(key).unwrap_or_default()),
                    insert_text: Some(format!("[⏳]({})", new_key)),
                    filter_text: Some(format!(
                        "_{}",
                        (&self.graph).get_ref_text(key).unwrap_or_default()
                    )),
                    sort_text: Some((&self.graph).get_ref_text(key).unwrap_or_default()),
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

        self.graph
            .keys()
            .iter()
            .map(|key| key.to_completion(&current_key.parent(), &self.graph, &self.base_path))
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
        self.graph
            .global_search(&params.query)
            .iter()
            .map(|p| p.path_to_symbol(&self.graph, &self.base_path))
            .filter(|p| !p.name.is_empty())
            .collect_vec()
            .to_response()
    }

    pub fn handle_goto_definition(&self, params: GotoDefinitionParams) -> GotoDefinitionResponse {
        let key = params
            .text_document_position_params
            .text_document
            .uri
            .to_key(&self.base_path);
        let relative_to = key.parent();
        let position = params.text_document_position_params.position;

        self.graph()
            .parser(&key)
            .and_then(|parser| parser.url_at(position.to_model()))
            .map(|url| {
                GotoDefinitionResponse::Scalar(Location::new(
                    self.base_path.resolve_relative_url(&url, &relative_to),
                    Range::default(),
                ))
            })
            .unwrap_or(GotoDefinitionResponse::Array(vec![]))
    }

    pub fn handle_document_formatting(&self, params: DocumentFormattingParams) -> Vec<TextEdit> {
        let key = params.text_document.uri.to_key(&self.base_path);

        let mut patch = self.graph.new_patch();
        patch
            .build_key(&key)
            .insert_from_iter((&self.graph).collect(&key).iter());

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
        self.graph
            .get_block_references_in(key)
            .into_iter()
            .filter_map(|id| {
                self.graph
                    .node_line_range(id)
                    .map(|range| (id, range.start))
            })
            .flat_map(|(id, line)| {
                (&self.graph)
                    .node(id)
                    .ref_key()
                    .map(|key| self.graph.get_block_references_to(&key))
                    .map(|refs| {
                        refs.into_iter()
                            .filter(|ref_id| !(&self.graph).get_node_key(*ref_id).eq(key))
                            .sorted_by_key(|ref_id| (&self.graph).get_node_key(*ref_id))
                            .unique_by(|ref_id| (&self.graph).get_node_key(*ref_id))
                            .flat_map(|id| (&self.graph).get_container_document_ref_text(id))
                            .map(|s| format!("↖{}", s))
                            .join(" ")
                    })
                    .filter(|text| !text.is_empty())
                    .map(|text| (text, line))
            })
            .map(|(text, line)| text.to_hint_at(line as u32))
            .collect_vec()
    }

    pub fn container_hint(&self, key: &Key) -> Vec<InlayHint> {
        self.graph
            .get_block_references_to(key)
            .iter()
            .flat_map(|id| (&self.graph).get_container_document_ref_text(*id))
            .sorted()
            .dedup()
            .map(|text| format!("↖{}", text).to_hint_at(0))
            .collect_vec()
    }

    pub fn refs_counter_hints(&self, key: &Key) -> Vec<InlayHint> {
        let inline_refs = self.graph.get_inline_references_to(key).len();

        if inline_refs > 0 {
            vec![format!("‹{}›", inline_refs).to_hint_at(0)]
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
            .graph
            .maybe_key(&key)
            .and_then(|key_node| key_node.to_child().and_then(|child| child.id()));

        let id2_opt = self
            .graph
            .maybe_key(&key)
            .and_then(|key_node| key_node.id());

        if id_opt.is_none() || id2_opt.is_none() {
            return vec![];
        }

        let id = id_opt.unwrap();
        let id2 = id2_opt.unwrap();

        let paths = self.graph.paths();

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
            .map(|p| p.to_nested_symbol(&self.graph, &self.base_path))
            .filter(|p| !p.name.is_empty())
            .collect_vec()
    }

    pub fn handle_prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Option<PrepareRenameResponse> {
        self.graph()
            .parser(&params.text_document.uri.to_key(&self.base_path))
            .and_then(|parser| parser.link_at(params.position.to_model()))
            .and_then(|link| {
                link.key_range()
                    .map(|range| PrepareRenameResponse::RangeWithPlaceholder {
                        range: range.to_lsp(),
                        placeholder: link.url().unwrap_or("".to_string()),
                    })
            })
    }

    pub fn handle_rename(
        &self,
        params: RenameParams,
    ) -> Result<Option<WorkspaceEdit>, ResponseError> {
        if self
            .graph
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
            self.graph()
                .parser(
                    &params
                        .text_document_position
                        .text_document
                        .uri
                        .to_key(&self.base_path),
                )
                .and_then(|parser| parser.url_at(params.text_document_position.position.to_model()))
                .map(|url| {
                    let key = Key::from_rel_link_url(&url, relative_to);

                    let affected_keys = self
                        .graph
                        .get_block_references_to(&key.clone())
                        .into_iter()
                        .chain(self.graph.get_inline_references_to(&key.clone()))
                        .map(|node_id| (&self.graph).node(node_id).node_key())
                        .filter(|k| k != &key)
                        .unique()
                        .sorted()
                        .collect_vec();

                    let mut patch = self.graph.new_patch();

                    patch
                        .build_key(&params.new_name.clone().into())
                        .insert_from_iter(
                            (&self.graph)
                                .collect(&key)
                                .change_key(&key, &params.new_name.clone().into())
                                .iter(),
                        );

                    affected_keys.iter().for_each(|affected_key| {
                        patch.build_key(&affected_key).insert_from_iter(
                            (&self.graph)
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

        let relative_to = &params
            .text_document_position
            .text_document
            .uri
            .to_key(&self.base_path)
            .parent();

        let key_under_cursor = self
            .graph()
            .parser(
                &params
                    .text_document_position
                    .text_document
                    .uri
                    .to_key(&self.base_path),
            )
            .and_then(|parser| parser.url_at(params.text_document_position.position.to_model()))
            .map(|url| Key::from_rel_link_url(&url, relative_to))
            .unwrap_or(key.clone());

        self.graph
            .get_block_references_to(&key_under_cursor.clone())
            .iter()
            .chain(
                self.graph
                    .get_inline_references_to(&key.clone())
                    .iter()
                    .filter(|_| params.context.include_declaration),
            )
            .map(|id| (id, (&self.graph).node(*id).node_key()))
            .dedup()
            .filter(|(_, backlink_key)| backlink_key.ne(&key))
            .map(|(id, key)| {
                Location::new(
                    key.to_full_url(&self.base_path),
                    Range::new(
                        Position::new(
                            self.graph
                                .node_line_range(*id)
                                .map(|f| f.start as u32)
                                .unwrap_or(0),
                            0,
                        ),
                        Position::new(
                            self.graph
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
        let base_path: &BasePath = &self.base_path;

        let (start_character, end_character) = if self.lsp_client == LspClient::Helix {
            if params.range.end.character - params.range.start.character == 1 {
                (params.range.start.character, params.range.start.character)
            } else {
                (params.range.start.character, params.range.end.character)
            }
        } else {
            (params.range.start.character, params.range.end.character)
        };

        let key = params.text_document.uri.to_key(base_path);
        let selection = actions::TextRange {
            start: actions::Position {
                line: params.range.start.line,
                character: start_character,
            },
            end: actions::Position {
                line: params.range.end.line,
                character: end_character,
            },
        };

        all_action_types(&self.configuration)
            .into_iter()
            .filter(|action_provider| params.only_includes(&action_provider.action_kind()))
            .flat_map(|action_type| action_type.action(key.clone(), selection.clone(), self))
            .map(|action| action.to_code_action())
            .collect_vec()
    }

    pub fn handle_code_action_resolve(&self, code_action: &CodeAction) -> CodeAction {
        let base_path: &BasePath = &self.base_path;

        let data = code_action.clone().data.unwrap();

        let key = Key::name(data.get("key").unwrap().as_str().unwrap());
        let range = data.get("range").unwrap();
        let start = range.get("start").unwrap();
        let end = range.get("end").unwrap();
        let selection = actions::TextRange {
            start: actions::Position {
                line: start.get("line").unwrap().as_u64().unwrap() as u32,
                character: start.get("character").unwrap().as_u64().unwrap() as u32,
            },
            end: actions::Position {
                line: end.get("line").unwrap().as_u64().unwrap() as u32,
                character: end.get("character").unwrap().as_u64().unwrap() as u32,
            },
        };

        let all_types = all_action_types(&self.configuration);

        let action_provider = all_types
            .iter()
            .find(|action_provider| {
                action_provider
                    .action_kind()
                    .eq(&code_action.clone().kind.unwrap())
            })
            .unwrap();

        let changes = action_provider.changes(key, selection, self).unwrap();

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

impl ActionContext for &Server {
    fn key_of(&self, node_id: NodeId) -> Key {
        (&self.graph).node(node_id).node_key()
    }

    fn collect(&self, key: &Key) -> Tree {
        (&self.graph).collect(key)
    }

    fn squash(&self, key: &Key, depth: u8) -> Tree {
        (&self.graph).squash(key, depth)
    }

    fn random_key(&self, parent: &str) -> Key {
        (&self.graph).random_key(parent)
    }

    fn markdown_options(&self) -> &MarkdownOptions {
        &self.configuration.markdown
    }

    fn default_model(&self) -> &Model {
        self.configuration.models.get("default").unwrap()
    }

    fn patch(&self) -> Graph {
        self.graph.new_patch()
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

    fn key_exists(&self, key: &Key) -> bool {
        self.graph.keys().contains(key)
    }

    fn get_block_references_to(&self, key: &Key) -> Vec<NodeId> {
        self.graph.get_block_references_to(key)
    }

    fn get_inline_references_to(&self, key: &Key) -> Vec<NodeId> {
        self.graph.get_inline_references_to(key)
    }

    fn get_ref_text(&self, key: &Key) -> Option<String> {
        self.graph.get_key_title(key)
    }

    fn unique_ids(&self, parent: &str, number: usize) -> Vec<String> {
        (&self.graph).unique_ids(parent, number)
    }

    fn random_keys(&self, parent: &str, number: usize) -> Vec<Key> {
        (&self.graph).random_keys(parent, number)
    }

    fn get_node_id_at(&self, key: &Key, line: usize) -> Option<NodeId> {
        (&self.graph).get_node_id_at(key, line)
    }

    fn get_document_markdown(&self, key: &Key) -> Option<String> {
        self.graph.get_document(key)
    }
}
