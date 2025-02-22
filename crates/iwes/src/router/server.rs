use itertools::Itertools;
use liwe::action::ActionType;
use liwe::graph::path::NodePath;
use liwe::model::graph::NodeIter;
use lsp_server::ResponseError;
use lsp_types::request::GotoDeclarationParams;
use lsp_types::*;

use liwe::graph::GraphContext;
use liwe::graph::SearchPath;
use liwe::model::Key;
use liwe::model::{self, InlineRange};

use liwe::parser::Parser;

use super::LspClient;
use super::ServerConfig;
use liwe::database::Database;
use liwe::database::DatabaseContext;

use self::extensions::*;

mod extensions;

pub struct Server {
    base_path: BasePath,
    database: Database,
    refs_extension: String,
    lsp_client: LspClient,
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

    fn name_to_url(&self, key: &str) -> Url {
        Url::parse(&format!("{}{}.md", self.base_path, key)).unwrap()
    }

    fn url_to_key(&self, url: &Url) -> Key {
        Key::from_rel_link_url(
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
                config.markdown_options.clone(),
            ),
            refs_extension: config.markdown_options.refs_extension.clone(),
            lsp_client: config.lsp_client,
        }
    }
    pub fn database(&self) -> impl DatabaseContext + '_ {
        &self.database
    }

    pub fn handle_did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        self.database.update_document(
            self.base_path.url_to_key(&params.text_document.uri.clone()),
            params.text.unwrap().clone(),
        );
    }

    pub fn handle_did_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
        self.database.update_document(
            self.base_path.url_to_key(&params.text_document.uri.clone()),
            params.content_changes.first().unwrap().text.clone(),
        );
    }

    pub fn handle_completion(&self, _: CompletionParams) -> CompletionResponse {
        CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items: self
                .database
                .graph()
                .keys()
                .iter()
                .map(|key| key.to_completion(self.database.graph(), &self.base_path))
                .collect_vec(),
        })
    }

    pub fn handle_workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> WorkspaceSymbolResponse {
        self.database
            .global_search(&params.query)
            .iter()
            .map(|p| path_to_symbol(p, self.database.graph(), &self.base_path))
            .collect_vec()
            .to_response()
    }

    pub fn handle_goto_definition(&self, params: GotoDeclarationParams) -> GotoDefinitionResponse {
        self.parser(
            &params
                .text_document_position_params
                .text_document
                .uri
                .to_key(&self.base_path),
        )
        .and_then(|parser| {
            parser.key_at(to_position(
                (&params).text_document_position_params.position,
            ))
        })
        .map(|key| {
            GotoDefinitionResponse::Scalar(Location::new(
                self.base_path.key_to_url(&key),
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
            .insert_from_iter(self.database.graph().visit(&key));

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
                        .visit_node(id)
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
            .map(|id| self.database.graph().get_container_doucment_ref_text(*id))
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

    pub fn handle_ducment_symbols(&self, params: DocumentSymbolParams) -> Vec<SymbolInformation> {
        let key = params.text_document.uri.to_key(&self.base_path);
        let id = self
            .database
            .graph()
            .visit_key(&key)
            .unwrap()
            .to_child()
            .expect("to have child")
            .id();
        let id2 = self.database.graph().visit_key(&key).unwrap().id();
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
                        placeholder: link.ref_key().unwrap().to_rel_link_url(),
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
            .visit_key(&params.new_name.clone().into())
            .is_some()
        {
            return Result::Err(ResponseError {
                code: 1,
                message: format!("The file name {} is already taken", params.new_name),
                data: None,
            });
        }

        Result::Ok(
            self.parser(
                &params
                    .text_document_position
                    .text_document
                    .uri
                    .to_key(&self.base_path),
            )
            .and_then(|parser| parser.key_at(to_position(params.text_document_position.position)))
            .map(|key| {
                let affected_keys = self
                    .database
                    .graph()
                    .get_block_references_to(&key.clone())
                    .into_iter()
                    .chain(self.database.graph().get_inline_references_to(&key.clone()))
                    .flat_map(|node_id| self.database.graph().visit_node(node_id).to_document())
                    .flat_map(|doc| doc.key())
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
                            .change_key_visitor(&key, &key, &params.new_name.clone().into())
                            .child()
                            .unwrap(),
                    );

                affected_keys.iter().for_each(|affected_key| {
                    patch.build_key(&affected_key).insert_from_iter(
                        self.database
                            .graph()
                            .change_key_visitor(
                                &affected_key,
                                &key,
                                &params.new_name.clone().into(),
                            )
                            .child()
                            .unwrap(),
                    );
                });

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
                                patch
                                    .export_key(&Key::from_rel_link_url(&params.new_name.clone()))
                                    .expect("to have key"),
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
            .map(|id| (id, self.database.graph().get_container_key(*id)))
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

        dbg!(self.lsp_client);

        context
            .get_node_id_at(
                &params.text_document.uri.to_key(base_path),
                params.range.start.line as usize,
            )
            .filter(|_| params.range.empty() || self.lsp_client == LspClient::Helix)
            .map(|node_id| {
                vec![
                    ActionType::SectionExtract,
                    ActionType::SectionExtractSubsections,
                    ActionType::ReferenceInlineSection,
                    ActionType::ListToSections,
                    ActionType::SectionToList,
                    ActionType::ListChangeType,
                    ActionType::ReferenceInlineQuote,
                ]
                .into_iter()
                .filter(|action_type| params.only_includes(&action_type.action_kind()))
                .chain(
                    vec![ActionType::ReferenceInlineList, ActionType::ListDetach]
                        .into_iter()
                        .filter(|action_type| {
                            params.only_includes_explicit(&action_type.action_kind())
                        }),
                )
                .map(|action_type| action_type.apply(node_id, context))
                .flatten()
                .map(|action| action.to_code_action(base_path))
                .collect_vec()
            })
            .unwrap_or_default()
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
