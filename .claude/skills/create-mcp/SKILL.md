---
name: create-mcp
description: Maak een nieuwe MCP-server in deze workspace of voeg tools, configuratie of resources toe aan een bestaande. Gebruik bij "maak een mcp", "nieuwe server", "voeg een tool toe".
---

# Een MCP-server maken

## 1. Bepaal de scope

Weet vóór je begint: welke servernaam (kort, kleine letters, zonder `-mcp`), welke tools,
welke externe API of bron, en welke configuratie of secrets. Vraag alleen wat je niet uit
het verzoek of de code kunt halen.

## 2. Scaffold

```sh
cargo xtask new <naam> --description "Eén regel, Engels"
```

Maak de mappen niet met de hand aan: het template levert `Cargo.toml`, `main.rs`, `lib.rs`
met een werkende test, `README.md` en `config.example.toml`. Het workspace-glob pikt de
crate automatisch op.

## 3. Implementeer in `servers/<naam>/src/`

- `main.rs` blijft zoals het is; CLI, config, logging en transport zitten in `mcp-core`.
- Vervang de voorbeeldtool `greet` én de voorbeeldtest.
- Wordt `lib.rs` groot, splits dan in modules (bijvoorbeeld `client.rs` voor de API,
  `tools.rs` voor de tool-router). Houd tools dun: parameters valideren, client aanroepen,
  resultaat vormgeven.
- Nieuwe dependencies die meer servers kunnen gebruiken komen in
  `[workspace.dependencies]` in de root `Cargo.toml`.
- Voor HTTP: gebruik `reqwest` met rustls (geen OpenSSL), anders breken de musl- en
  cross-builds.

### Tools

- De `description` en de doc-comments op parametervelden zijn wat het model ziet. Schrijf ze
  in het Engels en zeg wat de tool doet, wanneer je hem gebruikt en in welke eenheden of
  formaten de parameters staan.
- Verwachte fouten geef je terug als `Result<_, String>` (of `CallToolResult::error`), met
  een melding waarmee het model zich kan herstellen. Geen `unwrap` op externe data.
- Voor gestructureerde output: retourneer `Json<T>` met `T: Serialize + JsonSchema`.
- Markeer gedrag met annotaties: `#[tool(description = "...", annotations(read_only_hint = true))]`,
  en `destructive_hint = true` voor alles wat data wijzigt of geld kost.
- Zware of blokkerende logica hoort in een `async fn`, blokkerend werk in
  `tokio::task::spawn_blocking`.
- Houd output beperkt: pagineer of kap lange lijsten af.

De exacte `rmcp`-API verandert tussen versies. Twijfel je, lees dan de broncode van de
versie uit `Cargo.lock` (`cargo metadata` geeft het pad) in plaats van te gokken.

### Configuratie en secrets

- Voeg velden toe aan `Config`. Elk veld heeft een default of is `Option<T>`.
- Secrets zijn `Option<String>` en worden in `new()` gecontroleerd met een duidelijke fout
  die de omgevingsvariabele noemt (`<NAAM>_MCP_API_KEY`).
- Log nooit secrets en zet ze niet in `config.example.toml`.
- Bij een server die zonder secret niet start, moet de test een `Config` met dummywaarden
  meegeven.

### Nooit naar stdout schrijven

stdout is het MCP-protocol. Log met `tracing::{info,warn,error}` (gaat naar stderr).
Clippy weigert `println!` in servers.

## 4. Documenteer

Werk `servers/<naam>/README.md` bij (tabel met tools en configuratie) en
`config.example.toml`. Die README wordt de npm-pagina.

## 5. Verifieer

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p <naam>-mcp
cargo xtask smoke <naam>                 # echte handshake over stdio
```

Heeft de server verplichte configuratie: `cargo xtask smoke <naam> -- --set api_key=dummy`.
Meld pas dat het werkt als deze stappen slagen. Gebruik daarna de skill `run-mcp` om hem in
een client te hangen, en `publish-mcp` om te releasen.
