# Legacy Project Definition Summary

This page provides an English summary of the historical source document in [project-definition.md](project-definition.md).

## Status of the source document

- `project-definition.md` is a legacy planning artifact.
- it is written in Spanish.
- parts of it describe aspirational scope that is not implemented in the current codebase.

Use this summary plus [Spec Alignment](spec-alignment.md) for current decision-making.

## Original vision in one page

The original definition described `envlt` as:

- a local-first encrypted environment variable vault
- focused on developers and small teams
- designed to avoid cloud lock-in and subscription dependence
- centered around reproducible `.env` workflows

Core mental model from the original plan:

- source of truth is encrypted vault data
- `.env` files are generated artifacts, not permanent secret stores

## Original roadmap vs current state

| Original roadmap area                   | Current state   |
| --------------------------------------- | --------------- |
| Local encrypted CLI workflow            | Implemented     |
| Project add/list/set/use/run baseline   | Implemented     |
| Export/import encrypted bundles         | Implemented     |
| Variable typing and type-aware behavior | Implemented     |
| Expanded cloud sync features            | Deferred        |
| GUI app (`envlt-bar`)                   | Not implemented |
| Full auth lifecycle extensions          | Partial         |

For detailed implementation status, see [Spec Alignment](spec-alignment.md).

## Notes about outdated sections in the legacy document

Some sections in the original definition include planned crate/module structures that do not match the current workspace exactly. Treat those parts as historical design intent, not current architecture.

Current architecture references:

- [Architecture](architecture.md)
- [CLI Reference](cli-reference.md)
- [Roadmap](roadmap.md)

## How to use the legacy document safely

- use it for product narrative and original goals
- do not use it as the source of truth for current commands, module paths, or release process
- verify all implementation decisions against current docs and code
