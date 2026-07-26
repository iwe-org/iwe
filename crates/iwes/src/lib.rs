use std::str::FromStr;
use std::time::Duration;
use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use std::time::SystemTime;

use diwe::config::Configuration;
use lsp_server::Connection;

use crossbeam_channel::unbounded;
use diwe::fs::{new_for_path, new_from_hashmap};
use diwe::watcher::{start_poll_watcher, start_watcher, FsChange};
use router::{LspClient, Router, ServerConfig};

pub mod router;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq, Default)]
pub struct ServerParams {
    pub state: Option<HashMap<String, String>>,
    pub sequential_ids: Option<bool>,
    pub client_name: Option<String>,
    pub configuration: Configuration,
    pub base_path: String,
    #[serde(skip)]
    pub override_now: Option<SystemTime>,
    #[serde(skip)]
    pub watch_poll_interval: Option<Duration>,
}

pub fn main_loop(connection: Connection, params: ServerParams) -> Result<()> {
    let client = params
        .clone()
        .client_name
        .filter(|name| name.eq("helix"))
        .map(|_| LspClient::Helix)
        .unwrap_or(LspClient::Unknown);

    let watch_filesystem = params.state.is_none();
    let base_path = params.base_path.clone();
    let format = params.configuration.format;
    let watch_poll_interval = params.watch_poll_interval;

    let (fs_sender, fs_receiver) = unbounded::<FsChange>();

    let router = if let Some(state) = params.state {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: params.base_path.clone(),
                state: new_from_hashmap(state),
                sequential_ids: Some(true),
                lsp_client: client,
                configuration: params.configuration,
                override_now: params.override_now,
            },
        )
    } else {
        Router::new(
            connection.sender,
            ServerConfig {
                base_path: params.base_path.clone(),
                state: new_for_path(
                    &PathBuf::from_str(&params.base_path).expect("to work"),
                    params.configuration.format,
                ),
                sequential_ids: None,
                lsp_client: client,
                configuration: params.configuration,
                override_now: params.override_now,
            },
        )
    };

    let handler_sender = fs_sender.clone();
    let handler = move |change| {
        let _ = handler_sender.send(change);
    };
    let watch_root = PathBuf::from(base_path);
    let _watcher: Option<Box<dyn Send>> = if !watch_filesystem {
        None
    } else if let Some(interval) = watch_poll_interval {
        start_poll_watcher(watch_root, format, interval, handler)
            .map(|w| Box::new(w) as Box<dyn Send>)
    } else {
        start_watcher(watch_root, format, handler).map(|w| Box::new(w) as Box<dyn Send>)
    };
    let _fs_sender = fs_sender;

    router.run(connection.receiver, fs_receiver)
}
