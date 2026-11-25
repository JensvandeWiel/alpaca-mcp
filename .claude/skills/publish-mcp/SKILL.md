---
name: publish-mcp
description: Release en publiceer een MCP-server uit deze workspace naar npm zodat hij met npx en bunx te draaien is. Gebruik bij "publish", "release", "nieuwe versie", "naar npm".
---

# Een MCP-server publiceren

Publiceren is onomkeerbaar: een npm-versie kan niet opnieuw gebruikt worden. Voer
`git push` van een tag of `publish --execute` alleen uit na een expliciet akkoord van de
gebruiker op naam én versie.

## Controleer eerst (eenmalig)

`[workspace.metadata.mcp]` in de root `Cargo.toml`: scope `@alpaca-mcp`, licentie MIT
(`LICENSE` gaat mee in elk package) en repository
`https://github.com/JensvandeWiel/alpaca-mcp` (moet voor provenance exact kloppen). De
scope moet op npm bestaan en van de gebruiker zijn.

GitHub: secret `NPM_TOKEN` (npm automation- of granular token met publish-rechten op de
scope).

## Release (via CI, aanbevolen)

1. Werkmap schoon, op `main`, CI groen.
2. Verhoog `version` in `servers/<naam>/Cargo.toml` (semver). Voorversies zoals
   `0.2.0-beta.1` krijgen automatisch dist-tag `next`.
3. Werk de README van de server bij.
4. Controleer lokaal:
   ```sh
   cargo test --workspace              # werkt ook Cargo.lock bij
   cargo xtask smoke <naam> --runner npx
   cargo xtask publish <naam>          # dry run: toont de inhoud van de packages
   ```
5. Commit als `chore(<naam>): release <versie>`, inclusief `Cargo.lock` (CI bouwt met
   `--locked`), en breng hem via een PR op `main` (zie `CONTRIBUTING.md`). Tag daarna de
   gemergede commit.
6. Na akkoord:
   ```sh
   git tag <naam>-v<versie>
   git push origin <naam>-v<versie>
   ```
7. `.github/workflows/release.yml` bouwt zes platformen, controleert dat tag en versie
   gelijk zijn en publiceert eerst de platform-packages, dan het hoofdpackage.
8. Verifieer: `npm view <scope>/<naam>-mcp version` en
   `npx -y <scope>/<naam>-mcp@<versie> --version`.

## Als het misgaat

- Publiceren half gelukt: draai de publish-job opnieuw; bestaande versies worden
  overgeslagen.
- Fout in de code na publicatie: nieuwe patchversie, eventueel `npm deprecate` op de oude
  (met akkoord).
- `E403`/`E404` bij publish: token of scope-rechten kloppen niet.
- Provenance-fout: `repository` ontbreekt of wijkt af, of de repo is private.

## Handmatig (alleen als CI niet kan)

Een volledige release vraagt binaries voor alle platformen:

```sh
cargo xtask package <naam> --artifacts <map>   # <map>/<target-triple>/<binary>
cargo xtask publish <naam> --execute
```

Met alleen de lokale build is `--allow-partial` nodig; het package werkt dan alleen op dat
platform. Doe dat niet zonder dat de gebruiker die beperking kent. Vanaf Windows gaat het
uitvoerbaar-bit van Unix-binaries verloren (de launcher herstelt het, behalve in
alleen-lezen installaties). Inloggen doet de gebruiker zelf: `! npm login`.
