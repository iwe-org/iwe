use std::env;
use std::error::Error;
use std::fs::OpenOptions;

use iwes::main_loop;
use iwes::router::server::actions::all_action_types;
use iwes::router::server::actions::ActionProvider;
use iwes::ServerParams;
use liwe::model::config::load_config;
use lsp_types::CodeActionOptions;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CompletionOptions;
use lsp_types::ExecuteCommandOptions;
use lsp_types::HoverProviderCapability;
use lsp_types::InitializeParams;
use lsp_types::OneOf;
use lsp_types::RenameOptions;
use lsp_types::ServerCapabilities;
use lsp_types::WorkspaceFileOperationsServerCapabilities;
use lsp_types::WorkspaceServerCapabilities;

use lsp_server::Connection;
use lsp_types::TextDocumentSyncCapability;

use log::{debug, info};

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
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(std::io::stderr)
            .init();
    }

    info!("starting IWE LSP server");

    let configuration = load_config();

    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(&ServerCapabilities {
        references_provider: Some(OneOf::Left(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        definition_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(true),
            trigger_characters: Some(vec!["+".to_string()]),
            all_commit_characters: None,
            work_done_progress_options: Default::default(),
            completion_item: None,
        }),
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
                all_action_types(&configuration)
                    .iter()
                    .map(|it| it.action_kind())
                    .collect(),
            ),
            resolve_provider: Some(true),
            ..Default::default()
        })),
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec!["generate".to_string()],
            work_done_progress_options: Default::default(),
        }),
        workspace: Some(WorkspaceServerCapabilities {
            workspace_folders: None,
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                did_create: None,
                will_create: None,
                did_rename: None,
                will_rename: None,
                did_delete: None,
                will_delete: None,
            }),
        }),
        ..Default::default()
    })
    .unwrap();
    let initialization_params_value = match connection.initialize(server_capabilities) {
        Ok(it) => it,
        Err(e) => {
            if e.channel_is_disconnected() {
                io_threads.join()?;
            }
            return Err(e.into());
        }
    };

    let initialize_params: InitializeParams =
        serde_json::from_value(initialization_params_value).unwrap();

    let current_dir = env::current_dir().expect("to get current dir");
    let mut library_path = current_dir.clone();

    debug!("config: {:?}", configuration);

    if !configuration.library.path.is_empty() {
        library_path.push(configuration.clone().library.path);
    }

    main_loop(
        connection,
        ServerParams {
            client_name: initialize_params.client_info.map(|it| it.name),
            configuration: configuration.clone(),
            base_path: library_path.to_string_lossy().to_string(),
            ..Default::default()
        },
    )?;
    io_threads.join()?;

    // Shut down gracefully.
    debug!("shutting down server");
    Ok(())
}
