//! Builds the npm packages of one server.
//!
//! Layout (the same approach esbuild uses): a small main package with a Node
//! launcher, plus one package per platform holding the native binary. The main
//! package lists those as `optionalDependencies`, so npm and bun only download
//! the one that matches. No install scripts are involved.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::workspace::{self, Server, Workspace};

#[derive(Debug, Clone, Copy)]
pub struct Platform {
    /// `<process.platform>-<process.arch>` as Node reports it.
    pub key: &'static str,
    /// Rust target used for released binaries.
    pub target: &'static str,
    pub os: &'static str,
    pub cpu: &'static str,
}

const fn platform(
    key: &'static str,
    target: &'static str,
    os: &'static str,
    cpu: &'static str,
) -> Platform {
    Platform {
        key,
        target,
        os,
        cpu,
    }
}

/// Keep in sync with the build matrix in .github/workflows/release.yml.
/// Linux uses musl so one static binary runs on every distribution.
pub const PLATFORMS: [Platform; 6] = [
    platform("win32-x64", "x86_64-pc-windows-msvc", "win32", "x64"),
    platform("win32-arm64", "aarch64-pc-windows-msvc", "win32", "arm64"),
    platform("darwin-x64", "x86_64-apple-darwin", "darwin", "x64"),
    platform("darwin-arm64", "aarch64-apple-darwin", "darwin", "arm64"),
    platform("linux-x64", "x86_64-unknown-linux-musl", "linux", "x64"),
    platform(
        "linux-arm64",
        "aarch64-unknown-linux-musl",
        "linux",
        "arm64",
    ),
];

/// Written next to the packages so `publish` knows what was built.
#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    /// Platform key -> Rust target the binary was built for.
    pub platforms: BTreeMap<String, String>,
}

#[derive(clap::Args)]
pub struct Args {
    /// Server name, for example `hello`
    pub name: String,

    /// Rust target to build; may be repeated. Default: this machine
    #[arg(long = "target", value_name = "TRIPLE", conflicts_with = "artifacts")]
    pub targets: Vec<String>,

    /// Use prebuilt binaries from <DIR>/<target>/<binary> instead of building
    #[arg(long, value_name = "DIR")]
    pub artifacts: Option<PathBuf>,

    /// Fail unless the server has exactly this version (used by CI)
    #[arg(long, value_name = "VERSION")]
    pub expect_version: Option<String>,
}

pub fn output_dir(crate_name: &str) -> PathBuf {
    workspace::root().join("dist").join("npm").join(crate_name)
}

pub fn run(args: Args) -> Result<()> {
    let workspace = Workspace::load()?;
    let server = workspace.server(&args.name)?;
    if let Some(expected) = &args.expect_version
        && *expected != server.version
    {
        bail!(
            "{} has version {} but {expected} was expected; update servers/{}/Cargo.toml",
            server.crate_name,
            server.version,
            server.name
        );
    }

    let binaries = match &args.artifacts {
        Some(dir) => prebuilt(server, dir)?,
        None => build(&workspace, server, &args.targets)?,
    };
    assemble(&workspace, server, &binaries)?;
    Ok(())
}

type Binaries = Vec<(Platform, String, PathBuf)>;

fn exe_name(server: &Server, platform: Platform) -> String {
    let suffix = if platform.os == "win32" { ".exe" } else { "" };
    format!("{}{suffix}", server.crate_name)
}

fn prebuilt(server: &Server, dir: &Path) -> Result<Binaries> {
    let found: Binaries = PLATFORMS
        .into_iter()
        .filter_map(|p| {
            let path = dir.join(p.target).join(exe_name(server, p));
            path.is_file().then(|| (p, p.target.to_owned(), path))
        })
        .collect();
    if found.is_empty() {
        bail!(
            "no binaries found; expected {}/<target>/{}",
            dir.display(),
            server.crate_name
        );
    }
    Ok(found)
}

/// Maps a Rust target onto the npm platform it runs on.
fn platform_of(target: &str) -> Result<Platform> {
    let cpu = match target.split('-').next() {
        Some("x86_64") => "x64",
        Some("aarch64") => "arm64",
        _ => bail!("unsupported architecture in target `{target}`"),
    };
    let os = if target.contains("windows") {
        "win32"
    } else if target.contains("apple-darwin") {
        "darwin"
    } else if target.contains("linux") {
        "linux"
    } else {
        bail!("unsupported operating system in target `{target}`");
    };
    PLATFORMS
        .into_iter()
        .find(|p| p.os == os && p.cpu == cpu)
        .context("unsupported platform")
}

fn host_target() -> Result<String> {
    workspace::capture(Command::new("rustc").arg("-vV"))?
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .context("`rustc -vV` did not report a host target")
}

