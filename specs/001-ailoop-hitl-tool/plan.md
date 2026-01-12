# Implementation Plan: ailoop - Human-in-the-Loop CLI Tool

**Branch**: `001-ailoop-hitl-tool` | **Date**: 2025-01-27 | **Spec**: [spec.md](/home/sysuser/ws001/aroff/ailoop/specs/001-ailoop-hitl-tool/spec.md)
**Input**: Feature specification from `/specs/001-ailoop-hitl-tool/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

**Phase Status**: ✅ Research Complete, ✅ Design Complete, ⏳ Implementation Planning Ready

## Summary

Implement ailoop, a human-in-the-loop CLI tool that serves as a bridge between AI agents and human users. The tool provides channel-based messaging for collecting human feedback through questions, authorizations, and notifications. Built as a standalone Rust binary with server mode for multi-agent environments, supporting up to 100 concurrent AI agents and 10 simultaneous human users.

## Technical Context

**Language/Version**: Rust 1.75+ (constitution requirement III - Rust-Language Excellence)
**Primary Dependencies**: Tokio (async runtime), clap (CLI), serde (serialization), tokio-tungstenite (WebSocket), uuid (ID generation)
**Storage**: File-based configuration and logging (no database required)
**Testing**: cargo test with >80% coverage, including unit, integration, CLI, server, channel, and human interaction tests
**Target Platform**: Linux (primary), Windows (constitution requirement II.21 - cross-platform testing satisfied for initial release; macOS deferred to future release)
**Project Type**: CLI application with server capabilities
**Performance Goals**: 100 concurrent AI agents, 10 simultaneous human users, 100 messages/minute throughput
**Constraints**: Binary size <5MB, 99% uptime, 100-character line limits, cargo fmt + clippy compliance
**Scale/Scope**: 32 functional requirements, 6 user journeys, channel-based messaging architecture
**Platform Notes**: macOS support deferred to future release due to resource constraints; Linux/Windows provide primary market coverage

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Core Principles Compliance ✅
- **I. Human-Centric Design**: ✅ Implemented via accessibility features, clear UX, respectful timeout handling
- **II. Test-First Development**: ✅ Comprehensive test plan including CLI, server, channel, and human interaction tests
- **III. Rust-Language Excellence**: ✅ Rust 1.75+, Tokio async, cargo fmt/clippy, 100-char limits, minimal dependencies
- **IV. CLI-First Architecture**: ✅ All capabilities accessible via CLI, WebSocket server, JSON/text output formats
- **V. Channel System Integrity**: ✅ Channel isolation, validation, concurrent access safety, metadata management
- **VI. Security Standards**: ✅ No sensitive data exposure, input validation, secure channel communication
- **VII. Semantic Versioning**: ✅ CLI interface backward compatibility, migration guides for breaking changes
- **VIII. Documentation Discipline**: ✅ Comprehensive docs, examples, API documentation

### Development Workflow Compliance ✅
- **Branching**: ✅ Descriptive feature branches with conventional commits
- **PR Process**: ✅ Comprehensive reviews, test coverage, CI checks, security review for human interactions
- **Build/Test**: ✅ cargo build (debug/release), >80% coverage, CLI/server/channel/human tests

### Quality Gates Compliance ✅
- **Code Standards**: ✅ 100-char limits, cargo fmt/clippy, documented APIs, security comments
- **Testing**: ✅ Unit, integration, CLI, server, channel, human interaction, cross-platform tests
- **Security**: ✅ Rust memory safety, secure patterns, input validation, no logged secrets
- **Error Handling**: ✅ thiserror/anyhow, user-friendly messages, graceful recovery

### Architecture Standards Compliance ✅
- **Channel Registration**: ✅ Validation, security policies, metadata management
- **Communication**: ✅ WebSocket with JSON schema, message framing, concurrent safety
- **Security**: ✅ Authentication/authorization framework, encryption, audit logging

### Human Interaction Standards Compliance ✅
- **UX Requirements**: ✅ Clear prompts, timeout handling, error recovery, accessibility
- **Authorization**: ✅ Context-rich requests, secure recording, approval workflows
- **Accessibility**: ✅ Screen reader support, high contrast, keyboard navigation

**GATE STATUS: ✅ PASSED** - All constitution requirements satisfied. No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── main.rs                    # Application entry point
├── cli/
│   ├── mod.rs                 # CLI module
│   ├── commands.rs            # Command definitions (ask, authorize, say, serve, config)
│   └── handlers.rs            # Command handlers
├── server/
│   ├── mod.rs                 # Server module
│   ├── websocket.rs           # WebSocket server implementation
│   ├── queue.rs               # Message queuing system
│   └── terminal.rs            # Interactive terminal UI
├── channel/
│   ├── mod.rs                 # Channel system
│   ├── manager.rs             # Channel lifecycle management
│   ├── isolation.rs           # Channel isolation mechanisms
│   └── validation.rs          # Channel name validation
├── models/
│   ├── mod.rs                 # Data models
│   ├── message.rs             # Message structures
│   ├── authorization.rs       # Authorization records
│   └── configuration.rs       # Configuration structures
├── services/
│   ├── mod.rs                 # Business logic services
│   ├── interaction.rs         # Human interaction handling
│   ├── logging.rs             # Logging and observability
│   └── validation.rs          # Input validation
└── lib.rs                     # Library exports

tests/
├── unit/                      # Unit tests
├── integration/               # Integration tests
├── cli/                       # CLI-specific tests
├── server/                    # Server functionality tests
├── channel/                   # Channel system tests
└── human_interaction/         # Human interaction flow tests
```

