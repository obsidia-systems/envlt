# Technical Debt

This document tracks known technical debt in `envlt`. Items are actionable and include the affected files and recommended next steps.

Items are grouped by **severity** so the highest-risk problems are visible first.

---

## High Severity

### No safe-output regression test matrix
- **Area**: Safe Output
- **Files**: `crates/envlt-cli/tests/cli_flow.rs`
- **Problem**: There is no systematic test proving that a known secret value never leaks into stdout, stderr, tables, JSON, raw output, or error messages across commands.
- **Next step**: Add a test fixture with a known secret and assert non-appearance in all output paths.

---

## Medium Severity

### Missing memory zeroization inside envlt-core
- **Area**: Auth / Keyring / Crypto
- **Files**: `crates/envlt-core/src/auth.rs`, `crates/envlt-core/src/vault/crypto.rs`, `crates/envlt-core/src/bundle/format.rs`
- **Problem**: `crates/envlt-cli/src/cli.rs` now reads passphrases into `Zeroizing<String>` at the point of entry (env var, keyring, or prompt), so the copy the user types is cleared on drop. But `envlt-core`'s public API still takes `&str` and passes it through to `String`-based buffers internally (decrypted vault plaintext, scrypt output, keyring round-trip values), none of which are zeroized. This is a bigger surface than the CLI entry point and would mean changing `envlt-core`'s public signatures.
- **Next step**: Introduce `Zeroizing`/`secrecy` types through `VaultStore::load`/`save`, `crypto::encrypt`/`decrypt`, and `auth.rs`'s keyring round-trip, accepting the API break.

### Link resolution does not walk parent directories
- **Area**: Project Link
- **Files**: `crates/envlt-core/src/link.rs`, `crates/envlt-core/src/app/service.rs`
- **Problem**: `.envlt-link` is only checked in the current directory. Developers running commands from subdirectories must specify `--project` explicitly.
- **Next step**: Walk parent directories to find `.envlt-link`, similar to `.git` resolution.

### No migration subsystem
- **Area**: Vault Format
- **Files**: `crates/envlt-core/src/vault/model.rs`, `crates/envlt-core/src/vault/store.rs`
- **Problem**: `VaultData` has a `version` field, but an unsupported version hard-fails. There is no migration path.
- **Next step**: Introduce a `vault/migration.rs` module with versioned migrations, backup-before-migrate, and `doctor` migration diagnostics.

---

## Low Severity

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
