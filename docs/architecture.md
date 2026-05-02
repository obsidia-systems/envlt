# Architecture

This document describes the current implemented architecture, not the aspirational end-state from the original project definition.

## Workspace layout

```text
envlt/
├── Cargo.toml
├── crates/
│   ├── envlt-core/
│   └── envlt-cli/
└── docs/
```

## Component overview

```mermaid
flowchart LR
    A[envlt-cli] --> B[envlt-core]
    B --> C[vault model]
    B --> D[age encryption]
    B --> E[.env parser/writer]
    B --> F[bundle format]
    B --> G[link resolution]
```

## Design principles in the current implementation

- domain logic lives in `envlt-core`
- the CLI layer primarily handles argument parsing and user interaction
- vault writes are atomic
- format evolution is versioned
- project resolution is explicit or link-based

## Runtime flows

### Vault write flow

```mermaid
sequenceDiagram
    participant CLI as envlt-cli
    participant Core as envlt-core
    participant Store as VaultStore
    participant FS as Filesystem

    CLI->>Core: mutate project state
    Core->>Store: save(vault, passphrase)
    Store->>FS: copy existing vault to vault.age.bak
    Store->>FS: write temp file
    Store->>FS: persist temp file to vault.age
```

### Bundle flow

```mermaid
sequenceDiagram
    participant U as User
    participant CLI as envlt CLI
    participant Core as envlt-core

    U->>CLI: envlt export <project>
    CLI->>Core: load project snapshot
    Core->>Core: serialize project
    Core->>Core: derive key with scrypt
    Core->>Core: encrypt payload with ChaCha20-Poly1305
    Core-->>CLI: .evlt bytes

    U->>CLI: envlt import bundle.evlt
    CLI->>Core: decrypt bundle
    Core->>Core: validate format
    Core->>Core: import project snapshot
```

## Implemented storage model

Core domain types:

- `VaultData`
- `Project`
- `Variable`
- `VarType`
- `ActivityEvent`
- `ActivityAction`

Current `VarType` values:

- `Secret`
- `Config`
- `Plain`

Each `Project` maintains an `activity_log` (`Vec<ActivityEvent>`) that records variable lifecycle events. The log is part of the encrypted vault and travels with `.evlt` bundles. It survives variable deletion because it lives at the project level, not inside each `Variable`.

## Implemented persistence guarantees

- encrypted vault file
- basic version validation
- atomic write path through a temporary file
- automatic backup copy before overwrite

## Implemented CLI-to-core split

### `envlt-core`

Responsibilities:

- vault persistence
- encryption and decryption
- `.env` parsing and rendering
- bundle serialization
- project link resolution
- diffing and diagnostics
- generator logic

### `envlt-cli`

Responsibilities:

- command-line interface with `clap`
- prompts and interactive flow
- printing user-facing output
- passing validated input into the core service

## Not yet implemented

- cloud provider abstraction
- merge engine for external vaults
- migration subsystem beyond current version validation
- Keychain integration
- GUI crate
