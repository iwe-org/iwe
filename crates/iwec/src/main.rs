#[cfg(target_env = "musl")]
use mimalloc::MiMalloc;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use std::env;
use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use diwe::config::load_config;
use iwec::IweServer;
use rmcp::transport::stdio;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Transport {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(name = "iwec", version, about = "IWE MCP server")]
struct Cli {
    #[arg(long, value_enum, default_value_t = Transport::Stdio)]
    transport: Transport,

    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value_t = 8000)]
    port: u16,

    #[arg(long, value_name = "TEXT", conflicts_with = "instructions_file")]
    instructions: Option<String>,

    #[arg(long, value_name = "PATH")]
    instructions_file: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    workspace: Option<PathBuf>,
}

fn exit_with_error(message: String) -> ! {
    eprintln!("Error: {}", message);
    std::process::exit(1);
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if env::var("IWE_DEBUG").is_ok() {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()),
            )
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .init();
    }

    tracing::info!("starting IWE MCP server");

    let instructions = cli.instructions.or_else(|| {
        cli.instructions_file.as_ref().map(|path| {
            read_to_string(path).unwrap_or_else(|e| {
                exit_with_error(format!(
                    "Failed to read instructions file '{}': {}",
                    path.display(),
                    e
                ))
            })
        })
    });

    if let Some(workspace) = &cli.workspace {
        env::set_current_dir(workspace).unwrap_or_else(|e| {
            exit_with_error(format!(
                "Failed to open workspace '{}': {}",
                workspace.display(),
                e
            ))
        });
    }

    let configuration = load_config().unwrap_or_else(|e| exit_with_error(e));

    let current_dir = env::current_dir().expect("current dir");

    let mut server = IweServer::new(&current_dir.to_string_lossy(), &configuration);
    if let Some(instructions) = instructions {
        server = server.with_instructions(instructions);
    }
    server.start_watching();

    match cli.transport {
        Transport::Stdio => {
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;
            service.waiting().await?;
        }
        Transport::Http => {
            let bind_address = format!("{}:{}", cli.host, cli.port);
            let cancellation = CancellationToken::new();
            let service = StreamableHttpService::new(
                move || Ok(server.clone()),
                LocalSessionManager::default().into(),
                StreamableHttpServerConfig::default()
                    .with_cancellation_token(cancellation.child_token()),
            );
            let router = axum::Router::new().nest_service("/mcp", service);
            let listener = tokio::net::TcpListener::bind(&bind_address).await?;
            tracing::info!("listening on http://{}/mcp", bind_address);
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    tokio::signal::ctrl_c().await.ok();
                    cancellation.cancel();
                })
                .await?;
        }
    }

    Ok(())
}
