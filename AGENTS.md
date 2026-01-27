# AGENTS.md - Development Guidelines for Ailoop

This document provides comprehensive guidelines for agentic coding assistants working on the ailoop project. It covers build commands, testing, code style, and project conventions.

## Project Overview

Ailoop is a human-in-the-loop CLI tool for AI agent communication, implemented as a Rust workspace with Python and TypeScript SDKs. The project uses a multi-language architecture with strict quality controls.

## Build and Development Commands

### Rust Core (ailoop-cli, ailoop-core)

**Build Commands:**
```bash
# Build debug version
cargo build

# Build release version
cargo build --release

# Build specific workspace member
cargo build -p ailoop-cli

# Check compilation without building
cargo check
```

**Test Commands:**
```bash
# Run all tests in workspace
cargo test

# Run tests for specific package
cargo test -p ailoop-cli

# Run single test
cargo test test_name

# Run tests with verbose output
cargo test -- --nocapture

# Run integration tests only
cargo test --test forward_integration_test

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --workspace --out Html
```

**Lint and Format Commands:**
```bash
# Format code (auto-fix)
cargo fmt

# Check formatting (CI)
cargo fmt --check

# Run clippy linter
cargo clippy

# Run clippy with warnings as errors (CI)
cargo clippy -- -D warnings

# Run clippy on all targets and features
cargo clippy --all-targets --all-features -- -D warnings
```

### Python SDK (ailoop-py)

**Setup and Dependencies:**
```bash
cd ailoop-py

# Install dependencies
pip install -r requirements.txt
pip install -r requirements-dev.txt

# Install in development mode
pip install -e .
```

**Test Commands:**
```bash
cd ailoop-py

# Run all tests
python -m pytest tests/

# Run tests with coverage
python -m pytest tests/ -v --cov=ailoop --cov-report=xml

# Run single test file
python -m pytest tests/test_client.py

# Run single test function
python -m pytest tests/test_client.py::test_connect -v
```

**Lint and Type Check Commands:**
```bash
cd ailoop-py

# Type checking
mypy src/ailoop/ --ignore-missing-imports

# Linting
ruff check src/ailoop/

# Auto-fix linting issues
ruff check src/ailoop/ --fix

# Format code
ruff format src/ailoop/
```

### TypeScript SDK (ailoop-js)

**Setup and Dependencies:**
```bash
cd ailoop-js

# Install dependencies
npm install

# Clean install
npm ci
```

**Test Commands:**
```bash
cd ailoop-js

# Run all tests
npm test

# Run tests once (not watching)
npm test -- --watchAll=false

# Run tests with coverage
npm test -- --coverage --watchAll=false

# Run single test file
npm test -- src/client.test.ts
```

**Build and Quality Commands:**
```bash
cd ailoop-js

# Type checking
npm run type-check

# Linting
npm run lint

# Build package
npm run build

# Clean and rebuild
npm run clean && npm run build
```

### Custom Test Scripts

**Comprehensive Test Runner:**
```bash
# Run full test suite with reporting (requires cargo-nextest)
./scripts/run-tests.sh -o test-results.md -j test-results.json

# Check test script help
./scripts/run-tests.sh --help
```

**Version Checking:**
```bash
# Verify all package versions match
./scripts/check-versions.sh
```

## Code Style Guidelines

### Rust Code Style

**Formatting (enforced by rustfmt.toml):**
- Maximum line width: 100 characters
- Use spaces, not tabs (tab_spaces = 4)
- Standard Rust formatting conventions

**Naming Conventions:**
- Functions and variables: `snake_case`
- Types and traits: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

**Import Style:**
```rust
// Group imports by crate, then std, then external
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// Local imports
use crate::models::Message;
use crate::transport::Transport;
```

**Error Handling:**
- Use `anyhow::Result` for application errors
- Use `thiserror::Error` for library crate errors
- Prefer early returns with `?` operator
- Use `bail!` macro for immediate errors

**Async Code:**
- Use `tokio` as the async runtime
- Prefer `async fn` over manual futures
- Use `tokio::spawn` for background tasks
- Use appropriate synchronization primitives (`Arc<Mutex<T>>`, channels, etc.)

**Documentation:**
- Use `//!` for module-level documentation
- Use `///` for public API documentation
- Include examples in doc comments where helpful
- Document error conditions and panics

### Python Code Style

**Type Hints:**
- Use full type hints for all function parameters and return values
- Use `from __future__ import annotations` for forward references
- Use Union types or `|` syntax (Python 3.10+) for multiple types

**Import Style:**
```python
# Standard library imports
import asyncio
import json
from typing import Dict, List, Optional

# Third-party imports
import httpx

# Local imports
from .exceptions import ConnectionError
from .models import Message
```

