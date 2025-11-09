use std::panic;
use std::sync::Arc;

use anyhow::{bail, Result};
use crossbeam_channel::{select, Receiver, Sender};
use liwe::model::config::Configuration;
use liwe::model::State;
use log::{debug, error};
use lsp_server::{ErrorCode, Message, Request};
use lsp_server::{Notification, Response};
use lsp_types::{
    CodeAction, CodeActionParams, CompletionItem, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidSaveTextDocumentParams, DocumentFormattingParams,
    DocumentSymbolParams, ExecuteCommandParams, InlayHintParams, InlineValueParams,
    ReferenceParams, RenameParams, TextDocumentPositionParams, WorkspaceSymbolParams,
};
use lsp_types::{CompletionParams, GotoDefinitionParams};
use serde::Deserialize;
use serde_json::to_value;
use uuid::Uuid;

use self::server::Server;

pub mod server;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LspClient {
    Unknown,
    Helix,
}

pub struct ServerConfig {
    pub base_path: String,
    pub state: State,
    pub sequential_ids: Option<bool>,
    pub configuration: Configuration,
    pub lsp_client: LspClient,
}

#[derive(Clone)]
pub struct Router {
    server: Arc<Server>,
    sender: Sender<Message>,
}

impl Router {
    pub fn respond(&self, response: Response) {
        self.send(response.into());
    }

    fn send(&self, message: Message) {
        self.sender.send(message).unwrap()
    }

    fn delay_send(&self, message: Message) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            sender.send(message).unwrap();
        });
    }

    pub fn new(sender: Sender<Message>, config: ServerConfig) -> Self {
        debug!(
            "initializing LSP database at {}, with {} docs",
            config.base_path,
            config.state.len()
        );

        let router = Self {
            server: Arc::new(Server::new(config)),
            sender,
        };

        debug!("initializing LSP database complete");

        router
    }

    fn next_event(&self, inbox: &Receiver<Message>) -> Option<Message> {
        select! {
            recv(inbox) -> msg =>
                msg.ok()
        }
    }

    pub fn run(mut self, receiver: Receiver<Message>) -> Result<()> {
        use std::panic::AssertUnwindSafe;

        while let Some(message) = self.next_event(&receiver) {
            let shutdown = panic::catch_unwind(AssertUnwindSafe(|| self.handle_message(message)))
                .unwrap_or_else(|err| {
                    let error_message = if let Some(string) = err.downcast_ref::<&str>() {
                        format!("Panic occurred with message: {}", string)
                    } else if let Some(string) = err.downcast_ref::<String>() {
                        format!("Panic occurred with message: {}", string)
                    } else {
                        "Panic occurred with unknown cause".to_string()
                    };
                    error!("Panic message: {}", error_message);
                    false
                });

            if shutdown {
                return Ok(());
            }
        }
        bail!("client exited without proper shutdown sequence")
    }

    fn handle_message(&mut self, message: Message) -> bool {
        match message {
            Message::Request(req) => {
                let request = req;
                let self_clone = self.clone();
                let _ = std::thread::spawn(move || self_clone.on_request(request));
                false
            }
            Message::Notification(notification) => self.on_notification(notification),
            Message::Response(_) => false,
        }
    }

    fn on_notification(&mut self, notification: Notification) -> bool {
        if notification.method == "exit" {
            return true;
        }

        match notification.method.as_str() {
            "textDocument/didChange" => {
                let params = DidChangeTextDocumentParams::deserialize(notification.params).unwrap();
                Arc::get_mut(&mut self.server)
                    .unwrap()
                    .handle_did_change_text_document(params);
            }
            "textDocument/didSave" => {
                let params = DidSaveTextDocumentParams::deserialize(notification.params).unwrap();
                Arc::get_mut(&mut self.server)
                    .unwrap()
                    .handle_did_save_text_document(params);
            }
            "workspace/didChangeWatchedFiles" => {
                let params = DidChangeWatchedFilesParams::deserialize(notification.params).unwrap();
                Arc::get_mut(&mut self.server)
                    .unwrap()
                    .handle_did_change_watched_files(params);
            }
            default => {
                debug!("unhandled request: {}", default)
            }
        };

        false
    }

    fn on_request(&self, request: Request) -> bool {
        if request.method == "shutdown" {
            self.respond(Response {
                id: request.id.clone(),
                result: Some(serde_json::Value::Null),
                error: None,
            });

            return true;
        }

        if request.method.eq("workspace/executeCommand") {
            let params = ExecuteCommandParams::deserialize(request.params).unwrap();
            let result = self.server.handle_workspace_command(params);

            self.send(Message::Request(Request {
                id: Uuid::new_v4().to_string().into(),
                method: "workspace/applyEdit".to_string(),
                params: to_value(result).unwrap(),
            }));

            return false;
        }

        let response = match request.method.as_str() {
            "textDocument/inlayHint" => InlayHintParams::deserialize(request.params)
                .map(|params| self.server.handle_inlay_hints(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/inlineValues" => InlineValueParams::deserialize(request.params)
                .map(|params| self.server.handle_inline_values(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/documentSymbol" => DocumentSymbolParams::deserialize(request.params)
                .map(|params| self.server.handle_document_symbols(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/definition" => GotoDefinitionParams::deserialize(request.params)
                .map(|params| self.server.handle_goto_definition(params))
                .map(|response| to_value(response).unwrap()),
            "workspace/symbol" => WorkspaceSymbolParams::deserialize(request.params)
                .map(|params| self.server.handle_workspace_symbols(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/completion" => CompletionParams::deserialize(request.params)
                .map(|params| self.server.handle_completion(params))
                .map(|response| to_value(response).unwrap()),
            "completionItem/resolve" => CompletionItem::deserialize(request.params)
                .map(|params| self.server.resolve_completion(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/codeAction" => CodeActionParams::deserialize(request.params)
                .map(|params| self.server.handle_code_action(&params))
                .map(|response| to_value(response).unwrap()),
            "codeAction/resolve" => CodeAction::deserialize(request.params)
                .map(|params| self.server.handle_code_action_resolve(&params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/formatting" => DocumentFormattingParams::deserialize(request.params)
                .map(|params| self.server.handle_document_formatting(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/references" => ReferenceParams::deserialize(request.params)
                .map(|params| self.server.handle_references(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/prepareRename" => TextDocumentPositionParams::deserialize(request.params)
                .map(|params| self.server.handle_prepare_rename(params))
                .map(|response| to_value(response).unwrap()),
            "textDocument/rename" => RenameParams::deserialize(request.params).map(|params| {
                match self.server.handle_rename(params) {
                    Ok(response) => to_value(response).unwrap(),
                    Err(err) => to_value(err).unwrap(),
                }
            }),
            default => {
                panic!("unhandled request: {}", default)
            }
        };

        match response {
            Ok(value) => {
                self.respond(Response {
                    id: request.id,
                    result: Some(value),
                    error: None,
                });
                if request.method.as_str() == "codeAction/resolve" {
                    self.delay_send(Message::Request(Request {
                        id: Uuid::new_v4().to_string().into(),
                        method: "workspace/inlayHint/refresh".to_string(),
                        params: serde_json::Value::Null,
                    }));
                }
            }
            Err(_) => self.respond(Response::new_err(
                request.id,
                ErrorCode::InternalError as i32,
                "error handling request".to_string(),
            )),
        }

        false
    }
}
