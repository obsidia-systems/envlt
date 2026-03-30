# envlt

<p align="center">
  <strong>Local-first environment variable management for development workflows.</strong>
</p>

<p align="center">
  Encrypted vault. Portable bundles. Regenerable <code>.env</code> files. No cloud dependency required.
</p>

<p align="center">
  <a href="https://github.com/obsidia-systems/envlt/actions/workflows/ci.yml"><img src="https://github.com/obsidia-systems/envlt/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="MIT License"></a>
  <a href="https://github.com/obsidia-systems/envlt"><img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-blue" alt="Platform"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.85%2B-orange" alt="Rust 1.85+"></a>
</p>

## Overview

`envlt` is a Rust CLI for storing project environment variables inside an encrypted local vault instead of keeping secrets in plaintext `.env` files.

It is designed for the local development use case:

- import existing `.env` files
- bootstrap from `.env.example`
- regenerate `.env` files only when needed
- run commands with in-memory injected variables
- export/import portable encrypted project bundles

## Problem

The usual `.env` workflow creates avoidable friction:

- plaintext secrets remain on disk
- onboarding depends on manual copy and edit steps
- local state drifts across machines and teammates
- accidental commits happen when `.env` changes are mixed into normal work

`envlt` turns this into an encrypted, repeatable workflow with clear control points.

## Mental Model

`envlt` replaces this:

`.env` -> plaintext -> manual -> error-prone

with this:

`vault` -> encrypted -> reproducible -> controlled

The encrypted vault is your source of truth. `.env` files become generated artifacts only when needed.

## Real-world Before/After

Before:

```bash
cp .env.example .env
nano .env
```

After:

```bash
envlt use api-payments
envlt run node server.js
```

## Why `envlt`

- Local-first: no account, no remote service, no required network dependency
- Safer by default: encrypted vault, masked secret output, secure generator behavior
- Portable: share project snapshots with `.evlt` bundles
- Practical: use `run`, `use`, `diff`, `vars`, `gen`, `doctor`, and `auth` from a single CLI

## Quick Comparison

| Workflow need | Typical `.env` approach | `envlt` approach |
| --- | --- | --- |
| Local secret storage | plaintext file on disk | encrypted local vault |
| Team handoff | copy/paste or shared files | encrypted `.evlt` bundle |
| Run app locally | depends on current `.env` state | deterministic with `envlt run` |
| Regenerate files | manual edits and drift risk | `envlt use` from vault state |
| Offline usage | yes | yes |

## Status

Current implementation state:

- Phase 1: complete and extended
- Phase 2: implemented
- Phase 3: implemented for the current packaging milestone

Still intentionally out of scope for now:

- cloud sync
- GUI app

## Features

- encrypted local vault using `age`
- atomic writes with `vault.age.bak` backup
- `.env` and `.env.example` import
- `.envlt-link` project resolution
- typed variables: `Secret`, `Config`, `Plain`
- optional system keyring support for vault passphrase reuse
- secret-aware variable listing
- project removal with confirmation
- project-to-example and project-to-project diffing
- secure secret generation with interactive flow
- encrypted `.evlt` export/import
- local diagnostics through `envlt doctor`

## First 5 Minutes

```bash
envlt init
envlt auth save
envlt add api-payments
envlt run --project api-payments -- node server.js
```

Then move into common tasks:

```bash
envlt vars --project api-payments
envlt use --project api-payments
envlt set --project api-payments PORT=4000
envlt export api-payments --out bundle.evlt
envlt import bundle.evlt
envlt doctor --decrypt
```

Secret generation examples:

```bash
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --show
```

If the current directory contains `.envlt-link`, these commands can resolve the project automatically:

- `vars`
- `diff`
- `set`
- `use`
- `run`
- interactive `gen` save flow

## Installation

### Current supported path

Homebrew installation is available and is the recommended install path:

```bash
brew install obsidia-systems/tap/envlt
envlt --help
```

Cargo installation is still supported for contributors and local development:

```bash
cargo install --path crates/envlt-cli
envlt --help
```

### Install from GitHub Releases

If a release asset already exists, a user can install `envlt` manually by downloading the archive, extracting the binary, and placing it on their `PATH`.

Example:

```bash
tar -xzf envlt-linux-x86_64.tar.gz
chmod +x envlt
sudo mv envlt /usr/local/bin/envlt
envlt --help
```

This remains a supported manual installation path in addition to Homebrew.

On macOS, binaries downloaded from a browser may be blocked by Gatekeeper until the project ships signed and notarized artifacts. For a release you trust, you can remove the quarantine attribute after extracting it:

```bash
xattr -d com.apple.quarantine ./envlt
```

Then move the binary into your `PATH` and run it normally.

### Development usage from the repository

```bash
cargo run -p envlt-cli -- --help
```

## How It Works

```mermaid
flowchart LR
    A[.env or .env.example] --> B[envlt add]
    B --> C[Encrypted vault.age]
    C --> D[envlt vars]
    C --> E[envlt use]
    C --> F[envlt run]
    C --> G[envlt gen --set]
    C --> H[envlt export]
    H --> I[Encrypted .evlt bundle]
    I --> J[envlt import]
    J --> C
```

## Command Overview

| Command | Purpose |
| --- | --- |
| `envlt init` | Create the encrypted local vault |
| `envlt auth` | Manage stored vault authentication |
| `envlt add` | Import variables from `.env` or `.env.example` |
| `envlt list` | List stored projects |
| `envlt remove` | Remove a stored project |
| `envlt vars` | Show project variables and types |
| `envlt diff` | Compare against `.env.example` or another project |
| `envlt set` | Create or update variables |
| `envlt use` | Materialize a `.env` file |
| `envlt run` | Execute a child process with injected variables |
| `envlt gen` | Generate secure values and optionally store them |
| `envlt export` | Export a project to `.evlt` |
| `envlt import` | Import a `.evlt` bundle |
| `envlt doctor` | Diagnose vault and `.envlt-link` state |

## Security Notes

- the source of truth is an encrypted local vault at `~/.envlt/vault.age`
- vault passphrases can optionally be stored in the system keyring for the current `ENVLT_HOME`
- `envlt run` avoids writing `.env` files to disk
- bundles use a passphrase independent from the main vault passphrase
- `vars` masks `Secret` values
- `diff` reports categorized changes without printing values
- `gen --set` does not reveal generated values unless `--show` is explicitly used

For details, see [Security](docs/security.md).

## Documentation

Start with:

- [Documentation Index](docs/README.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Legacy Project Definition Summary (English)](docs/legacy-project-definition-summary.md)

Primary documents:

- [Getting Started](docs/getting-started.md)
- [CLI Reference](docs/cli-reference.md)
- [Architecture](docs/architecture.md)
- [Security](docs/security.md)
- [Roadmap](docs/roadmap.md)
- [Spec Alignment](docs/spec-alignment.md)
- [Contributing](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

## Development

Quality gates:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Repository release baseline already includes:

- CI on Linux and macOS
- release workflow scaffolding for tagged builds

## Distribution Status

What is already ready:

- project documentation
- license
- changelog
- contributor guide
- CI workflow
- release workflow
- Homebrew tap and install path

What still remains:

- refine the Homebrew formula toward source-first installation
- add signing and notarization for macOS artifacts
