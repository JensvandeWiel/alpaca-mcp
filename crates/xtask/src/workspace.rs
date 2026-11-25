//! Facts about this workspace and helpers to run external tools.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask lives in <root>/crates/xtask")
        .to_path_buf()
}

/// `[workspace.metadata.mcp]` in the root Cargo.toml.
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct Settings {
    pub npm_scope: String,
    pub license: String,
    pub repository: String,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let path = root().join("Cargo.toml");
        let manifest: toml::Table = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?
            .parse()?;
        match manifest
            .get("workspace")
            .and_then(|w| w.get("metadata"))
            .and_then(|m| m.get("mcp"))
        {
            Some(table) => table
                .clone()
                .try_into()
                .context("invalid [workspace.metadata.mcp]"),
            None => Ok(Self::default()),
        }
    }

    pub fn npm_name(&self, crate_name: &str) -> String {
        if self.npm_scope.is_empty() {
            crate_name.to_owned()
        } else {
            format!("{}/{crate_name}", self.npm_scope)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Server {
    /// Directory name below servers/, for example `hello`.
    pub name: String,
    /// Crate and binary name, for example `hello-mcp`.
    pub crate_name: String,
    pub npm_name: String,
    pub version: String,
    pub description: String,
    pub dir: PathBuf,
}

pub struct Workspace {
    pub settings: Settings,
    pub target_dir: PathBuf,
    pub servers: Vec<Server>,
}

#[derive(Deserialize)]
struct Metadata {
    target_directory: PathBuf,
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    description: Option<String>,
    manifest_path: PathBuf,
}

impl Workspace {
    pub fn load() -> Result<Self> {
        let settings = Settings::load()?;
        let output = capture(
            Command::new("cargo")
                .args(["metadata", "--no-deps", "--format-version", "1"])
                .current_dir(root()),
        )?;
        let metadata: Metadata = serde_json::from_str(&output)?;

        let mut servers: Vec<Server> = metadata
            .packages
            .into_iter()
            .filter_map(|package| {
                let dir = package.manifest_path.parent()?.to_path_buf();
                if dir.parent()?.file_name()? != OsStr::new("servers") {
                    return None;
                }
                Some(Server {
                    name: dir.file_name()?.to_str()?.to_owned(),
                    npm_name: settings.npm_name(&package.name),
                    crate_name: package.name,
                    version: package.version,
                    description: package.description.unwrap_or_default(),
                    dir,
                })
            })
            .collect();
        servers.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            settings,
            target_dir: metadata.target_directory,
            servers,
        })
    }

    /// Finds a server by directory name (`hello`) or crate name (`hello-mcp`).
    pub fn server(&self, name: &str) -> Result<&Server> {
        self.servers
            .iter()
            .find(|s| s.name == name || s.crate_name == name)
            .with_context(|| {
                let known: Vec<&str> = self.servers.iter().map(|s| s.name.as_str()).collect();
                format!("no server `{name}`; available: {}", known.join(", "))
            })
    }
}

pub fn list(json: bool) -> Result<()> {
    let workspace = Workspace::load()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&workspace.servers)?);
    } else {
        for s in &workspace.servers {
            println!(
                "{:<20} {:<10} {:<40} {}",
                s.name, s.version, s.npm_name, s.description
            );
        }
    }
    Ok(())
}

/// A command for an external tool. npm and npx are batch files on Windows.
pub fn tool(name: &str) -> Command {
    if cfg!(windows) && matches!(name, "npm" | "npx") {
        Command::new(format!("{name}.cmd"))
    } else {
        Command::new(name)
    }
}

pub fn execute(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("cannot start {command:?}"))?;
    if !status.success() {
        bail!("{command:?} failed with {status}");
    }
    Ok(())
}

pub fn capture(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("cannot start {command:?}"))?;
    if !output.status.success() {
        bail!(
            "{command:?} failed with {}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?)
}
