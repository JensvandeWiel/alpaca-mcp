//! Publishes the output of `package` to the npm registry.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};

use crate::{
    package::{self, Manifest, PLATFORMS},
    workspace::{self, Workspace},
};

#[derive(clap::Args)]
pub struct Args {
    /// Server name, for example `hello`
    name: String,

    /// Really publish. Without this flag only `npm pack --dry-run` runs
    #[arg(long)]
    execute: bool,

    /// Attach npm provenance (CI with a public repository only)
    #[arg(long)]
    provenance: bool,

    /// npm dist-tag. Default: `next` for pre-releases, else `latest`
    #[arg(long)]
    tag: Option<String>,

    /// Allow publishing without a release binary for every platform
    #[arg(long)]
    allow_partial: bool,
}

pub fn run(args: Args) -> Result<()> {
    let workspace = Workspace::load()?;
    let server = workspace.server(&args.name)?;
    let out = package::output_dir(&server.crate_name);
    let manifest: Manifest = serde_json::from_str(
        &fs::read_to_string(out.join("manifest.json")).with_context(|| {
            format!(
                "nothing packaged; run `cargo xtask package {}` first",
                server.name
            )
        })?,
    )?;

    if manifest.version != server.version {
        bail!(
            "dist/ holds version {} but {} is at {}; run `cargo xtask package` again",
            manifest.version,
            server.crate_name,
            server.version
        );
    }

    let incomplete: Vec<&str> = PLATFORMS
        .iter()
        .filter(|p| manifest.platforms.get(p.key).map(String::as_str) != Some(p.target))
        .map(|p| p.key)
        .collect();
    if !incomplete.is_empty() {
        let message = format!("no release binary for: {}", incomplete.join(", "));
        if args.execute && !args.allow_partial {
            bail!("{message}. Release through CI, or pass --allow-partial");
        }
        eprintln!("warning: {message}");
    }

    let tag = args.tag.unwrap_or_else(|| {
        if server.version.contains('-') {
            "next".to_owned()
        } else {
            "latest".to_owned()
        }
    });

    // The main package goes last: it must not appear before its dependencies.
    let packages = manifest
        .platforms
        .keys()
        .map(|key| (out.join(key), format!("{}-{key}", server.npm_name)))
        .chain([(out.join("main"), server.npm_name.clone())]);

    for (dir, name) in packages {
        if !args.execute {
            println!("--- dry run: {name}@{} (tag {tag})", server.version);
            workspace::execute(
                workspace::tool("npm")
                    .args(["pack", "--dry-run"])
                    .current_dir(&dir),
            )?;
        } else if is_published(&dir, &name, &server.version) {
            // Makes a half-finished release safe to run again.
            println!("{name}@{} is already published, skipping", server.version);
        } else {
            let mut npm = workspace::tool("npm");
            npm.args(["publish", "--access", "public", "--tag", &tag])
                .current_dir(&dir);
            if args.provenance {
                npm.arg("--provenance");
            }
            workspace::execute(&mut npm)?;
        }
    }

    if !args.execute {
        println!("dry run only; add --execute to publish");
    }
    Ok(())
}

fn is_published(dir: &Path, name: &str, version: &str) -> bool {
    workspace::capture(
        workspace::tool("npm")
            .args(["view", &format!("{name}@{version}"), "version"])
            .current_dir(dir),
    )
    .is_ok_and(|output| output.trim() == version)
}
