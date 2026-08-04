# Threat Model

This document defines what `envlt` is designed to protect, what it does not protect, and what assumptions the security model depends on.

## Scope

`envlt` is a local-first development tool for managing environment variables and secrets. It is designed for individual developers and small teams that want to avoid plaintext `.env` files as the source of truth.

The primary security goal is to reduce accidental exposure of local development secrets without requiring a cloud service.

## Assets

The main assets are:

- vault contents stored in `~/.envlt/vault.age` or the configured `ENVLT_HOME`, including each variable's full version history, not only its current value
- a project's environments (e.g. `local`, `staging`, `prod`), each with its own fully independent set of variables
- vault passphrase
- optional system keyring credential created by `envlt auth save`
- project variables stored inside the vault
- generated `.env` files created by `envlt pull`
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
- unnecessary plaintext exposure to anything that reads the project working directory -- including AI coding assistants -- by making `envlt run` the default way to get variables into a process, and by having `doctor` flag a `.env` file left behind next to a linked project (see [AI Coding Assistants and Local Files](#ai-coding-assistants-and-local-files))

## Does Not Protect Against

`envlt` does not protect against:

- malware or a compromised process running as the same user
- a compromised operating system account
- a terminal, shell plugin, debugger, or agent that captures command output or environment variables
- secrets intentionally printed by the user or by a child process
- plaintext `.env` files after `envlt pull` materializes them -- once written, the file is readable by anything with filesystem access to the project directory. This now explicitly includes AI coding assistants (Claude Code, Cursor, Copilot, and similar tools): they routinely read project files as part of normal operation and are not malicious actors in the traditional sense, they simply don't distinguish a `.env` file from any other file in the repo
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

`envlt pull` writes plaintext variables to disk. This is useful for tools that require `.env` files, but the generated file should be deleted when it is no longer needed and should remain ignored by Git.

### Bundles

`.evlt` bundles use a bundle passphrase independent from the vault passphrase. Sharing a bundle is safer than sharing raw `.env` files, but the bundle passphrase must be shared through a separate channel. A bundle carries exactly one environment, flattened to its current values (no other environment and no version history included), so exporting `staging` can never leak `prod`, and a leaked bundle exposes only the values that were current at export time.

### Version History

Every variable keeps its full value history (`Secret` and `Plain` alike), bounded by `max_versions`. This is a deliberate widening of what the vault passphrase protects: previously, compromising the passphrase exposed only current values; now it exposes each variable's recent history too. This does not create a new trust boundary -- the passphrase was already the single point of failure for current secrets -- but it does increase what a single compromise reveals. See [Security: Environments, and full version history for Secret values](security.md#environments-and-full-version-history-for-secret-values) for the full reasoning.

### AI Coding Assistants and Local Files

As of 2026, AI coding assistants (Claude Code, Cursor, GitHub Copilot, and similar tools) routinely read files across a project's working directory as part of normal operation -- indexing the codebase, answering questions, or acting autonomously. A `.env` file sitting on disk is not distinguished from any other file: if it is in the directory the assistant can see, its contents can end up in a prompt, a log, or a request to a third-party model provider. This is not a hypothetical: industry reporting in 2026 documented both a large jump in AI-assisted commits leaking secrets and a real vulnerability (in LangChain Core) where prompt injection triggered serialization that dumped environment variables.

`envlt`'s existing design already reduces this exposure, and it predates this specific threat being named:

- `envlt run` injects variables directly into a child process and never writes a `.env` file, so there is nothing on disk for an assistant (or anything else scanning the working directory) to read.
- `envlt doctor` reports a `stray_env_file` warning when a `.env` file is found next to a resolved `.envlt-link`, pointing at `envlt run` as the alternative.

This does not change the trust boundary described elsewhere in this document -- `envlt` still cannot protect a `.env` file once it exists on disk, from an AI assistant or anything else with the same access. The practical mitigation is to materialize a `.env` file only when a tool genuinely requires one, delete it promptly, and prefer `run` by default.

## User Responsibilities

Users should:

- choose strong vault and bundle passphrases
- keep `.env` files in `.gitignore`
- prefer `envlt run` over `envlt pull` when a file is not required
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

- deserialized vault contents (`Variable.value` and similar) are not zeroized in memory once loaded, only the passphrase and transient encrypt/decrypt buffers are -- see `docs/tech-debt.md`
- full per-variable version history means a compromised vault passphrase now exposes a `Secret`'s recent past values, not only its current one (bounded by `max_versions`) -- see [Version History](#version-history)
- no `auth rotate` or `auth generate` yet
- backup rotation keeps three generations (`vault.age.bak`, `.bak.1`, `.bak.2`); no configurable retention count yet

## Review Policy

This threat model should be reviewed whenever `envlt` adds:

- new storage formats
- cloud sync
- merge/conflict resolution
- GUI or editor integrations
- new ways to print, export, or inject secrets
- new authentication or keyring behavior
