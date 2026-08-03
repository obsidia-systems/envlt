# Feature Ideas & Product Backlog

This document captures the current product state, known gaps, and a structured backlog of possible implementations for `envlt`. It merges the historical roadmap context with the previous `recommendations-2026.md` items.

> **Principle**: `envlt` should remain a local-first encrypted `.env` tool. Avoid turning it into a full cloud secrets platform unless explicitly decided.

---

## Current State

### What Is Already Done

| Area | Status |
| --- | --- |
| Local encrypted vault | Done |
| Core CLI workflow (`init`, `add`, `set`, `unset`, `use`, `run`, `list`, `remove`) | Done |
| `.evlt` export/import | Done |
| Variable typing and type inference | Done |
| `.env.example` bootstrap | Done |
| Diffing | Done with a safe-summary baseline |
| Secret generation | Done with secure storage defaults |
| Diagnostics with `doctor` | Done |
| System keyring integration | Partial |
| Homebrew install path | Done |
| `.env` parser with quotes, escapes, comments, and `export` | Done |
| `.env` writer with safe quoting and roundtrip preservation | Done |
| Atomic `.env` materialization with restrictive permissions | Done |
| Shell completion generation | Done |
| `envlt check --example` for automation | Done |
| Threat model document | Done |
| Integration recipes document | Done |

### Known Gaps

| Area | Gap | Why It Matters |
| --- | --- | --- |
| Recovery | Vault and bundle recovery guidance can be stronger | Users need confidence before storing important local state |
| Collaboration | Bundle import/overwrite behavior can become more transparent | Small teams need predictable handoff workflows |

### Completed Milestones (Historical Reference)

- **Milestone 1 — Core UX Polish**: `.env` parser/writer robustness, atomic writes, safe materialization, shell completions.
- **Milestone 2 — Security Trust Baseline**: Threat model, secret generation review, memory-handling docs.
- **Milestone 3 — Workflow Integrations**: `direnv`, Docker Compose, CI, VS Code, AI agent recipes.
- **Milestone 4 — Recovery And Collaboration**: Bundle validation, overwrite flags, backup docs, `doctor`, 104 tests passing.

---

## Active Feature Ideas

The items below are grouped by **domain affinity** and **dependency**. Each entry notes whether it overlaps with existing technical debt (see [`tech-debt.md`](./tech-debt.md)) and what it depends on.

### 1. Vault Durability & Concurrency Hardening — Done
- **Overlap with tech-debt**: Resolved — locking, fsync, permissions, and backup rotation all implemented.
- **Dependencies**: None.
- **Scope** (implemented):
  - `VaultStore::lock` — a cross-process advisory lock (`fs4`) around read-modify-write sequences, with a configurable timeout (`ENVLT_LOCK_TIMEOUT_MS`).
  - `ENVLT_HOME` restricted to `0700`, `vault.age`/backups to `0600` on Unix.
  - The temp vault file and its directory are fsynced before/after the atomic rename.
  - `save()` rotates three backup generations (`vault.age.bak`, `.bak.1`, `.bak.2`) instead of one.
  - `doctor` reports vault/backup presence and format version.
- **Why**: This deepens trust in the current storage model without replacing it.

### 2. Auth & Keyring Hardening — Done
- **Overlap with tech-debt**: Resolved — macOS shell-out, zeroization, doc drift, and silent keyring failures all addressed.
- **Dependencies**: None.
- **Scope** (implemented):
  - macOS no longer shells out to `security`; all three platforms go through the same in-process `keyring::Entry` API (Keychain Services, Credential Manager, Secret Service), fixing a latent bug where Linux/Windows silently used keyring's in-memory mock store.
  - Passphrases are held in `Zeroizing` from the CLI prompt/env-var entry point through the keyring round-trip and the transient encrypt/decrypt plaintext buffers in `envlt-core`.
  - `docs/architecture.md` and `docs/security.md` reflect the implemented keyring flow and precedence order (`ENVLT_PASSPHRASE` → keyring → prompt).
  - `read_passphrase` fails with the real keyring error instead of silently prompting when stdin isn't a terminal (scripts/CI); interactive sessions keep the warn-and-prompt fallback.
