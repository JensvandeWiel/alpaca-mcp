# alpaca-mcp

Meerdere MCP-servers in Rust ([`rmcp`](https://crates.io/crates/rmcp)), met één gedeelde
basis en distribueerbaar via `npx` en `bunx`.

## Vereisten

Rust (stable), Node 18 of nieuwer, en Bun voor `bunx`.

## Snel starten

```sh
cargo xtask new weather --description "Weather forecasts"
cargo test -p weather-mcp
cargo xtask smoke weather                  # direct
cargo xtask smoke weather --runner npx     # als npm-package, zonder publiceren
cargo xtask config weather --runner cargo  # snippet voor je MCP-client
```

## Commando's

| Commando | Doel |
|---|---|
| `cargo xtask new <naam>` | Nieuwe server uit het template |
| `cargo xtask list [--json]` | Alle servers met versie en npm-naam |
| `cargo xtask smoke <naam> [--runner cargo\|npx\|bunx]` | End-to-end handshake en `tools/list` |
| `cargo xtask config <naam> [--runner npx\|bunx\|cargo\|binary] [--client claude\|vscode]` | Client-configuratie |
| `cargo xtask package <naam>` | npm-packages in `dist/npm/` |
| `cargo xtask publish <naam> [--execute]` | Publiceren (zonder `--execute` een dry run) |
| `cargo xtask lint-commits --range <range>` | Commit messages controleren |

## Indeling

```
crates/mcp-core/    CLI, configuratie, logging, stdio-transport
crates/xtask/       automatisering en templates
servers/<naam>/     een server (crate <naam>-mcp); `hello` is het voorbeeld
.claude/skills/     create-mcp, run-mcp, publish-mcp
.github/workflows/  ci.yml en release.yml
```

Een server bestaat uit een `Config`-struct en tools; `main.rs` is één aanroep:

```rust
mcp_core::run(mcp_core::server_spec!(), HelloServer::new).await
```

## Configuratie

Elke server werkt hetzelfde. Later wint:

1. defaults in de code
2. TOML-bestand (`--config`, `<NAAM>_MCP_CONFIG`, of `alpaca-mcp/<naam>-mcp.toml` in de
   config-map van de gebruiker)
3. omgevingsvariabelen, bijvoorbeeld `HELLO_MCP_GREETING=Hoi` (`__` voor geneste sleutels)
4. `--set greeting=Hoi`

`<binary> check` valideert de configuratie, `--log debug` zet logging (stderr) ruimer.

## Distributie

Elk package `<scope>/<naam>-mcp` bevat alleen een kleine Node-launcher. De binary zit in een
package per platform (`-win32-x64`, `-win32-arm64`, `-darwin-x64`, `-darwin-arm64`,
`-linux-x64`, `-linux-arm64`) dat als optional dependency wordt meegeïnstalleerd. Er zijn
geen install-scripts. Linux-binaries zijn statisch (musl).

Releasen: versie verhogen en een tag `<naam>-v<versie>` pushen.

### Instellingen

`[workspace.metadata.mcp]` in `Cargo.toml` bepaalt de npm-scope (`@alpaca-mcp`), de licentie
en de repository van de packages. Publiceren vraagt publish-rechten op die scope en het
secret `NPM_TOKEN` in GitHub.

## Bijdragen

Zie [CONTRIBUTING.md](CONTRIBUTING.md): Conventional Commits, branches en pull requests.
Na het clonen: `git config core.hooksPath .githooks`.

## Licentie

[MIT](LICENSE)
