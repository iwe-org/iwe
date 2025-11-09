#![allow(dead_code)]
#![allow(deprecated)]

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    str::FromStr,
    time::Duration,
};

use extend::ext;

use assert_json_diff::assert_json_eq;
use crossbeam_channel::{after, select, Receiver};
use liwe::{model::config::Configuration, state::from_indoc};
use lsp_server::{Connection, Message, Notification, Request, ResponseError};
use lsp_types::{notification::*, request::*, *};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use iwes::{main_loop, ServerParams};
use liwe::model::config::MarkdownOptions;

pub struct Fixture {
    req_id: Cell<i32>,
    messages: RefCell<Vec<Message>>,
    client: Connection,
    _thread: std::thread::JoinHandle<()>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Health {
    Ok,
    Warning,
    Error,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct ServerStatusParams {
    pub health: Health,
    pub quiescent: bool,
    pub message: Option<String>,
}

#[ext]
pub impl Uri {
    fn to_edit(self, new_content: &str) -> DocumentChangeOperation {
        self.to_edit_with_range(
            new_content,
            Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
        )
    }

    fn to_code_action_params(self, line: u32, kind: &str) -> CodeActionParams {
        self.to_code_action_params_at_position(line, 0, kind)
    }

    fn to_code_action_params_at_position(
        self,
        line: u32,
        character: u32,
        kind: &str,
    ) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: self },
            range: Range::new(
                Position::new(line, character),
                Position::new(line, character),
            ),
            context: CodeActionContext {
                only: Some(vec![CodeActionKind::from(kind.to_string())]),
                ..Default::default()
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        }
    }

    fn to_code_action_params_with_trigger(self, line: u32, kind: &str) -> CodeActionParams {
        CodeActionParams {
            text_document: TextDocumentIdentifier { uri: self },
            range: Range::new(Position::new(line, 0), Position::new(line, 0)),
            context: CodeActionContext {
                diagnostics: Default::default(),
                only: Some(vec![CodeActionKind::from(kind.to_string())]),
                trigger_kind: Some(CodeActionTriggerKind::INVOKED),
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
        }
    }

    fn to_completion_params(self, line: u32, character: u32) -> CompletionParams {
        CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: self },
                position: Position::new(line, character),
            },
            context: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }
    }

    fn to_text_document_position_params(
        self,
        line: u32,
        character: u32,
    ) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: self },
            position: Position::new(line, character),
        }
    }

    fn to_goto_definition_params(self, line: u32, character: u32) -> GotoDefinitionParams {
        GotoDefinitionParams {
            text_document_position_params: self.to_text_document_position_params(line, character),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }
    }

    fn to_reference_params(
        self,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> ReferenceParams {
        ReferenceParams {
            text_document_position: self.to_text_document_position_params(line, character),
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
            partial_result_params: PartialResultParams {
                partial_result_token: None,
            },
            context: ReferenceContext {
                include_declaration,
            },
        }
    }

    fn to_rename_params(self, line: u32, character: u32, new_name: String) -> RenameParams {
        RenameParams {
            text_document_position: self.to_text_document_position_params(line, character),
            new_name,
            work_done_progress_params: Default::default(),
        }
    }

    fn to_inlay_hint_params(self) -> InlayHintParams {
        InlayHintParams {
            text_document: TextDocumentIdentifier { uri: self },
            work_done_progress_params: Default::default(),
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        }
    }

    fn to_document_formatting_params(self) -> DocumentFormattingParams {
        DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri: self },
            options: Default::default(),
            work_done_progress_params: Default::default(),
        }
    }

    fn to_did_change_params(self, version: i32, text: String) -> DidChangeTextDocumentParams {
        DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri: self, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        }
    }

    fn to_did_save_params(self, text: Option<String>) -> DidSaveTextDocumentParams {
        DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: self },
            text,
        }
    }

    fn to_file_delete_params(self) -> DidChangeWatchedFilesParams {
        DidChangeWatchedFilesParams {
            changes: vec![FileEvent {
                uri: self,
                typ: FileChangeType::DELETED,
            }],
        }
    }

    fn to_edit_with_range(self, new_content: &str, range: Range) -> DocumentChangeOperation {
        DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: self,
                version: None,
            },
            edits: vec![OneOf::Left(TextEdit {
                range,
                new_text: new_content.to_string(),
            })],
        })
    }

    fn to_create_file(self) -> DocumentChangeOperation {
        self.to_create_file_with_options(false, false)
    }

    fn to_create_file_with_options(
        self,
        overwrite: bool,
        ignore_if_exists: bool,
    ) -> DocumentChangeOperation {
        DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
            uri: self,
            options: Some(CreateFileOptions {
                overwrite: Some(overwrite),
                ignore_if_exists: Some(ignore_if_exists),
            }),
            annotation_id: None,
        }))
    }

    fn to_delete_file(self) -> DocumentChangeOperation {
        DocumentChangeOperation::Op(ResourceOp::Delete(DeleteFile {
            uri: self,
            options: None,
        }))
    }

    fn to_symbol_info(
        self,
        name: &str,
        kind: SymbolKind,
        line_start: u32,
        line_end: u32,
    ) -> SymbolInformation {
        SymbolInformation {
            kind,
            location: Location {
                uri: self,
                range: Range::new(Position::new(line_start, 0), Position::new(line_end, 0)),
            },
            name: name.to_string(),
            container_name: None,
            tags: None,
            deprecated: None,
        }
    }

    fn to_location(self, line_start: u32, line_end: u32) -> Location {
        Location::new(
            self,
            Range::new(Position::new(line_start, 0), Position::new(line_end, 0)),
        )
    }
}