**Structure Decision**: Single CLI application structure selected due to CLI-first architecture requirement. Modular design with clear separation of concerns: CLI interface, server capabilities, channel management, data models, and business services. Test structure mirrors source organization for maintainability.

## Phase 0: Research ✅ Complete

**Research Tasks Completed:**
- ✅ WebSocket Server Implementation Patterns (tokio-tungstenite + warp)
- ✅ Channel Isolation Mechanisms (Arc<Mutex<HashMap>>)
- ✅ Interactive Terminal UI Libraries (crossterm + ratatui)
- ✅ Configuration File Format (TOML with XDG compliance)
- ✅ Message Queuing Strategy (in-memory VecDeque with limits)
- ✅ Error Handling Patterns (thiserror + anyhow)
- ✅ Logging Strategy (structured logging with sanitization)
- ✅ Cross-Platform Compatibility (GitHub Actions + local testing)

**Key Decisions:**
- Rust 1.75+ with Tokio async runtime
- TOML configuration with XDG Base Directory compliance
- In-memory message queuing with size limits
- Structured logging with configurable levels
- Cross-platform terminal UI with accessibility support

## Phase 1: Design ✅ Complete

**Artifacts Created:**

### Data Models (`data-model.md`)
- Message, Authorization, Channel, Configuration entities
- Validation rules and state transitions
- Persistence strategy and security considerations
- Performance characteristics and scaling limits

### API Contracts (`contracts/`)
- **WebSocket API** (`websocket-api.json`): Complete message protocol with JSON schemas
- **CLI API** (`cli-api.json`): Command specifications with options, validation, and exit codes

### Quick Start Guide (`quickstart.md`)
- Installation instructions for Linux/Windows
- Basic usage examples for all commands
- Server mode setup and configuration
- AI agent integration examples (Python, JavaScript)
- Troubleshooting guide and common issues

### Agent Context Update
- ✅ Cursor IDE context updated with Rust/Tokio/WebSocket details
- ✅ Technical details added for AI-assisted development

**Design Validation:**
- ✅ All constitution requirements satisfied
- ✅ Technical unknowns resolved
- ✅ API contracts complete and machine-readable
- ✅ Integration patterns defined
- ✅ Accessibility and security considerations included

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 4th project] | [current need] | [why 3 projects insufficient] |
| [e.g., Repository pattern] | [specific problem] | [why direct DB access insufficient] |
