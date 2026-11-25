# alpaca-mcp

Rust-workspace met meerdere MCP-servers op basis van `rmcp`, gedistribueerd via npm
(`npx` / `bunx`).

## Indeling

- `crates/mcp-core`: gedeelde bootstrap (CLI, configuratie, logging, stdio)
- `crates/xtask`: `cargo xtask new|list|smoke|config|package|publish|lint-commits`, templates in
  `crates/xtask/templates/`
- `servers/<naam>`: één crate per server, crate en binary heten `<naam>-mcp`

## Controle

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask smoke <naam>
```

## Regels

- stdout is het MCP-protocol: log met `tracing`, nooit `println!` in servers.
- Code, tool-beschrijvingen en package-README's in het Engels; overige documentatie in het
  Nederlands.
- Wat elke server nodig heeft hoort in `mcp-core`, niet gekopieerd per server.
- Wijzig je een server-template, controleer dan met een tijdelijke `cargo xtask new` dat het
  resultaat compileert en de test slaagt.
- Platformlijst staat in `crates/xtask/src/package.rs` en in
  `.github/workflows/release.yml`; houd ze gelijk.
- Publiceren of tags pushen alleen na expliciet akkoord.
- Commit messages en PR-titels zijn Engelse Conventional Commits; werk op een branch en
  open een PR volgens `.github/pull_request_template.md`. Zie `CONTRIBUTING.md`.

Skills: `create-mcp`, `run-mcp`, `publish-mcp`.
