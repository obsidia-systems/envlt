# CLI Reference

This document describes the current CLI surface.

## Command summary

| Command | Description |
| --- | --- |
| `envlt init` | Initialize the encrypted vault |
| `envlt auth` | Manage stored vault authentication |
| `envlt add <project>` | Import `.env` or `.env.example` content |
| `envlt list` | List stored projects |
| `envlt remove <project>` | Remove a stored project |
| `envlt env list` | List a project's environments |
| `envlt env add <name>` | Add a new, empty environment to a project |
| `envlt env remove <name>` | Remove an environment and all its variables |
| `envlt env use <name>` | Pin the default environment for the current directory |
| `envlt vars` | Show variables and types |
| `envlt history` | Show the activity log for a project or variable |
| `envlt check` | Verify a project against `.env.example` |
| `envlt diff` | Compare against `.env.example` or another project |
| `envlt doctor` | Diagnose the local vault and link state |
| `envlt completions` | Generate shell completion scripts |
| `envlt set` | Create or update a variable |
| `envlt unset` | Delete a variable |
| `envlt use` | Write a `.env` file from the vault |
| `envlt run` | Run a child process with injected variables |
| `envlt gen` | Generate secure values |
| `envlt export` | Export a project to `.evlt` |
| `envlt import` | Import a `.evlt` bundle |

## Exit behavior baseline

`envlt` uses this practical baseline:

- `0` when command execution succeeds
- non-zero when an actionable error occurred
- warning-only output does not necessarily imply a failure exit code

For setup and recovery paths, see [Troubleshooting](troubleshooting.md).

## Environments

Every project has at least one environment (`local`, seeded automatically by `add`/`init`); `envlt env add <name>` creates more, e.g. `staging` or `prod`. Variables are fully duplicated per environment -- there is no inheritance between them, so a variable set in `local` has no effect on `staging` until you explicitly set it there too. A project must always keep at least one environment; `envlt env remove` refuses to delete the last one.

`vars`, `set`, `unset`, `history`, `check`, `use`, `run`, `gen`, and `export` all accept `--env <NAME>`, resolved in this order:

1. the explicit `--env` flag
2. the environment recorded on the nearest `.envlt-link`, set with `envlt env use <name>`
3. `local`

