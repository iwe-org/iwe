use std::cmp::Ordering;
use std::default;

use itertools::Itertools;
use lib::model::graph::MarkdownOptions;
use lib::model::rank::node_rank;
use lib::parser::Pos;
use lsp_types::request::GotoDeclarationParams;
use lsp_types::*;

use lib::graph::{Graph, GraphContext};
use lib::model::Content;
use lib::{key, model::Key};

use lib::parser::Parser;

use super::ServerConfig;
use lib::database::Database;
use lib::database::DatabaseContext;

use change_list_type_action::change_list_type_action;
use extract_list_action::extract_list_action;
use extract_section_action::extract_section_action;
use extract_selection_action::code_action;
use extract_sub_sections_action::extract_sub_sections_action;
use inline_section_action::inline_section_action;
use list_to_section::list_to_section;
use section_to_list_action::section_to_list_action;

use self::extensions::*;

mod change_list_type_action;
mod debug_code_action;
mod extensions;
mod extract_list_action;
mod extract_section_action;
mod extract_selection_action;
mod extract_sub_sections_action;
mod inline_section_action;
mod list_to_section;
mod section_to_list_action;

pub trait CodeActionContext: GraphContext + ServerContext + DatabaseContext {}

pub struct Server {
    base_path: BasePath,
    database: Database,
    refs_extension: String,
}

pub struct BasePath {
    base_path: String,
}

impl BasePath {
    fn new(base_path: String) -> Self {
        Self { base_path }
    }

    fn key_to_url(&self, key: &Key) -> Url {
        Url::parse(&format!("{}{}.md", self.base_path, key)).unwrap()
    }

    fn url_to_key(&self, url: &Url) -> Key {
        url.to_string()
            .trim_start_matches(&self.base_path)
            .trim_end_matches(".md")
            .to_string()
    }
}

pub trait ServerContext {
    fn selection(&self, url: &Url, range: &Range) -> String;
}

impl DatabaseContext for &Server {
    fn parser(&self, id: &Key) -> Option<Parser> {
        self.database().parser(id)
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
        }
    }
    pub fn database(&self) -> impl DatabaseContext + '_ {
        &self.database
    }

    pub fn handle_did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        eprintln!(
            "did change text document {}",
            &params.text_document.uri.to_string()
        );
    }
    pub fn handle_did_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
        eprintln!(
            "did change text document {}",
            &params.text_document.uri.to_string()
        );
        self.database.update_document(
            &self.base_path.url_to_key(&params.text_document.uri.clone()),
            params.content_changes.first().unwrap().text.clone(),
        );
    }

    pub fn handle_completion(&self, params: CompletionParams) -> CompletionResponse {
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
        eprintln!("workspace symbols reqested");

        self.database
            .graph()
            .paths()
            .iter()
            .sorted_by(|a, b| {
                let primary = node_rank(self.database.graph(), b.last_id())
                    .cmp(&node_rank(self.database.graph(), a.last_id()));

                if primary == Ordering::Equal {
                    self.database
                        .graph()
                        .get_key(a.target())
                        .cmp(&self.database.graph().get_key(b.target()))
                } else {
                    primary
                }
            })
            .map(|p| p.to_symbol(self.database.graph(), &self.base_path))
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
        .and_then(|parser| parser.link_at(to_pos((&params).text_document_position_params.position)))
        .map(|link| {
            GotoDefinitionResponse::Scalar(Location::new(
                self.base_path
                    .key_to_url(&link.trim_end_matches(&self.refs_extension).to_string()),
                Range::default(),
            ))
        })
        .unwrap_or(GotoDefinitionResponse::Array(vec![]))
    }

    pub fn handle_document_formatting(&self, params: DocumentFormattingParams) -> Vec<TextEdit> {
        let key = params.text_document.uri.to_key(&self.base_path);

        let mut patch = Graph::new();
        patch
            .build_key(&key)
            .insert_from_iter(self.database.graph().visitor(&key));

        vec![TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: patch.export_key(&key).unwrap(),
        }]
    }

    pub fn handle_inlay_hints(&self, params: InlayHintParams) -> Vec<InlayHint> {
        let key = params.text_document.uri.to_key(&self.base_path);

        let inline_refs = self
            .database
            .graph()
            .get_inline_references_to(&key.clone())
            .len();

        self.database
            .graph()
            .get_block_references_to(&key.clone())
            .iter()
            .map(|id| self.database.graph().get_container_doucment_ref_text(*id))
            .sorted()
            .dedup()
            .map(|text| hint(&format!("↖{}", text)))
            .chain(
                vec![hint(&format!("‹{}›", inline_refs))]
                    .into_iter()
                    .filter(|h| inline_refs > 0),
            )
            .collect_vec()
    }

    pub fn handle_inline_values(&self, params: InlineValueParams) -> Vec<InlineValue> {
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
                    key.to_url(&self.base_path),
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
        vec![
            section_to_list_action(self.database.graph(), &self.base_path, params),
            list_to_section(self.database.graph(), &self.base_path, params),
            inline_section_action(self.database.graph(), &self.base_path, params),
            extract_section_action(self.database.graph(), &self.base_path, params),
            extract_sub_sections_action(self.database.graph(), &self.base_path, params),
            extract_list_action(self.database.graph(), &self.base_path, params),
            change_list_type_action(self.database.graph(), &self.base_path, params),
        ]
        .into_iter()
        .flatten()
        .collect_vec()
    }
}

fn hint_at(text: &str, at: u32) -> InlayHint {
    InlayHint {
        label: InlayHintLabel::String(text.to_string()),
        position: Position::new(at, 120),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: None,
        data: None,
    }
}

fn hint(text: &str) -> InlayHint {
    InlayHint {
        label: InlayHintLabel::String(text.to_string()),
        position: Position::new(0, 120),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: None,
        data: None,
    }
}

fn to_pos(value: lsp_types::Position) -> Pos {
    Pos {
        line: value.line as usize + 1,
        column: value.character as usize + 1,
    }
}
