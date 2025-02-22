use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use liwe::model::graph::MarkdownOptions;
use lsp_server::Connection;

use liwe::fs::{new_for_path, new_from_hashmap};
use router::{LspClient, Router, ServerConfig};

mod router;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq)]
pub struct InitializeParams {
    pub state: Option<HashMap<String, String>>,
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

    let router = if let Some(state) = initialize_params.state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: base_path.clone(),
                state: new_from_hashmap(state),
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
