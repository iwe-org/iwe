use actions::{all_action_types, ActionContext, ActionProvider};
use itertools::Itertools;
use liwe::{
    graph::{DatabaseContext, Graph, GraphContext},
    model::{
        config::{Command, Configuration, MarkdownOptions},
        is_ref_url,
        node::NodePointer,
        tree::Tree,
        Key, NodeId,
    },
};
use lsp_server::ResponseError;
use lsp_types::*;
use liwe::model::node::Node;

use super::{LspClient, ServerConfig};

pub enum DefinitionResult {
    Internal(GotoDefinitionResponse),
    External(String),
}

use self::base_path::BasePath;
use self::extensions::*;
use self::search::SearchIndex;

pub mod actions;
pub mod base_path;
mod extensions;
pub mod search;

pub struct Server {
    base_path: BasePath,
    graph: Graph,
    lsp_client: LspClient,
    configuration: Configuration,
    search_index: SearchIndex,
}

impl Server {
    pub fn new(config: ServerConfig) -> Server {
        let graph = Graph::from_state(
            config.state,
            config.sequential_ids.unwrap_or(false),
            config.configuration.markdown.clone(),
            config.configuration.library.frontmatter_document_title.clone(),
        );
        let mut search_index = SearchIndex::new();
        search_index.update(&graph);

        Server {
            base_path: BasePath::from_path(&config.base_path),
            graph,
            lsp_client: config.lsp_client,
            configuration: config.configuration,
            search_index,
        }
    }
    pub fn graph(&self) -> impl DatabaseContext + '_ {
        &self.graph
    }

    pub fn handle_hover(&self, params: HoverParams) -> Option<Hover> {
        let key = params
            .text_document_position_params
            .text_document
            .uri
            .to_key(&self.base_path);
        let relative_to = key.parent();
        let position = params.text_document_position_params.position;

        let url = self
            .graph()
            .parser(&key)
            .and_then(|parser| parser.url_at(position.to_model()))?;

        let url = url.split('#').next().unwrap_or(url.as_str());
        let url = url.split('?').next().unwrap_or(url);

        if url.is_empty() || !is_ref_url(url) {
            return None;
        }

        let target_key = Key::from_rel_link_url(url, &relative_to);
        let markdown = self.graph.to_markdown_skip_frontmatter(&target_key);

        if markdown.trim().is_empty() {
            return None;
        }

        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        })
    }

    pub fn handle_did_save_text_document(&mut self, params: DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            self.graph.update_document(
                self.base_path.url_to_key(&params.text_document.uri.clone()),
                text,
            );
            self.search_index.update(&self.graph);
        }
    }

    pub fn handle_did_change_text_document(&mut self, params: DidChangeTextDocumentParams) {
        let Some(content) = params.content_changes.first() else {
            return;
        };
        self.graph.update_document(
            self.base_path.url_to_key(&params.text_document.uri.clone()),
            content.text.clone(),
        );
        self.search_index.update(&self.graph);
    }

    pub fn handle_did_change_watched_files(&mut self, params: DidChangeWatchedFilesParams) {
        for change in params.changes {
            match change.typ {
                FileChangeType::DELETED => {
                    let key = self.base_path.url_to_key(&change.uri);
                    self.graph.remove_document(key);
                    self.search_index.update(&self.graph);
                }
                FileChangeType::CREATED => {}
                FileChangeType::CHANGED => {}
                _ => {}
            }
        }
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
            .map(|key| {
                key.to_completion(
                    &current_key.parent(),
                    &self.graph,
                    &self.configuration.completion,
                    &self.base_path,
                )
            })
            .sorted_by(|a, b| a.label.cmp(&b.label))
            .collect_vec()
    }

    pub fn handle_completion(&self, params: CompletionParams) -> CompletionResponse {
        let min_length = self.configuration.completion.min_prefix_length.unwrap_or(3);

        if min_length > 0 {
            let position = params.text_document_position.position;
            let key = params
                .text_document_position
                .text_document
                .uri
                .to_key(&self.base_path);

            let prefix_len = self
                .graph
                .get_document(&key)
                .and_then(|content| {
                    let line = content.lines().nth(position.line as usize)?;
                    let before_cursor =
                        &line[..std::cmp::min(position.character as usize, line.len())];
                    let prefix = before_cursor.split_whitespace().last().unwrap_or("");
                    Some(prefix.len())
                })
                .unwrap_or(0);

            if prefix_len < min_length {
                return CompletionResponse::List(CompletionList {
                    is_incomplete: false,
                    items: vec![],
                });
            }
        }

        CompletionResponse::List(CompletionList {
            is_incomplete: false,
            items: self.handle_link_completion(params),
        })
    }

    pub fn resolve_completion(&self, completion: CompletionItem) -> CompletionItem {
        completion
    }

    pub fn handle_workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
    ) -> WorkspaceSymbolResponse {
        self.search_index
            .search(&params.query)
            .iter()
            .map(|p| p.path_to_symbol(&self.base_path))
            .filter(|p| !p.name.is_empty())
            .collect_vec()
            .to_response()
    }

    pub fn handle_goto_definition(&self, params: GotoDefinitionParams) -> DefinitionResult {
        let key = params
            .text_document_position_params
            .text_document
            .uri
            .to_key(&self.base_path);
        let relative_to = key.parent();
        let position = params.text_document_position_params.position;

        let Some(url) = self
            .graph()
            .parser(&key)
            .and_then(|parser| parser.url_at(position.to_model()))
        else {
            return DefinitionResult::Internal(GotoDefinitionResponse::Array(vec![]));
        };

        if !is_ref_url(&url) {
            return DefinitionResult::External(url);
        }

        DefinitionResult::Internal(GotoDefinitionResponse::Scalar(Location::new(
            self.base_path.resolve_relative_url(&url, &relative_to),
            Range::default(),
        )))
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
            .chain(self.refs_counter_hints(&key))
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
                            .filter_map(|ref_id| {
                                let ref_key = (&self.graph).get_node_key(ref_id)?;
                                if ref_key.eq(key) {
                                    None
                                } else {
                                    Some((ref_id, ref_key))
                                }
                            })
                            .sorted_by_key(|(_, ref_key)| ref_key.clone())
                            .unique_by(|(_, ref_key)| ref_key.clone())
                            .flat_map(|(id, _)| (&self.graph).get_container_document_ref_text(id))
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
        let Some(id) = self
            .graph
            .maybe_key(&key)
            .and_then(|key_node| key_node.to_child().and_then(|child| child.id()))
        else {
            return vec![];
        };

        let Some(id2) = self
            .graph
            .maybe_key(&key)
            .and_then(|key_node| key_node.id())
        else {
            return vec![];
        };

        let paths = self.graph.paths();

        paths
            .iter()
            .filter(|p| p.contains(id) || p.contains(id2))
            .filter(|p| p.ids().len() > 1)
            .sorted_by(|a, b| {
                for (x, y) in a.ids().iter().zip(b.ids().iter()) {
                    if x != y {
                        return y.cmp(x);
                    }
                }
                b.ids().len().cmp(&a.ids().len())
            })
            .map(|p| p.drop_first())
            .filter(|p| p.ids().len() < 4)
            .filter_map(|p| p.to_nested_symbol(&self.graph, &self.base_path))
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
                        patch.build_key(affected_key).insert_from_iter(
                            (&self.graph)
                                .collect(affected_key)
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

        let Some(data) = code_action.data.clone() else {
            return code_action.clone();
        };

        let Some(key) = data
            .get("key")
            .and_then(|v| v.as_str())
            .map(Key::name)
        else {
            return code_action.clone();
        };

        let Some(range) = data.get("range") else {
            return code_action.clone();
        };

        let Some(start) = range.get("start") else {
            return code_action.clone();
        };

        let Some(end) = range.get("end") else {
            return code_action.clone();
        };

        let Some(selection) = (|| {
            Some(actions::TextRange {
                start: actions::Position {
                    line: start.get("line")?.as_u64()? as u32,
                    character: start.get("character")?.as_u64()? as u32,
                },
                end: actions::Position {
                    line: end.get("line")?.as_u64()? as u32,
                    character: end.get("character")?.as_u64()? as u32,
                },
            })
        })() else {
            return code_action.clone();
        };

        let all_types = all_action_types(&self.configuration);

        let Some(action_provider) = all_types.iter().find(|action_provider| {
            code_action
                .kind
                .as_ref()
                .map(|kind| action_provider.action_kind().eq(kind))
                .unwrap_or(false)
        }) else {
            return code_action.clone();
        };

        let Some(changes) = action_provider.changes(key, selection, self) else {
            return code_action.clone();
        };

        let lsp_changes = actions::into_lsp_changes(changes);

        let mut action = code_action.clone();
        action.edit = Some(WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(
                lsp_changes
                    .iter()
                    .map(|change| change.to_document_change(base_path))
                    .collect_vec(),
            )),
            ..Default::default()
        });

        action
    }

    pub fn handle_folding_range(&self, params: FoldingRangeParams) -> Vec<FoldingRange> {
        let key = params.text_document.uri.to_key(&self.base_path);

        let Some(tree) = self
            .graph
            .maybe_key(&key)
            .map(|p| p.collect_tree())
        else {
            return vec![];
        };

        let mut ranges = Vec::new();
        self.collect_folding_ranges(&tree, &mut ranges, 1);
        ranges
    }

    fn collect_folding_ranges(&self, tree: &Tree, ranges: &mut Vec<FoldingRange>, level: u8) {
        let mut next_level = level;
        if let Some(id) = tree.id {
            if let Some(line_range) = self.graph.node_line_range(id) {
                let (end_line, collapsed_text) = match &tree.node {
                    Node::Section(inlines) => {
                        let end = self.section_end_line(tree, line_range.end);
                        let header_prefix = "#".repeat(level as usize);
                        let text: String = inlines.iter().map(|i| i.plain_text()).collect();
                        next_level = level + 1;
                        (Some((end - 1) as u32), Some(format!("{} {}", header_prefix, text)))
                    }
                    Node::Raw(lang, _) if line_range.end > line_range.start + 1 => {
                        (Some((line_range.end - 1) as u32), lang.clone())
                    }
                    Node::Quote() if line_range.end > line_range.start + 1 => {
                        (Some((line_range.end - 1) as u32), None)
                    }
                    Node::BulletList()
                        if tree.children.len() > 1 && line_range.end > line_range.start + 1 =>
                    {
                        let end = self.section_end_line(tree, line_range.end);
                        let first_item_text = tree
                            .children
                            .first()
                            .map(|child| child.node.plain_text())
                            .filter(|s| !s.is_empty())
                            .map(|s| format!("- {}", s));
                        (Some((end - 1) as u32), first_item_text)
                    }
                    Node::OrderedList()
                        if tree.children.len() > 1 && line_range.end > line_range.start + 1 =>
                    {
                        let end = self.section_end_line(tree, line_range.end);
                        let first_item_text = tree
                            .children
                            .first()
                            .map(|child| child.node.plain_text())
                            .filter(|s| !s.is_empty())
                            .map(|s| format!("1. {}", s));
                        (Some((end - 1) as u32), first_item_text)
                    }
                    Node::Table(_) if line_range.end > line_range.start + 1 => {
                        (Some((line_range.end - 1) as u32), None)
                    }
                    _ => (None, None),
                };

                if let Some(end_line) = end_line {
                    ranges.push(FoldingRange {
                        start_line: line_range.start as u32,
                        start_character: None,
                        end_line,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text,
                    });
                }
            }
        }

        for child in &tree.children {
            self.collect_folding_ranges(child, ranges, next_level);
        }
    }

    fn section_end_line(&self, tree: &Tree, default: usize) -> usize {
        self.max_end_line_recursive(tree).unwrap_or(default)
    }

    fn max_end_line_recursive(&self, tree: &Tree) -> Option<usize> {
        let own_end = tree.id.and_then(|id| self.graph.node_line_range(id)).map(|r| r.end);
        let children_max = tree
            .children
            .iter()
            .filter_map(|c| self.max_end_line_recursive(c))
            .max();
        match (own_end, children_max) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
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

    fn get_command(&self, name: &str) -> Option<&Command> {
        self.configuration.commands.get(name)
    }

    fn graph(&self) -> &Graph {
        &self.graph
    }

    fn patch(&self) -> Graph {
        self.graph.new_patch()
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

    fn get_link_key_at(&self, key: &Key, line: usize, character: usize) -> Option<Key> {
        let parser = self.graph().parser(key)?;
        let position = liwe::model::Position {
            line,
            character,
        };
        let url = parser.url_at(position)?;

        if !is_ref_url(&url) {
            return None;
        }

        let target_key = Key::from_rel_link_url(&url, &key.parent());
        self.graph.maybe_key(&target_key)?;
        Some(target_key)
    }

    fn get_link_text_at(&self, key: &Key, line: usize, character: usize) -> Option<String> {
        let parser = self.graph().parser(key)?;
        let position = liwe::model::Position {
            line,
            character,
        };
        let link = parser.link_at(position)?;
        Some(link.to_plain_text())
    }
}
