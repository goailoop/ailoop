# Git Hooks Setup

This document describes the git hooks configuration for the ailoop Rust project.

## Overview

This project uses git hooks to enforce code quality standards and ensure consistency across the codebase. The setup combines the [pre-commit framework](https://pre-commit.com) with custom git hooks.

## Current Setup

### Pre-commit Framework

The project uses pre-commit hooks for fast, automated checks before each commit:

- **Framework**: pre-commit (installed via `pip install pre-commit`)
- **Configuration**: `.pre-commit-config.yaml`
- **Installed hooks**:
  - `trailing-whitespace`: Removes trailing whitespace
  - `end-of-file-fixer`: Ensures files end with a newline
  - `check-yaml`: Validates YAML files
  - `check-added-large-files`: Prevents accidentally committing large files
  - `check-merge-conflict`: Checks for merge conflict markers
  - `debug-statements`: Checks for Python debug statements
  - `fmt`: Rust code formatting with rustfmt
  - `cargo-check`: Basic Rust compilation check
  - `clippy`: Rust linting with clippy
  - `test`: Runs Rust unit tests

### Custom Git Hooks

Additional hooks are available in `.githooks/` and can be installed manually:

- **pre-push**: Runs comprehensive checks before pushing (full test suite, documentation build)
- **commit-msg**: Validates conventional commit message format

## Installation

### Automatic Installation

Run the installation script:

```bash
./scripts/install-git-hooks.sh
```

This script will:
- Detect if pre-commit is available and configured
- Install pre-commit hooks if available
- Fall back to copying hooks from `.githooks/` if pre-commit is not available

### Manual Installation

If you prefer manual setup:

```bash
# Install pre-commit hooks
pre-commit install

# Install additional hooks
cp .githooks/commit-msg .git/hooks/
chmod +x .git/hooks/commit-msg
```

## Usage

### Pre-commit Hooks

Pre-commit hooks run automatically before each commit. They will:
- Format your code
- Run linting checks
- Execute unit tests

If any check fails, the commit will be blocked until you fix the issues.

### Commit Message Validation

The commit-msg hook validates that commit messages follow conventional commit format and that commit types match actual file changes:

```
type(scope): description

Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert
```

**Validation includes:**
- **Format validation**: Ensures messages follow `type(scope): description` pattern
- **Content validation**: Ensures commit types match the files being changed
  - `docs` commits must modify documentation files (.md, .rst, .txt, docs/, README)
  - `test` commits must modify test files or test directories
  - `ci` commits must modify CI/CD configuration (.github/, Jenkinsfile, etc.)
  - `feat`/`fix`/`refactor` commits must modify source code files
  - And more specific validations for each commit type

Examples:
- `feat(cli): add new command option`
- `fix(auth): resolve login timeout issue`
- `docs(readme): update installation instructions`

### Bypassing Hooks

In rare cases, you may need to bypass hooks:

```bash
# Skip all hooks
git commit --no-verify

# Skip pre-commit hooks only
SKIP=fmt git commit
```

## Configuration

### Pre-commit Configuration

Edit `.pre-commit-config.yaml` to modify hook behavior:

```yaml
repos:
  - repo: https://github.com/doublify/pre-commit-rust
    rev: v1.0
    hooks:
      - id: fmt
      - id: cargo-check
      - id: clippy
      - id: test
```

### Hook Configuration

- **rustfmt**: Configure in `rustfmt.toml`
- **clippy**: Configure in `.clippy.toml`

## Troubleshooting

### Hooks Not Running

- Ensure hooks are executable: `chmod +x .git/hooks/*`
- Check git version: `git --version`
- Verify hook installation: `ls -la .git/hooks/`

### Pre-commit Issues

- Install pre-commit: `pip install pre-commit`
- Update hooks: `pre-commit autoupdate`
- Clear cache: `pre-commit clean`

### Rust Toolchain Issues

- Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Install components: `rustup component add rustfmt clippy`
- Check installation: `cargo --version`, `rustc --version`

## Best Practices

- Always run `cargo fmt` and `cargo clippy` before committing
- Use descriptive commit messages following conventional commits
- Run full test suite before pushing: `cargo test`
- Keep hooks fast to avoid slowing down development workflow

## Related Files

- `.pre-commit-config.yaml`: Pre-commit framework configuration
- `.githooks/`: Custom hook scripts (fallback)
- `scripts/install-git-hooks.sh`: Installation script
- `rustfmt.toml`: Code formatting configuration
- `.clippy.toml`: Linting configuration
