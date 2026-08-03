# Technical Debt

This document tracks known technical debt in `envlt`. Items are actionable and include the affected files and recommended next steps.

Items are grouped by **severity** so the highest-risk problems are visible first.

No High or Medium Severity items are currently open.

---

## Low Severity

### Domain model itself is not zeroized
- **Area**: Vault / Domain Model
- **Files**: `crates/envlt-core/src/vault/model.rs`
- **Problem**: Passphrase handling is now zeroized end-to-end (CLI entry point, keyring round-trip, and the transient plaintext buffers moving through `crypto::encrypt`/`decrypt` and bundle encryption). But once a vault is decrypted, each `Variable.value` lives as a plain `String` inside `VaultData` for as long as the `AppService`/`VaultData` value is alive, and is not zeroized when dropped.
- **Next step**: This would require wrapping every secret-bearing field across the domain model (`Variable`, `ActivityEvent`, diff/history results) in `Zeroizing`, which is a much larger change than the passphrase plumbing. Revisit only if a concrete threat model calls for it (e.g. defending against memory-dump forensics), since it adds real complexity to routine code (`Debug`, `Clone`, `Serialize` all need to interoperate with the wrapper) for a `.env`-scale local tool.
