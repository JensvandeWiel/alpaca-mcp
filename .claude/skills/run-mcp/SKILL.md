---
name: run-mcp
description: Draai, test, configureer of debug een MCP-server uit deze workspace, ook via npx/bunx, en koppel hem aan Claude Code, Claude Desktop, Cursor of VS Code.
---

# Een MCP-server draaien en configureren

## Draaien en testen

```sh
cargo xtask list                          # welke servers zijn er
cargo run -p <naam>-mcp -- check          # configuratie valideren
cargo xtask smoke <naam>                  # handshake + tools/list
cargo xtask smoke <naam> --runner npx     # bouwt npm-packages, installeert lokaal, start via npx
cargo xtask smoke <naam> --runner bunx    # idem via bunx
cargo xtask smoke <naam> -- --set k=v     # argumenten voor de server na `--`
```

De npx/bunx-varianten werken zonder publiceren: ze installeren tarballs in
`dist/sandbox/`.

Interactief tools aanroepen kan met de MCP Inspector (downloadt van npm):
`npx @modelcontextprotocol/inspector cargo run -q -p <naam>-mcp`.

## Configuratie (later wint)

1. defaults in de `Config`-struct
2. TOML-bestand: `--config <pad>`, anders `<NAAM>_MCP_CONFIG`, anders
   `<config-map van de gebruiker>/alpaca-mcp/<naam>-mcp.toml` als dat bestaat
3. omgevingsvariabelen `<NAAM>_MCP_<KEY>`, genest met `__`
4. `--set key=value`, genest met een punt

Voorbeeld voor `hello`: `HELLO_MCP_GREETING=Hoi`. Onbekende sleutels zijn een fout.
Logniveau: `--log debug` of `<NAAM>_MCP_LOG`; `--log rmcp=info` toont protocolverkeer.

## Koppelen aan een client

```sh
cargo xtask config <naam> --runner cargo                  # deze checkout
cargo xtask config <naam> --runner binary                 # release-binary (eerst bouwen)
cargo xtask config <naam> --runner npx                    # gepubliceerd package
cargo xtask config <naam> --runner bunx --pin             # vaste versie
cargo xtask config <naam> --client vscode --env NAAM_MCP_KEY=waarde -- --log debug
```

Het commando print alleen JSON. Voeg het samen met het bestaande bestand
(`.mcp.json`, `claude_desktop_config.json`, `.vscode/mcp.json`) zonder andere servers te
overschrijven. Vraag eerst toestemming voordat je configuratie buiten deze repo wijzigt, en
zet geen secrets in bestanden die gecommit worden (gebruik in `.mcp.json` `${VARIABELE}`).

De npx-snippet krijgt op Windows automatisch `cmd /c`. Genereer hem op het
besturingssysteem waar de client draait.

## Problemen

| Symptoom | Oorzaak / oplossing |
|---|---|
| Client meldt ongeldige JSON | Iets schrijft naar stdout; gebruik `tracing` |
| Start timeout met `--runner cargo` | Eerste build duurt lang; bouw vooraf of gebruik `binary` |
| Build faalt: bestand in gebruik (Windows) | Een client draait de binary; stop de server daar |
| `unsupported platform` / package ontbreekt | Niet voor dit platform gepubliceerd, of optional dependencies uitgeschakeld |
| `invalid configuration` | Draai `check`; let op typfouten in sleutels en env-variabelen |
