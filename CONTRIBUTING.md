# Contributing

Thanks for your interest in contributing to `envlt`.

## Development setup

Requirements:

- Rust `1.75.0`
- `rustfmt`
- `clippy`

Install the local binary for iterative testing:

```bash
cargo install --path crates/envlt-cli
```

Or run directly from the repository:

```bash
cargo run -p envlt-cli -- --help
```

## Quality gates

Before opening a PR, run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Contribution expectations

- keep the CLI secure by default
- avoid printing secrets in logs or command output
- prefer implementation-driven documentation updates in the same change
- keep `envlt-core` as the main home for domain logic
- keep `envlt-cli` focused on parsing, prompts, and presentation

## Documentation policy

If behavior changes, update the relevant docs in the same PR:

- `README.md`
- `docs/getting-started.md`
- `docs/cli-reference.md`
- `docs/roadmap.md`
- `docs/spec-alignment.md`

## Current roadmap priority

Near-term contributions are most valuable in:

- release engineering
- packaging readiness
- output hardening
- validation and recovery
- Keychain support after packaging readiness
