#![allow(dead_code)]

use liwe::model::config::Configuration;
use rmcp::model::*;
use rmcp::{ClientHandler, ServiceExt, service::RunningService, RoleClient};
use iwec::IweServer;

#[derive(Default, Clone)]
struct TestClient;

impl ClientHandler for TestClient {}

pub struct Fixture {
    client: RunningService<RoleClient, TestClient>,
    _server_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Fixture {
    pub async fn with_documents(documents: Vec<(&str, &str)>) -> Self {
        Self::with_documents_and_config(documents, Configuration::default()).await
    }

    pub async fn with_documents_and_config(documents: Vec<(&str, &str)>, config: Configuration) -> Self {
        let server = IweServer::from_documents_with_config(documents, config);
        let (server_transport, client_transport) = tokio::io::duplex(65536);

        let server_handle = tokio::spawn(async move {
            let service = server.serve(server_transport).await?;
            service.waiting().await?;
            anyhow::Ok(())
        });

        let client = TestClient.serve(client_transport).await.expect("client to connect");

        Self {
            client,
            _server_handle: server_handle,
        }
    }

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> CallToolResult {
        self.try_call_tool(name, arguments)
            .await
            .expect("tool call to succeed")
    }

    pub async fn try_call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<CallToolResult, rmcp::ServiceError> {
        let args = match arguments {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            other => panic!("arguments must be an object, got: {other}"),
        };

        let params = CallToolRequestParams::new(name.to_string()).with_arguments(args);
        let response = self
            .client
            .send_request(ClientRequest::CallToolRequest(Request::new(params)))
            .await?;

        match response {
            ServerResult::CallToolResult(result) => Ok(result),
            other => panic!("expected CallToolResult, got: {other:?}"),
        }
    }

    pub fn result_text(result: &CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| match &c.raw {
                RawContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn result_json(result: &CallToolResult) -> serde_json::Value {
        let text = Self::result_text(result);
        serde_json::from_str(&text).expect("result to be valid JSON")
    }

    pub async fn list_resources(&self) -> ListResourcesResult {
        let response = self
            .client
            .send_request(ClientRequest::ListResourcesRequest(
                rmcp::model::RequestOptionalParam::default(),
            ))
            .await
            .expect("list resources to succeed");

        match response {
            ServerResult::ListResourcesResult(result) => result,
            other => panic!("expected ListResourcesResult, got: {other:?}"),
        }
    }

    pub async fn read_resource(&self, uri: &str) -> ReadResourceResult {
        let params = ReadResourceRequestParams::new(uri.to_string());
        let response = self
            .client
            .send_request(ClientRequest::ReadResourceRequest(Request::new(params)))
            .await
            .expect("read resource to succeed");

        match response {
            ServerResult::ReadResourceResult(result) => result,
            other => panic!("expected ReadResourceResult, got: {other:?}"),
        }
    }

    pub async fn list_prompts(&self) -> ListPromptsResult {
        let response = self
            .client
            .send_request(ClientRequest::ListPromptsRequest(
                rmcp::model::RequestOptionalParam::default(),
            ))
            .await
            .expect("list prompts to succeed");

        match response {
            ServerResult::ListPromptsResult(result) => result,
            other => panic!("expected ListPromptsResult, got: {other:?}"),
        }
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> GetPromptResult {
        let params = match arguments {
            serde_json::Value::Object(map) => {
                let string_map: serde_json::Map<String, serde_json::Value> = map
                    .into_iter()
                    .map(|(k, v)| {
                        let s = v.as_str().unwrap_or_default().to_string();
                        (k, serde_json::Value::String(s))
                    })
                    .collect();
                GetPromptRequestParams::new(name.to_string()).with_arguments(string_map)
            }
            serde_json::Value::Null => GetPromptRequestParams::new(name.to_string()),
            other => panic!("arguments must be an object, got: {other}"),
        };
        let response = self
            .client
            .send_request(ClientRequest::GetPromptRequest(Request::new(params)))
            .await
            .expect("get prompt to succeed");

        match response {
            ServerResult::GetPromptResult(result) => result,
            other => panic!("expected GetPromptResult, got: {other:?}"),
        }
    }
}
