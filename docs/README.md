# Documentation Index

This directory contains the current technical documentation for `envlt`.

## Start by goal

- New user onboarding: [Getting Started](getting-started.md)
- Daily command usage: [CLI Reference](cli-reference.md)
- Security review: [Security](security.md)
- Architecture deep dive: [Architecture](architecture.md)
- Product direction: [Roadmap](roadmap.md)
- Troubleshooting and recovery: [Troubleshooting](troubleshooting.md)
- Historical context (English summary): [Legacy Project Definition Summary](legacy-project-definition-summary.md)

## Recommended reading order

1. [Getting Started](getting-started.md)
2. [CLI Reference](cli-reference.md)
3. [Architecture](architecture.md)
4. [Security](security.md)
5. [Roadmap](roadmap.md)
6. [Spec Alignment](spec-alignment.md)
7. [Releasing](releasing.md)
8. [Troubleshooting](troubleshooting.md)
9. [Contributing](../CONTRIBUTING.md)
10. [Changelog](../CHANGELOG.md)

Historical source document:

- [Original Project Definition](project-definition.md)
- [Legacy Project Definition Summary](legacy-project-definition-summary.md)

## Documentation map

```mermaid
flowchart TD
    A[README.md] --> B[docs/README.md]
    B --> C[getting-started.md]
    B --> D[cli-reference.md]
    B --> E[architecture.md]
    B --> F[security.md]
    B --> G[roadmap.md]
    B --> H[spec-alignment.md]
    H --> I[project-definition.md]
```

## Document roles

| Document | Purpose |
| --- | --- |
| `getting-started.md` | Installation, first-run workflow, and common usage paths |
| `cli-reference.md` | Command-by-command reference |
| `troubleshooting.md` | First-run failures, diagnostics, and recovery playbooks |
| `architecture.md` | Core design, storage model, and runtime flow |
| `security.md` | Current security model and operational guidance |
| `roadmap.md` | What is still missing and what is planned next |
| `spec-alignment.md` | Verification of the current implementation against the original specification |
| `legacy-project-definition-summary.md` | English summary of the original definition and current status |
| `releasing.md` | Release checklist, required inputs, and release workflow expectations |
| `../CONTRIBUTING.md` | Contributor expectations and local development workflow |
| `../CHANGELOG.md` | Project-level change history |

## Documentation principles

This documentation set is intentionally:

- implementation-driven
- technical rather than promotional
- consolidated to avoid fragmented partial explanations
- explicit about what is implemented versus planned