- **Why**: Authentication is a trust boundary; it should be boring and avoid accidental secret exposure.

### 3. Link Resolution Improvements — Done
- **Overlap with tech-debt**: Resolved — parent-directory walking implemented.
- **Dependencies**: None.
- **Scope** (implemented):
  - `link::find_project_link` walks upward from the current directory until a `.envlt-link` is found, the same way `.git` is resolved.
  - `doctor` shows the resolved link's directory, and warns if a `.env` file is left next to it (see [Threat Model: AI Coding Assistants and Local Files](../docs/threat-model.md#ai-coding-assistants-and-local-files)).
  - `remove_project` removes the link from wherever it was actually found, not just the current directory.
- **Not done**: explicit link schema/version validation, and a dedicated link-status command (currently folded into `doctor`).
- **Why**: Makes `envlt vars`, `envlt run`, `envlt use`, and future UI work naturally inside real repositories.

### 4. Safe-Output Regression Tests — Done
- **Overlap with tech-debt**: Resolved — the safe-output test matrix now exists.
- **Dependencies**: None.
- **Scope** (implemented):
  - `safe_output_never_leaks_a_known_secret_across_commands_and_formats` plants one known secret value and asserts it never appears in stdout/stderr for `vars`, `diff` (both modes), `doctor --decrypt`, `history` (project and variable level), `gen --set`, `export`, `import`, and representative error paths, across `table`, `raw`, and `json` output.
- **Why**: Safe output is a core promise of a secret-management tool.

### 5. Migration Infrastructure — Done
- **Overlap with tech-debt**: Resolved — `vault/migration.rs` now exists.
- **Dependencies**: None (enables later features).
- **Scope** (implemented):
  - Introduced a `vault/migration.rs` module with one versioned step per format change (`migrate_v1_to_v2`, ...), operating on the raw TOML table so future migrations can restructure fields, not just default them.
  - `VaultStore::load` rejects anything outside `MIN_SUPPORTED_VAULT_VERSION..=VAULT_VERSION`.
  - A pre-migration backup (`vault.v{old}.pre-migration.age`) is written before migrating.
  - `doctor` reports a `vault_format` check showing the current version or that a migration was just applied.
  - Fixture vaults for old versions live in `vault::store::tests` and `app::service::tests::vault_v1_migration_loads_with_empty_activity_log`.
- **Why**: Required before introducing `Environment` or any vault format change.

### 6. Bundle Sharing Enhancements
- **Overlap with tech-debt**: Medium — covers KDF parameters and dry-run/inspect.
- **Dependencies**: None.
- **Scope**:
  - Store KDF parameters in the `.evlt` bundle header so future `scrypt` changes do not break old bundles.
  - Add `import --dry-run`.
  - Add a bundle inspection command that shows safe metadata only.
  - Make overwrite previews explicit.
  - Use restrictive permissions for exported bundles where possible.
- **Why**: Bundles are the collaboration path; users need to know what they are importing.

### 7. Terminal UI (TUI)
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: Link resolution (3) is highly recommended first so the TUI feels natural from subdirectories.
- **Scope**:
  - Running `envlt` with no subcommand opens an interactive terminal UI (e.g., `ratatui` + `crossterm`).
  - Resolve the project via `.envlt-link` (with parent-directory search) or explicit context.
  - Show active project, selected environment, vault status, and link status.
  - List variables with `Secret` values hidden by default; reveal only through intentional, temporary action.
  - Allow creating, editing, deleting, and retagging variables.
  - Allow materializing `.env` through an explicit action with warning.
  - Allow running `doctor` from inside the UI.
- **Architecture rule**: The UI must be an **adapter over `AppService`**, not a second implementation of vault rules.
- **Security rules**: Never show `Secret` values by default; avoid copying secrets to logs or panic messages; consider clipboard support only as an explicit, timed operation.
- **Why**: Reduces command memorization and makes the vault discoverable without weakening the CLI model.

