use hello_mcp::HelloServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_core::run(mcp_core::server_spec!(), HelloServer::new).await
}
