# Threat Model

This document defines what `envlt` is designed to protect, what it does not protect, and what assumptions the security model depends on.

## Scope

`envlt` is a local-first development tool for managing environment variables and secrets. It is designed for individual developers and small teams that want to avoid plaintext `.env` files as the source of truth.

The primary security goal is to reduce accidental exposure of local development secrets without requiring a cloud service.

## Assets

The main assets are:

- vault contents stored in `~/.envlt/vault.age` or the configured `ENVLT_HOME`
- vault passphrase
- optional system keyring credential created by `envlt auth save`
- project variables stored inside the vault
- generated `.env` files created by `envlt use`
- encrypted `.evlt` bundles created by `envlt export`
- bundle passphrases used for `.evlt` import/export
- `.envlt-link` project references

## Trust Boundaries

`envlt` assumes these boundaries:

- the local operating system protects one user's files from other users
- the terminal session is controlled by the user
- the system keyring is trusted if the user chooses `envlt auth save`
- a child process launched by `envlt run` is allowed to read the injected environment variables
- a generated `.env` file is plaintext and must be treated as sensitive

## Protects Against

`envlt` is intended to help with:

- accidental commits of plaintext `.env` files by making the encrypted vault the source of truth
- casual local disk inspection of stored project secrets
- unsafe sharing of raw `.env` files by using encrypted `.evlt` bundles
- accidental terminal disclosure in commands that mask or summarize values, such as `vars` and `diff`
- onboarding drift between `.env.example` and the user's local secret state
- repeated manual copy/paste of secrets during local setup

## Does Not Protect Against

`envlt` does not protect against:

- malware or a compromised process running as the same user
- a compromised operating system account
- a terminal, shell plugin, debugger, or agent that captures command output or environment variables
- secrets intentionally printed by the user or by a child process
- plaintext `.env` files after `envlt use` materializes them
- weak vault or bundle passphrases chosen by the user
- shell history exposure when secrets are typed directly into commands
- remote team access control, audit logs, approvals, or enterprise policy enforcement
- production-grade secret rotation across external services

## Important Behaviors

### Vault Storage

The vault is encrypted on disk using `age` passphrase encryption. The vault passphrase is required to decrypt the local source of truth unless it is supplied through `ENVLT_PASSPHRASE` or stored in the system keyring.

### Keyring Storage

`envlt auth save` stores the vault passphrase in the operating system keyring. This improves local convenience but expands the trust boundary to the user's logged-in OS session and keyring implementation.

### Runtime Injection

`envlt run` injects variables into a child process environment without writing a `.env` file. This reduces disk exposure, but the child process and same-user process environment access remain part of the trust boundary.

### Materialized Files

`envlt use` writes plaintext variables to disk. This is useful for tools that require `.env` files, but the generated file should be deleted when it is no longer needed and should remain ignored by Git.

### Bundles

`.evlt` bundles use a bundle passphrase independent from the vault passphrase. Sharing a bundle is safer than sharing raw `.env` files, but the bundle passphrase must be shared through a separate channel.

## User Responsibilities

Users should:

- choose strong vault and bundle passphrases
- keep `.env` files in `.gitignore`
- prefer `envlt run` over `envlt use` when a file is not required
- avoid typing secrets directly into shell commands when possible
- treat `ENVLT_PASSPHRASE` as sensitive automation input
- share `.evlt` bundles and bundle passphrases through separate channels
- rotate any credential that may have been exposed outside `envlt`

## Security Non-Goals

The current project does not aim to provide:

- cloud-hosted secrets management
- centralized team access control
- audit logging across machines
- service-token based production secret delivery
- native Windows support outside WSL
- prevention of same-user process inspection
- complete in-memory secret zeroization

## Current Hardening Gaps

Known areas for improvement:

- stronger `.env` parser and writer compatibility
- atomic and restrictive plaintext `.env` materialization
- broader output redaction tests
- explicit memory zeroization strategy where practical
- stronger malformed bundle validation and recovery guidance
- clearer automation checks for `.env.example` drift

## Review Policy

This threat model should be reviewed whenever `envlt` adds:

- new storage formats
- cloud sync
- merge/conflict resolution
- GUI or editor integrations
- new ways to print, export, or inject secrets
- new authentication or keyring behavior
