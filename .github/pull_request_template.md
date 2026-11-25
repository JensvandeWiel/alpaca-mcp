<!-- Title: <type>(<scope>): <description>, for example `feat(hello): add farewell tool` -->

## What and why

<!-- What changes, and which problem does it solve? Link issues with `Closes #123`. -->

## How it was verified

<!-- Commands you ran and what you saw. -->

## Breaking changes

<!-- "None", or what users must change. Mark the title with `!`. -->

## Checklist

- [ ] The title follows Conventional Commits
- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass
- [ ] `cargo xtask smoke <server>` passes for every affected server
- [ ] Tests cover the change
- [ ] README and `config.example.toml` are up to date
- [ ] No secrets in code, config or logs
