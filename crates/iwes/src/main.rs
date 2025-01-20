use std::env;
use std::error::Error;
use std::fs::OpenOptions;

use iwes::main_loop;
use liwe::action::ActionType;
use liwe::model::graph::Settings;
use lsp_types::CodeActionKind;
use lsp_types::CodeActionOptions;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CompletionOptions;
use lsp_types::OneOf;
use lsp_types::RenameOptions;
use lsp_types::ServerCapabilities;

use lsp_server::Connection;
use lsp_types::TextDocumentSyncCapability;

use log::{debug, info};

const CONFIG_FILE_NAME: &str = "config.json";
const IWE_MARKER: &str = ".iwe";

pub fn all_action_types() -> Vec<ActionType> {
    vec![
        ActionType::ListChangeType,
        ActionType::ListDetach,
        ActionType::ListToSections,
        ActionType::ReferenceInlineSection,
        ActionType::ReferenceInlineList,
        ActionType::ReferenceInlineQuote,
        ActionType::SectionExtractSubsections,
        ActionType::SectionToList,
        ActionType::SectionExtract,
    ]
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    if env::var("IWE_DEBUG").is_ok() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("iwe.log")
                    .expect("to open log file"),
            )
            .init();
    } else {
        tracing_subscriber::fmt::init()
    }

    info!("starting IWE LSP server");

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
            code_action_kinds: Some(
                all_action_types()
                    .iter()
                    .map(|it| CodeActionKind::new(it.identifier()))
                    .collect(),
            ),
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

    debug!("setttings: {:?}", settings);

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
    debug!("shutting down server");
    Ok(())
}
