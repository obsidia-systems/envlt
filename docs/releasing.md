# Releasing

This document describes the current release process for `envlt`.

## What you need before creating a release

You need five things:

| Item | Why it matters |
| --- | --- |
| Release version | Must match the version you want to publish, for example `0.1.0` |
| Git tag | Triggers the release workflow, for example `v0.1.0` |
| Updated changelog | Gives users and package maintainers a stable change summary |
| Public GitHub repository | Required for release assets and Homebrew consumption |
| Passing CI on `main` | Reduces the chance of shipping broken artifacts |

## Information you should prepare

Before the first public release, decide and record:

- the version number, for example `0.1.0`
- the exact Git tag, for example `v0.1.0`
- the short release title, for example `envlt v0.1.0`
- the release notes summary
- whether the release is intended as:
  - initial preview
  - alpha
  - beta
  - stable

For Homebrew later, you will also need:

- the final asset URLs from GitHub Releases
- SHA256 checksums for each archive
- the repository name for the tap

## Current workflow behavior

The repository already contains:

- CI workflow: `.github/workflows/ci.yml`
- Release workflow: `.github/workflows/release.yml`

The release workflow currently:

- runs on tags matching `v*`
- builds explicit per-architecture CLI release binaries
- creates `.tar.gz` archives
- generates `.sha256` checksum files
- uploads workflow artifacts
- uploads release assets to GitHub Releases

## What users can do today

With the current GitHub release assets, users can:

- download the appropriate `.tar.gz` archive from GitHub Releases
- extract the `envlt` binary
- move it into a directory on their `PATH`
- run `envlt --help`

Example manual install flow on Linux:

```bash
tar -xzf envlt-linux-x86_64.tar.gz
chmod +x envlt
sudo mv envlt /usr/local/bin/envlt
envlt --help
```

This means the project already supports manual binary distribution, even though Homebrew and native Linux package-manager installs are not implemented yet.

On macOS, users may also need to remove the quarantine attribute from a trusted downloaded binary before first execution:

```bash
xattr -d com.apple.quarantine ./envlt
```

## Current architecture coverage

The current release workflow now defines an explicit target matrix:

- `envlt-linux-x86_64.tar.gz`
- `envlt-linux-aarch64.tar.gz`
- `envlt-macos-x86_64.tar.gz`
- `envlt-macos-aarch64.tar.gz`

This makes the download surface suitable for both manual installation and future Homebrew formulas.

## Current macOS limitation

The project does not yet ship signed or notarized macOS artifacts.

That means:

- Apple Gatekeeper may block first launch for browser-downloaded binaries
- users may need to remove quarantine manually for trusted binaries
- signing and notarization should be added before calling the macOS packaging story fully polished

## Release checklist

### 1. Prepare the repository state

- make sure `main` is green in CI
- make sure `CHANGELOG.md` reflects the release contents
- confirm `README.md` and docs are aligned with the release

### 2. Confirm the version

Current workspace version is defined in `Cargo.toml`.

Before tagging, ensure:

- the workspace version is the version you want to release
- the changelog matches that version

### 3. Create and push the tag

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers the release workflow.

If you already created the tag locally and then changed release-facing files such as `CHANGELOG.md`, recreate the tag before pushing it:

```bash
git tag -d v0.1.0
git tag v0.1.0
git push origin main
git push origin v0.1.0
```

### 4. Validate the GitHub release

After the workflow finishes, verify:

- release assets were uploaded
- each archive has a matching `.sha256` file
- artifact names are correct
- the generated GitHub release notes are acceptable

### 5. Capture Homebrew inputs

For the future formula, collect:

- archive URL
- checksum from the `.sha256` file
- exact binary name inside the archive

## Recommended first release shape

For the first public release:

- tag: `v0.1.0`
- release title: `envlt v0.1.0`
- release type: pre-Homebrew public baseline

Suggested positioning:

- local-first encrypted environment vault for development workflows
- core CLI ready
- packaging and Homebrew integration next

## What still remains after the first release

The release workflow is a baseline, not the final packaging story.

Still pending after the first successful tagged release:

- refine artifact naming if needed
- decide Homebrew tap structure
- create the Homebrew formula
- add macOS signing and notarization
- optionally expand platform matrix later