**Error Handling:**
- Use custom exception classes inheriting from `AiloopError`
- Prefer specific exceptions over generic ones
- Use context managers for resource cleanup
- Log errors appropriately with the `logging` module

**Async Code:**
- Use `asyncio` for asynchronous operations
- Prefer `async with` and `async for` where applicable
- Use appropriate async libraries (`httpx` for HTTP, `websockets` for WebSocket)

**Documentation:**
- Use docstrings for all public functions and classes
- Follow Google/NumPy docstring format
- Include type information in docstrings when not using type hints

### TypeScript Code Style

**Type Safety:**
- Use strict TypeScript configuration
- Avoid `any` type except when interfacing with external APIs
- Define interfaces for all data structures
- Use union types and discriminated unions where appropriate

**Import Style:**
```typescript
// External imports
import axios, { AxiosInstance } from 'axios';
import WebSocket from 'isomorphic-ws';

// Local imports
import { Message, MessageFactory } from './models';
import { ConnectionError, ValidationError } from './types';
```

**Error Handling:**
- Use custom error classes extending `Error`
- Use try/catch with specific error types
- Provide meaningful error messages
- Handle async errors with proper promise rejections

**Async Code:**
- Use `async/await` syntax consistently
- Handle promise rejections appropriately
- Use appropriate async patterns (Promise.all, Promise.race, etc.)

**Documentation:**
- Use JSDoc comments for public APIs
- Include `@param` and `@returns` tags
- Document error conditions and exceptions

## Project Conventions

### General Practices

**Security:**
- Never log sensitive information (passwords, tokens, keys)
- Use environment variables for configuration
- Validate all inputs, especially from external sources
- Use secure defaults (authorization defaults to DENIED)

**Testing:**
- Write unit tests for all public functions
- Use integration tests for complex workflows
- Mock external dependencies in unit tests
- Aim for high test coverage (>80%)

### Database/Persistence
- SQLite MUST NOT be used for any persistence or storage within the ailoop project. This decision has been made to standardize persistence technologies and avoid fragmentation.

**Version Management:**
- Keep versions synchronized across all packages
- Use semantic versioning (MAJOR.MINOR.PATCH)
- Update version in all relevant files when releasing

### Commit Messages

Follow conventional commit format:
```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Test additions/modifications
- `chore`: Maintenance tasks

### Pre-commit Hooks

The project uses pre-commit hooks for quality control:
- Trailing whitespace removal
- End-of-file fixes
- YAML validation
- Large file detection
- Merge conflict detection
- Debug statement detection
- Rust formatting and compilation checks

## Cursor Rules

From `.cursor/rules/specify-rules.mdc`:

**Active Technologies:**
- Rust 1.75+ with Tokio async runtime, clap CLI framework, serde serialization, tokio-tungstenite WebSocket support, uuid for ID generation

**Project Structure:**
```
src/
tests/
```

**Commands:**
- `cargo test` - Run tests
- `cargo clippy` - Run linter

**Code Style:**
- Follow standard Rust conventions
- Constitution requirement: 100-character line limits

## Quality Assurance

### CI Pipeline

The project uses GitHub Actions for continuous integration:
- **verify-versions**: Ensures package versions match across components
- **test-rust**: Runs Rust tests, formatting, and clippy on Ubuntu
- **test-python**: Tests Python SDK on multiple Python versions (3.11, 3.12)
- **test-typescript**: Tests TypeScript SDK on multiple Node versions (16, 18, 20)
- **test-multiplatform**: Cross-platform testing on Ubuntu, macOS, and Windows

### Pre-commit Quality Checks

Before committing, ensure:
1. Code is properly formatted (`cargo fmt`, `ruff format`)
2. Linting passes (`cargo clippy`, `ruff check`, `mypy`)
3. Tests pass (`cargo test`, `pytest`, `npm test`)
4. No large files are accidentally committed
5. No debug statements remain in code

## Development Workflow

1. **Setup**: Clone repository and install dependencies
2. **Development**: Make changes following style guidelines
3. **Testing**: Run relevant tests and ensure CI will pass
4. **Quality**: Format, lint, and check compilation
5. **Commit**: Use conventional commit messages
6. **PR**: Create pull request with description of changes

## Troubleshooting

**Common Issues:**
- **Compilation errors**: Run `cargo check` to identify issues quickly
- **Test failures**: Use `--nocapture` flag to see test output
- **Formatting issues**: Run `cargo fmt` to auto-fix most formatting problems
- **Type errors**: Check with `mypy` (Python) or `npm run type-check` (TypeScript)

**Performance:**
- Use `cargo build --release` for performance testing
- Profile with `cargo flamegraph` if needed
- Check for unnecessary allocations in hot paths

This document should be updated whenever new tools, conventions, or processes are added to the project.
