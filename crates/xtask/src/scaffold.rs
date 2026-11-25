//! `cargo xtask new <name>`: a working server from templates/server/.

use std::{fs, process::Command};

use anyhow::{Context, Result, bail};

use crate::workspace::{self, Settings};

#[derive(clap::Args)]
pub struct Args {
    /// Lowercase name such as `weather`; the crate becomes `weather-mcp`
    name: String,

    /// One line describing the server
    #[arg(long)]
    description: Option<String>,
}

const FILES: [(&str, &str); 5] = [
    (
        "Cargo.toml",
        include_str!("../templates/server/Cargo.toml.tmpl"),
    ),
    (
        "README.md",
        include_str!("../templates/server/README.md.tmpl"),
    ),
    (
        "config.example.toml",
        include_str!("../templates/server/config.example.toml.tmpl"),
    ),
    (
        "src/main.rs",
        include_str!("../templates/server/main.rs.tmpl"),
    ),
    (
        "src/lib.rs",
        include_str!("../templates/server/lib.rs.tmpl"),
    ),
];

pub fn run(args: Args) -> Result<()> {
    let name = args.name.strip_suffix("-mcp").unwrap_or(&args.name);
    validate(name)?;

    let dir = workspace::root().join("servers").join(name);
    if dir.exists() {
        bail!("{} already exists", dir.display());
    }

    let settings = Settings::load()?;
    let crate_name = format!("{name}-mcp");
    let description = args
        .description
        .unwrap_or_else(|| format!("{name} MCP server"));
    if description.chars().any(char::is_control) {
        bail!("the description must be a single line");
    }

    let replacements = [
        ("{{name}}", name.to_owned()),
        ("{{crate}}", crate_name.clone()),
        ("{{ident}}", crate_name.replace('-', "_")),
        ("{{type}}", format!("{}Server", pascal_case(name))),
        ("{{env}}", crate_name.replace('-', "_").to_uppercase()),
        ("{{npm}}", settings.npm_name(&crate_name)),
        (
            "{{description_toml}}",
            toml::Value::String(description.clone()).to_string(),
        ),
        // Last, so placeholder-like text in the description stays untouched.
        ("{{description}}", description),
    ];

    fs::create_dir_all(dir.join("src"))?;
    for (file, template) in FILES {
        let content = replacements
            .iter()
            .fold(template.to_owned(), |text, (from, to)| {
                text.replace(from, to)
            });
        let path = dir.join(file);
        fs::write(&path, content).with_context(|| format!("cannot write {}", path.display()))?;
    }

    // Long names change how rustfmt wraps the generated code.
    let formatted = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .args([dir.join("src/main.rs"), dir.join("src/lib.rs")])
        .status();
    if !formatted.is_ok_and(|status| status.success()) {
        eprintln!("warning: rustfmt failed; run `cargo fmt` yourself");
    }

    println!("created servers/{name}");
    println!("  cargo test -p {crate_name}");
    println!("  cargo xtask smoke {name}");
    Ok(())
}

fn validate(name: &str) -> Result<()> {
    let valid = name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    // `mcp-core` and `core-mcp` would be confusing next to each other.
    if !valid || matches!(name, "mcp" | "core" | "xtask") {
        bail!("invalid name `{name}`: use lowercase letters, digits and single dashes");
    }
    Ok(())
}

fn pascal_case(name: &str) -> String {
    name.split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}
