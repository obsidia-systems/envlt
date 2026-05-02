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

### `envlt vars [--project <name>]`

Show variable names, types, and masked or visible values depending on type.

```bash
envlt vars --project api-payments
envlt vars
envlt vars --project api-payments --format raw
envlt vars --project api-payments --format json
```

Output behavior:

- `Secret` values are masked
- `Config` and `Plain` values are shown

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt history`

Show the activity log for a project or a specific variable.

```bash
envlt history --project my-api
envlt history --project my-api HELLO_WORLD
envlt history HELLO_WORLD
envlt history --project my-api --format raw
```

Behavior:

- without a key, shows the full project activity log
- with a key, shows only events for that variable
- secret values are masked automatically (`********`)
- events include creation, updates, type changes, and deletion
- the log survives variable deletion
- the default per-project limit is 20 events (configurable via `ENVLT_HISTORY_LIMIT`)

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
envlt diff --project api-payments --example .env.example --format raw
envlt diff --project api-payments --example .env.example --format json
```

Reports:

- shared keys
- keys missing from the vault
- keys present only in the vault

Output format:

- `mode\texample`
- `project\t<name>`
- `example\t<path>`
- `summary\tshared=...\tmissing=...\textra=...`
- categorized key lines

#### Compare two projects

```bash
envlt diff --project api-payments api-auth
envlt diff --project api-payments api-auth --format raw
envlt diff --project api-payments api-auth --format json
```

Reports:

- shared keys
- keys with changed values
- keys with changed types
- keys only on the left project
- keys only on the right project

Output format:

- `mode\tproject`
- `left\t<name>`
- `right\t<name>`
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
- `.envlt-link` state in the current directory
- vault decryption and linked-project validation when `--decrypt` is used

Exit behavior:

- returns success when there are only warnings
- returns non-zero when real errors are detected

Common recovery steps for doctor failures are documented in [Troubleshooting](troubleshooting.md#doctor-and-vault-state-checks).

Output formats:

- `--format table` (default)
- `--format raw`
- `--format json`

### `envlt set [--project <name>] <KEY=VALUE>`

Create or update a variable.

```bash
envlt set --project api-payments PORT=4000
envlt set --project api-payments --secret JWT_SECRET=supersecret
envlt set --project api-payments --plain APP_NAME=my-app
```

Type flags:

- `--secret`
- `--config`
- `--plain`

### `envlt unset [--project <name>] <KEY>`

Delete a variable from a project.

```bash
envlt unset --project api-payments JWT_SECRET
envlt unset JWT_SECRET
```

### `envlt use [--project <name>] [--out <path>]`

Write a `.env` file from the vault.

```bash
envlt use --project api-payments
envlt use --project api-payments --out .env.local
envlt use
```

### `envlt run [--project <name>] -- <command> [args...]`

Run a child process with variables injected from the vault.

```bash
envlt run --project api-payments -- node server.js
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
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --show
envlt gen --type jwt-secret --set JWT_SECRET --project api-payments --silent
```

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

### `envlt export <project> --out <path>`

Export a project as an encrypted `.evlt` bundle.

```bash
envlt export api-payments --out bundle.evlt
```

### `envlt import <path> [--overwrite]`

Import a bundle into the local vault.

```bash
envlt import bundle.evlt
envlt import bundle.evlt --overwrite
```

Behavior:

- fails by default if the project already exists
- replaces the full project snapshot when `--overwrite` is used

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