`diff` accepts both `--env` and `--other-env`; see [`envlt diff`](#envlt-diff) below.

## Commands

### `envlt init`

Initialize the local encrypted vault.

```bash
envlt init
```

Behavior:

- creates the `envlt` home directory
- creates `vault.age`
- prompts for passphrase confirmation

### `envlt auth`

Manage vault passphrase storage in the system keyring.

#### `envlt auth save`

```bash
envlt auth save
```

Behavior:

- reads the passphrase from `ENVLT_PASSPHRASE` or an interactive prompt
- verifies that the passphrase can decrypt the current vault
- saves the passphrase in the system keyring for the current `ENVLT_HOME`
- for stable keyring lookups, prefer an absolute `ENVLT_HOME`

#### `envlt auth clear`

```bash
envlt auth clear
```

Behavior:

- removes the stored vault passphrase from the system keyring
- does not modify the vault itself

#### `envlt auth status`

```bash
envlt auth status
envlt auth status --format raw
envlt auth status --format json
```

Behavior:

- reports whether `ENVLT_PASSPHRASE` is currently set
- reports whether a stored system keyring credential exists for the current `ENVLT_HOME`

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt add <project>`

Import variables into the vault and create `.envlt-link`.

```bash
envlt add api-payments
envlt add api-payments --file .env.local
envlt add api-payments --from-example .env.example
envlt add api-payments --project-path /path/to/project
```

### `envlt list`

List stored projects.

```bash
envlt list
envlt list --format raw
envlt list --format json
```

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt remove <project> [--yes]`

Remove a project from the vault.

```bash
envlt remove api-payments
envlt remove api-payments --yes
```

Behavior:

- asks for confirmation by default
- supports `--yes` for automation
- clears `.envlt-link` when it points to the removed project

### `envlt env`

Manage a project's environments.

#### `envlt env list [--project <name>]`

```bash
envlt env list --project api-payments
envlt env list --project api-payments --format raw
```

#### `envlt env add <name> [--project <name>]`

```bash
envlt env add staging --project api-payments
envlt env add prod --project api-payments
```

Behavior:

- fails if the environment already exists
- the new environment starts empty; it does not inherit variables from any other environment

#### `envlt env remove <name> [--project <name>] [--yes]`

```bash
envlt env remove staging --project api-payments
envlt env remove staging --project api-payments --yes
```

Behavior:

- deletes the environment and everything in it, including every variable's version history -- this cannot be undone
- asks for confirmation by default; `--yes` skips it, for automation
- fails if the environment doesn't exist, or if it's the project's only remaining environment (every project must keep at least one)
- there is no rename; recreate the environment and re-set its variables instead

#### `envlt env use <name> [--project <name>]`

```bash
envlt env use staging --project api-payments
envlt env use staging
```

Behavior:

- pins `<name>` as the current directory's default environment by writing it into `.envlt-link`, so later `--env`-aware commands run from here can omit `--env`
- fails if the environment doesn't exist in the vault, so a typo doesn't silently link to nothing
- re-run it to switch a directory's default to a different (existing) environment at any time

Output formats (`env list`):

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt vars [--project <name>] [--env <name>]`

Show variable names, types, and masked or visible values depending on type, for one environment.

```bash
envlt vars --project api-payments
envlt vars
envlt vars --project api-payments --env staging
envlt vars --project api-payments --format raw
envlt vars --project api-payments --format json
```

Output behavior:

- `Secret` values are masked
- `Plain` values are shown in full

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt history [--project <name>] [--env <name>]`

Show the history for a project's environment, or a specific variable within it.

```bash
envlt history --project my-api
envlt history --project my-api HELLO_WORLD
envlt history HELLO_WORLD
envlt history --project my-api --env staging
envlt history --project my-api --format raw
```

Behavior:

- without a key, shows every variable's history in the environment
- with a key, shows only that variable's history
- secret values are masked automatically (`********`)
- events include creation, updates, type changes, and deletion
- history is reconstructed on demand from each variable's version list (not a separately stored log), so a deleted variable's history survives -- it just disappears from `vars`/`.env` output/`run`/`diff`/`check`
- each variable keeps its most recent `max_versions` values (default 10), configurable via `max_versions` in `config.toml` or the `ENVLT_MAX_VERSIONS` environment variable (env var wins if both are set); older values are dropped and no longer appear in history

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt check`

Verify that a project satisfies a `.env.example` contract.

```bash
envlt check --project api-payments .env.example
envlt check .env.example
```

Exit behavior:

- `0` when all required variables are present
- non-zero when variables are missing

This is useful for automation, pre-commit hooks, and CI checks.

### `envlt diff`

#### Compare against `.env.example`

```bash
envlt diff --project api-payments --example .env.example
envlt diff --example .env.example
envlt diff --project api-payments --env staging --example .env.example
envlt diff --project api-payments --example .env.example --format raw
envlt diff --project api-payments --example .env.example --format json
```

`--example` conflicts with a second project name and with `--other-env` -- it always compares one project's environment against a file.

Reports:

- shared keys
- keys missing from the vault
- keys present only in the vault

Output format:

- `mode\texample`
- `project\t<name>`
- `environment\t<name>`
- `example\t<path>`
- `summary\tshared=...\tmissing=...\textra=...`
- categorized key lines

#### Compare two projects, or two environments

```bash
envlt diff --project api-payments api-auth
envlt diff --project api-payments --other-env staging
envlt diff --project api-payments --env staging api-auth --other-env prod
envlt diff --project api-payments api-auth --format raw
envlt diff --project api-payments api-auth --format json
```

`--env`/`--other-env` select which environment of each side to compare:

- with a second project name and no `--other-env`, both sides compare the same environment name (`--env`, or its default) across the two projects
- with `--other-env` and no second project name, both sides are the *same* project, comparing `--env` against `--other-env`
- both a second project name and `--other-env` can be given together, to compare two different projects' two different environments

Reports:

- shared keys
- keys with changed values
- keys with changed types
- keys only on the left side
- keys only on the right side

Output format:

- `mode\tproject`
- `left\t<project>@<environment>`
- `right\t<project>@<environment>`
- `summary\tshared=...\tchanged_values=...\tchanged_types=...\tonly_left=...\tonly_right=...`
- categorized key lines

### `envlt doctor [--decrypt]`

Run local diagnostics.

```bash
envlt doctor
envlt doctor --decrypt
envlt doctor --format raw
envlt doctor --format json
```

Checks currently include:

- `envlt` home path
- vault presence
- backup presence
- `.envlt-link` state, resolved from the current directory or any parent
- vault decryption, format version, and linked-project validation when `--decrypt` is used
- a stray `.env` file sitting next to a resolved `.envlt-link` (warns, since anything that reads the working directory -- including AI coding assistants -- can read it in plaintext; prefer `envlt run`)

Exit behavior:

- returns success when there are only warnings
- returns non-zero when real errors are detected

Common recovery steps for doctor failures are documented in [Troubleshooting](troubleshooting.md#doctor-and-vault-state-checks).

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt set [--project <name>] [--env <name>] <KEY=VALUE>`

Create or update a variable in one environment. Every set appends a new version rather than overwriting in place -- see [Architecture](architecture.md#version-history).

```bash
envlt set --project api-payments PORT=4000
envlt set --project api-payments --env staging PORT=4000
envlt set --project api-payments --secret JWT_SECRET=supersecret
envlt set --project api-payments --plain APP_NAME=my-app
```

Type flags:

- `--secret`
- `--plain`

### `envlt unset [--project <name>] [--env <name>] <KEY>`

Delete a variable from one environment. Its version history is kept (see `envlt history`); unsetting an already-deleted key is an error.

```bash
envlt unset --project api-payments JWT_SECRET
envlt unset --project api-payments --env staging JWT_SECRET
envlt unset JWT_SECRET
```

### `envlt use [--project <name>] [--env <name>] [--out <path>]`

Write a `.env` file from one environment in the vault.

```bash
envlt use --project api-payments
envlt use --project api-payments --env staging
envlt use --project api-payments --out .env.local
envlt use
```

### `envlt run [--project <name>] [--env <name>] -- <command> [args...]`

Run a child process with one environment's variables injected from the vault.

```bash
envlt run --project api-payments -- node server.js
envlt run --project api-payments --env staging -- node server.js
envlt run -- npm run dev
```

### `envlt gen`

Generate secure values.

```bash
envlt gen --list-types
envlt gen --list-types --format raw
envlt gen --list-types --format json
envlt gen
envlt gen --type jwt-secret
envlt gen --type password
envlt gen --len 64 --hex
envlt gen --len 32 --symbols
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --env staging
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --show
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --silent
```

`--env` selects which environment `--set` stores the generated value in; it has no effect without `--set`.

Supported presets:

- `jwt-secret`
- `uuid`
- `api-key`
- `token`
- `password`

Current behavior:

- supports flag-driven generation
- supports a guided interactive path
- can store the generated value directly in the vault
- does not reveal stored generated values by default
- supports `--show` as an explicit reveal flag
- treats `--show` and `--silent` as conflicting flags

Notes:

- `--format` currently applies to `--list-types` output

### `envlt export <project> [--env <name>] --out <path>`

Export one environment as an encrypted `.evlt` bundle. The bundle carries only that environment, flattened to current values (no version history, no soft-deleted variables), so a shared bundle can never expose another environment or a variable's past values.

```bash
envlt export api-payments --out bundle.evlt
envlt export api-payments --env staging --out staging-bundle.evlt
```

### `envlt import <path> [--overwrite] [--dry-run] [--inspect]`

Import a bundle into the local vault, into the environment it was exported from (creating that environment if the project already exists but doesn't have it yet).

```bash
envlt import bundle.evlt
envlt import bundle.evlt --overwrite
envlt import bundle.evlt --inspect
envlt import bundle.evlt --dry-run
```

Behavior:

- fails by default if the project already exists
- with `--overwrite`, merges into the existing project's matching environment: each incoming variable is recorded as a new version (so existing history is preserved) rather than silently replacing the environment
- `--inspect` prints the bundle's unencrypted header (project name, environment, export time, envlt version) and exits; it needs no vault or bundle passphrase, since the header sits outside the encrypted payload
- `--dry-run` decrypts the bundle and checks it against the vault (project name, environment, variable keys and types, whether it would create or overwrite) without writing anything; it still needs both passphrases, since checking for a name conflict requires decrypting the vault
- `--dry-run` and `--inspect` are mutually exclusive
- a bundle exported by an older, incompatible `envlt` (a bundle format version this build no longer understands) fails with a message asking the sender to re-export it

### `envlt completions <shell>`

Generate shell completion scripts.

Supported shells:

- `bash`
- `zsh`
- `fish`
- `powershell`
- `elvish`

Example:

```bash
envlt completions bash > /usr/local/etc/bash_completion.d/envlt
envlt completions zsh > /usr/local/share/zsh/site-functions/_envlt
envlt completions fish > ~/.config/fish/completions/envlt.fish
```
