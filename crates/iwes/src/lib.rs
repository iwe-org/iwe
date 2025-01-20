use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Result;
use liwe::model::graph::MarkdownOptions;
use lsp_server::Connection;

use liwe::fs::new_for_path;
use liwe::state::new_form_indoc;
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
    markdown_options: MarkdownOptions,
) -> Result<()> {
    let initialize_params: InitializeParams = serde_json::from_value(params_value).unwrap();

    let router = if let Some(state) = &(&initialize_params).state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: base_path.clone(),
                state: new_form_indoc(state),
                sequential_ids: Some(true),
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
                markdown_options,
            },
        )
    };

    router.run(connection.receiver)
}
