#![allow(clippy::print_stderr)]

use std::env;
use std::error::Error;

use lib::model::graph::Settings;
use lsp::main_loop;
use lsp_types::CodeActionOptions;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CompletionOptions;
use lsp_types::OneOf;
use lsp_types::RenameOptions;
use lsp_types::ServerCapabilities;

use lsp_server::Connection;
use lsp_types::TextDocumentSyncCapability;

const CONFIG_FILE_NAME: &str = "config.json";
const IWE_MARKER: &str = ".iwe";

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    eprintln!("starting IWE LSP server");

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        references_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        completion_provider: Some(CompletionOptions::default()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            lsp_types::TextDocumentSyncKind::FULL,
        )),
        inlay_hint_provider: Some(OneOf::Left(true)),
        inline_value_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: Default::default(),
        })),
        code_action_provider: Some(CodeActionProviderCapability::Options(CodeActionOptions {
            code_action_kinds: Some(vec![
                lsp_types::CodeActionKind::QUICKFIX,
                // lsp_types::CodeActionKind::REFACTOR,
                lsp_types::CodeActionKind::REFACTOR_EXTRACT,
                lsp_types::CodeActionKind::REFACTOR_INLINE,
                lsp_types::CodeActionKind::REFACTOR_REWRITE,
                // lsp_types::CodeActionKind::SOURCE,
                lsp_types::CodeActionKind::SOURCE_ORGANIZE_IMPORTS,
            ]),
            ..Default::default()
        })),
        ..Default::default()
    })
    .unwrap();
    let initialization_params = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };

    let current_dir = env::current_dir().expect("to get current dir");
    let mut iwe_marker_path = current_dir.clone();
    iwe_marker_path.push(IWE_MARKER);

    if !iwe_marker_path.is_dir() {
        eprintln!("Use `iwe init` command to initilize IWE at this location");
        return Ok(());
    }

    let mut config_path = current_dir.clone();
    config_path.push(IWE_MARKER);
    config_path.push(CONFIG_FILE_NAME);

    let settings = {
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Settings>(&content).ok())
            .unwrap_or(Settings::default())
    };

    let mut library_path = current_dir.clone();

    if !settings.library.path.is_empty() {
        library_path.push(settings.library.path);
    }

    main_loop(
        connection,
        initialization_params,
        library_path.to_string_lossy().to_string(),
        settings.markdown,
    )?;
    io_threads.join()?;

    // Shut down gracefully.
    eprintln!("shutting down server");
    Ok(())
}
