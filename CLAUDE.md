# alpaca-mcp

Rust workspace with multiple MCP servers built on `rmcp`, distributed through npm
(`npx` / `bunx`).

## Layout

- `crates/mcp-core`: shared bootstrap (CLI, configuration, logging, stdio)
- `crates/xtask`: `cargo xtask new|list|smoke|config|package|publish|lint-commits`,
  templates in `crates/xtask/templates/`
- `servers/<name>`: one crate per server, crate and binary are named `<name>-mcp`

## Checks

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask smoke <name>
```

## Rules

- stdout is the MCP protocol: log with `tracing`, never `println!` in servers.
- Everything in the repository is written in English: code, comments, documentation, skills,
  commit messages and pull requests.
- What every server needs belongs in `mcp-core`, not copied per server.
- After changing a server template, verify with a temporary `cargo xtask new` that the
  result compiles and its test passes.
- The platform list lives in `crates/xtask/src/package.rs` and in
  `.github/workflows/build.yml`; keep them in sync.
- Publish or push release tags only after explicit approval.
- Commit messages and PR titles are Conventional Commits; work on a branch and open a PR
  following `.github/pull_request_template.md`. See `CONTRIBUTING.md`.

Skills: `create-mcp`, `run-mcp`, `publish-mcp`.
