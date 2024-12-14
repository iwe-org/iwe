use std::u32;

use extend::ext;
use itertools::Itertools;
use lib::graph::path::NodePath;
use lsp_types::*;

use lib::model::{self, Content, Key};
use lib::{graph::GraphContext, key};

use super::BasePath;

#[ext]
pub impl CodeActionParams {
    fn only_includes(&self, kind: &CodeActionKind) -> bool {
        self.clone()
            .context
            .only
            .map(|only| only.contains(kind))
            .unwrap_or(true)
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
pub impl Url {
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

    fn to_override_file(&self, base_path: &BasePath, text: Content) -> TextDocumentEdit {
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
        // for each fragment in the path, get the text and join them with a space
        self.ids()
            .iter()
            .map(|id| context.get_text(*id).trim().to_string())
            .collect_vec()
            .join(" • ")
    }

    fn nested_render(&self, context: impl GraphContext) -> String {
        // for each fragment in the path, get the text and join them with a space
        let last = self
            .ids()
            .last()
            .map(|id| context.get_text(*id).trim().to_string())
            .unwrap();

        self.ids()
            .iter()
            .skip(1)
            .map(|f| "  ")
            .collect_vec()
            .join("")
            + &last
    }

    fn to_symbol(&self, context: impl GraphContext, base_path: &BasePath) -> SymbolInformation {
        let target = self.target();
        let line = context.node_line_number(target).unwrap_or(0);

        SymbolInformation {
            name: self.render(context),
            kind: SymbolKind::OBJECT,
            deprecated: None,
            tags: None,
            location: Location {
                uri: base_path.key_to_url(&context.get_key(self.target())),
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
                uri: base_path.key_to_url(&context.get_key(self.target())),
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
pub impl Key {
    fn to_url(&self, base_path: &BasePath) -> Url {
        base_path.key_to_url(&self.clone())
    }

    fn to_link(&self, text: String) -> String {
        format!("[{}]({})", text, key::without_extension(self))
    }

    fn to_completion(&self, context: impl GraphContext, base_path: &BasePath) -> CompletionItem {
        CompletionItem {
            preselect: Some(true),
            label: context.get_ref_text(self).unwrap_or_default(),
            insert_text: Some(self.to_link(context.get_ref_text(self).unwrap_or_default())),
            filter_text: Some(
                context
                    .get_ref_text(self)
                    .unwrap_or_default()
                    .replace(" ", "")
                    .to_lowercase(),
            ),
            documentation: None,
            ..Default::default()
        }
    }
}
