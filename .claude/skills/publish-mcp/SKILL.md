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
`https://github.com/JensvandeWiel/alpaca-mcp` (must match exactly for trusted publishing and
provenance). The scope must exist on npm and belong to the user, with 2FA enabled on the
account.

## Authentication

CI uses npm trusted publishing (OIDC). npm can only trust a publisher for a package that
already exists, so a **new server** needs a token for its first release. `hello` is only an
example and should not be published. The npm steps below are for the user; never ask for the
token or handle it yourself.

1. Create a granular access token: *Bypass 2FA* checked, packages and scopes
   *Read and write (publish and stage)* limited to `@alpaca-mcp`, no organization
   permissions, short expiration. Add it as the GitHub secret `NPM_TOKEN`.
2. Release the first version (below).
3. Within 2 days of the next release, trust the workflow for all seven packages. This needs
   npm 11.15.0 or later and an interactive `npm login` (tokens cannot do this):
   ```powershell
   $server = '<name>'
   $suffixes = '', '-win32-x64', '-win32-arm64', '-darwin-x64', '-darwin-arm64', '-linux-x64', '-linux-arm64'
   foreach ($suffix in $suffixes) {
     npm trust github "@alpaca-mcp/$server-mcp$suffix" --file release.yml --repo JensvandeWiel/alpaca-mcp --allow-publish --yes
     Start-Sleep -Seconds 2
   }
   ```
   The first call asks for 2FA; choose to skip it for 5 minutes. `--allow-publish` is
   required: the workflow publishes directly.
4. Delete the secret `NPM_TOKEN`, revoke the token and release a new patch version **within
   2 days**. An unused trust configuration expires; recreate it if that happens.
5. After that release succeeds, set each package to *Settings → Publishing access → Require
   two-factor authentication and disallow tokens*.

Adding another server later: repeat these steps with a new token.

npm announced that direct publishing with bypass-2FA tokens ends around January 2027. After
that the first release of a new package has to go through `npm stage publish`, which this
workspace does not support yet. Check npm's current documentation.

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
- `ENEEDAUTH`: the trusted publisher does not match (repository, `release.yml`), expired, or
  was never set up for one of the seven packages.
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