### 8. Project Environments
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: Migration infrastructure (5) is **required** before changing the vault model.
- **Scope**:
  - Introduce `Environment` as an explicit domain concept under `Project`.
  - Migrate existing projects into a `local` environment.
  - Keep variables fully scoped to one environment in the first version.
  - Allow `.envlt-link` to store project + optional default environment.
  - Recommended CLI shape:
    ```bash
    envlt env list --project api-payments
    envlt env add staging --project api-payments
    envlt vars --project api-payments --env staging
    envlt set DATABASE_URL=... --project api-payments --env staging --secret
    envlt use --project api-payments --env staging --out .env
    envlt run --project api-payments --env staging -- node server.js
    envlt diff --project api-payments --env staging --other-env prod
    ```
  - In the TUI: project selector → environment selector → variables scoped to selection.
- **Open design questions** (to settle before implementation):
  - Should each project have a default environment?
  - Should variables be duplicated per environment, or should there be shared project-level variables plus overrides?
  - Should `.evlt` bundles export one environment, selected environments, or the whole project?
- **Deferred**: Shared/project-level inherited variables until there is strong demand.
- **Why**: Most teams already think in environments (`dev`, `staging`, `prod`).

### 9. Configuration File — Done
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: None.
- **Scope** (implemented):
  - `config::Config` reads from `ENVLT_HOME/config.toml`, resolved via `VaultStore::config()`.
  - Supports `history_limit` (default 20) and `lock_timeout_ms` (default 5000), the two settings that previously lived only as env-var reads scattered in `vault/model.rs` and `vault/store.rs`.
  - `ENVLT_HISTORY_LIMIT` / `ENVLT_LOCK_TIMEOUT_MS` still override the config file value when set.
  - The file is fully optional -- a missing or partially-filled `config.toml` falls back to defaults field by field.
  - Invalid TOML or an unparseable env var override both produce a specific `EnvltError` (`ConfigParse` / `InvalidConfigValue`) instead of a silent fallback or a generic error.
  - `doctor` reports a new `config` check showing the resolved values and whether they came from `config.toml` or defaults.
- **Not done**: a dedicated `envlt config` command to view/edit the file (currently edited by hand); revisit if users ask for it.
- **Why**: Persistent preferences without polluting the user's shell environment.

### 10. Improve `envlt run`
- **Overlap with tech-debt**: None — enhancement.
- **Dependencies**: None.
- **Scope**:
  - Document whether vault variables override inherited environment variables.
  - Add optional collision warnings.
  - Consider `--clean` for a minimal environment.
  - Preserve child exit codes accurately.
  - Handle Unix signal exits more faithfully where supported.
- **Why**: `run` is the safest daily workflow because it avoids writing plaintext `.env` files.

### 11. Supply-Chain Trust Basics — Mostly Done
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: None.
- **Scope** (implemented):
  - Added a `supply-chain` CI job running `cargo-audit` (via `rustsec/audit-check`) and `cargo-deny` (via `EmbarkStudios/cargo-deny-action`) on every PR and push to `main`.
  - Added `deny.toml`: license allowlist (MIT/Apache-2.0/BSD/MPL-2.0/Unicode-3.0/Zlib/Unlicense/LGPL-2.1-or-later), and an explicit, justified `ignore` list for the two known advisories (`RUSTSEC-2026-0190` anyhow unsoundness -- envlt never calls `downcast_mut`; `RUSTSEC-2024-0370` proc-macro-error unmaintained -- only reached transitively through `age`).
  - Release checksums already existed (`shasum -a 256` per artifact); now paired with `actions/attest-build-provenance` per artifact for cryptographic build provenance.
  - Added a release smoke test (`envlt --version` / `--help`) after each cross-platform build, before packaging.
  - Added `make audit` / `make deny` targets, folded into `make check` and `make release-check`.
- **Not done**: SBOM publishing. Revisit if a consumer actually asks for one; `cargo-deny`'s license/advisory data already covers most of what an SBOM would be used for at this project's size.
- **Why**: Developers need extra confidence before installing a tool that manages secrets.

