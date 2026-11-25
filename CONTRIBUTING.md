# Bijdragen

## Eenmalig na het clonen

```sh
git config core.hooksPath .githooks   # controleert elke commit message
```

## Branches

Werk niet direct op `main`. Naam: `<type>/<korte-omschrijving>`, bijvoorbeeld
`feat/weather-server` of `fix/config-env`.

## Conventional Commits

Commit messages en PR-titels zijn Engels en volgen
[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>)!: <description>

<optionele body: waarom, niet alleen wat>

<optionele footers, bijvoorbeeld "BREAKING CHANGE: ..." of "Closes #12">
```

| Type | Gebruik | Versie van de server |
|---|---|---|
| `feat` | Nieuwe functionaliteit | minor |
| `fix` | Bugfix | patch |
| `perf` | Sneller of zuiniger | patch |
| `refactor` | Herstructurering zonder gedragswijziging | geen |
| `docs` | Alleen documentatie | geen |
| `test` | Alleen tests | geen |
| `build` | Build, packaging, dependencies | geen, of patch |
| `ci` | Workflows | geen |
| `style` | Alleen opmaak | geen |
| `chore` | Overig onderhoud en releases | geen |
| `revert` | Draait een eerdere commit terug | afhankelijk |

**Scope** (optioneel, aanbevolen): de servernaam (`hello`), `mcp-core`, `xtask`, `deps` of
`release`.

**Breaking change:** zet `!` voor de dubbele punt en licht het toe in een
`BREAKING CHANGE:`-footer. Dat is een major-versie (onder `1.0.0`: een minor-versie).

Regels die automatisch gecontroleerd worden: bekend type, scope in kleine letters, niet
leeg, geen punt aan het eind, eerste regel maximaal 100 tekens (streef naar 72) en een
witregel vóór de body. Gebruik de gebiedende wijs: `add`, niet `added`.

```
feat(hello): add farewell tool
fix(mcp-core): ignore empty environment variables
feat(xtask)!: require a description for new servers
chore(hello): release 0.2.0
```

Zelf controleren: `cargo xtask lint-commits --range origin/main..HEAD`.

## Pull requests

- Eén onderwerp per PR; splits losse wijzigingen. Houd de diff reviewbaar.
- De titel is een geldige Conventional Commit: bij squash merge wordt dat de commit op
  `main`.
- Vul het PR-template in, met hoe je het gecontroleerd hebt.
- Werk in uitvoering open je als draft.
- Mergen kan als CI groen is en alle opmerkingen zijn afgehandeld. Gebruik **squash merge**
  en verwijder daarna de branch.

### Vóór het openen

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask smoke <naam>
```

### Review

Let op: niets naar stdout in servers, geen secrets in code of logs, duidelijke
tool-beschrijvingen, gemarkeerde destructieve tools, tests voor nieuw gedrag en
bijgewerkte documentatie.

## Dependency-updates

Renovate (`renovate.json`) opent update-PR's met conventionele titels zoals
`chore(deps): ...`. Behandel ze als elke andere PR: mergen als CI groen is.

## Releases

Elke server heeft zijn eigen versie. Release met een commit
`chore(<naam>): release <versie>` en de tag `<naam>-v<versie>`; zie
`.claude/skills/publish-mcp/SKILL.md`.
