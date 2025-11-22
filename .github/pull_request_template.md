## Description

<!-- Provide a clear and concise description of what this PR does -->

## Type of Change

<!-- Mark relevant items with an 'x' -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Test improvement
- [ ] CI/CD improvement

## Related Issues

<!-- Link related issues using keywords: Fixes #123, Closes #456, Related to #789 -->

Fixes #

## Changes Made

<!-- List the key changes introduced by this PR -->

-
-
-

## Testing

<!-- Describe how you tested your changes -->

### Test Coverage

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass locally: `cargo test --workspace --all-features`

### Manual Testing

<!-- Describe manual testing performed -->

```bash
# Example: Steps to test manually
cargo run --example my_example
```

## Performance Impact

<!-- If applicable, describe the performance impact of your changes -->

- [ ] No significant performance impact
- [ ] Performance improved (provide benchmark results)
- [ ] Performance degraded (justify why the change is necessary)

## Documentation

- [ ] Code is documented with inline comments where necessary
- [ ] Public API changes are documented with doc comments
- [ ] User-facing documentation updated (README, guides, etc.)
- [ ] CHANGELOG.md updated with changes
- [ ] Examples updated if API changed

## Code Quality Checklist

<!-- Ensure all checks pass before requesting review -->

- [ ] Code follows project style guidelines: `cargo fmt --all`
- [ ] No new warnings: `cargo clippy --workspace --all-features -- -D warnings`
- [ ] All pre-push checks pass: `./scripts/pre-push.sh`
- [ ] License headers included in new files (Apache 2.0)
- [ ] No sensitive information (credentials, keys) committed

## Breaking Changes

<!-- If this PR introduces breaking changes, describe them and provide migration guide -->

### Migration Guide

<!-- How should users migrate from the old API to the new one? -->

```rust
// Before
old_api_usage();

// After
new_api_usage();
```

## Screenshots/Logs

<!-- If applicable, add screenshots or log output to help explain your changes -->

## Additional Context

<!-- Add any other context about the PR here -->

## Reviewer Notes

<!-- Any specific areas you'd like reviewers to focus on? -->

---

**By submitting this pull request, I confirm that:**

- [ ] I have read the [Contributing Guidelines](../CONTRIBUTING.md)
- [ ] My code follows the project's coding standards
- [ ] I have performed a self-review of my code
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