---

## Analysis: Convergence, Overlap & Complement

| Idea | Converges / Overlaps With | Complements | Notes |
|------|---------------------------|-------------|-------|
| **1. Vault Durability** | Tech-debt: locking, fsync, permissions, backups | 5 (Migration), `doctor` | Foundation work; makes every future vault change safer. |
| **2. Auth Hardening** | Tech-debt: macOS args, zeroization, docs | 4 (Safe-output), 11 (Trust) | Cleaning the trust boundary benefits all commands. |
| **3. Link Resolution** | Tech-debt: parent-directory walk | 7 (TUI), 8 (Environments) | Required for a natural TUI experience in real repos. |
| **4. Safe-Output Tests** | Tech-debt: test matrix | 2 (Auth), 7 (TUI), 8 (Environments) | Regression net; should be in place before UI/env changes add new output paths. |
| **5. Migration Infra** | Tech-debt: migration subsystem | 8 (Environments) | **Hard prerequisite** for any vault format evolution. |
| **6. Bundle Sharing** | Tech-debt: KDF params, dry-run | 8 (Environments) | Can be done independently, but bundles will eventually need env-scoped export. |
| **7. Terminal UI** | — | 3 (Link), 8 (Environments), 9 (Config) | Best built after link resolution works; config file can store UI preferences later. |
| **8. Environments** | — | 3 (Link), 5 (Migration), 6 (Bundle), 7 (TUI) | **Requires migration infra first.** Link and TUI make it usable; bundles need env-scoped export rules. |
| **9. Config File** | — | 7 (TUI), 10 (Run) | Independent, but the TUI can read defaults (e.g., output format) from it. |
| **10. Improve `run`** | — | 9 (Config), 8 (Environments) | Config could store `--clean` preference; environments add `--env` to `run`. |
| **11. Supply-Chain** | — | 2 (Auth), overall trust | Independent release-hygiene work. |

### Key Dependency Chain
```text
1 (Vault Durability) ─┬→ 4 (Safe-Output Tests)
2 (Auth Hardening) ───┤
3 (Link Resolution) ──┼→ 7 (TUI) ──┐
5 (Migration) ────────┴→ 8 (Environments) ─┘
6 (Bundle) ───────────────────────────────┘
9 (Config) ───────────────────────────────┘
10 (Run) ─────────────────────────────────┘
11 (Supply-Chain) ────────────────────────┘
```

### Suggested Implementation Order

Items 1–6, 9, and 11 are done (see each item above for what was implemented). What's left, in order:

7. **Terminal UI** — builds on link resolution and safe-output guarantees, both now in place.
8. **Project Environments** — uses the migration infrastructure, which now exists.
10. **Improve `envlt run`** — polishes the safest daily workflow.

---

## Deferred / Out of Scope

These remain valid ideas but are not near-term polish work:

| Item | Reason |
|------|--------|
| Native Windows support outside WSL | Current WSL path is sufficient for the target audience. |
| Cloud sync (`cloud link`, `cloud status`, `sync`) | Conflicts with local-first mission; deferred until merge strategy is defined. |
| Remote conflict detection and resolution | Requires cloud sync or multi-user semantics first. |
| GUI (`envlt-bar`) | Deferred until the TUI proves the interaction model. |
| Apple signing and notarization | Not planned for the current Homebrew-centric distribution strategy. |
| Replace `vault.age` with SQLite | Perceived performance is not a problem at typical `.env` scale; metadata leakage risks. |
| Manual per-variable encryption in plaintext SQLite | Avoids metadata leakage but adds complexity without a proven need. |
| Shared project-level variable inheritance | Defer until environment demand proves it is necessary. |

---

## Roadmap Policy

Near-term work should improve one of these outcomes:

1. The tool handles real `.env` files correctly.
2. The tool avoids accidental secret exposure.
3. The user can understand the security boundaries quickly.
4. The tool fits common local workflows without a cloud account.
5. Recovery and handoff behavior is predictable.
