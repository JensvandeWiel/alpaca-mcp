//! Shared bootstrap for every MCP server in this workspace.
//!
//! A server only defines its configuration struct and its tools; [`run`] takes
//! care of the command line, configuration loading, logging and the stdio
//! transport.
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     mcp_core::run(mcp_core::server_spec!(), MyServer::new).await
//! }
//! ```

mod cli;
mod config;

pub use crate::cli::run;
pub use crate::config::{LoadedConfig, load_config};

/// Static identity of a server. Create it with [`server_spec!`].
#[derive(Debug, Clone, Copy)]
pub struct ServerSpec {
    /// Binary name, for example `hello-mcp`.
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
}

impl ServerSpec {
    /// Prefix of all environment variables of this server: `hello-mcp` -> `HELLO_MCP`.
    pub fn env_prefix(&self) -> String {
        self.name.replace('-', "_").to_ascii_uppercase()
    }
}

/// Builds a [`ServerSpec`] from the calling crate's Cargo metadata.
#[macro_export]
macro_rules! server_spec {
    () => {
        $crate::ServerSpec {
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            description: env!("CARGO_PKG_DESCRIPTION"),
        }
    };
}
