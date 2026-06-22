use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use rmcp::model::{
    CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, RawContent,
};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::ServiceExt;

struct ServerProcess {
    child: Child,
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("local addr")
        .port()
}

async fn wait_until_listening(addr: &str) {
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start listening on {addr}");
}

#[tokio::test]
async fn http_transport_serves_the_graph() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("welcome.md"),
        "# Welcome\n\nFirst document.\n",
    )
    .expect("write document");

    let port = free_port();
    let _server = ServerProcess {
        child: Command::new(env!("CARGO_BIN_EXE_iwec"))
            .arg("--transport")
            .arg("http")
            .arg("--port")
            .arg(port.to_string())
            .current_dir(dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn iwec"),
    };

    let addr = format!("127.0.0.1:{port}");
    wait_until_listening(&addr).await;

    let transport = StreamableHttpClientTransport::from_uri(format!("http://{addr}/mcp"));
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("iwec-test-client", "0.0.1"),
    );
    let client = client_info
        .serve(transport)
        .await
        .expect("client to connect");

    let result = client
        .call_tool(
            CallToolRequestParams::new("iwe_find").with_arguments(
                serde_json::json!({ "query": "welcome" })
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await
        .expect("tool call to succeed");

    let text = result
        .content
        .iter()
        .filter_map(|c| match &c.raw {
            RawContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<String>();
    let json: serde_json::Value = serde_json::from_str(&text).expect("result to be valid JSON");

    assert_eq!(
        json,
        serde_json::json!([
            {
                "key": "welcome",
                "title": "Welcome",
                "references": [],
                "includes": [],
                "referencedBy": [],
                "includedBy": []
            }
        ])
    );

    client.cancel().await.expect("client to disconnect");
}
