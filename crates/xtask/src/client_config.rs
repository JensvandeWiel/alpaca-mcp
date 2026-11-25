//! Prints the snippet that registers a server in an MCP client.

use anyhow::Result;
use clap::ValueEnum;
use serde_json::{Map, Value, json};

use crate::workspace::{self, Server, Workspace};

#[derive(Clone, Copy, ValueEnum)]
enum Runner {
    /// Published package through Node
    Npx,
    /// Published package through Bun
    Bunx,
    /// This checkout through `cargo run`
    Cargo,
    /// This checkout's release binary
    Binary,
}

#[derive(Clone, Copy, ValueEnum)]
enum Client {
    /// Claude Code (.mcp.json), Claude Desktop and Cursor
    Claude,
    /// VS Code (.vscode/mcp.json)
    Vscode,
}

#[derive(clap::Args)]
pub struct Args {
    /// Server name, for example `hello`
    name: String,

    #[arg(long, value_enum, default_value = "npx")]
    runner: Runner,

    #[arg(long, value_enum, default_value = "claude")]
    client: Client,

    /// Pin npx/bunx to the current version instead of `latest`
    #[arg(long)]
    pin: bool,

    /// Environment variable for the server; may be repeated
    #[arg(long = "env", value_name = "KEY=VALUE")]
    env: Vec<String>,

    /// Arguments passed to the server, after `--`
    #[arg(last = true)]
    server_args: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let workspace = Workspace::load()?;
    let server = workspace.server(&args.name)?;
    let (command, mut command_args) = command_line(&workspace, server, args.runner, args.pin);
    command_args.extend(args.server_args);

    let mut entry = Map::new();
    if let Client::Vscode = args.client {
        entry.insert("type".into(), json!("stdio"));
    }
    entry.insert("command".into(), json!(command));
    entry.insert("args".into(), json!(command_args));
    if !args.env.is_empty() {
        let env: Map<String, Value> = args
            .env
            .iter()
            .map(|pair| {
                let (key, value) = pair
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("--env expects KEY=VALUE, got `{pair}`"))?;
                Ok((key.to_owned(), json!(value)))
            })
            .collect::<Result<_>>()?;
        entry.insert("env".into(), Value::Object(env));
    }

    let key = match args.client {
        Client::Claude => "mcpServers",
        Client::Vscode => "servers",
    };
    let snippet = json!({ key: { server.name.as_str(): entry } });
    println!("{}", serde_json::to_string_pretty(&snippet)?);
    Ok(())
}

fn command_line(
    workspace: &Workspace,
    server: &Server,
    runner: Runner,
    pin: bool,
) -> (String, Vec<String>) {
    let version = if pin { &server.version } else { "latest" };
    let package = format!("{}@{version}", server.npm_name);
    let strings = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    match runner {
        // MCP clients on Windows cannot start the npx batch file directly.
        Runner::Npx if cfg!(windows) => ("cmd".into(), strings(&["/c", "npx", "-y", &package])),
        Runner::Npx => ("npx".into(), strings(&["-y", &package])),
        Runner::Bunx => ("bunx".into(), strings(&[&package])),
        Runner::Cargo => {
            let manifest = workspace::root().join("Cargo.toml");
            (
                "cargo".into(),
                strings(&[
                    "run",
                    "--quiet",
                    "--release",
                    "--manifest-path",
                    &manifest.to_string_lossy(),
                    "-p",
                    &server.crate_name,
                    "--",
                ]),
            )
        }
        Runner::Binary => {
            let file = format!("{}{}", server.crate_name, std::env::consts::EXE_SUFFIX);
            let path = workspace.target_dir.join("release").join(file);
            (path.to_string_lossy().into_owned(), Vec::new())
        }
    }
}
