.PHONY: help build test fmt clippy check release release-check clean

# Default target
help:
	@echo "envlt development workflow"
	@echo ""
	@echo "Targets:"
	@echo "  make build        - Build release binary"
	@echo "  make test         - Run all tests"
	@echo "  make fmt          - Check formatting"
	@echo "  make clippy       - Run clippy lints"
	@echo "  make check        - Run fmt + clippy + test (quality gates)"
	@echo "  make release-check- Check everything before tagging (recommended)"
	@echo "  make release      - Full release workflow: bump, commit, tag"
	@echo "  make clean        - Clean build artifacts"

# Development
build:
	cargo build --locked --release -p envlt-cli

test:
	cargo test --locked -p envlt-core -p envlt-cli

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --locked --all-targets --all-features -- -D warnings

# Quality gates (runs on CI)
check: fmt clippy test

# Pre-release checklist (run this before creating a tag!)
release-check:
	@echo "=== Step 1: Verify working directory is clean ==="
	@test -z "$$(git status --porcelain)" || (echo "Error: Working directory is not clean. Commit changes first."; exit 1)
	@echo "OK: Working directory is clean"
	@echo ""
	@echo "=== Step 2: Verify we're on main ==="
	@test "$$(git rev-parse --abbrev-ref HEAD)" = "main" || (echo "Error: Not on main branch. Switch to main first."; exit 1)
	@echo "OK: On main branch"
	@echo ""
	@echo "=== Step 3: Check version in Cargo.toml ==="
	@echo "Current version:"
	@grep "^version" Cargo.toml
	@echo ""
	@echo "=== Step 4: Verify Cargo.lock is up to date ==="
	cargo update --workspace --locked
	@echo "OK: Cargo.lock is in sync"
	@echo ""
	@echo "=== Step 5: Run tests ==="
	cargo test --locked -p envlt-core -p envlt-cli
	@echo "OK: All tests pass"
	@echo ""
	@echo "=== Step 6: Run formatting checks ==="
	cargo fmt --all -- --check
	@echo "OK: Formatting is correct"
	@echo ""
	@echo "=== Step 7: Run clippy ==="
	cargo clippy --locked --all-targets --all-features -- -D warnings
	@echo "OK: Clippy passes"
	@echo ""
	@echo "=== Step 8: Check changelog ==="
	@echo "Latest entry in CHANGELOG.md:"
	@head -10 CHANGELOG.md | grep -E '^## \[' || true
	@echo ""
	@echo "========================================"
	@echo "All checks passed! Ready for release."
	@echo "========================================"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Update CHANGELOG.md if not already done"
	@echo "  2. git add -A && git commit -m 'release: vX.Y.Z'"
	@echo "  3. git tag vX.Y.Z"
	@echo "  4. git push origin main && git push origin vX.Y.Z"

# Full release workflow (interactive, for maintainers)
release:
	@echo "This will create a new release. Press Ctrl+C to cancel."
	@sleep 2
	$(MAKE) release-check
	@echo ""
	@echo "Creating release..."
	@echo "Don't forget to update CHANGELOG.md before tagging!"

clean:
	cargo clean
	@echo "Cleaned build artifacts"
