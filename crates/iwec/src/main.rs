use std::env;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use iwec::IweServer;
use liwe::model::config::load_config;
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

    #[arg(long, default_value_t = 8000)]
    port: u16,
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

    let configuration = load_config().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let current_dir = env::current_dir().expect("current dir");
    let mut library_path = current_dir.clone();
    if !configuration.library.path.is_empty() {
        library_path.push(configuration.library.path.clone());
    }

    let server = IweServer::new(&library_path.to_string_lossy(), &configuration);
    server.start_watching();

    match cli.transport {
        Transport::Stdio => {
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!("serving error: {:?}", e);
            })?;
            service.waiting().await?;
        }
        Transport::Http => {
            let bind_address = format!("127.0.0.1:{}", cli.port);
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
