# Changelog

All notable changes to this project should be documented in this file.

The format is based on Keep a Changelog, and the project intends to follow Semantic Versioning.

## [Unreleased]

### Added

- **Environments**: a project's variables now live under one or more named environments (`local` by default, plus e.g. `staging`/`prod`), fully duplicated per environment with no inheritance between them. New `envlt env list`/`envlt env add <name>` commands, and a `--env <NAME>` flag on `vars`, `set`, `unset`, `history`, `check`, `use`, `run`, `gen`, and `export` (resolved as: explicit `--env` → the environment recorded on `.envlt-link` → `local`). `diff` gains `--env`/`--other-env`; when a second project name is omitted but `--other-env` is given, it compares two environments of the same project
- `envlt env use <name>` pins an environment as the current directory's default by writing it into `.envlt-link`'s `environment` field (re-runnable at any time, `kubectl config use-context`-style), so `--env` can be omitted on later commands; fails if the environment doesn't exist. `envlt env remove <name>` deletes an environment and everything in it (variables and their full version history), asking for confirmation by default (`--yes` to skip, same pattern as `envlt remove` for projects); every project must keep at least one environment, so removing the last one is an error
- **Full per-variable version history**: every variable (`Secret` and `Plain` alike) now keeps its past values, analogous to HashiCorp Vault KV v2's versioning, bounded by a new `max_versions` setting (default 10, see `config.rs`). `envlt history` is reconstructed on demand from this version list (via `synthesize_variable_events`) instead of a separately hand-maintained activity log, so the two can no longer drift out of sync with each other. Unsetting a variable tombstones it (`deleted_at`) rather than removing it, so its history survives deletion in `envlt history` while it disappears from `vars`/`.env` output/`run`/`diff`/`check`; setting it again revives it and continues the same version chain
- Safe-output regression test (`safe_output_never_leaks_a_known_secret_across_commands_and_formats` in `crates/envlt-cli/tests/cli_flow.rs`) that plants one known secret value and asserts it never appears in stdout/stderr for `vars`, `diff` (both modes), `doctor --decrypt`, `history` (project and variable level), `gen --set`, `export`, `import`, or representative error paths, across `table`, `raw`, and `json` output
- New `vault/migration.rs` module: vault format migrations are now versioned steps operating on the raw TOML table (not just `serde(default)` on the current struct), so a future format change can rename or restructure fields, not only add them. `VaultStore::load` rejects any version outside `MIN_SUPPORTED_VAULT_VERSION..=VAULT_VERSION`, writes a pre-migration backup (`vault.v{old}.pre-migration.age`) before migrating, and `doctor` reports a new `vault_format` check showing the current version or that a migration was just applied
- `envlt import` gained `--inspect` (show the bundle's unencrypted header -- project name, export time, envlt version -- without needing any passphrase) and `--dry-run` (decrypt and check the bundle against the vault, previewing variable keys/types and whether it would create or overwrite a project, without writing anything)
- `VaultStore::save` now rotates up to two additional numbered backups (`vault.age.bak.1`, `vault.age.bak.2`) instead of keeping only `vault.age.bak`, so one corrupted write can no longer destroy every prior good backup
- `doctor` now warns when a `.env` file sits next to a resolved `.envlt-link`, since anything that reads the working directory -- including AI coding assistants -- can read it in plaintext; the warning points at `envlt run` as the alternative that never writes the file
- Added a `supply-chain` CI job (`cargo-audit` + `cargo-deny`, via `deny.toml`), a release smoke test (`envlt --version`/`--help`) on every built binary before packaging, and build provenance attestation (`actions/attest-build-provenance`) per release artifact; `make audit` and `make deny` targets mirror this locally and are now part of `make check`
- New `config.rs` module: `ENVLT_HOME/config.toml` now holds persistent `history_limit` and `lock_timeout_ms` preferences (both optional, both still overridable via `ENVLT_HISTORY_LIMIT`/`ENVLT_LOCK_TIMEOUT_MS`), instead of those settings only existing as env-var reads scattered in `vault/model.rs` and `vault/store.rs`. Invalid TOML or an unparseable env var override now produce a specific, actionable error instead of a silent fallback. `doctor` reports a new `config` check showing the resolved values and their source

### Changed

- **Breaking**: vault format bumped to v3 for environments and full version history. `Project.variables`/`Project.activity_log` are gone, replaced by `Project.environments: BTreeMap<String, Environment>`, each holding `Environment.variables: BTreeMap<String, Variable>` where `Variable.versions: Vec<VariableVersion>` replaces the old single `value`/`var_type`/`updated_at` fields. A v1 or v2 vault is migrated automatically on next load (v1→v2→v3, one step at a time, same as before): each project's flat variables become its `local` environment, and the old `activity_log` is dropped rather than converted, since it already stored `None` for every `Secret` old/new value and converting only the `Plain` entries would produce a history that is inexplicably deeper for some variables than others. The pre-migration ciphertext is preserved verbatim (`vault.v{old}.pre-migration.age`), so nothing is destructively lost
- **Breaking**: bundle format bumped to v2. A `.evlt` bundle now carries exactly one environment, flattened to current values only (no version history, no soft-deleted variables), so sharing one environment's bundle can never leak another environment's secrets or a secret's past values. `BundleHeader` gained an `environment` field. Bundles exported by older `envlt` (bundle format v1) can no longer be imported; `envlt import`/`import --inspect`/`import --dry-run` now report "This bundle was exported by an older envlt (bundle format v{n}) and can no longer be imported. Ask the sender to re-export it with the current envlt version." instead of a generic version-mismatch error
- **Breaking**: `config.toml`'s `history_limit` setting and the `ENVLT_HISTORY_LIMIT` env var are renamed to `max_versions`/`ENVLT_MAX_VERSIONS`, reflecting that it now bounds each variable's own version count rather than a project-wide activity log length. An existing `config.toml` with `history_limit` keeps working via `#[serde(alias = "history_limit")]`; `doctor`'s `config` check now reports `max_versions=N` instead of `history_limit=N`
- **Breaking**: merged `VarType::Config` into `VarType::Plain`. Nothing in the codebase ever treated the two differently -- both were shown in full and masked identically -- so `envlt set`'s `--config` flag is removed and `envlt vars`/`history`/`import --dry-run` now report `plain` instead of `config` for those variables. Existing vaults and `.evlt` bundles with a stored `Config` value keep loading correctly (`#[serde(alias = "Config")]`) and are rewritten as `Plain` the next time they are saved; no vault version bump or migration step was needed

### Fixed

- `.envlt-link` resolution (`resolve_project_name`, `remove_project`, `doctor`) now walks up through parent directories, similar to `.git` discovery, so commands work from any subdirectory of a linked project (for example, a package folder inside a monorepo) instead of requiring `--project` or an exact match in the current directory
- Fixed a panic in `Project::push_activity_event` when `ENVLT_HISTORY_LIMIT=0`
- `VaultStore` now restricts `ENVLT_HOME` to `0700` and `vault.age`/`vault.age.bak` to `0600` on Unix, and `fsync`s the vault file and directory on save
- Added a cross-process advisory lock (`VaultStore::lock`, backed by `fs4`) so two concurrent `envlt` processes can no longer silently overwrite each other's vault changes; configurable via `ENVLT_LOCK_TIMEOUT_MS` (default 5s)
- `.evlt` bundles now record the scrypt `log_n`/`r`/`p` used to derive the encryption key in `BundleHeader`, instead of always re-deriving with `Params::recommended()` at decode time; bundles created before this change keep decrypting via a documented legacy default (log_n=17, r=8, p=1)
- `envlt-cli` now reads vault and bundle passphrases into `Zeroizing<String>` (via the new `zeroize` dependency), so the copy typed by the user or read from an env var is cleared from memory as soon as it goes out of scope, instead of lingering as a plain `String` for the rest of the process
- Upgraded `keyring` from `3.6` (no backend features enabled, so it silently fell back to keyring's in-memory mock store on Linux and Windows -- `envlt auth save` did not actually persist a passphrase on those platforms) to `4.1` with its default native backends (macOS Keychain, Windows Credential Manager, Linux Secret Service) enabled for all three platforms
- Removed the macOS-specific shell-out to `security add-generic-password`/`find-generic-password`/`delete-generic-password` in `crates/envlt-core/src/auth.rs`, which passed the passphrase via the `-w` command-line argument (visible in process listings). macOS now goes through the same in-process `keyring::Entry` API as Linux and Windows, backed natively by Keychain Services
- Extended passphrase zeroization from the CLI entry point into `envlt-core`: `auth::load_stored_passphrase` now returns `Zeroizing<String>` instead of a plain `String` for both the primary and legacy keyring round-trip (including the post-write verification read), `vault::crypto::decrypt` returns `Zeroizing<Vec<u8>>`, and the full plaintext TOML that flows through `VaultStore::load`/`save` and `bundle::encrypt_project_bundle`/`decrypt_project_bundle` (including the derived scrypt key) is now wrapped in `Zeroizing` so it is cleared on drop instead of left in a plain heap-allocated `String`/`Vec`/array. This does not extend to the deserialized domain model itself (`Variable.value` etc. remain plain `String`s for the lifetime of the loaded vault) -- see `docs/tech-debt.md` for that boundary
- `read_passphrase` now fails with the real keyring error instead of silently falling through to an interactive prompt when stdin isn't a terminal (scripts, CI), since that prompt could never be answered anyway; interactive sessions keep the existing warn-and-prompt behavior

## [0.3.0] - 2026-05-02

### Added

- **Variable Activity Log**: per-project audit trail that records variable lifecycle events (creation, updates, type changes, deletion) in an encrypted `activity_log` attached to each `Project`
- New `envlt history` command to inspect activity logs at project or variable level, with `--format table` (default), `--format raw`, and `--format json`
- New `ActivityEvent` and `ActivityAction` domain types in `envlt-core`
- Automatic secret masking in history entries: `Secret` variables store `None` for old/new values; `Config` and `Plain` store values in clear
- `Project::push_activity_event` with FIFO limit (default 20 events per project, configurable via `ENVLT_HISTORY_LIMIT`)
- `AppService::project_activity_log` and `AppService::variable_history` query methods
- Activity events are generated automatically by `set_variable`, `unset_variable`, `add_project_from_env_file`, `add_project_from_example`, and `import_project_bundle`
- Import with `--overwrite` preserves existing activity log and appends per-variable `VariableUpdated` / `VariableTypeChanged` / `VariableCreated` events
- `envlt vars` now displays a `last modified` column with per-variable timestamps
- Vault format version bumped from `1` to `2`; `VaultStore::load()` automatically migrates v1 vaults to v2 on read (next `save()` persists in new format)
- Unit tests for event generation, secret masking, log limit enforcement, and v1→v2 migration
- Integration tests for `history` command output, secret masking in stdout, deleted variable history survival, and `vars` last-modified column

### Changed

- `VariableView` now includes `updated_at` field for display in `vars` output
- `envlt-cli/Cargo.toml` adds `chrono` dependency for timestamp formatting

## [0.2.2] - 2026-05-01

### Fixed

- Pinned `rpassword` to `=7.4.0` to avoid `let` chains syntax incompatible with CI toolchain
- Resolved `--locked` release build failure caused by `rpassword 7.5.1`

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
