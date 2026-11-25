use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use rmcp::{ServerHandler, ServiceExt, transport::stdio};
use serde::de::DeserializeOwned;
use tracing_subscriber::EnvFilter;

use crate::{ServerSpec, load_config};

#[derive(Debug, Parser)]
struct Cli {
    /// Configuration file (TOML). Env: <PREFIX>_CONFIG
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Override a configuration value; may be repeated
    #[arg(long = "set", global = true, value_name = "KEY=VALUE", value_parser = parse_key_value)]
    overrides: Vec<(String, String)>,

    /// Log filter, for example `debug` or `hello_mcp=trace`. Env: <PREFIX>_LOG
    #[arg(long, global = true, value_name = "FILTER")]
    log: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve MCP over stdin/stdout (default)
    Serve,
    /// Validate the configuration and exit
    Check,
}

fn parse_key_value(raw: &str) -> Result<(String, String), String> {
    match raw.split_once('=') {
        Some((key, value)) if !key.is_empty() => Ok((key.to_owned(), value.to_owned())),
        _ => Err(format!("expected KEY=VALUE, got `{raw}`")),
    }
}

/// Runs a server: parses the command line, loads `C`, builds the handler with
/// `build` and serves it.
pub async fn run<C, S, F>(spec: ServerSpec, build: F) -> Result<()>
where
    C: DeserializeOwned,
    S: ServerHandler,
    F: FnOnce(C) -> Result<S>,
{
    let command = Cli::command()
        .name(spec.name)
        .version(spec.version)
        .about(spec.description)
        .after_help(format!(
            "Every configuration key can also be set with an environment variable:\n  \
             {}_<KEY> (use `__` for nested keys).",
            spec.env_prefix()
        ));
    let cli = Cli::from_arg_matches(&command.get_matches())?;

    // Variables that are not valid UTF-8 cannot be configuration values; skip them
    // instead of panicking like `std::env::vars` would.
    let env: HashMap<String, String> = std::env::vars_os()
        .filter_map(|(k, v)| Some((k.into_string().ok()?, v.into_string().ok()?)))
        .collect();

    let log = cli
        .log
        .or_else(|| env.get(&format!("{}_LOG", spec.env_prefix())).cloned());
    init_logging(log.as_deref())?;

    let loaded = load_config::<C>(&spec, cli.config.as_deref(), env, &cli.overrides)?;
    let server = build(loaded.config).context("failed to initialise the server")?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Check => {
            match &loaded.file {
                Some(path) => eprintln!("config file: {}", path.display()),
                None => eprintln!("config file: none (defaults, environment and --set only)"),
            }
            eprintln!("{} {}: configuration OK", spec.name, spec.version);
        }
        Command::Serve => {
            tracing::info!(name = spec.name, version = spec.version, "serving on stdio");
            let service = server
                .serve(stdio())
                .await
                .context("MCP initialisation failed")?;
            service.waiting().await?;
        }
    }
    Ok(())
}

/// Logs always go to stderr: stdout belongs to the MCP protocol.
fn init_logging(filter: Option<&str>) -> Result<()> {
    let filter =
        match filter {
            Some(filter) => EnvFilter::try_new(filter)
                .with_context(|| format!("invalid log filter `{filter}`"))?,
            // rmcp logs whole protocol messages at `info`, which drowns client log panes.
            None => EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,rmcp=warn")),
        };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
    Ok(())
}
