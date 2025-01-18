#![allow(dead_code, unused_imports, unused_variables)]

use std::{
    cell::{Cell, RefCell},
    error::Error,
    io::ErrorKind,
    time::Duration,
};

use assert_json_diff::assert_json_eq;
use crossbeam_channel::{after, select, Receiver};
use difference::Changeset;
use lib::model::graph::MarkdownOptions;
use lsp_server::{Connection, Message, Notification, Request, ResponseError};
use lsp_types::{
    notification::{DidChangeTextDocument, Exit},
    request::{
        CodeActionRequest, Completion, Formatting, GotoDefinition, GotoTypeDefinition,
        GotoTypeDefinitionResponse, InlayHintRequest, References, Shutdown, WorkspaceSymbolRequest,
    },
    CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, CompletionParams,
    CompletionResponse, DidChangeTextDocumentParams, DocumentFormattingParams,
    GotoDefinitionParams, GotoDefinitionResponse, InlayHint, InlayHintParams, Location,
    PrepareRenameResponse, ReferenceParams, RenameParams, TextDocumentPositionParams, TextEdit,
    Url, WorkspaceEdit, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{to_string_pretty, Value};

use lsp::{main_loop, InitializeParams};

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

pub fn uri(number: u32) -> Url {
    Url::from_file_path(format!("/basepath/{}.md", number)).unwrap()
}

pub fn uri_from(key: &str) -> Url {
    Url::from_file_path(format!("/basepath/{}.md", key)).unwrap()
}

pub fn action_kinds(name: &'static str) -> Option<Vec<CodeActionKind>> {
    Some(vec![CodeActionKind::new(name)])
}

pub fn action_kind(name: &'static str) -> Option<CodeActionKind> {
    Some(CodeActionKind::new(name))
}

impl Fixture {
    pub fn new() -> Fixture {
        Self::with("\n")
    }
    pub fn with(indoc: &str) -> Fixture {
        Self::with_options(indoc, MarkdownOptions::default())
    }
    pub fn with_options(indoc: &str, markdown_options: MarkdownOptions) -> Fixture {
        let (connection, client) = Connection::memory();

        let content = indoc.to_string();

        let _thread: std::thread::JoinHandle<()> = std::thread::Builder::new()
            .name("test server".to_owned())
            .spawn(move || {
                main_loop(
                    connection,
                    serde_json::to_value(InitializeParams {
                        state: if content.is_empty() {
                            None
                        } else {
                            Some(content.clone())
                        },
                        sequential_ids: Some(true),
                    })
                    .unwrap(),
                    "/basepath".to_string(),
                    markdown_options,
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

    pub fn format_doucment(&self, params: DocumentFormattingParams, expected: Vec<TextEdit>) {
        self.assert_response::<Formatting>(params, Some(expected));
    }

    pub fn rename(&self, params: RenameParams, expected: WorkspaceEdit) {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/rename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
    }

    pub fn rename_err(&self, params: RenameParams, expected: ResponseError) {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/rename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
    }

    pub fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
        expected: PrepareRenameResponse,
    ) {
        let id = self.req_id.get();
        self.req_id.set(id.wrapping_add(1));

        let actual = self.send_request_(Request::new(
            id.into(),
            "textDocument/prepareRename".to_string(),
            params,
        ));

        assert_json_eq!(&expected, &actual);
    }

    pub fn references(&self, params: ReferenceParams, expected: Vec<Location>) {
        self.assert_response::<References>(params, Some(expected));
    }

    pub fn inlay_hint(&self, params: InlayHintParams, expected: Vec<InlayHint>) {
        self.assert_response::<InlayHintRequest>(params, Some(expected));
    }

    pub fn code_action(&self, params: CodeActionParams, expected: CodeActionResponse) {
        self.assert_response::<CodeActionRequest>(params, Some(expected));
    }

    pub fn completion(&self, params: CompletionParams, expected: CompletionResponse) {
        self.assert_response::<Completion>(params, Some(expected));
    }

    pub fn go_to_definition(&self, params: GotoDefinitionParams, expected: GotoDefinitionResponse) {
        self.assert_response::<GotoDefinition>(params, Some(expected));
    }

    pub fn did_change_text_document(&self, params: DidChangeTextDocumentParams) {
        self.notification::<DidChangeTextDocument>(params);
    }

    pub fn workspace_symbols(
        &self,
        params: WorkspaceSymbolParams,
        response: WorkspaceSymbolResponse,
    ) {
        self.assert_response::<WorkspaceSymbolRequest>(params, Some(response));
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
        self.wait_for_message_cond(1, &|msg: &Message| match msg {
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
        let msg = msg.map(|msg| {
            self.messages.borrow_mut().push(msg.clone());
            msg
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
