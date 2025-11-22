# Main Branch Development Workflow

## Overview

IPPAN now follows a **trunk-based development** workflow where all development occurs directly on the `main` branch. This document outlines the configuration, rationale, and guidelines for this approach.

## Configuration

### Branch Strategy

- **Primary Branch**: `main`
- **Feature Branches**: Not required for most changes
- **Release Strategy**: Direct commits to `main`
- **CI/CD Triggers**: All workflows run on `main` branch only

### CI/CD Configuration

All GitHub Actions workflows have been configured to trigger exclusively on the `main` branch:

- **Build & Test** (`ci.yml`): Runs on every push and PR to `main`
- **AI Determinism** (`ai-determinism.yml`): Validates AI determinism on `main` changes
- **No Float Runtime** (`no-float-runtime.yml`): Ensures no f32/f64 in runtime code on `main`
- **IPPAN Test Suite** (`ippan-test-suite.yml`): Manual trigger for comprehensive testing
- **Nightly Validation** (`nightly-validation.yml`): Automated nightly validation
- **CodeQL Security** (`codeql.yml`): Security analysis on `main` commits
- **Auto Cleanup** (`auto-cleanup.yml`): Scheduled cleanup of old workflow runs

### Removed Branch References

The following branch references have been removed from CI workflows:
- `develop`
- `fix/stabilize-2025-11-08`
- All `cursor/*` branches (used temporarily, now deprecated)

## Development Guidelines

**License Notice:** IPPAN is licensed under the IPPAN Community Source License (source-available with restricted forks/competing networks and commercial use). See `LICENSE.md` for details.

### Making Changes

1. **Work directly on `main`**:
   ```bash
   git checkout main
   git pull origin main
   # Make your changes
   git add .
   git commit -m "descriptive commit message"
   git push origin main
   ```

2. **For Cursor AI Development**:
   - Ensure Cursor is configured to use `main` as the base branch
   - Disable automatic feature branch creation
   - All commits should target `main` directly

3. **Code Quality Gates**:
   - All commits must pass CI checks before merge
   - Tests must pass: `cargo test --workspace`
   - Linting must pass: `cargo clippy --workspace --all-targets`
   - Formatting must be correct: `cargo fmt --all -- --check`
   - No f32/f64 in runtime code (enforced by CI)

### Commit Guidelines

- **Atomic commits**: Each commit should represent a single logical change
- **Descriptive messages**: Use clear, concise commit messages following conventional commits format
- **Test coverage**: Ensure new code has appropriate test coverage
- **Documentation**: Update relevant docs with code changes

### Example Commit Message Format

```
<type>: <description>

[optional body]

[optional footer]
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `docs`: Documentation changes
- `ci`: CI/CD configuration changes
- `chore`: Maintenance tasks

## Rationale

### Why Trunk-Based Development?

1. **Simplicity**: No branch management overhead
2. **Continuous Integration**: All changes are integrated immediately
3. **Faster Feedback**: CI runs on every commit, catching issues early
4. **Single Source of Truth**: `main` always represents the current state
5. **Reduced Merge Conflicts**: Frequent small commits reduce conflict probability

### Safety Mechanisms

1. **Comprehensive CI**: Multiple workflow validation on every commit
2. **Automated Testing**: Full test suite runs on every push
3. **Determinism Checks**: AI and consensus validation ensures reproducibility
4. **Security Scanning**: CodeQL runs regularly on `main`
5. **Nightly Validation**: Comprehensive validation suite runs nightly

### When to Use Feature Branches

Feature branches may still be used for:
- **Major experimental changes**: Large refactors or architectural changes
- **External contributions**: PRs from forks
- **Multi-developer coordination**: When multiple developers need to collaborate on a complex feature

In these cases, create a short-lived feature branch and merge back to `main` as soon as possible.

## Cursor Configuration

### Recommended Settings

For Cursor AI development on this project:

1. **Base Branch**: `main`
2. **Default Branch**: `main`
3. **Auto-create branches**: Disabled
4. **Workspace Settings** (`.cursor/config.yaml`):
   ```yaml
   branch:
     default: main
     base: main
     auto_create: false
   ```

## Emergency Procedures

### Broken Build on Main

If a commit breaks the build on `main`:

1. **Immediate Revert**:
   ```bash
   git revert <commit-hash>
   git push origin main
   ```

2. **Fix Forward**: If revert is not feasible, push a fix immediately:
   ```bash
   # Fix the issue
   git add .
   git commit -m "fix: resolve broken build from <commit-hash>"
   git push origin main
   ```

### CI Failures

1. Check the GitHub Actions tab for failure details
2. Fix locally and verify: `cargo test --workspace`
3. Push the fix to `main`

## Monitoring and Validation

### Continuous Monitoring

- **GitHub Actions**: Monitor workflow runs in real-time
- **Project Status**: Check `PROJECT_STATUS.md` for current health metrics
- **Nightly Reports**: Review nightly validation results

### Key Metrics

The following metrics are tracked automatically:
- Test suite pass rate
- Code coverage percentage
- Build time
- Security scan results
- Determinism validation status

## Migration Notes

### From Feature Branch Workflow

If you're used to feature branches:
- Think of each commit as a mini-PR
- Keep commits small and focused
- Run tests locally before pushing
- Communicate with team about major changes

### Temporary Branches

The `cursor/configure-main-branch-development-workflow-0258` branch was created to implement this workflow. It has been merged into `main` and should not be used for future development.

## Additional Resources

- [CI/CD Guide](.github/CI_CD_GUIDE.md)
- [Developer Guide](docs/DEVELOPER_GUIDE.md)
- [Consensus Documentation](docs/consensus/README.md)
- [AI Determinism](docs/ai/D-GBDT.md)

## Questions?

For questions or issues with this workflow:
1. Check existing documentation in `/docs`
2. Review GitHub Actions workflows in `.github/workflows`
3. Consult the AGENTS.md file for maintainer contacts

---

**Last Updated**: 2025-11-14  
**Status**: Active  
**Owner**: All IPPAN Contributors
