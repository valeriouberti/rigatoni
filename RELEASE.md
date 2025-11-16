# Release Guide

This document describes how to release new versions of Rigatoni to crates.io.

## Prerequisites

### 1. Get a crates.io API Token

1. Log in to [crates.io](https://crates.io/)
2. Go to Account Settings → API Tokens
3. Create a new token with name "GitHub Actions"
4. Copy the token (you won't be able to see it again)

### 2. Add Token to GitHub Secrets

1. Go to your GitHub repository
2. Navigate to Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Name: `CARGO_REGISTRY_TOKEN`
5. Value: Paste your crates.io API token
6. Click "Add secret"

## Release Process

### Manual Release Steps

1. **Update Version Number**

Edit `Cargo.toml` in the workspace root:

```toml
[workspace.package]
version = "0.2.0"  # Update version
```

2. **Update CHANGELOG.md**

Add a new section for the release:

```markdown
## [0.2.0] - 2025-01-20

### Added
- New feature X
- New feature Y

### Changed
- Improved performance of Z

### Fixed
- Fixed bug in component A
```

3. **Commit Changes**

```bash
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"
git push origin main
```

4. **Create and Push Tag**

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push tag to trigger release workflow
git push origin v0.2.0
```

### What Happens Next

The GitHub Actions workflow will automatically:

1. ✅ Run all CI checks (tests, clippy, fmt, etc.)
2. ✅ Verify version in `Cargo.toml` matches the git tag
3. ✅ Publish `rigatoni-core` to crates.io
4. ⏳ Wait 30 seconds for crates.io to index
5. ✅ Publish `rigatoni-destinations` to crates.io
6. ⏳ Wait 30 seconds for crates.io to index
7. ✅ Publish `rigatoni-stores` to crates.io
8. ✅ Create GitHub Release with auto-generated notes

## Release Commands

### Complete Release Workflow

```bash
# 1. Update version in Cargo.toml
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml

# 2. Update CHANGELOG.md (manually edit)
$EDITOR CHANGELOG.md

# 3. Commit version bump
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 0.2.0"

# 4. Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# 5. Push commits and tag
git push origin main
git push origin v0.2.0
```

### Manual Publication (Local)

If you need to publish manually from your local machine:

```bash
# 1. Log in to crates.io
cargo login

# 2. Publish in dependency order
cd rigatoni-core
cargo publish

# Wait 2-3 minutes for crates.io to index

cd ../rigatoni-destinations
cargo publish

# Wait 2-3 minutes for crates.io to index

cd ../rigatoni-stores
cargo publish
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version (1.0.0 → 2.0.0): Breaking changes
- **MINOR** version (0.1.0 → 0.2.0): New features, backwards compatible
- **PATCH** version (0.1.0 → 0.1.1): Bug fixes, backwards compatible

### Examples

```bash
# Bug fix release
v0.1.0 → v0.1.1

# New features (backwards compatible)
v0.1.1 → v0.2.0

# Breaking changes
v0.2.0 → v1.0.0
```

## Troubleshooting

### Version Mismatch Error

**Error:**
```
❌ Version mismatch!
Git tag version (v0.2.0) does not match Cargo.toml version (0.1.0)
```

**Solution:**
```bash
# Update Cargo.toml to match the tag
# OR delete and recreate the tag with correct version
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

### Publish Failed: Already Published

**Error:**
```
error: crate version `0.2.0` is already uploaded
```

**Solution:**

You cannot republish the same version. Increment the version:

```bash
# Bump to next patch version
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0

# Update to v0.2.1
# Edit Cargo.toml: version = "0.2.1"
git add Cargo.toml
git commit -m "chore: bump version to 0.2.1"
git tag -a v0.2.1 -m "Release v0.2.1"
git push origin main
git push origin v0.2.1
```

### Publish Failed: Dependency Not Found

**Error:**
```
error: no matching package named `rigatoni-core` found
```

**Solution:**

Wait longer for crates.io to index the previous crate. The workflow waits 30 seconds, but sometimes you need to wait 2-3 minutes.

Retry the publish:

```bash
cd rigatoni-destinations
cargo publish --token $CARGO_REGISTRY_TOKEN
```

### CI Checks Failed

The release will not proceed if any CI check fails.

**Solution:**

1. Check the failed job in GitHub Actions
2. Fix the issue locally
3. Push the fix to main
4. Wait for CI to pass
5. Create and push the tag again

## Dry Run

Test the publish process without actually publishing:

```bash
# Test each crate
cd rigatoni-core
cargo publish --dry-run

cd ../rigatoni-destinations
cargo publish --dry-run

cd ../rigatoni-stores
cargo publish --dry-run
```

This will verify:
- Package can be built
- All dependencies are available
- Package metadata is valid
- No files are excluded that shouldn't be

## Pre-Release Checklist

Before creating a release tag, ensure:

- [ ] All tests pass locally: `cargo test --workspace --all-features`
- [ ] Clippy has no warnings: `cargo clippy --workspace --all-features -- -D warnings`
- [ ] Code is formatted: `cargo fmt --all -- --check`
- [ ] Documentation builds: `cargo doc --workspace --all-features --no-deps`
- [ ] Version updated in `Cargo.toml`
- [ ] CHANGELOG.md updated with release notes
- [ ] All changes committed to main branch
- [ ] CI passing on main branch

## Post-Release Tasks

After a successful release:

1. **Verify on crates.io**

Visit the crates.io pages:
- https://crates.io/crates/rigatoni-core
- https://crates.io/crates/rigatoni-destinations
- https://crates.io/crates/rigatoni-stores

2. **Check docs.rs**

Verify documentation built successfully:
- https://docs.rs/rigatoni-core
- https://docs.rs/rigatoni-destinations
- https://docs.rs/rigatoni-stores

3. **Announce Release**

- Update project README if needed
- Create announcement in GitHub Discussions
- Tweet/post about the release (if applicable)

## Release Branches

For pre-release versions:

```bash
# Alpha release
git tag -a v0.2.0-alpha.1 -m "Release v0.2.0-alpha.1"

# Beta release
git tag -a v0.2.0-beta.1 -m "Release v0.2.0-beta.1"

# Release candidate
git tag -a v0.2.0-rc.1 -m "Release v0.2.0-rc.1"
```

## Yanking a Release

If you need to yank a problematic release:

```bash
# Yank from crates.io (prevents new projects from using it)
cargo yank --vers 0.2.0 rigatoni-core
cargo yank --vers 0.2.0 rigatoni-destinations
cargo yank --vers 0.2.0 rigatoni-stores

# Unyank if needed
cargo yank --undo --vers 0.2.0 rigatoni-core
```

**Note:** Yanking doesn't delete the release, it just prevents new projects from using it. Existing projects with the version in their `Cargo.lock` can still download it.

## Emergency Hotfix Release

For critical bug fixes on a released version:

```bash
# Create hotfix branch from the tag
git checkout -b hotfix/v0.2.1 v0.2.0

# Make your fix
git add .
git commit -m "fix: critical bug"

# Bump patch version
# Edit Cargo.toml: 0.2.0 → 0.2.1

# Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to 0.2.1"
git tag -a v0.2.1 -m "Hotfix release v0.2.1"

# Push
git push origin hotfix/v0.2.1
git push origin v0.2.1

# Merge back to main
git checkout main
git merge hotfix/v0.2.1
git push origin main
```

## Questions?

For questions about the release process, open an issue or discussion on GitHub.
