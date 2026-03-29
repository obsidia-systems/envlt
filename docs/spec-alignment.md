# Spec Alignment

This document verifies the current implementation against the original project definition.

## High-level conclusion

The current codebase broadly matches and exceeds the original Phase 1 definition, fully covers the intended Phase 2 scope, and now provides a packaging-ready implementation of the currently chosen Phase 3 scope.

It does **not** yet match the complete product vision from the original project definition document.

## Alignment summary

| Original area | Current status | Notes |
| --- | --- | --- |
| Phase 1 local vault workflow | Implemented | Includes extra features beyond the initial MVP |
| Phase 2 export/import bundles | Implemented | Portable `.evlt` bundles are working |
| Phase 3 variable typing | Implemented for the current milestone | `VarType`, inference, `add --from-example`, `diff`, and `gen` are implemented and usable |
| Phase 4 cloud sync | Missing | Intentionally deferred |
| Phase 5 release/distribution | Partial | CI, tagged releases, and Homebrew now exist; signing, notarization, and broader packaging are still missing |
| Phase 6 Keychain | Missing | Planned, not started |
| Phase 7 GUI | Missing | Planned, not started |

## Implemented vs original definition

### Already implemented

- `init`
- `add`
- `list`
- `remove`
- `set`
- `use`
- `run`
- `export`
- `import`
- `vars`
- `diff --example`
- project-to-project `diff`
- `gen`
- `doctor`
- `.envlt-link`
- typed variables
- basic vault backup

### Present in the original definition but still missing

- `cloud link`
- `cloud status`
- `sync`
- conflict resolution for external vaults
- `auth rotate`
- `auth generate`
- `auth status`
- `auth keychain enable`
- `auth keychain disable`
- GUI crate and application
- signed and notarized macOS distribution

## Where the implementation intentionally differs

### Scope ordering

The current implementation chose to:

- advance deeper into local CLI usability
- defer cloud sync
- defer GUI
- defer Keychain

This is a reasonable product decision because it improves the installable local-first story before adding higher-complexity surface area.

### Diff and generation depth

The original definition described richer long-term experiences for diffing and secret generation. The current implementation now has a stable baseline for packaging:

- `gen` is secure by default when storing values
- `diff` is intentionally safe-summary only

What remains in this area is expansion, not baseline completeness:

- more generator presets if the product wants them
- optional richer diff presentation later

## Current feature completeness by area

| Area | Completeness |
| --- | --- |
| Local development workflow | Strong |
| Bundle portability | Strong |
| Diagnostics | Good starting point |
| Secret generation | Strong baseline, not exhaustive |
| Release engineering | In progress |
| Cloud sync | Not started |
| Keychain | Not started |
| GUI | Not started |

## Practical answer

If the question is:

> "Are there no missing features left?"

The practical answer is:

- for the core local CLI, the project is already strong
- for the complete original product vision, several meaningful features are still missing

The next sensible milestone is not adding more major product areas immediately. It is finishing release readiness and hardening what already exists.
