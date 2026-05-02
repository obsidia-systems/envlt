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
| Output hardening | Safe-output guarantees should be tested command by command | Users must trust that secrets are not printed accidentally |
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

### 1. Vault Durability & Concurrency Hardening
- **Overlap with tech-debt**: High — covers file locking, fsync, permissions, and backup rotation.
- **Dependencies**: None.
- **Scope**:
  - Add a lockfile under `ENVLT_HOME` to prevent concurrent writers (timeout + stale-lock detection).
  - Set restrictive permissions on `ENVLT_HOME` (`0700`) and `vault.age` / `vault.age.bak` (`0600`) on Unix.
  - fsync the temporary vault file before renaming, and fsync the directory after rename where supported.
  - Consider rotating more than one backup.
  - Make `doctor` report lock and backup health.
- **Why**: This deepens trust in the current storage model without replacing it.

### 2. Auth & Keyring Hardening
- **Overlap with tech-debt**: High — covers macOS `security` CLI exposure, memory zeroization, doc drift, and silent failures.
- **Dependencies**: None.
- **Scope**:
  - Pass the passphrase through stdin (not `-w`) when calling macOS `security`, or use a safer native API.
  - Evaluate `secrecy` / `zeroize` for passphrase memory handling.
  - Update `docs/architecture.md` and `docs/security.md` to reflect implemented keyring support.
  - Document the exact precedence order: `ENVLT_PASSPHRASE` → keyring → prompt.
  - Add tests around keyring target scoping by `ENVLT_HOME`.
  - Make keyring failures more visible (optional explicit mode before falling back to prompt).
- **Why**: Authentication is a trust boundary; it should be boring and avoid accidental secret exposure.

### 3. Link Resolution Improvements
- **Overlap with tech-debt**: Medium — covers parent-directory walking.
- **Dependencies**: None.
- **Scope**:
  - Search upward from the current directory until a `.envlt-link` is found.
  - Have `doctor` show the resolved link path.
  - Validate link schema/version.
  - Add a command or output path for link status.
- **Why**: Makes `envlt vars`, `envlt run`, `envlt use`, and future UI work naturally inside real repositories.

### 4. Safe-Output Regression Tests
- **Overlap with tech-debt**: High — covers the missing safe-output test matrix.
- **Dependencies**: None.
- **Scope**:
  - Seed a known secret value in test vaults.
  - Assert that the known secret never appears in stdout/stderr unless explicitly revealed.
  - Cover `vars`, `diff`, `doctor`, `gen --set`, `import`, `export`, and error paths.
  - Cover `table`, `json`, and `raw` output where applicable.
- **Why**: Safe output is a core promise of a secret-management tool.

### 5. Migration Infrastructure
- **Overlap with tech-debt**: Medium — covers the missing migration subsystem.
- **Dependencies**: None (enables later features).
- **Scope**:
  - Introduce a `vault/migration.rs` module.
  - Support versioned migrations from old `VaultData` shapes.
  - Create a backup before any migration.
  - Make `doctor` report migration readiness.
  - Keep fixture vaults for old versions in tests.
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

### 9. Configuration File
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: None.
- **Scope**:
  - Add a `Config` struct that reads from `~/.envlt/config.toml` (or `ENVLT_HOME/config.toml`).
  - Support `history_limit` with a sensible default (e.g. 20).
  - Allow envvars to override config file values (envvar wins).
  - Keep the config file optional; default behavior should work without one.
  - Validate config on load and emit actionable errors for invalid values.
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

### 11. Supply-Chain Trust Basics
- **Overlap with tech-debt**: None — new feature.
- **Dependencies**: None.
- **Scope**:
  - Add `cargo audit` and `cargo deny` to CI.
  - Publish checksums for releases.
  - Consider GitHub artifact attestations or `cosign`.
  - Publish an SBOM.
  - Add release smoke tests for the built binary.
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

1. **Vault Durability & Concurrency** — makes everything below safer.
2. **Auth & Keyring Hardening** — fixes the trust boundary.
3. **Link Resolution** — improves daily UX and enables the TUI.
4. **Safe-Output Regression Tests** — establishes a regression net before new output surfaces appear.
5. **Migration Infrastructure** — unlocks format evolution.
6. **Bundle Sharing Enhancements** — improves collaboration safety.
7. **Terminal UI** — builds on link resolution and safe-output guarantees.
8. **Project Environments** — uses migration, link, and TUI foundations.
9. **Configuration File** — adds persistent preferences.
10. **Improve `envlt run`** — polishes the safest daily workflow.
11. **Supply-Chain Trust** — hardens release confidence.

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
