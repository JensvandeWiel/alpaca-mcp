# Contributing

The repository is in English: code, documentation, commit messages, issues and pull
requests.

## Once after cloning

```sh
git config core.hooksPath .githooks   # checks every commit message
```

## Branches

Do not work directly on `main`. Name branches `<type>/<short-description>`, for example
`feat/weather-server` or `fix/config-env`.

## Conventional Commits

Commit messages and PR titles follow
[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>)!: <description>

<optional body: why, not only what>

<optional footers, for example "BREAKING CHANGE: ..." or "Closes #12">
```

| Type | Use for | Server version |
|---|---|---|
| `feat` | New functionality | minor |
| `fix` | Bug fix | patch |
| `perf` | Faster or leaner | patch |
| `refactor` | Restructuring without behaviour change | none |
| `docs` | Documentation only | none |
| `test` | Tests only | none |
| `build` | Build, packaging, dependencies | none, or patch |
| `ci` | Workflows | none |
| `style` | Formatting only | none |
| `chore` | Other maintenance and releases | none |
| `revert` | Reverts an earlier commit | depends |

**Scope** (optional, recommended): the server name (`hello`), `mcp-core`, `xtask`, `deps` or
`release`.

**Breaking change:** put `!` before the colon and explain it in a `BREAKING CHANGE:`
footer. That is a major version (below `1.0.0`: a minor version).

Rules that are checked automatically: known type, lowercase scope, non-empty description, no
trailing period, first line of at most 100 characters (aim for 72) and a blank line before
the body. Use the imperative mood: `add`, not `added`.

```
feat(hello): add farewell tool
fix(mcp-core): ignore empty environment variables
feat(xtask)!: require a description for new servers
chore(hello): release 0.2.0
```

To check yourself: `cargo xtask lint-commits --range origin/main..HEAD`.

## Pull requests

- One topic per PR; split unrelated changes. Keep the diff reviewable.
- The title is a valid Conventional Commit: with a squash merge it becomes the commit on
  `main`.
- Fill in the PR template, including how you verified the change.
- Open work in progress as a draft.
- A PR can be merged when CI is green and all comments are resolved. Use **squash merge**
  and delete the branch afterwards.

### Before opening

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask smoke <name>
```

### Review

Look for: nothing written to stdout in servers, no secrets in code or logs, clear tool
descriptions, destructive tools annotated, tests for new behaviour and updated
documentation.

## Dependency updates

Renovate (`renovate.json`) opens update PRs with conventional titles such as
`chore(deps): ...`. Treat them like any other PR: merge when CI is green.

## Releases

Every server has its own version. Release with a commit `chore(<name>): release <version>`
and the tag `<name>-v<version>`; see `.claude/skills/publish-mcp/SKILL.md`.
