//! End-to-end check over real stdio: start the server the way a client would,
//! initialise and list the tools. With `--runner npx|bunx` the npm packages are
//! built, installed into a throwaway directory and started from there.

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use serde_json::{Value, json};

use crate::{
    package,
    workspace::{self, Server, Workspace},
};

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Runner {
    Cargo,
    Npx,
    Bunx,
}

#[derive(clap::Args)]
pub struct Args {
    /// Server name, for example `hello`
    name: String,

    #[arg(long, value_enum, default_value = "cargo")]
    runner: Runner,

    /// Arguments passed to the server, after `--`
    #[arg(last = true)]
    server_args: Vec<String>,
}

pub fn run(args: Args) -> Result<()> {
    let workspace = Workspace::load()?;
    let server = workspace.server(&args.name)?;

    let mut command = match args.runner {
        Runner::Cargo => cargo_command(&workspace, server)?,
        runner => packaged_command(&workspace, server, runner)?,
    };
    command.args(&args.server_args);

    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("cannot start {command:?}"))?;

    let result = handshake(&mut child);
    let exit = shut_down(&mut child);
    result?;
    exit
}

fn cargo_command(workspace: &Workspace, server: &Server) -> Result<Command> {
    workspace::execute(
        Command::new("cargo")
            .args(["build", "-p", &server.crate_name])
            .current_dir(workspace::root()),
    )?;
    let file = format!("{}{}", server.crate_name, std::env::consts::EXE_SUFFIX);
    Ok(Command::new(workspace.target_dir.join("debug").join(file)))
}

fn packaged_command(workspace: &Workspace, server: &Server, runner: Runner) -> Result<Command> {
    let binaries = package::build(workspace, server, &[])?;
    package::assemble(workspace, server, &binaries)?;

    let sandbox = workspace::root()
        .join("dist")
        .join("sandbox")
        .join(&server.crate_name);
    if sandbox.exists() {
        fs::remove_dir_all(&sandbox)?;
    }
    fs::create_dir_all(&sandbox)?;
    fs::write(sandbox.join("package.json"), "{ \"private\": true }\n")?;

    let out = package::output_dir(&server.crate_name);
    let mut tarballs = Vec::new();
    for dir in binaries
        .iter()
        .map(|(p, ..)| out.join(p.key))
        .chain([out.join("main")])
    {
        let output = workspace::capture(
            workspace::tool("npm")
                .args(["pack", "--json", "--pack-destination"])
                .arg(&sandbox)
                .current_dir(&dir),
        )?;
        let packed: Value = serde_json::from_str(&output)?;
        let file = packed[0]["filename"]
            .as_str()
            .context("npm pack did not report a file name")?;
        // npm 8 reports scoped names as `@scope/name.tgz`.
        tarballs.push(format!(
            "./{}",
            file.trim_start_matches('@').replace('/', "-")
        ));
    }

    // --offline: the packages are not published, nothing has to come from the registry.
    workspace::execute(
        workspace::tool("npm")
            .args(["install", "--offline", "--no-audit", "--no-fund"])
            .args(&tarballs)
            .current_dir(&sandbox),
    )?;

    let mut command = if runner == Runner::Npx {
        let mut command = workspace::tool("npx");
        // Without `--`, npx parses the server's flags as its own after `--no`.
        command.args(["--offline", "--no", "--", &server.crate_name]);
        command
    } else {
        let mut command = workspace::tool("bunx");
        command.args(["--no-install", &server.crate_name]);
        command
    };
    command.current_dir(sandbox);
    Ok(command)
}

fn handshake(child: &mut Child) -> Result<()> {
    let mut stdin = child.stdin.take().context("no stdin")?;
    let stdout = child.stdout.take().context("no stdout")?;

    let (sender, lines) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });

    let mut send = |message: Value| -> Result<()> {
        writeln!(stdin, "{message}")?;
        stdin.flush()?;
        Ok(())
    };

    send(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "xtask-smoke", "version": "0" }
        }
    }))?;
    let initialized = response(&lines, 1)?;
    println!(
        "server: {} {} (protocol {})",
        initialized["serverInfo"]["name"].as_str().unwrap_or("?"),
        initialized["serverInfo"]["version"].as_str().unwrap_or("?"),
        initialized["protocolVersion"].as_str().unwrap_or("?"),
    );

    send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
    send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }))?;
    let listed = response(&lines, 2)?;
    for tool in listed["tools"].as_array().context("no tools array")? {
        println!(
            "tool:   {} - {}",
            tool["name"].as_str().unwrap_or("?"),
            tool["description"].as_str().unwrap_or("")
        );
    }
    println!("smoke test passed");
    Ok(())
}

/// Waits for the response with `id`. Anything on stdout that is not JSON-RPC is
/// a bug in the server.
fn response(lines: &Receiver<String>, id: u64) -> Result<Value> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let line = lines
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .with_context(|| format!("no response to request {id} (see server output above)"))?;
        let message: Value = serde_json::from_str(&line)
            .with_context(|| format!("server wrote a non-JSON line to stdout: {line}"))?;
        if message["id"] == id {
            if let Some(error) = message.get("error") {
                bail!("request {id} failed: {error}");
            }
            return Ok(message["result"].clone());
        }
    }
}

/// stdin is closed by now, so a well behaved server exits on its own.
fn shut_down(child: &mut Child) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("server exited with {status}");
            }
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    child.kill()?;
    child.wait()?;
    bail!("server did not exit after stdin was closed")
}
