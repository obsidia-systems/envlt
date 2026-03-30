# Getting Started

This guide covers the current supported way to install and use `envlt`.

## Who this guide is for

- developers evaluating `envlt` for local-first secret management
- teams that want reproducible `.env` workflows without cloud dependency

## Outcome

By the end of this guide you will:

- create and unlock your local encrypted vault
- attach a project and verify variables
- run your app using vault-backed variables

## Installation

### Install with Homebrew

```bash
brew install obsidia-systems/tap/envlt
envlt --help
```

### Install from the repository

```bash
cargo install --path crates/envlt-cli
envlt --help
```

### Install from GitHub Releases

If the project already has published release assets, you can install `envlt` manually from the release archive.

Example on Linux:

```bash
tar -xzf envlt-linux-x86_64.tar.gz
chmod +x envlt
sudo mv envlt /usr/local/bin/envlt
envlt --help
```

This is currently a manual binary installation flow, not a native package-manager install.

On macOS, downloaded binaries may carry Apple's quarantine attribute. Until signed and notarized releases are in place, remove it for trusted release binaries after extraction:

```bash
xattr -d com.apple.quarantine ./envlt
```

### Run without installing

```bash
cargo run -p envlt-cli -- --help
```

## Environment variables

| Variable | Purpose |
| --- | --- |
| `ENVLT_HOME` | Override the vault home directory |
| `ENVLT_PASSPHRASE` | Provide the vault passphrase non-interactively |
| `ENVLT_BUNDLE_PASSPHRASE` | Provide the bundle passphrase non-interactively |
| `ENVLT_GEN_TYPE` | Drive interactive `gen` selection |
| `ENVLT_GEN_SAVE` | Answer whether interactive `gen` should store the result |
| `ENVLT_GEN_SET_KEY` | Set the target key for interactive `gen` storage |
| `ENVLT_GEN_PROJECT` | Set the target project for interactive `gen` storage |

When setting `ENVLT_HOME`, prefer an absolute path so keyring lookup stays consistent across shells and working directories.

## First-run workflow

```mermaid
sequenceDiagram
    participant U as User
    participant C as envlt CLI
    participant V as vault.age

    U->>C: envlt init
    C->>V: create encrypted vault
    U->>C: envlt add api-payments
    C->>V: store project snapshot
    C->>U: create .envlt-link
    U->>C: envlt vars
    C->>V: read project variables
    U->>C: envlt run -- npm run dev
    C->>V: load variables in memory
```

Expected outcome after this flow:

- a working encrypted vault at `~/.envlt/vault.age`
- one linked project through `.envlt-link`
- successful command execution with injected environment variables

## Common workflows

### Import an existing `.env`

```bash
envlt init
envlt auth save
envlt add api-payments
envlt vars --project api-payments
```

Use `envlt auth save` if you want later commands to load the vault passphrase from the system keyring instead of prompting every time.

### Bootstrap from `.env.example`

```bash
envlt add api-payments --from-example .env.example
```

`envlt` keeps default values already present in the example file and prompts only for missing ones.

### Materialize a `.env`

```bash
envlt use --project api-payments
envlt use --project api-payments --out .env.local
```

Use this path when tools require a file on disk.

### Run without writing `.env`

```bash
envlt run --project api-payments -- node server.js
```

Use this path when you want lower disk exposure and reproducible runtime injection.

## Decision table

| If you want to... | Use this command | Why |
| --- | --- | --- |
| Start a process with vault variables only in memory | `envlt run --project <name> -- <cmd>` | Avoid writing `.env` files |
| Create a local env file for tools that require one | `envlt use --project <name> [--out <path>]` | Controlled materialization from encrypted state |
| Share project variables with a teammate | `envlt export <name> --out bundle.evlt` | Portable encrypted handoff |
| Bring a shared project snapshot into your machine | `envlt import bundle.evlt` | Fast local bootstrap |
| Check local health and links | `envlt doctor [--decrypt]` | Detect setup and decryption issues early |

### Generate and store a secret

```bash
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --show
```

Output policy:

- generation without storage prints the value unless `--silent`
- generation with storage prints a success message by default
- `--show` explicitly reveals the stored generated value
- `--silent` suppresses all output and conflicts with `--show`

### Share a project snapshot

```bash
envlt export api-payments --out bundle.evlt
envlt import bundle.evlt
```

### Remove a project

```bash
envlt remove api-payments
envlt remove api-payments --yes
```

By default, `envlt` asks for confirmation before deleting a project from the vault.

### Diagnose the local setup

```bash
envlt doctor
envlt doctor --decrypt
```

### Save the vault passphrase to the system keyring

```bash
envlt auth save
envlt auth status
envlt auth clear
```

Resolution order for vault access is:

- `ENVLT_PASSPHRASE`
- stored system keyring credential
- interactive prompt

## Automatic project resolution

When a directory contains `.envlt-link`, these commands can resolve the project automatically:

- `vars`
- `diff`
- `set`
- `use`
- `run`
- interactive parts of `gen`

Example `.envlt-link`:

```toml
project = "api-payments"
envlt_version = "1.0"
```

## Current limitations

- the Homebrew formula currently uses prebuilt release assets and may still evolve toward a source-first formula
- macOS artifacts are not signed or notarized yet, so Gatekeeper may block them until quarantine is removed manually
- Cloud sync is not implemented
- `gen` still lacks all planned presets
- `diff` intentionally does not provide before/after value views in this milestone

## Troubleshooting

If something fails during onboarding, start here:

- [Troubleshooting](troubleshooting.md)
- [CLI Reference](cli-reference.md)
