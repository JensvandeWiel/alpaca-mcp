//! `cargo xtask <command>`: everything around the MCP servers in `servers/`.

mod client_config;
mod commits;
mod package;
mod publish;
mod scaffold;
mod smoke;
mod workspace;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cargo xtask",
    bin_name = "cargo xtask",
    about = "Manage the MCP servers"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new server in servers/<name>
    New(scaffold::Args),
    /// List the servers in this workspace
    List {
        /// Machine readable output
        #[arg(long)]
        json: bool,
    },
    /// Start a server, do the MCP handshake and list its tools
    Smoke(smoke::Args),
    /// Print the MCP client configuration for a server
    Config(client_config::Args),
    /// Build the npm packages of a server into dist/npm/
    Package(package::Args),
    /// Publish packaged npm packages (dry run unless --execute)
    Publish(publish::Args),
    /// Check commit messages or a PR title against Conventional Commits
    LintCommits(commits::Args),
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::New(args) => scaffold::run(args),
        Command::List { json } => workspace::list(json),
        Command::Smoke(args) => smoke::run(args),
        Command::Config(args) => client_config::run(args),
        Command::Package(args) => package::run(args),
        Command::Publish(args) => publish::run(args),
        Command::LintCommits(args) => commits::run(args),
    }
}
