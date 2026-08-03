# Security

This document summarizes the current security model implemented in `envlt`.

For explicit guarantees, non-goals, assumptions, and user responsibilities, see [Threat Model](threat-model.md).

## Current security properties

### Encrypted local source of truth

- the vault is stored at `~/.envlt/vault.age`
- sensitive project state is not stored as plaintext on disk
- access depends on the vault passphrase

### Optional system keyring support

- `envlt auth save` can store the vault passphrase in the operating system keyring
- the stored credential is scoped to the current `ENVLT_HOME`
- use an absolute `ENVLT_HOME` when overriding it to keep keyring targeting consistent
- `ENVLT_PASSPHRASE` still takes precedence over the keyring when both exist
- on macOS, Windows, and Linux, `envlt` talks to the native credential store (Keychain Services, Credential Manager, Secret Service) in-process via the `keyring` crate; it never shells out to a CLI tool or passes the passphrase as a command-line argument, so it isn't visible in process listings

### Passphrase handling in memory

- `envlt-cli` reads the vault and bundle passphrases into `Zeroizing<String>`, which is zeroed out as soon as it goes out of scope, rather than left as an ordinary `String` for the lifetime of the process
- `envlt-core` extends this through the passphrase's full round-trip: the copy read back from the system keyring (including the write-verification read), the decrypted vault/project plaintext returned by `vault::crypto::decrypt`, the plaintext TOML passed into and out of `VaultStore::load`/`save`, and the scrypt-derived bundle key are all held in `Zeroizing` wrappers and cleared on drop
- this does not extend to the deserialized vault contents themselves -- once loaded, a `Variable`'s value lives as a plain `String` for as long as the vault stays in memory; see `docs/tech-debt.md` for why that boundary is intentional

### Basic backup strategy

- when an existing vault is overwritten, `envlt` creates `vault.age.bak`
- this helps basic local recovery after accidental corruption or operational mistakes

### Separate bundle protection

- exported `.evlt` bundles use a passphrase independent from the vault passphrase
- sharing a bundle does not require sharing the master vault passphrase

### Reduced disk exposure

- `envlt run` injects variables into a child process without writing a `.env` file
- `envlt use` writes a `.env` file and should therefore be treated as a temporary artifact

### AI coding assistant exposure

- AI coding assistants (Claude Code, Cursor, Copilot, and similar tools) read project files as part of normal operation and do not distinguish a `.env` file from any other file in the working directory -- see [Threat Model: AI Coding Assistants and Local Files](threat-model.md#ai-coding-assistants-and-local-files) for the full reasoning
- `envlt doctor` reports a `stray_env_file` warning when a `.env` file is found next to a resolved `.envlt-link`, so a materialized file doesn't linger unnoticed
- `envlt run` remains the recommended default specifically because it avoids this exposure entirely by never writing the file

### Safer defaults in output

- `vars` masks `Secret` values
- `diff` does not print secret values
- `doctor` reports state and errors, not secret payloads

## Current limitations

- passphrase handling is zeroized end-to-end, but the deserialized vault contents are not (see [Passphrase handling in memory](#passphrase-handling-in-memory))
- no cloud sync or remote conflict resolution
- no advanced partial-redaction policy across every output path
- no `auth rotate` or `auth generate` yet

## Operational guidance

- use a strong vault passphrase
- treat system session security as part of the trust boundary when using `envlt auth save`
- avoid leaving materialized `.env` files around longer than needed
- prefer `envlt run` when a file on disk is not required
- share `.evlt` bundles and bundle passphrases through separate channels
- keep backups of the `envlt` home directory if the vault matters to your workflow

## Planned hardening areas

- additional auth lifecycle commands (`auth rotate`, `auth generate`)
- rotating more than the current three kept backups if real-world use shows a need
- an `import --dry-run`/`--inspect` style preview for other destructive operations