#[ext]
pub impl &str {
    fn to_text_edit(self, line_start: u32, line_end: u32) -> TextEdit {
        TextEdit {
            range: Range::new(Position::new(line_start, 0), Position::new(line_end, 0)),
            new_text: self.to_string(),
        }
    }

    fn to_text_edit_full(self) -> TextEdit {
        TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
            new_text: self.to_string(),
        }
    }

    fn to_inlay_hint(self, line: u32, character: u32) -> InlayHint {
        InlayHint {
            label: InlayHintLabel::String(self.to_string()),
            position: Position::new(line, character),
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
pub impl Vec<DocumentChangeOperation> {
    fn to_workspace_edit(self) -> WorkspaceEdit {
        WorkspaceEdit {
            document_changes: Some(DocumentChanges::Operations(self)),
            ..Default::default()
        }
    }
}

#[ext]
pub impl WorkspaceEdit {
    fn to_code_action(self, title: &str, kind: &'static str) -> CodeAction {
        CodeAction {
            title: title.to_string(),
            kind: action_kind(kind),
            edit: Some(self),
            ..Default::default()
        }
    }
}

pub fn uri(number: u32) -> Uri {
    Uri::from_str(&format!("file:///basepath/{}.md", number)).unwrap()
}

#[allow(unused, dead_code)]
pub fn uri_from(key: &str) -> Uri {
    Uri::from_str(&format!("file:///basepath/{}.md", key)).unwrap()
}

#[allow(unused, dead_code)]
pub fn action_kinds(name: &'static str) -> Option<Vec<CodeActionKind>> {
    Some(vec![CodeActionKind::new(name)])
}

#[allow(unused, dead_code)]
pub fn action_kind(name: &'static str) -> Option<CodeActionKind> {
    Some(CodeActionKind::new(name))
}

pub fn completion_item(
    label: &str,
    insert_text: &str,
    filter_text: &str,
    sort_text: &str,
) -> CompletionItem {
    CompletionItem {
        documentation: None,
        filter_text: Some(filter_text.to_string()),
        sort_text: Some(sort_text.to_string()),
        insert_text: Some(insert_text.to_string()),
        label: label.to_string(),
        preselect: Some(true),
        ..Default::default()
    }
}

pub fn completion_list(items: Vec<CompletionItem>) -> CompletionResponse {
    CompletionResponse::List(CompletionList {
        is_incomplete: false,
        items,
    })
}

pub fn workspace_symbol_params(query: &str) -> WorkspaceSymbolParams {
    WorkspaceSymbolParams {
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        query: query.to_string(),
    }
}

pub fn workspace_symbol_response(symbols: Vec<SymbolInformation>) -> WorkspaceSymbolResponse {
    WorkspaceSymbolResponse::Flat(symbols)
}

pub fn goto_definition_response_empty() -> GotoDefinitionResponse {
    GotoDefinitionResponse::Array(vec![])
}

pub fn goto_definition_response_single(uri: Uri) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Scalar(Location::new(uri, Range::default()))
}

pub fn prepare_rename_response(range: Range, placeholder: String) -> PrepareRenameResponse {
    PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }
}

