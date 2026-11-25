use woodpecker_mcp::WoodpeckerServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    mcp_core::run(mcp_core::server_spec!(), WoodpeckerServer::new).await
}
