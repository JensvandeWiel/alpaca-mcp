# alpaca-mcp

Multiple MCP servers in Rust ([`rmcp`](https://crates.io/crates/rmcp)) with one shared
foundation, distributed through `npx` and `bunx`.

## Requirements

Rust (stable), Node 18 or newer, and Bun for `bunx`.

## Quick start

```sh
cargo xtask new weather --description "Weather forecasts"
cargo test -p weather-mcp
cargo xtask smoke weather                  # direct
cargo xtask smoke weather --runner npx     # as an npm package, without publishing
cargo xtask config weather --runner cargo  # snippet for your MCP client
```

## Commands

| Command | Purpose |
|---|---|
| `cargo xtask new <name>` | New server from the template |
| `cargo xtask list [--json]` | All servers with version and npm name |
| `cargo xtask smoke <name> [--runner cargo\|npx\|bunx]` | End-to-end handshake and `tools/list` |
| `cargo xtask config <name> [--runner npx\|bunx\|cargo\|binary] [--client claude\|vscode]` | Client configuration |
| `cargo xtask package <name>` | npm packages in `dist/npm/` |
| `cargo xtask publish <name> [--execute]` | Publish (a dry run without `--execute`) |
| `cargo xtask lint-commits --range <range>` | Check commit messages |

## Layout

```
crates/mcp-core/    CLI, configuration, logging, stdio transport
crates/xtask/       automation and templates
servers/<name>/     a server (crate <name>-mcp); `hello` is the example
.claude/skills/     create-mcp, run-mcp, publish-mcp
.github/workflows/  ci.yml, pr.yml and release.yml
```

A server consists of a `Config` struct and tools; `main.rs` is a single call:

```rust
mcp_core::run(mcp_core::server_spec!(), HelloServer::new).await
```

## Configuration

Every server works the same way. Later sources win:

1. defaults in the code
2. TOML file (`--config`, `<NAME>_MCP_CONFIG`, or `alpaca-mcp/<name>-mcp.toml` in the user's
   config directory)
3. environment variables, for example `HELLO_MCP_GREETING=Hi` (`__` for nested keys)
4. `--set greeting=Hi`

`<binary> check` validates the configuration, `--log debug` makes logging (stderr) more
verbose.

## Distribution

Each package `<scope>/<name>-mcp` only contains a small Node launcher. The binary lives in a
package per platform (`-win32-x64`, `-win32-arm64`, `-darwin-x64`, `-darwin-arm64`,
`-linux-x64`, `-linux-arm64`) that is installed as an optional dependency. There are no
install scripts. Linux binaries are static (musl).

To release: bump the version and push a tag `<name>-v<version>`.

### Settings

`[workspace.metadata.mcp]` in `Cargo.toml` sets the npm scope (`@alpaca-mcp`), license and
repository of the packages. Publishing requires publish rights on that scope and the
`NPM_TOKEN` secret in GitHub.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md): Conventional Commits, branches and pull requests.
After cloning: `git config core.hooksPath .githooks`.

## License

[MIT](LICENSE)