pub fn response_error(code: i32, message: String) -> ResponseError {
    ResponseError {
        code,
        message,
        data: None,
    }
}

pub type Documents = Vec<(&'static str, &'static str)>;

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unused, dead_code)]
impl Fixture {
    pub fn new() -> Fixture {
        Self::with("\n")
    }

    pub fn with_documents(kv: Documents) -> Fixture {
        let state: HashMap<String, String> = kv
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self::with_options_and_client(state, Configuration::default(), "")
    }

    pub fn with(indoc: &str) -> Fixture {
        Self::with_options_and_client(from_indoc(indoc), Configuration::default(), "")
    }

    pub fn with_options(indoc: &str, markdown_options: MarkdownOptions) -> Fixture {
        let config = Configuration {
            markdown: markdown_options,
            ..Default::default()
        };

        Self::with_options_and_client(from_indoc(indoc), config, "")
    }

    pub fn with_config(indoc: &str, config: Configuration) -> Fixture {
        Self::with_options_and_client(from_indoc(indoc), config, "")
    }

    pub fn with_client(indoc: &str, client: &str) -> Fixture {
        Self::with_options_and_client(from_indoc(indoc), Configuration::default(), client)
    }

    pub fn with_options_and_client(
        state: HashMap<String, String>,
        configuration: Configuration,
        lsp_client_name: &str,
    ) -> Fixture {
        let (connection, client) = Connection::memory();
        let client_name = Some(lsp_client_name.to_string());

        let _thread: std::thread::JoinHandle<()> = std::thread::Builder::new()
            .name("test server".to_owned())
            .spawn(move || {
                main_loop(
                    connection,
                    ServerParams {
                        state: if state.is_empty() {
                            None
                        } else {
                            Some(state.clone())
                        },
                        client_name,
                        sequential_ids: Some(true),
                        base_path: "/basepath".to_string(),
                        configuration,
                    },
                )
                .unwrap()
            })
            .expect("failed to spawn a thread");

        Fixture {
            req_id: Cell::new(1),
            messages: Default::default(),
            client,
            _thread,
        }
    }

