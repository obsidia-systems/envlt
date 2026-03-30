# Security

This document summarizes the current security model implemented in `envlt`.

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

### Basic backup strategy

- when an existing vault is overwritten, `envlt` creates `vault.age.bak`
- this helps basic local recovery after accidental corruption or operational mistakes

### Separate bundle protection

- exported `.evlt` bundles use a passphrase independent from the vault passphrase
- sharing a bundle does not require sharing the master vault passphrase

### Reduced disk exposure

- `envlt run` injects variables into a child process without writing a `.env` file
- `envlt use` writes a `.env` file and should therefore be treated as a temporary artifact

### Safer defaults in output

- `vars` masks `Secret` values
- `diff` does not print secret values
- `doctor` reports state and errors, not secret payloads

## Current limitations

- no explicit zeroization strategy in memory
- no cloud sync or remote conflict resolution
- no advanced partial-redaction policy across every output path
- no full migration and disaster-recovery subsystem yet

## Operational guidance

- use a strong vault passphrase
- treat system session security as part of the trust boundary when using `envlt auth save`
- avoid leaving materialized `.env` files around longer than needed
- prefer `envlt run` when a file on disk is not required
- share `.evlt` bundles and bundle passphrases through separate channels
- keep backups of the `envlt` home directory if the vault matters to your workflow

## Planned hardening areas

- possible integration with `secrecy` and `zeroize`
- deeper secure-store hardening and additional auth lifecycle commands
- stricter bundle validation and better recovery paths
- richer safe-output rules for diagnostics and diffing
