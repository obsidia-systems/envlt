# AGENTS.md

This file gives coding agents the context needed to work safely and effectively on `envlt`.

`envlt` is a Rust CLI for local-first environment variable management. It stores project environment variables in an encrypted local vault instead of relying on plaintext `.env` files as the source of truth.

## Project Mission

`envlt` should remain focused on:

- encrypted local vault workflows for development secrets
- regenerable `.env` files only when needed
- running commands with in-memory injected variables
- safe output that avoids accidental secret exposure
- portable encrypted project handoff through `.evlt` bundles
- practical developer onboarding without requiring a cloud account

Do not steer the project toward a full cloud secrets platform unless explicitly asked.

## Domain Vocabulary

Use the project vocabulary consistently:

- **Vault**: the encrypted local source of truth, stored as `vault.age` under `ENVLT_HOME` or `~/.envlt`.
- **VaultData**: the serialized domain state inside the encrypted vault.
- **Project**: a named set of environment variables stored in the vault.
- **Variable**: a key/value pair inside a project.
- **VarType**: the variable classification: `Secret`, `Config`, or `Plain`.
- **Secret**: a sensitive value that must be masked by default.
- **Config**: non-secret configuration that may be shown normally.
- **Plain**: explicitly non-sensitive plaintext-like value.
- **Project link**: `.envlt-link`, used to resolve the active project from a working directory.
- **Bundle**: encrypted `.evlt` export/import payload for project handoff.
- **Run environment**: variables loaded from the vault and injected into a child process by `envlt run`.
- **Doctor report**: diagnostics about vault, backup, decryptability, and link state.

Future product discussions may introduce **Environment** as a domain concept under `Project`, for workflows such as `local`, `dev`, `staging`, and `prod`. Do not implement this without a migration plan.

## Repository Layout

- `crates/envlt-core`: domain logic, vault persistence, encryption/decryption, `.env` parsing/rendering, bundles, project links, diagnostics, generators, and auth helpers.
- `crates/envlt-cli`: command-line parsing, prompts, interactive flows, child process execution, and user-facing output.
- `docs`: technical documentation, roadmap, security model, integrations, recommendations, and prototypes.
- `README.md`: user-facing overview and quickstart.
- `CONTRIBUTING.md`: contributor workflow and release expectations.
- `Makefile`: common local quality gates.

Keep domain behavior in `envlt-core`. Keep `envlt-cli` focused on parsing, prompting, command dispatch, and presentation.

## Important Files

- `crates/envlt-core/src/app/service.rs`: main application facade. Most command workflows pass through `AppService`.
- `crates/envlt-core/src/vault/model.rs`: `VaultData`, `Project`, `Variable`, `VarType`, and type inference.
- `crates/envlt-core/src/vault/store.rs`: vault file paths, load/save, atomic writes, backups.
- `crates/envlt-core/src/vault/crypto.rs`: `age` passphrase encryption/decryption.
- `crates/envlt-core/src/env/parser.rs`: `.env` and `.env.example` parsing.
- `crates/envlt-core/src/env/writer.rs`: generated `.env` rendering.
- `crates/envlt-core/src/link.rs`: `.envlt-link` read/write/remove behavior.
- `crates/envlt-core/src/bundle/format.rs`: `.evlt` archive encoding, decoding, encryption, and decryption.
- `crates/envlt-core/src/auth.rs`: keyring and passphrase storage helpers.
- `crates/envlt-cli/src/main.rs`: top-level CLI command dispatch.
- `crates/envlt-cli/src/commands/*`: individual command adapters.
- `crates/envlt-cli/tests/cli_flow.rs`: end-to-end CLI behavior tests.
- `docs/recommendations-2026.md`: prioritized product and architecture recommendations.
- `docs/envlt-tui-prototype.html`: static prototype for a possible interactive terminal UI concept.

## Architecture Rules

- Prefer small, focused changes that preserve the current local-first model.
- Use `AppService` as the seam between CLI commands and core behavior.
- Do not duplicate vault mutation logic in CLI command modules.
- Do not introduce a database unless the task explicitly asks for storage redesign.
- Preserve the full-vault encrypted file model unless there is a concrete product requirement to change it.
- Treat vault format changes as migration-sensitive. Update docs and tests if `VaultData`, `Project`, `Variable`, or bundle format changes.
- Keep `.evlt` bundle compatibility in mind when changing project serialization.

