use std::env;

use anyhow::Result;
use iwec::IweServer;
use liwe::model::config::load_config;
use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
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

    let configuration = load_config();

    let current_dir = env::current_dir().expect("current dir");
    let mut library_path = current_dir.clone();
    if !configuration.library.path.is_empty() {
        library_path.push(configuration.library.path.clone());
    }

    let server = IweServer::new(&library_path.to_string_lossy(), &configuration);
    server.start_watching();

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
