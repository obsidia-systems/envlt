# Technical Debt

This document tracks known technical debt in `envlt`. Items are actionable and include the affected files and recommended next steps.

Items are grouped by **severity** so the highest-risk problems are visible first.

---

## High Severity

### macOS passphrase exposed via process arguments
- **Area**: Auth / Keyring
- **Files**: `crates/envlt-core/src/auth.rs`
- **Problem**: `macos_set_password` passes the passphrase to `security add-generic-password` via the `-w` argument. This exposes the secret in process listings, system logs, and shell history.
- **Next step**: Pass the passphrase through stdin instead of arguments, or use a native keyring API that avoids command-line exposure.

### No file locking for concurrent writers
- **Area**: Vault / Storage
- **Files**: `crates/envlt-core/src/vault/store.rs`
- **Problem**: Two concurrent `envlt` processes can race on `vault.age`. The temporary-file atomic write helps, but there is no cross-process lock.
- **Next step**: Add a lockfile mechanism under `ENVLT_HOME` with timeout and stale-lock detection.

### No safe-output regression test matrix
- **Area**: Safe Output
- **Files**: `crates/envlt-cli/tests/cli_flow.rs`
- **Problem**: There is no systematic test proving that a known secret value never leaks into stdout, stderr, tables, JSON, raw output, or error messages across commands.
- **Next step**: Add a test fixture with a known secret and assert non-appearance in all output paths.

---

## Medium Severity

### Missing memory zeroization for passphrases
- **Area**: Auth / Keyring
- **Files**: `crates/envlt-core/src/auth.rs`, `crates/envlt-cli/src/cli.rs`
- **Problem**: Passphrases are held in standard Rust `String` values. There is no explicit zeroization after use, so the secret may remain in memory longer than necessary.
- **Next step**: Evaluate `secrecy` and `zeroize` crates for holding and clearing sensitive strings.

### Missing fsync and restrictive permissions
- **Area**: Vault / Storage
- **Files**: `crates/envlt-core/src/vault/store.rs`
- **Problem**: `VaultStore::save` does not fsync the temp file or the directory before/after persisting, and does not enforce `0700` on `ENVLT_HOME` or `0600` on `vault.age`.
- **Next step**: Add fsync calls and permission setting on Unix platforms.

### Link resolution does not walk parent directories
- **Area**: Project Link
- **Files**: `crates/envlt-core/src/link.rs`, `crates/envlt-core/src/app/service.rs`
- **Problem**: `.envlt-link` is only checked in the current directory. Developers running commands from subdirectories must specify `--project` explicitly.
- **Next step**: Walk parent directories to find `.envlt-link`, similar to `.git` resolution.

### KDF parameters not stored in bundle header
- **Area**: Bundle
- **Files**: `crates/envlt-core/src/bundle/format.rs`
- **Problem**: `.evlt` bundles rely on `scrypt::Params::recommended()` at decode time. If the recommended parameters change in a future Rust version, old bundles may not decrypt.
- **Next step**: Serialize KDF parameters into `BundleHeader`.

### No migration subsystem
- **Area**: Vault Format
- **Files**: `crates/envlt-core/src/vault/model.rs`, `crates/envlt-core/src/vault/store.rs`
- **Problem**: `VaultData` has a `version` field, but an unsupported version hard-fails. There is no migration path.
- **Next step**: Introduce a `vault/migration.rs` module with versioned migrations, backup-before-migrate, and `doctor` migration diagnostics.

---

## Low Severity

### Documentation drift around keyring support
- **Area**: Auth / Keyring
- **Files**: `docs/architecture.md`
- **Problem**: `architecture.md` states "Keychain integration" as not yet implemented, but macOS and cross-platform keyring support already exist in `auth.rs`.
- **Next step**: Update `architecture.md` and `docs/security.md` to reflect the implemented keyring flow, precedence order, and scoping by `ENVLT_HOME`.

### Auth error handling is silent on keyring failure
- **Area**: Auth / Keyring
- **Files**: `crates/envlt-cli/src/cli.rs`
- **Problem**: When `load_stored_passphrase` fails, only a warning is printed and the CLI falls back to an interactive prompt. This can hide configuration or permission problems.
- **Next step**: Consider a more explicit mode (for example, `auth status` already reports this; the prompt path could optionally require confirmation before falling back).

### Single backup only
- **Area**: Vault / Storage
- **Files**: `crates/envlt-core/src/vault/store.rs`
- **Problem**: Only one backup (`vault.age.bak`) is kept. A corrupted write overwrites the previous good backup.
- **Next step**: Consider rotating a small number of backups.

### No bundle dry-run or inspect command
- **Area**: Bundle
- **Files**: `crates/envlt-cli/src/commands/import.rs`
- **Problem**: Users cannot preview bundle contents or validate a bundle without importing it.
- **Next step**: Add `import --dry-run` and a bundle metadata inspection path.

### architecture.md is out of date on implemented features
- **Area**: Documentation
- **Files**: `docs/architecture.md`
- **Problem**: Lists keyring and migration as not implemented, but keyring exists.
- **Next step**: Audit `architecture.md` against the current codebase and update the implemented vs not-yet-implemented sections.