pub fn build(workspace: &Workspace, server: &Server, targets: &[String]) -> Result<Binaries> {
    let targets = if targets.is_empty() {
        vec![host_target()?]
    } else {
        targets.to_vec()
    };

    targets
        .into_iter()
        .map(|target| {
            let platform = platform_of(&target)?;
            workspace::execute(
                Command::new("cargo")
                    .args(["build", "--release", "-p", &server.crate_name])
                    .args(["--target", &target])
                    .current_dir(workspace::root()),
            )?;
            let binary = workspace
                .target_dir
                .join(&target)
                .join("release")
                .join(exe_name(server, platform));
            Ok((platform, target, binary))
        })
        .collect()
}

pub fn assemble(workspace: &Workspace, server: &Server, binaries: &Binaries) -> Result<()> {
    let out = output_dir(&server.crate_name);
    if out.exists() {
        fs::remove_dir_all(&out).with_context(|| format!("cannot clean {}", out.display()))?;
    }

    let readme = fs::read_to_string(server.dir.join("README.md"))
        .unwrap_or_else(|_| format!("# {}\n\n{}\n", server.crate_name, server.description));
    let mut optional = BTreeMap::new();
    let mut launcher_map = BTreeMap::new();
    let mut manifest = Manifest {
        version: server.version.clone(),
        platforms: BTreeMap::new(),
    };

    for (platform, target, binary) in binaries {
        let name = format!("{}-{}", server.npm_name, platform.key);
        let dir = out.join(platform.key);
        let destination = dir.join("bin").join(exe_name(server, *platform));
        fs::create_dir_all(dir.join("bin"))?;
        fs::copy(binary, &destination)
            .with_context(|| format!("cannot copy {}", binary.display()))?;
        make_executable(&destination)?;

        let mut package = base_package(workspace, server, &name);
        package["description"] =
            json!(format!("{} binary for {}", server.crate_name, platform.key));
        package["os"] = json!([platform.os]);
        package["cpu"] = json!([platform.cpu]);
        write_json(&dir.join("package.json"), &package)?;
        fs::write(
            dir.join("README.md"),
            format!(
                "# {name}\n\nNative binary for `{}`. Install that package instead.\n",
                server.npm_name
            ),
        )?;

        optional.insert(name.clone(), server.version.clone());
        launcher_map.insert(platform.key, name);
        manifest
            .platforms
            .insert(platform.key.to_owned(), target.clone());
    }

    let main = out.join("main");
    fs::create_dir_all(main.join("bin"))?;
    let launcher = include_str!("../templates/npm/cli.js")
        .replace("{{bin}}", &server.crate_name)
        .replace(
            "{{packages}}",
            &serde_json::to_string_pretty(&launcher_map)?,
        );
    fs::write(main.join("bin").join("cli.js"), launcher)?;
    make_executable(&main.join("bin").join("cli.js"))?;

    let mut package = base_package(workspace, server, &server.npm_name);
    package["bin"] = json!({ server.crate_name.as_str(): "bin/cli.js" });
    package["engines"] = json!({ "node": ">=18" });
    package["optionalDependencies"] = json!(optional);
    write_json(&main.join("package.json"), &package)?;
    fs::write(main.join("README.md"), readme)?;

    // npm always includes a LICENSE file that sits next to package.json.
    let license = workspace::root().join("LICENSE");
    if license.is_file() {
        for dir in manifest
            .platforms
            .keys()
            .map(String::as_str)
            .chain(["main"])
        {
            fs::copy(&license, out.join(dir).join("LICENSE")).context("cannot copy LICENSE")?;
        }
    }

    write_json(
        &out.join("manifest.json"),
        &serde_json::to_value(&manifest)?,
    )?;
    println!(
        "packaged {} {} for {} -> {}",
        server.npm_name,
        server.version,
        manifest
            .platforms
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        out.display()
    );
    Ok(())
}

fn base_package(workspace: &Workspace, server: &Server, name: &str) -> Value {
    let mut package = json!({
        "name": name,
        "version": server.version,
        "description": server.description,
        "files": ["bin"],
        "publishConfig": { "access": "public" },
    });
    let settings = &workspace.settings;
    if !settings.license.is_empty() {
        package["license"] = json!(settings.license);
    }
    if !settings.repository.is_empty() {
        package["repository"] = json!({
            "type": "git",
            "url": format!("git+{}.git", settings.repository.trim_end_matches(".git")),
            "directory": format!("servers/{}", server.name),
        });
    }
    package
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("cannot write {}", path.display()))
}

/// CI artifacts lose their mode bits. Windows has none, so the launcher also
/// repairs them at run time.
fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
