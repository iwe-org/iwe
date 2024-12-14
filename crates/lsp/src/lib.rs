#![allow(dead_code, unused_imports, unused_variables, deprecated)]
use std::error::Error;
use std::{env, path::PathBuf};

use anyhow::Result;
use itertools::Itertools;
use lsp_server::Connection;

use lib::fs::new_for_path;
use lib::state::new_form_indoc;
use router::{Router, ServerConfig};

mod router;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Eq, Hash)]
pub struct InitializeParams {
    pub state: Option<String>,
    pub sequential_ids: Option<bool>,
}

pub fn main_loop(
    connection: Connection,
    params_value: serde_json::Value,
    base_path: String,
) -> Result<()> {
    let initialize_params: InitializeParams = serde_json::from_value(params_value).unwrap();

    let router = if let Some(state) = &(&initialize_params).state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path,
                state: new_form_indoc(state),
                sequential_ids: Some(true),
            },
        )
    } else {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path,
                state: new_for_path(&env::current_dir().expect("to get current dir")),
                sequential_ids: None,
            },
        )
    };

    router.run(connection.receiver)
}
