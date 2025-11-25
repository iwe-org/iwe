use extend::ext;
use itertools::Itertools;
use liwe::graph::path::NodePath;
use liwe::model::node::NodePointer;
use lsp_types::*;

use liwe::graph::GraphContext;

use super::search::SearchPath;
use liwe::model::{self, Content, InlineRange, Key};

use super::actions::Change;
use super::BasePath;

#[ext]
pub impl CodeActionParams {
    fn only_includes(&self, kind: &CodeActionKind) -> bool {
        if let Some(only) = self.clone().context.only {
            only.contains(kind)
        } else {
            true
        }
    }

    fn only_includes_explicit(&self, kind: &CodeActionKind) -> bool {
        self.clone()
            .context
            .only
            .map(|only| only.contains(kind))
            .unwrap_or(false)
    }
}

#[ext]
pub impl Vec<DocumentChangeOperation> {
    fn to_code_action(self, title: String, kind: CodeActionKind) -> CodeActionOrCommand {
        CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(kind),
            edit: Some(WorkspaceEdit {
                document_changes: Some(DocumentChanges::Operations(self)),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

#[ext]
pub impl Range {
    fn just_lines(&self) -> Range {
        Range::new(
            Position::new(self.start.line, 0),
            Position::new(self.end.line + 1, 0),
        )
    }
    fn select_lines_in_range(&self, text: &str) -> String {
        text.lines()
            .take((self.end.line + 1) as usize)
            .skip(self.start.line as usize)
            .collect::<Vec<&str>>()
            .join("\n")
    }

    fn empty(&self) -> bool {
        self.start.eq(&self.end)
    }
}

#[ext]
pub impl SearchPath {
    #[allow(deprecated)]
    fn path_to_symbol(
        &self,
        context: impl GraphContext,
        base_path: &BasePath,
    ) -> SymbolInformation {
        let kind = if self.root {
            SymbolKind::NAMESPACE
        } else {
            SymbolKind::OBJECT
        };

        SymbolInformation {
            name: SearchPath::render_path(&self.path, context),
            kind,
            deprecated: None,
            tags: None,
            location: Location {
                uri: base_path.key_to_url(&self.key),
                range: Range::new(
                    Position {
                        line: (self.line),
                        character: 0,
                    },
                    Position {
                        line: (self.line) + 1,
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
}

#[ext]
pub impl Position {
    fn to_model(self) -> model::Position {
        model::Position {
            line: self.line as usize,
            character: self.character as usize,
        }
    }
}

#[ext]
pub impl InlineRange {
    fn to_lsp(self) -> Range {
        Range::new(
            Position::new(self.start.line as u32, self.start.character as u32),
            Position::new(self.end.line as u32, self.end.character as u32),
        )
    }
}

#[ext]
pub impl Change {
    fn to_document_change(&self, base_path: &BasePath) -> DocumentChangeOperation {
        match self {
            Change::Remove(remove) => DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
                uri: remove.key.to_full_url(base_path),
                options: None,
            })),
            Change::Create(create) => DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                uri: create.key.to_full_url(base_path),
                options: Some(CreateFileOptions {
                    overwrite: Some(false),
                    ignore_if_exists: Some(false),
                }),
                annotation_id: None,
            })),
            Change::Update(update) => {
                let insert_extracted_text = TextEdit {
                    range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                    new_text: update.markdown.clone(),
                };
                let edit = TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: update.key.to_full_url(base_path),
                        version: None,
                    },
                    edits: vec![OneOf::Left(insert_extracted_text)],
                };

                DocumentChangeOperation::Edit(edit)
            }
        }
    }
}

#[ext]
pub impl Uri {
    fn to_key(&self, base_path: &BasePath) -> Key {
        base_path.url_to_key(&self.clone())
    }

    fn to_create_file_op(&self) -> DocumentChangeOperation {
        DocumentChangeOperation::Op(self.to_create_file())
    }

    fn to_create_file(&self) -> ResourceOp {
        ResourceOp::Create(CreateFile {
            uri: self.clone(),
            options: Some(CreateFileOptions {
                overwrite: Some(false),
                ignore_if_exists: Some(false),
            }),
            annotation_id: None,
        })
    }

    fn to_delete_file(&self) -> ResourceOp {
        ResourceOp::Delete(DeleteFile {
            uri: self.clone(),
            options: None,
        })
    }