## Security Rules

- Never print secret values by default.
- Never add tests, examples, logs, fixtures, or docs containing real credentials.
- Use fake obvious values in examples, such as `example-secret`, `sk_test_example`, or `postgres://user:pass@localhost/db`.
- Preserve masking behavior for `Secret` values in `vars`, `diff`, `doctor`, and related outputs.
- Prefer `envlt run` guidance over `envlt use` when plaintext `.env` files are not required.
- Treat materialized `.env` files as plaintext artifacts.
- Be careful with process arguments, environment variables, panic messages, and command output when passphrases or secrets are involved.
- If changing auth/keyring behavior, verify `ENVLT_PASSPHRASE` precedence and keyring target scoping by `ENVLT_HOME`.

## Development Commands

Use the Makefile when possible:

```bash
make test
make fmt
make clippy
make check
```

Equivalent direct commands:

```bash
cargo test --locked -p envlt-core -p envlt-cli
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Run the CLI locally:

```bash
cargo run -p envlt-cli -- --help
cargo run -p envlt-cli -- init
```

Use an isolated home when testing commands manually:

```bash
ENVLT_HOME=/tmp/envlt-dev cargo run -p envlt-cli -- list
```

Use `ENVLT_PASSPHRASE` only with fake local test passphrases when automating manual checks.

## Testing Instructions

- Run `make check` before considering a code change complete.
- Add or update tests for behavior changes.
- For CLI behavior, prefer tests in `crates/envlt-cli/tests/cli_flow.rs`.
- For domain behavior, prefer focused unit tests in the relevant `envlt-core` module.
- For `.env` parser/writer changes, include roundtrip and edge-case tests.
- For safe-output changes, include regression tests proving known secret values do not appear in stdout or stderr.
- For vault or bundle format changes, include malformed input tests and compatibility/migration tests where applicable.

## Documentation Instructions

If behavior changes, update relevant documentation in the same change:

- `README.md` for user-visible overview changes.
- `docs/getting-started.md` for first-run workflow changes.
- `docs/cli-reference.md` for command, flag, or output changes.
- `docs/security.md` and `docs/threat-model.md` for security boundary changes.
- `docs/architecture.md` for crate/module/storage/flow changes.
- `docs/roadmap.md` or `docs/recommendations-2026.md` for roadmap-level direction changes.
- `docs/spec-alignment.md` when implementation alignment changes.

Keep docs implementation-driven. Do not promise cloud sync, GUI, team policies, or remote services unless implemented.

## Product Direction Notes

Current recommendations favor:

- hardening `vault.age` writes with locking, permissions, fsync, and better backups
- improving `.envlt-link` resolution from subdirectories
- strengthening keyring/auth behavior and docs
- adding safe-output regression tests
- eventually adding an interactive terminal UI as an adapter over `AppService`
- eventually adding project environments only with an explicit migration plan

Avoid premature rewrites such as:

- replacing `vault.age` with SQLite only for perceived performance
- manually encrypting individual values in a plaintext SQLite database
- adding cloud sync before local migrations and conflict semantics are defined
- implementing terminal UI logic separately from `envlt-core`

## Coding Style

- Rust edition: 2021.
- Minimum Rust version: 1.85.
- Clippy `all` is denied at the workspace level.
- Prefer clear domain names over generic helpers.
- Prefer minimal changes and avoid speculative abstractions.
- Avoid backward-compatibility code unless persisted data, public CLI behavior, or an explicit requirement needs it.
- Use `anyhow` in CLI-facing code where appropriate and domain-specific errors in `envlt-core`.

## Pull Request Expectations

- Keep changes focused.
- Keep CI green with `make check`.
- Include tests for new behavior.
- Update docs for user-visible or architecture-visible changes.
- Do not commit secrets, generated local vaults, `.env` files, or machine-specific artifacts.
- Keep `Cargo.lock` updated if dependencies change.
