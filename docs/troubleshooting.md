# Troubleshooting

Use this guide when `envlt` commands fail during setup or day-to-day usage.

## Quick triage

Run:

```bash
envlt doctor
envlt doctor --decrypt
```

Then classify the issue:

| Symptom                         | Likely cause                                   | First action                                                      |
| ------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------- |
| `vault.age` missing             | vault not initialized or wrong `ENVLT_HOME`    | run `envlt init` and verify `ENVLT_HOME`                          |
| passphrase prompt keeps failing | wrong passphrase source                        | unset `ENVLT_PASSPHRASE`, retry prompt, validate keyring          |
| project not resolved            | missing or stale `.envlt-link`                 | run commands with `--project` or recreate link via `envlt add`    |
| bundle import fails             | wrong bundle passphrase or conflicting project | retry with correct passphrase, then use `--overwrite` if intended |
| macOS binary blocked            | quarantine attribute present                   | remove quarantine from trusted binary                             |

## Initialization and authentication

### `envlt init` fails or vault cannot be found

Checks:

1. Confirm home path assumptions.
2. If using `ENVLT_HOME`, use an absolute path.
3. Re-run initialization.

```bash
envlt init
envlt doctor
```

### Repeated passphrase errors

Resolution order for vault access:

1. `ENVLT_PASSPHRASE`
2. stored keyring credential
3. interactive prompt

If the wrong source is winning, reset explicitly:

```bash
unset ENVLT_PASSPHRASE
envlt auth status
envlt auth clear
envlt auth save
```

## Project resolution and `.envlt-link`

### Command says project is missing

Run explicit project targeting first:

```bash
envlt vars --project api-payments
```

If this works, your local link is missing or stale. Recreate it from a known project:

```bash
envlt add api-payments
```

### `.envlt-link` exists but points to removed project

Either restore/import the project or create a new link by adding the intended project again.

## Run vs use confusion

| Situation                                 | Preferred command                           |
| ----------------------------------------- | ------------------------------------------- |
| You only need variables in process memory | `envlt run --project <name> -- <cmd>`       |
| Tooling requires a file on disk           | `envlt use --project <name> [--out <path>]` |

If a process cannot see variables, verify project selection explicitly and retry with `--project`.

## Bundle export/import issues

### Import reports conflict

By default import fails if project already exists.

```bash
envlt import bundle.evlt --overwrite
```

Use `--overwrite` only when replacement is intentional.

### Import or export passphrase problems

Checks:

1. Confirm bundle passphrase source (`ENVLT_BUNDLE_PASSPHRASE` or prompt).
2. Retry without stale shell exports.
3. Re-export bundle if corruption is suspected.

## Doctor and vault state checks

Use `envlt doctor --decrypt` when behavior looks inconsistent.

What to inspect from output:

- effective `envlt` home path
- `vault.age` presence
- backup presence (`vault.age.bak`)
- `.envlt-link` validity
- decryption/read status

If warnings appear without hard errors, command exit can still be successful.

## macOS Gatekeeper block

For trusted downloaded release binaries:

```bash
xattr -d com.apple.quarantine ./envlt
```

Then move binary into your `PATH` and retry.

## When to escalate

If recovery steps fail, gather these details before opening an issue:

- output of `envlt doctor --decrypt`
- your install method (Homebrew, Cargo, release binary)
- whether `ENVLT_HOME` is set
- failing command and exact arguments
- platform and architecture
