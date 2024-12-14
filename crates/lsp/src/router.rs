use std::time::Instant;

use anyhow::{bail, Result};
use crossbeam_channel::{select, Receiver, Sender};
use itertools::Itertools;
use lsp_server::{ErrorCode, Message, Request};
use lsp_server::{Notification, Response};
use lsp_types::request::DocumentSymbolRequest;
use lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyOutgoingCallsParams,
    CodeActionParams, DidChangeTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, DocumentSymbolParams, InlayHintParams, InlineValueParams,
    ReferenceParams, WorkspaceSymbolParams,
};
use lsp_types::{CompletionParams, GotoDefinitionParams};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::to_value;

use lib::model::State;

use self::server::Server;

pub type ReqHandler = fn(&mut Router, Response);
type ReqQueue = lsp_server::ReqQueue<(String, Instant), ReqHandler>;

pub mod server;

pub struct ServerConfig {
    pub base_path: String,
    pub state: State,
    pub sequential_ids: Option<bool>,
}

pub struct Router {
    server: Server,
    sender: Sender<Message>,
}

impl Router {
    pub fn respond(&mut self, response: Response) {
        self.send(response.into());
    }

    fn send(&self, message: Message) {
        self.sender.send(message).unwrap()
    }

    pub fn new(sender: Sender<Message>, config: ServerConfig) -> Self {
        eprintln!(
            "initializing LSP database at {}, with {} docs",
            config.base_path,
            config.state.len()
        );

        let db = Self {
            server: Server::new(config),
            sender,
        };

        eprintln!("initializing LSP database complete");

        db
    }

    fn next_event(&self, inbox: &Receiver<Message>) -> Option<Message> {
        select! {
            recv(inbox) -> msg =>
                msg.ok()
        }
    }

    pub fn run(mut self, receiver: Receiver<Message>) -> Result<()> {
        while let Some(message) = self.next_event(&receiver) {
            let shutdown = self.handle_message(message);

            if shutdown {
                return Ok(());
            }
        }
        bail!("client exited without proper shutdown sequence")
    }

    fn handle_message(&mut self, message: Message) -> bool {
        match message {
            Message::Request(req) => self.on_request(req),
            Message::Notification(notification) => self.on_notification(notification),
            Message::Response(res) => {
                return false;
            }
        }
    }

    fn on_notification(&mut self, notification: Notification) -> bool {
        if notification.method == "exit" {
            return true;
        }

        match notification.method.as_str() {
            "textDocument/didChange" => self.server.handle_did_change_text_document(
                DidChangeTextDocumentParams::deserialize(notification.params).unwrap(),
            ),
            "textDocument/didSave" => self.server.handle_did_save_text_document(
                DidSaveTextDocumentParams::deserialize(notification.params).unwrap(),
            ),
            default => {
                eprintln!("unhandled notification: {}", default)
            }
        };

        false
    }

    fn on_request(&mut self, request: Request) -> bool {
        if request.method == "shutdown" {
            self.respond(Response {
                id: request.id.clone(),
                result: Some(serde_json::Value::Null),
                error: None,
            });

            return true;
        }

        let response = match request.method.as_str() {
            "textDocument/inlayHint" => InlayHintParams::deserialize(request.params)
                .map(|params| self.server.handle_inlay_hints(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/inlineValues" => InlineValueParams::deserialize(request.params)
                .map(|params| self.server.handle_inline_values(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/documentSymbol" => DocumentSymbolParams::deserialize(request.params)
                .map(|params| self.server.handle_ducment_symbols(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/definition" => GotoDefinitionParams::deserialize(request.params)
                .map(|params| self.server.handle_goto_definition(params))
                .map(|response| to_value(response).unwrap()),
            "workspace/symbol" => WorkspaceSymbolParams::deserialize(request.params)
                .map(|params| self.server.handle_workspace_symbols(params)) // Completion::METHOD => {
                .map(|respons| to_value(respons).unwrap()),
            "textDocument/completion" => CompletionParams::deserialize(request.params)
                .map(|params| self.server.handle_completion(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/codeAction" => CodeActionParams::deserialize(request.params)
                .map(|params| self.server.handle_code_action(&params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/formatting" => DocumentFormattingParams::deserialize(request.params)
                .map(|params| self.server.handle_document_formatting(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/references" => ReferenceParams::deserialize(request.params)
                .map(|params| self.server.handle_references(params))
                .map(|response| to_value(response).unwrap()),
            default => {
                panic!("unhandled request: {}", default)
            }
        };

        match response {
            Ok(value) => self.respond(Response {
                id: request.id,
                result: Some(value),
                error: None,
            }),
            Err(err) => self.respond(Response::new_err(
                request.id,
                ErrorCode::InternalError as i32,
                "error handling request".to_string(),
            )),
        }

        false
    }
}
