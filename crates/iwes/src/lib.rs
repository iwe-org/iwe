use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use liwe::model::config::Configuration;
use lsp_server::Connection;

use liwe::fs::{new_for_path, new_from_hashmap};
use router::{LspClient, Router, ServerConfig};

pub mod router;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Default)]
pub struct ServerParams {
    pub state: Option<HashMap<String, String>>,
    pub sequential_ids: Option<bool>,
    pub client_name: Option<String>,
    pub configuration: Configuration,
    pub base_path: String,
}

pub fn main_loop(connection: Connection, params: ServerParams) -> Result<()> {
    let client = params
        .clone()
        .client_name
        .filter(|name| name.eq("helix"))
        .map(|_| LspClient::Helix)
        .unwrap_or(LspClient::Unknown);

    let router = if let Some(state) = params.state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: params.base_path.clone(),
                state: new_from_hashmap(state),
                sequential_ids: Some(true),
                lsp_client: client,
                configuration: params.configuration,
            },
        )
    } else {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: params.base_path.clone(),
                state: new_for_path(&PathBuf::from_str(&params.base_path).expect("to work")),
                sequential_ids: None,
                lsp_client: client,
                configuration: params.configuration,
            },
        )
    };

    router.run(connection.receiver)
}
