use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use liwe::model::{graph::MarkdownOptions, Key};
use lsp_server::Connection;

use liwe::fs::new_for_path;
use router::{LspClient, Router, ServerConfig};

mod router;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Hash)]
pub struct InitializeParams {
    pub state: Option<String>,
    pub sequential_ids: Option<bool>,
    pub client_name: Option<String>,
}

pub fn main_loop(
    connection: Connection,
    params_value: serde_json::Value,
    base_path: String,
    markdown_options: MarkdownOptions,
) -> Result<()> {
    let initialize_params: InitializeParams = serde_json::from_value(params_value).unwrap();

    let client = initialize_params
        .clone()
        .client_name
        .filter(|name| name.eq("helix"))
        .map(|_| LspClient::Helix)
        .unwrap_or(LspClient::Unknown);

    let router = if let Some(state) = &(&initialize_params).state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: base_path.clone(),
                state: state
                    .split("\n_\n")
                    .enumerate()
                    .map(|(index, text)| {
                        (
                            Key::from_file_name(&(index + 1).to_string()),
                            text.trim().to_string(),
                        )
                    })
                    .collect(),
                sequential_ids: Some(true),
                lsp_client: client,
                markdown_options,
            },
        )
    } else {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: base_path.clone(),
                state: new_for_path(&PathBuf::from_str(&base_path).expect("to work")),
                sequential_ids: None,
                lsp_client: client,
                markdown_options,
            },
        )
    };

    router.run(connection.receiver)
}
