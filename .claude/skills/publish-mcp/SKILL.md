---
name: publish-mcp
description: Release and publish an MCP server from this workspace to npm so it runs with npx and bunx. Use for "publish", "release", "new version", "to npm".
---

# Publishing an MCP server

Publishing cannot be undone: an npm version can never be reused. Run `git push` of a tag or
`publish --execute` only after the user explicitly approved both name and version.

## Check first (once)

`[workspace.metadata.mcp]` in the root `Cargo.toml`: scope `@alpaca-mcp`, license MIT
(`LICENSE` is included in every package) and repository
`https://github.com/JensvandeWiel/alpaca-mcp` (must match exactly for provenance). The scope
must exist on npm and belong to the user.

GitHub: secret `NPM_TOKEN` (npm automation or granular token with publish rights on the
scope).

## Release (through CI, recommended)

1. Clean working directory, up-to-date `main`, green CI.
2. Bump `version` in `servers/<name>/Cargo.toml` (semver). Pre-releases such as
   `0.2.0-beta.1` automatically get the dist-tag `next`.
3. Update the server's README.
4. Verify locally:
   ```sh
   cargo test --workspace              # also updates Cargo.lock
   cargo xtask smoke <name> --runner npx
   cargo xtask publish <name>          # dry run: shows the package contents
   ```
5. Commit as `chore(<name>): release <version>`, including `Cargo.lock` (CI builds with
   `--locked`), and get it onto `main` through a PR (see `CONTRIBUTING.md`). Then tag the
   merged commit.
6. After approval:
   ```sh
   git tag <name>-v<version>
   git push origin <name>-v<version>
   ```
7. `.github/workflows/release.yml` builds six platforms, checks that tag and version match,
   and publishes the platform packages first and the main package last.
8. Verify: `npm view <scope>/<name>-mcp version` and
   `npx -y <scope>/<name>-mcp@<version> --version`.

## When something goes wrong

- Publishing half succeeded: rerun the publish job; existing versions are skipped.
- Bug found after publishing: release a new patch version, optionally `npm deprecate` the
  old one (with approval).
- `E403`/`E404` on publish: wrong token or scope permissions.
- Provenance error: `repository` is missing or differs, or the repository is private.

## Manually (only when CI is not possible)

A full release needs binaries for all platforms:

```sh
cargo xtask package <name> --artifacts <dir>   # <dir>/<target-triple>/<binary>
cargo xtask publish <name> --execute
```

With only the local build `--allow-partial` is required; the package then only works on that
platform. Do not do that unless the user knows this limitation. Publishing from Windows loses
the executable bit of Unix binaries (the launcher restores it, except in read-only
installations). The user logs in themselves: `! npm login`.
