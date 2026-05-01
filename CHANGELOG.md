# Changelog

All notable changes to this project should be documented in this file.

The format is based on Keep a Changelog, and the project intends to follow Semantic Versioning.

## [Unreleased]

## [0.2.1] - 2026-05-01

### Fixed

- Updated `Cargo.lock` to include `clap_complete` dependency, fixing `--locked` release builds

## [0.2.0] - 2026-05-01

### Added

- `.env` parser with full real-world compatibility: comments, blank lines, whitespace around `=`, empty values, single-quoted values, double-quoted values, escape sequences (`\n`, `\t`, `\r`, `\\`, `\"`, `\'`), and optional `export` prefixes
- `.env` writer with safe quoting: automatically wraps values containing spaces, quotes, backslashes, `#`, newlines, tabs, carriage returns, or `=` in double quotes with proper escaping
- Roundtrip preservation between `envlt add` and `envlt use` for common `.env` inputs
- `envlt check <.env.example>` command for automation and pre-commit validation (exit `0` when complete, non-zero when missing variables)
- `envlt completions <shell>` command generating shell completion scripts for `bash`, `zsh`, `fish`, `powershell`, and `elvish`
- Atomic `.env` materialization in `envlt use` via `NamedTempFile` with `persist`
- Restrictive Unix file permissions (`0o600`) on generated `.env` files
- Warning message when using `envlt use` to remind users that `.env` files are plaintext artifacts
- `docs/threat-model.md` with explicit security boundaries, guarantees, non-goals, assumptions, and user responsibilities
- `docs/integrations.md` with practical recipes for `direnv`, Docker Compose, GitHub Actions/local CI, VS Code, AI coding agents, and monorepos
- Comprehensive parser and writer unit tests covering all supported `.env` syntax variants
- Integration tests for `envlt check` (success and failure paths)
- Integration test for `envlt completions bash` output validation
- Unix-specific test verifying restrictive file permissions on materialized `.env` files

### Changed

- Replaced modulo-biased random selection in `generate_from_alphabet` and `generate_memorable_password` with rejection sampling for unbiased output
- Updated `docs/roadmap.md` with measurable milestones and explicit product positioning (local-first, no cloud account required)
- Updated `docs/getting-started.md` to recommend Homebrew and document Windows via WSL
- Updated `docs/releasing.md` to reflect current distribution policy (Homebrew recommended, Apple signing/notarization not planned)
- Updated `docs/spec-alignment.md` to match current release strategy
- Updated `docs/troubleshooting.md` to remove stale Gatekeeper instructions and point users toward Homebrew
- Updated `docs/security.md` to link to the formal threat model
- Updated `docs/cli-reference.md` and `README.md` to include `envlt check` and `envlt completions`

### Fixed

- `envlt use` no longer writes directly to the target path, preventing partial or corrupted `.env` files on interruption
- Generator presets (`api-key`, `token`, `password`) no longer exhibit modulo bias when selecting from alphabets

## [0.1.0] - 2026-03-29

### Added

- Encrypted local vault with atomic persistence and backup
- `.env` and `.env.example` import flows
- `.envlt-link` project resolution
- Variable typing and inference
- `vars`, `diff`, `gen`, `doctor`
- `.evlt` export and import
- Consolidated English documentation set
- GitHub Actions CI and release workflow scaffolding

### Changed

- `gen --set` is now secure by default and does not reveal generated values unless `--show` is used
- `diff` uses a stable safe-summary output format
