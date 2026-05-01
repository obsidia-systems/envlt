# Integrations

This document shows practical ways to use `envlt` with common development workflows.

The preferred pattern is:

- use `envlt run` when a tool can read variables from the process environment
- use `envlt use` only when a tool requires a physical `.env` file
- use `vars`, `diff`, and `doctor` for safe diagnostics because they avoid printing secret values by default

## direnv

`direnv` automatically loads and unloads environment variables when entering or leaving a directory.

The safest current pattern is to use `envlt run` directly instead of loading secrets into the interactive shell:

```bash
envlt run -- npm run dev
```

If a project already depends on `direnv`, use a generated local `.env` file and keep it ignored by Git:

```bash
envlt use --out .env.local
```

Then in `.envrc`:

```bash
dotenv .env.local
```

Operational notes:

- `.env.local` is plaintext after generation.
- `.env.local` should be listed in `.gitignore`.
- run `direnv allow` only after reviewing `.envrc`.
- prefer regenerating `.env.local` from the vault instead of editing it manually.

A future `envlt shell` command could print shell exports directly, but that is not implemented yet.

## Docker Compose

Docker Compose has two different environment behaviors:

- process environment variables inherited from the `docker compose` command
- `.env` file interpolation used by Compose itself

If Compose can use inherited process variables for your project, prefer:

```bash
envlt run -- docker compose up
```

If Compose requires a `.env` file for interpolation, materialize it explicitly:

```bash
envlt use --out .env
docker compose up
```

Operational notes:

- `.env` is plaintext after generation.
- `.env` should be listed in `.gitignore`.
- prefer `envlt diff --example .env.example` before starting Compose to catch missing variables.

## GitHub Actions And Local CI Checks

`envlt` is primarily a local development tool, not a hosted CI secrets manager.

Useful CI-style checks are still possible without exposing values. For example, validate that the local vault matches the committed `.env.example` contract:

```bash
envlt diff --example .env.example --format raw
```

For local automation, provide the vault passphrase through the environment:

```bash
ENVLT_PASSPHRASE="$ENVLT_PASSPHRASE" envlt diff --example .env.example --format json
```

Operational notes:

- do not commit `ENVLT_PASSPHRASE` or bundle passphrases.
- avoid running `envlt use` in hosted CI unless the plaintext file is strictly needed and cleaned up.
- prefer platform-native secret stores for production CI/CD.

A future `envlt check --example .env.example` command could provide stricter automation-oriented exit behavior.

## VS Code

VS Code tasks can run commands through `envlt` without creating a `.env` file.

Example `.vscode/tasks.json` task:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "dev",
      "type": "shell",
      "command": "envlt run -- npm run dev",
      "problemMatcher": []
    }
  ]
}
```

For debug configurations that require an environment file, generate one intentionally:

```bash
envlt use --out .env.local
```

Then point the debugger to `.env.local` if the language extension supports env files.

Operational notes:

- keep `.env.local` out of Git.
- prefer tasks using `envlt run` for daily development.
- use `envlt vars` when checking names and types because secret values are masked.

## AI Coding Agents

AI coding agents can help diagnose project setup, but they should not receive secret values.

Safe commands to ask an agent to run:

```bash
envlt doctor
envlt doctor --decrypt
envlt vars --format raw
envlt diff --example .env.example --format raw
```

Avoid asking an agent to run commands that reveal or create plaintext secrets unless you explicitly intend that behavior:

```bash
envlt use
envlt gen --show
envlt run -- printenv
```

Recommended agent workflow:

- use `doctor` to inspect vault and link health
- use `vars` to inspect variable names and types without revealing secrets
- use `diff` to compare the vault against `.env.example`
- prefer `envlt run` for application execution instead of exposing `.env` files to the workspace

## Monorepos

For monorepos, use one `.envlt-link` per runnable project directory.

Example layout:

```text
repo/
├── apps/
│   ├── api/.envlt-link
│   └── web/.envlt-link
└── packages/
```

Run commands from the linked project directory:

```bash
cd apps/api
envlt run -- npm run dev
```

Operational notes:

- use explicit `--project` when running commands from the repository root.
- keep project names stable because `.envlt-link` stores the vault project reference.
