# Contributing

Thanks for your interest in contributing to `envlt`.

## Development setup

Requirements:

- Rust `1.85.0` or newer
- `rustfmt`
- `clippy`

Install the local binary for iterative testing:

```bash
cargo install --path crates/envlt-cli
```

Or run directly from the repository:

```bash
cargo run -p envlt-cli -- --help
```

## Branching strategy

This project uses a simple branching model:

- `main` — the only long-lived branch. It should always be green.
- `feature/<short-name>` — short-lived branches for new work.

There is no `develop` or `staging` branch. Simplicity is preferred because `envlt` is a CLI tool with deliberate releases rather than continuous deployment.

### Starting new work

```bash
# Make sure main is up to date
git checkout main
git pull origin main

# Create a feature branch
git checkout -b feature/short-description

# Work, commit, push
git push -u origin feature/short-description
```

## Daily workflow

Use the provided `Makefile` for common tasks:

```bash
# Run tests
make test

# Check formatting
make fmt

# Run clippy
make clippy

# Run all quality gates (fmt + clippy + test)
make check
```

### Before opening a pull request

1. Run `make check` locally and fix any failures.
2. Update documentation if behavior changes (see [Documentation policy](#documentation-policy)).
3. Make sure `Cargo.lock` is included if dependencies changed.
4. Open a PR against `main`.

The CI workflow runs the same checks on every pull request. A PR should only be merged when CI passes.

## Pull request expectations

- Keep the CLI secure by default.
- Avoid printing secrets in logs or command output.
- Prefer implementation-driven documentation updates in the same change.
- Keep `envlt-core` as the main home for domain logic.
- Keep `envlt-cli` focused on parsing, prompts, and presentation.
- Include tests for new behavior.
- Keep the change focused; split unrelated work into separate PRs.

## Documentation policy

If behavior changes, update the relevant docs in the same PR:

- `README.md`
- `docs/getting-started.md`
- `docs/cli-reference.md`
- `docs/roadmap.md`
- `docs/spec-alignment.md`

## Release workflow

Only maintainers create releases. The process is deliberate and checklist-driven.

### Version policy

This project follows [Semantic Versioning](https://semver.org/):

| Bump | When |
|------|------|
| **Patch** `0.2.1` | Bug fixes, lockfile updates, docs corrections |
| **Minor** `0.3.0` | New commands, new features, behavior additions |
| **Major** `1.0.0` | Breaking CLI changes, vault format changes |

### Before tagging

Run the release checklist:

```bash
make release-check
```

This verifies:
- Working directory is clean
- You are on `main`
- `Cargo.lock` is in sync with `Cargo.toml`
- All tests pass (`cargo test --locked`)
- Formatting is clean (`cargo fmt --check`)
- Clippy is clean (`cargo clippy`)

### Creating a release

1. Update `CHANGELOG.md` with the new version entry.
2. Bump the version in `Cargo.toml` if needed.
3. Run `cargo update --workspace` so `Cargo.lock` reflects the new version.
4. Commit:
   ```bash
   git add -A
   git commit -m "release: v0.X.Y"
   ```
5. Create and push the tag:
   ```bash
   git tag v0.X.Y
   git push origin main
   git push origin v0.X.Y
   ```

The GitHub Actions release workflow triggers automatically on tags starting with `v`. It builds binaries for Linux (x86_64, aarch64) and macOS (x86_64, aarch64), creates a GitHub release, and opens a PR in the Homebrew tap.

### When a release fails

If the release workflow fails, do **not** delete and recreate the tag. Deleting tags that were already pushed is confusing for users and for GitHub.

Instead, fix the issue on `main`, then create a **patch release**:

```bash
# Fix the issue, update CHANGELOG, bump version
git add -A
git commit -m "release: v0.X.Y+1"
git tag v0.X.Y+1
git push origin main
git push origin v0.X.Y+1
```

## Common issues

### `Cargo.lock is out of date`

If you see this error in CI or locally with `--locked`:

```
error: the lock file needs to be updated but --locked was passed
```

Run:

```bash
cargo update --workspace
```

Then commit the updated `Cargo.lock`. This usually happens after adding a new dependency.

### Tests fail after a formatting-only change

Run `cargo fmt --all` before pushing. The CI enforces clean formatting.

### `clippy` fails on code I didn't touch

Clippy rules can change with new Rust versions. If CI fails on existing code, fix the lint in a separate commit within the same PR.

## Current roadmap priority

Near-term contributions are most valuable in:

- `.env` compatibility and edge cases
- safe output policies and hardening
- integration recipes (direnv, Docker Compose, VS Code, agents)
- recovery and diagnostics improvements

For the full roadmap, see [`docs/roadmap.md`](docs/roadmap.md).
