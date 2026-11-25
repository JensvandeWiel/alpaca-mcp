---
name: create-mcp
description: Create a new MCP server in this workspace, or add tools, configuration or resources to an existing one. Use for "create an MCP", "new server", "add a tool".
---

# Creating an MCP server

## 1. Determine the scope

Know before you start: the server name (short, lowercase, without `-mcp`), which tools,
which external API or source, and which configuration or secrets. Only ask what you cannot
derive from the request or the code.

## 2. Scaffold

```sh
cargo xtask new <name> --description "One line"
```

Do not create the directories by hand: the template provides `Cargo.toml`, `main.rs`,
`lib.rs` with a working test, `README.md` and `config.example.toml`. The workspace glob picks
up the crate automatically.

## 3. Implement in `servers/<name>/src/`

- `main.rs` stays as it is; CLI, config, logging and transport live in `mcp-core`.
- Replace the example tool `greet` and the example test.
- When `lib.rs` grows large, split it into modules (for example `client.rs` for the API,
  `tools.rs` for the tool router). Keep tools thin: validate parameters, call the client,
  shape the result.
- New dependencies that more servers can use go into `[workspace.dependencies]` in the root
  `Cargo.toml`.
- For HTTP use `reqwest` with rustls (no OpenSSL), otherwise the musl and cross builds break.

### Tools

- The `description` and the doc comments on parameter fields are what the model sees. State
  what the tool does, when to use it, and the units or formats of the parameters.
- Return expected errors as `Result<_, String>` (or `CallToolResult::error`) with a message
  the model can recover from. No `unwrap` on external data.
- For structured output return `Json<T>` with `T: Serialize + JsonSchema`.
- Annotate behaviour: `#[tool(description = "...", annotations(read_only_hint = true))]`,
  and `destructive_hint = true` for everything that changes data or costs money.
- Heavy logic belongs in an `async fn`, blocking work in `tokio::task::spawn_blocking`.
- Keep output bounded: paginate or truncate long lists.

The exact `rmcp` API changes between versions. When in doubt, read the source of the version
in `Cargo.lock` (`cargo metadata` gives the path) instead of guessing.

### Configuration and secrets

- Add fields to `Config`. Every field has a default or is an `Option<T>`.
- Secrets are `Option<String>` and are checked in `new()` with a clear error that names the
  environment variable (`<NAME>_MCP_API_KEY`).
- Never log secrets and do not put them in `config.example.toml`.
- For a server that does not start without a secret, the test must pass a `Config` with
  dummy values.

### Never write to stdout

stdout is the MCP protocol. Log with `tracing::{info,warn,error}` (goes to stderr). Clippy
rejects `println!` in servers.

## 4. Document

Update `servers/<name>/README.md` (tables of tools and configuration) and
`config.example.toml`. That README becomes the npm page.

## 5. Verify

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p <name>-mcp
cargo xtask smoke <name>                 # real handshake over stdio
```

If the server has required configuration:
`cargo xtask smoke <name> -- --set api_key=dummy`. Only report that it works once these steps
pass. Then use the `run-mcp` skill to hook it into a client, and `publish-mcp` to release.