    fn to_override_file_op(&self, base_path: &BasePath, text: String) -> DocumentChangeOperation {
        DocumentChangeOperation::Edit(self.to_override_file(base_path, text))
    }

    fn to_override_new_file_op(
        &self,
        base_path: &BasePath,
        text: String,
    ) -> DocumentChangeOperation {
        DocumentChangeOperation::Edit(self.to_override_new_file(base_path, text))
    }

    fn to_override_new_file(&self, _: &BasePath, text: Content) -> TextDocumentEdit {
        let insert_extracted_text = TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: text,
        };
        TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: self.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(insert_extracted_text)],
        }
    }

    fn to_override_file(&self, _: &BasePath, text: Content) -> TextDocumentEdit {
        let insert_extracted_text = TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: text,
        };
        TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: self.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(insert_extracted_text)],
        }
    }

    fn to_delete_file_op(&self) -> DocumentChangeOperation {
        DocumentChangeOperation::Op(self.to_delete_file())
    }

    fn to_update_file_range_op(&self, text: String, range: Range) -> DocumentChangeOperation {
        DocumentChangeOperation::Edit(self.to_update_file_range(text, range))
    }

    fn to_update_file_range(&self, text: String, range: Range) -> TextDocumentEdit {
        let insert_extracted_text = TextEdit {
            range,
            new_text: text,
        };
        TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: self.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(insert_extracted_text)],
        }
    }
}

#[ext]
pub impl Vec<SymbolInformation> {
    fn to_response(self) -> WorkspaceSymbolResponse {
        WorkspaceSymbolResponse::Flat(self)
    }
}

#[ext]
pub impl NodePath {
    fn render(&self, context: impl GraphContext) -> String {
        self.ids()
            .iter()
            .map(|id| context.get_text(*id).trim().to_string())
            .collect_vec()
            .join(" • ")
    }

    fn nested_render(&self, context: impl GraphContext) -> String {
        let last = self
            .ids()
            .last()
            .map(|id| context.get_text(*id).trim().to_string())
            .unwrap();

        self.ids()
            .iter()
            .skip(1)
            .map(|_| "  ")
            .collect_vec()
            .join("")
            + &last
    }

    #[allow(deprecated)]
    fn to_nested_symbol(
        &self,
        context: impl GraphContext,
        base_path: &BasePath,
    ) -> SymbolInformation {
        let target = self.target();
        let line = context.node_line_number(target).unwrap_or(0);

        SymbolInformation {
            name: self.nested_render(context),
            kind: SymbolKind::OBJECT,
            deprecated: None,
            tags: None,
            location: Location {
                uri: base_path.key_to_url(&context.node(self.target()).node_key()),
                range: Range::new(
                    Position {
                        line: (line as u32),
                        character: 0,
                    },
                    Position {
                        line: (line as u32) + 1,
                        character: 0,
                    },
                ),
            },
            container_name: None,
        }
    }
}

#[ext]
pub impl String {
    fn to_url(&self, base_path: &BasePath) -> Uri {
        base_path.name_to_url(&self.clone())
    }

    fn to_hint_at(self, line: u32) -> InlayHint {
        InlayHint {
            label: InlayHintLabel::String(self),
            position: Position::new(line, 120),
            kind: None,
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: None,
            data: None,
        }
    }
}

#[ext]
pub impl Key {
    fn to_full_url(&self, base_path: &BasePath) -> Uri {
        base_path.key_to_url(&self.clone())
    }

    fn to_link(&self, text: String, relative_to: &str, refs_extension: &str) -> String {
        format!(
            "[{}]({}{})",
            text,
            self.to_rel_link_url(relative_to),
            refs_extension
        )
    }

    fn to_completion(
        &self,
        relative_to: &str,
        context: impl GraphContext,
        _: &BasePath,
    ) -> CompletionItem {
        let ref_text = context.get_ref_text(self).unwrap_or_default();
        let refs_extension = &context.markdown_options().refs_extension;

        CompletionItem {
            preselect: Some(true),
            label: format!("🔗 {}", ref_text),
            sort_text: Some(ref_text.clone()),
            insert_text: Some(self.to_link(ref_text.clone(), relative_to, refs_extension)),
            filter_text: Some(ref_text.replace(" ", "").to_lowercase()),
            command: None,
            documentation: None,
            ..Default::default()
        }
    }
}
