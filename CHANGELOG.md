# Changelog

All notable changes to this project should be documented in this file.

The format is based on Keep a Changelog, and the project intends to follow Semantic Versioning.

## [Unreleased]

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