    pub fn notification<N>(&self, params: N::Params)
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        self.send_notification(Notification::new(N::METHOD.to_owned(), params))
    }

    pub(crate) fn expect_notification<N>(&self, expected: Value)
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        while let Some(Message::Notification(actual)) =
            recv_timeout(&self.client.receiver).unwrap_or_else(|_| panic!("timed out"))
        {
            if actual.method == N::METHOD {
                let actual = actual
                    .clone()
                    .extract::<Value>(N::METHOD)
                    .expect("was not able to extract notification");

                assert_json_eq!(&expected, &actual);
                return;
            }
            continue;
        }
        panic!("never got expected notification");
    }

    pub fn request<R>(&self, params: R::Params, expected_resp: Value)
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let actual = self.send_request::<R>(params);
        assert_json_eq!(&expected_resp, &actual);
    }

    pub fn assert_response<R>(&self, params: R::Params, expected: R::Result)
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let actual: Value = self.send_request::<R>(params);
        assert_json_eq!(&expected, &actual);
    }

    pub fn format_document(
        &self,
        params: DocumentFormattingParams,
        expected: Vec<TextEdit>,
    ) -> &Self {
        self.assert_response::<Formatting>(params, Some(expected));
        self
    }

    pub fn rename(&self, params: RenameParams, expected: WorkspaceEdit) -> &Self {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/rename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
        self
    }

    pub fn rename_err(&self, params: RenameParams, expected: ResponseError) -> &Self {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/rename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
        self
    }

    pub fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
        expected: PrepareRenameResponse,
    ) -> &Self {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/prepareRename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
        self
    }

    pub fn references(&self, params: ReferenceParams, expected: Vec<Location>) -> &Self {
        self.assert_response::<References>(params, Some(expected));
        self
    }

    pub fn inlay_hint(&self, params: InlayHintParams, expected: Vec<InlayHint>) -> &Self {
        self.assert_response::<InlayHintRequest>(params, Some(expected));
        self
    }

    pub fn no_code_action(&self, params: CodeActionParams) -> &Self {
        let mut actual: Value = self.send_request::<CodeActionRequest>(params);
        assert_json_eq!(&Some::<Vec<CodeActionOrCommand>>(vec![]), &actual);
        self
    }

    pub fn code_action_menu(&self, params: CodeActionParams, expected: CodeAction) -> &Self {
        let mut expected_no_edits = expected.clone();
        expected_no_edits.edit.take();

        let actual: Value = self.send_request::<CodeActionRequest>(params);
        let actual_action = actual.as_array().unwrap().first().unwrap();
        let mut actual_no_data = actual_action.as_object().unwrap().clone();

        actual_no_data.remove("data");

        assert_json_eq!(&expected_no_edits, &actual_no_data);
        self
    }

    pub fn code_action(&self, params: CodeActionParams, expected: CodeAction) -> &Self {
        let mut expected_no_edits = expected.clone();
        expected_no_edits.edit.take();

        let actual: Value = self.send_request::<CodeActionRequest>(params);
        let actual_action = actual.as_array().unwrap().first().unwrap();
        let mut actual_no_data = actual_action.as_object().unwrap().clone();

        actual_no_data.remove("data");

        assert_json_eq!(&expected_no_edits, &actual_no_data);

        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual_with_edits = self.send_request_(Request::new(
            id.into(),
            "codeAction/resolve".to_string(),
            actual_action,
        ));

        let mut actual_with_edits_no_data = actual_with_edits.as_object().unwrap().clone();
        actual_with_edits_no_data.remove("data");

        assert_json_eq!(&expected, &actual_with_edits_no_data);
        self
    }

    pub fn completion(&self, params: CompletionParams, expected: CompletionResponse) -> &Self {
        self.assert_response::<Completion>(params, Some(expected));
        self
    }

    pub fn go_to_definition(
        &self,
        params: GotoDefinitionParams,
        expected: GotoDefinitionResponse,
    ) -> &Self {
        self.assert_response::<GotoDefinition>(params, Some(expected));
        self
    }

    pub fn did_change_text_document(&self, params: DidChangeTextDocumentParams) -> &Self {
        self.notification::<DidChangeTextDocument>(params);
        self
    }

    pub fn did_save_text_document(&self, params: DidSaveTextDocumentParams) -> &Self {
        self.notification::<DidSaveTextDocument>(params);
        self
    }

    pub fn did_delete_files(&self, params: DidChangeWatchedFilesParams) -> &Self {
        self.notification::<DidChangeWatchedFiles>(params);
        self
    }

    pub fn workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
        response: WorkspaceSymbolResponse,
    ) -> &Self {
        self.assert_response::<WorkspaceSymbolRequest>(params, Some(response));
        self
    }

    pub fn send_request<R>(&self, params: R::Params) -> Value
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
    {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        self.send_request_(Request::new(id.into(), R::METHOD.to_owned(), params))
    }

    fn send_request_(&self, r: Request) -> Value {
        let id = r.id.clone();
        self.client.sender.send(r.clone().into()).unwrap();
        while let Some(msg) = self
            .recv()
            .unwrap_or_else(|Timeout| panic!("timeout: {r:?}"))
        {
            match msg {
                Message::Request(req) => {
                    if req.method == "client/registerCapability" {
                        let params = req.params.to_string();
                        if ["workspace/didChangeWatchedFiles", "textDocument/didSave"]
                            .into_iter()
                            .any(|it| params.contains(it))
                        {
                            continue;
                        }
                    }
                    panic!("unexpected request: {req:?}")
                }
                Message::Notification(_) => (),
                Message::Response(response) => {
                    assert_eq!(response.id, id);
                    if let Some(err) = response.error {
                        panic!("error response: {err:#?}");
                    }
                    return response.result.unwrap();
                }
            }
        }
        panic!("no response for {r:?}");
    }

    pub fn wait_until_workspace_is_loaded(self) -> Fixture {
        self.wait_for_message_cond(1, |msg: &Message| match msg {
            Message::Notification(n) if n.method == "experimental/serverStatus" => {
                let status = n
                    .clone()
                    .extract::<ServerStatusParams>("experimental/serverStatus")
                    .unwrap();
                if status.health != Health::Ok {
                    panic!(
                        "server errored/warned while loading workspace: {:?}",
                        status.message
                    );
                }
                status.quiescent
            }
            _ => false,
        })
        .unwrap_or_else(|Timeout| panic!("timeout while waiting for ws to load"));
        self
    }

    fn wait_for_message_cond(
        &self,
        n: usize,
        cond: impl Fn(&Message) -> bool,
    ) -> Result<(), Timeout> {
        let mut total = 0;
        for msg in self.messages.borrow().iter() {
            if cond(msg) {
                total += 1
            }
        }
        while total < n {
            let msg = self.recv()?.expect("no response");
            if cond(&msg) {
                total += 1;
            }
        }
        Ok(())
    }

    fn recv(&self) -> Result<Option<Message>, Timeout> {
        let msg = recv_timeout(&self.client.receiver)?;
        let msg = msg.inspect(|msg| {
            self.messages.borrow_mut().push(msg.clone());
        });
        Ok(msg)
    }

    fn send_notification(&self, notification: Notification) {
        let r = self.client.sender.send(Message::Notification(notification));

        if r.is_err() {
            eprintln!("failed to send notification: {:?}", r.err());
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.request::<Shutdown>((), Value::Null);
        self.notification::<Exit>(());
    }
}

