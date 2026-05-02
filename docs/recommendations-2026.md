# 2026 Recommendations

This document captures recommended next steps for `envlt` as a developer-focused, local-first encrypted environment vault.

The main recommendation is to deepen the current encrypted-file vault before replacing it with a database. The current model is simple, auditable, and appropriate for typical `.env` scale. The highest-value work is making the vault operation more durable, safer under concurrency, easier to use, and easier to trust.

## Product Direction

`envlt` should stay focused on this lane:

- local-first encrypted `.env` workflows
- fast onboarding for developers who do not want to memorize many commands
- safe visibility into project variables without accidental secret exposure
- portable project handoff through `.evlt` bundles
- strong local operational guarantees without requiring a server

Avoid turning `envlt` into a full HashiCorp Vault replacement unless the product intentionally expands into multi-user policies, audit logs, dynamic secrets, remote access, and high-availability storage.

## Storage Recommendation

Keep the full-vault encrypted file model for now.

Current model:

```text
vault.age -> decrypt -> VaultData -> mutate Project/Variable state -> encrypt -> atomic save
```

This remains a good fit because most projects have tens or hundreds of variables, not millions. A full-vault decrypt/encrypt cycle is acceptable at this scale and keeps the security model easier to understand.

Do not migrate to SQLite just because the vault is a file. A normal SQLite database with encrypted values can leak metadata such as project names, variable names, timestamps, row counts, and update patterns. If a database becomes necessary later, prefer SQLCipher or another page-encrypted database over manual per-value encryption.

Consider a database only if `envlt` grows into:

- very large vaults
- local history and audit trails
- complex search/query behavior
- concurrent multi-process editing
- partial sync
- multi-user collaboration state

## Priority 1: Harden Vault Writes

Files:

- `crates/envlt-core/src/vault/store.rs`
- `crates/envlt-core/src/app/service.rs`

Problem:

The vault is the encrypted source of truth. Writes are currently atomic through a temporary file and backup, but the write path can be made stronger.

Recommended changes:

- add a lockfile under `ENVLT_HOME` to prevent concurrent writers
- set restrictive permissions on the envlt home directory (`0700` on Unix)
- set restrictive permissions on `vault.age` and `vault.age.bak` (`0600` on Unix)
- fsync the temporary vault file before persisting it
- fsync the vault directory after the persist/rename step where supported
- consider backup rotation beyond a single `vault.age.bak`
- make `doctor` report lock and backup health

Why this matters:

This improves trust in the vault without changing the storage model. It protects against concurrent command races, crash windows, and overly-permissive filesystem defaults.

## Priority 2: Fix Auth And Keyring Drift

Files:

- `crates/envlt-core/src/auth.rs`
- `docs/architecture.md`
- `docs/security.md`

Problem:

The docs and implementation are not fully aligned around keyring support. The macOS keyring path should also be reviewed carefully so vault passphrases are not exposed through command-line process arguments.

Recommended changes:

- avoid passing passphrases as process arguments when shelling out to macOS `security`
- prefer stdin or a safer keyring API path for secret material
- update `docs/architecture.md` so it no longer says Keychain integration is not implemented
- document the exact precedence order: `ENVLT_PASSPHRASE`, keyring, prompt
- add tests around keyring target scoping by `ENVLT_HOME`

Why this matters:

Authentication is part of the user's trust boundary. It should be boring, explicit, and free of avoidable secret exposure.

## Priority 3: Improve `.envlt-link` Resolution

Files:

- `crates/envlt-core/src/link.rs`
- `crates/envlt-core/src/app/service.rs`

Problem:

Developers often run commands from subdirectories. If project resolution only checks the current directory, the link workflow feels fragile.

Recommended changes:
- search upward from the current directory until a `.envlt-link` is found
- have `doctor` show the resolved link path
- validate link schema/version
- add a command or output path for link status

Why this matters:

This makes `envlt vars`, `envlt run`, `envlt use`, `envlt set`, and the future interactive interface feel natural inside real repositories.

## Priority 4: Add A Terminal UI

Goal:

Running `envlt` with no subcommand should open a simple terminal UI for the current repository, similar in spirit to `k9s`, `lazygit`, or `opencode`.

The terminal UI should reduce command memorization. It should make the vault discoverable while preserving safe secret defaults.

Initial behavior:

```text
envlt
```

Recommended flow:

- start in the current directory
- resolve the project through explicit context or `.envlt-link`, including parent-directory search
- ask for the vault passphrase or use `ENVLT_PASSPHRASE`/keyring
- show the active project, selected environment, vault status, and link status
- list variables with `Secret` values hidden by default
- allow reveal only through an intentional action
- allow creating, editing, deleting, and retagging variables
- allow materializing `.env` through an explicit action with warning
- allow running `doctor` from inside the UI

Possible Rust crates:

- `ratatui` for the terminal UI
- `crossterm` for terminal input/output
- existing `envlt-core` as the domain layer behind the UI

Recommended architecture:

```text
envlt-cli command dispatch
  -> interactive terminal adapter
  -> envlt-core AppService
  -> VaultStore / Project / Variable / Link / Bundle
```

Do not move vault rules into the terminal UI. The UI should be an adapter over `AppService`, not a second implementation of project mutation.

Security rules:

- never show `Secret` values by default
- reveal one value at a time, after explicit action
- make reveal state temporary
- avoid copying secrets to logs or panic messages
- consider clipboard support only as an explicit, timed operation
- ensure screenshots/demos use fake values

Why this matters:

The CLI already has many useful commands. A terminal UI can make those workflows approachable without weakening the CLI model. It also gives developers a safe place to inspect and edit project variables without repeatedly consulting `--help`.

## Priority 5: Add Environments Inside A Project

Goal:

A single `Project` should support multiple environments such as `dev`, `staging`, `prod`, `local`, or `test`.

Current mental model:

```text
VaultData
  -> Project
    -> Variable
```

Recommended future model:

```text
VaultData
  -> Project
    -> Environment
      -> Variable
```

The exact naming should be settled before implementation. Prefer `Environment` if the user-facing vocabulary is `dev`, `staging`, `prod`. Avoid overloading `Project` to mean both an application and a deploy target.

Recommended CLI shape:

```bash
envlt env list --project api-payments
envlt env add staging --project api-payments
envlt vars --project api-payments --env staging
envlt set DATABASE_URL=... --project api-payments --env staging --secret
envlt use --project api-payments --env staging --out .env
envlt run --project api-payments --env staging -- node server.js
envlt diff --project api-payments --env staging --other-env prod
```

Recommended terminal UI behavior:

- project selector at the top level
- environment selector inside each project
- variables scoped to the selected environment
- optional comparison between environments
- warnings when writing a `prod` environment to a plaintext `.env` file

Open design questions:

- should each project have a default environment?
- should existing projects migrate into a `default` or `local` environment?
- should variables be duplicated per environment, or should there be shared project-level variables plus environment overrides?
- should `.envlt-link` include both project and default environment?
- should `.evlt` bundles export one environment, selected environments, or the whole project?

Suggested first version:

- introduce `Environment` as an explicit domain concept
- migrate existing projects into `local`
- keep variables fully scoped to one environment
- allow `.envlt-link` to store project plus optional environment
- defer shared/project-level inherited variables until there is strong demand

Why this matters:

Most teams already think in environments. Adding environment support makes `envlt` closer to real workflows while still keeping a local-first model.

## Priority 6: Strengthen Bundle Sharing

Files:

- `crates/envlt-core/src/bundle/format.rs`
- `crates/envlt-cli/src/commands/export.rs`
- `crates/envlt-cli/src/commands/import.rs`

Recommended changes:

- store KDF parameters in the `.evlt` bundle header
- add `import --dry-run`
- add a bundle inspection command that shows safe metadata only
- make overwrite previews explicit
- use restrictive permissions for exported bundles where possible

Why this matters:

Bundles are the collaboration path. Developers need to know what they are importing before changing their vault.

## Priority 7: Add Migration Infrastructure

Files:

- `crates/envlt-core/src/vault/model.rs`
- `crates/envlt-core/src/vault/store.rs`

Recommended changes:

- add a `vault/migration.rs` module
- support versioned migrations from old `VaultData` shapes
- create backup before migration
- make `doctor` report migration readiness
- keep fixture vaults for old versions

Why this matters:

Environments, richer metadata, and future encryption changes will require safe format evolution.

## Priority 8: Add Safe-Output Regression Tests

Files:

- `crates/envlt-cli/tests/cli_flow.rs`
- `crates/envlt-cli/src/commands/*`

Recommended changes:

- seed a known secret value in test vaults
- assert that the known secret never appears in stdout/stderr unless explicitly revealed
- cover `vars`, `diff`, `doctor`, `gen --set`, `import`, `export`, and error paths
- cover `table`, `json`, and `raw` output where applicable

Why this matters:

Safe output is one of the core promises of a secret-management tool.

## Priority 9: Improve `envlt run`

Files:

- `crates/envlt-cli/src/commands/run.rs`
- `crates/envlt-core/src/app/service.rs`

Recommended changes:

- document whether vault variables override inherited environment variables
- add optional collision warnings
- consider `--clean` for a minimal environment
- preserve child exit codes accurately
- handle Unix signal exits more faithfully where supported

Why this matters:

`run` is the safest daily workflow because it avoids writing plaintext `.env` files.

## Priority 10: Add Supply-Chain Trust Basics

Files:

- `.github/workflows/*`
- `docs/releasing.md`

Recommended changes:

- add `cargo audit`
- add `cargo deny`
- publish checksums
- consider GitHub artifact attestations or `cosign`
- publish an SBOM
- add release smoke tests for the built binary

Why this matters:

Developers need extra confidence before installing a tool that manages secrets.

## Recommended Sequence

Suggested implementation order:

1. Harden vault writes with locking, permissions, and fsync.
2. Fix auth/keyring safety and documentation drift.
3. Improve `.envlt-link` parent-directory resolution.
4. Add safe-output regression tests.
5. Add the terminal UI as an adapter over `AppService`.
6. Design and implement `Environment` support with migration.
7. Strengthen bundle metadata, dry-run import, and inspection.
8. Improve `envlt run` process semantics.
9. Add supply-chain and release trust basics.

## Non-Recommendations For Now

Avoid these until there is stronger evidence:

- replacing `vault.age` with SQLite only for perceived performance
- implementing manual per-variable encryption in a plaintext SQLite database
- adding cloud sync before local format migration and conflict semantics are clear
- adding shared project-level variable inheritance in the first environment implementation
- building a GUI before the terminal UI proves the interaction model
