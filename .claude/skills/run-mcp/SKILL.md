---
name: run-mcp
description: Run, test, configure or debug an MCP server from this workspace, also through npx/bunx, and connect it to Claude Code, Claude Desktop, Cursor or VS Code.
---

# Running and configuring an MCP server

## Run and test

```sh
cargo xtask list                          # which servers exist
cargo run -p <name>-mcp -- check          # validate the configuration
cargo xtask smoke <name>                  # handshake + tools/list
cargo xtask smoke <name> --runner npx     # build npm packages, install locally, start via npx
cargo xtask smoke <name> --runner bunx    # same through bunx
cargo xtask smoke <name> -- --set k=v     # server arguments go after `--`
```

The npx/bunx variants work without publishing: they install tarballs in `dist/sandbox/`.

To call tools interactively use the MCP Inspector (downloaded from npm):
`npx @modelcontextprotocol/inspector cargo run -q -p <name>-mcp`.

## Configuration (later wins)

1. defaults in the `Config` struct
2. TOML file: `--config <path>`, else `<NAME>_MCP_CONFIG`, else
   `<user config dir>/alpaca-mcp/<name>-mcp.toml` if it exists
3. environment variables `<NAME>_MCP_<KEY>`, nested with `__`
4. `--set key=value`, nested with a dot

Example for `hello`: `HELLO_MCP_GREETING=Hi`. Unknown keys are an error. Log level:
`--log debug` or `<NAME>_MCP_LOG`; `--log rmcp=info` shows protocol traffic.

## Connect to a client

```sh
cargo xtask config <name> --runner cargo                  # this checkout
cargo xtask config <name> --runner binary                 # release binary (build it first)
cargo xtask config <name> --runner npx                    # published package
cargo xtask config <name> --runner bunx --pin             # fixed version
cargo xtask config <name> --client vscode --env NAME_MCP_KEY=value -- --log debug
```

The command only prints JSON. Merge it into the existing file (`.mcp.json`,
`claude_desktop_config.json`, `.vscode/mcp.json`) without overwriting other servers. Ask for
permission before changing configuration outside this repository, and never put secrets in
files that get committed (use `${VARIABLE}` in `.mcp.json`).

On Windows the npx snippet automatically gets `cmd /c`. Generate it on the operating system
where the client runs.

## Troubleshooting

| Symptom | Cause / solution |
|---|---|
| Client reports invalid JSON | Something writes to stdout; use `tracing` |
| Startup timeout with `--runner cargo` | The first build is slow; build beforehand or use `binary` |
| Build fails: file in use (Windows) | A client is running the binary; stop the server there |
| `unsupported platform` / package missing | Not published for this platform, or optional dependencies disabled |
| `invalid configuration` | Run `check`; look for typos in keys and environment variables |