struct Timeout;

fn recv_timeout(receiver: &Receiver<Message>) -> Result<Option<Message>, Timeout> {
    let timeout = if cfg!(target_os = "macos") {
        Duration::from_secs(300)
    } else {
        Duration::from_secs(120)
    };
    select! {
        recv(receiver) -> msg => Ok(msg.ok()),
        recv(after(timeout)) -> _ => Err(Timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_edit() {
        let operation = uri(1).to_edit("test content");

        if let DocumentChangeOperation::Edit(edit) = operation {
            assert_eq!(edit.text_document.uri, uri(1));
            if let OneOf::Left(text_edit) = &edit.edits[0] {
                assert_eq!(text_edit.new_text, "test content");
            } else {
                panic!("Expected TextEdit");
            }
        } else {
            panic!("Expected Edit operation");
        }
    }

    #[test]
    fn test_create_workspace_edit() {
        let operations = vec![uri(1).to_edit("content1"), uri(2).to_edit("content2")];

        let workspace_edit = operations.to_workspace_edit();

        if let Some(DocumentChanges::Operations(ops)) = workspace_edit.document_changes {
            assert_eq!(ops.len(), 2);
        } else {
            panic!("Expected Operations");
        }
    }

    #[test]
    fn test_create_code_action() {
        let code_action = vec![uri(1).to_edit("content")]
            .to_workspace_edit()
            .to_code_action("Test Action", "refactor.extract");

        assert_eq!(code_action.title, "Test Action");
        assert_eq!(
            code_action.kind,
            Some(CodeActionKind::new("refactor.extract"))
        );
        assert!(code_action.edit.is_some());
    }
}


