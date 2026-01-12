<!--
Constitution for ailoop - Human-in-the-Loop CLI Tool
Version: 1.0.0 (2025-01-27)
Initial creation: Core principles for human-in-the-loop CLI architecture
-->
# ailoop Constitution

## Core Principles

### I. Human-Centric Design (NON-NEGOTIABLE)
ailoop serves as the critical bridge between AI agents and human users. All functionality prioritizes human usability, accessibility, and safety. Interface design follows human-computer interaction best practices: clear prompts, informative feedback, graceful error handling, and consistent user experience. Human input collection must be reliable, secure, and respectful of user time. No automated decisions that affect human workflows without explicit consent.

### II. Test-First Development (NON-NEGOTIABLE)
All code changes require comprehensive testing before implementation. Features are developed using TDD principles: write tests first, ensure they fail, then implement functionality. All tests must pass before PR submission. Test coverage must exceed 80% for new code. Required test types include:
- **Unit tests**: Isolate individual functions and methods
- **Integration tests**: Verify component interactions and workflows
- **CLI tests**: Validate command-line interface functionality
- **Server tests**: Ensure HTTP server reliability and WebSocket communication
- **Channel tests**: Verify channel-based messaging and isolation
- **Human interaction tests**: Validate question/answer flows and authorization patterns
- **Cross-platform tests**: Ensure functionality across Linux, macOS, and Windows

Tests must pass in both debug and release modes. No regressions permitted - existing functionality must remain intact. Binary must be built before running CLI tests as they reference target/debug/ailoop or target/release/ailoop.

### III. Rust-Language Excellence
Maintain consistent high-quality standards across Rust codebases. Rust code follows the official Rust Style Guide with cargo fmt formatting, cargo clippy linting, and 100-character line limits. Code requires descriptive naming and clean, maintainable structure. Use appropriate Rust idioms and leverage type system for safety. Minimize dependencies to maintain small binary size and fast compilation. Async operations use Tokio runtime for non-blocking I/O.

### IV. CLI-First and Server Architecture
Every ailoop capability must be accessible via command-line interface. The CLI exposes functionality through consistent text-based protocols: stdin/args for input, stdout for output, stderr for errors. Server mode provides WebSocket-based real-time communication. Support both human-readable and JSON output formats for all commands. Channel-based messaging ensures proper message routing and isolation.

### V. Channel System Integrity
Channel-based messaging forms the foundation of ailoop's communication model. Channels must provide complete isolation between different workflows and users. Channel names are validated, message routing is reliable, and concurrent access is safely managed. Public channels require explicit security considerations. Channel metadata and history must be properly managed and secured.

### VI. Security and Privacy Standards
Human-in-the-loop systems require exceptional security and privacy protection. All user interactions must be secure, with proper authentication and authorization. Sensitive information must never be logged or exposed. Channel access controls prevent unauthorized message interception. Server endpoints must use secure protocols (HTTPS/WSS) in production. Input validation prevents injection attacks and malformed data.

### VII. Semantic Versioning and Compatibility
All ailoop components follow strict semantic versioning (MAJOR.MINOR.PATCH). Major versions indicate breaking changes to CLI interface or channel protocols, minor versions add functionality, patch versions fix bugs. Channel protocols maintain backward compatibility where possible. Breaking changes require migration guides and deprecation notices with appropriate warning periods.

### VIII. Documentation Discipline
All changes require corresponding documentation updates. User-facing changes update README.md, API changes update protocol documentation, new features include examples. Documentation maintains accuracy with code - outdated docs are treated as bugs. Code must include comprehensive docstrings and comments for maintainability. Command usage and channel behavior must be clearly documented.

## Development Workflow

### Branching and Commit Standards
All changes follow conventional commit format with types: feat, fix, docs, style, refactor, test, chore. Feature branches use descriptive names (feature/channel-auth, fix/server-timeout). Commits must be atomic and well-described. Rebasing required before PR submission to maintain clean history.

### Pull Request Process
PRs require comprehensive review: code standards compliance, test coverage for new features, documentation updates, all tests passing in both debug and release modes. PR checklist verification mandatory. No breaking changes without discussion. CI checks must pass before merge. Security-related changes require additional review for human interaction implications.

### Build and Test Requirements
Rust builds require cargo build (debug) and cargo build --release (production). Testing includes unit tests, integration tests, CLI tests, server tests, channel tests, and human interaction tests. Release mode testing verifies production readiness. Formatting (cargo fmt) and linting (cargo clippy) enforced. Binary size must remain under 5MB for release builds.

## Quality Gates

### Code Standards Enforcement
Maximum 100-character line lengths. No trailing whitespace. Consistent indentation (4 spaces for Rust). Import sorting required. All public APIs documented. Security-sensitive code must include threat modeling comments. Human interaction code must include usability considerations in comments.

### Testing Coverage Requirements
Aim for comprehensive test coverage (>80% for new code). Unit tests isolate individual functions. Integration tests verify component interactions. CLI tests validate command-line functionality. Server and channel tests ensure communication reliability. Human interaction tests validate user experience flows. Test coverage must be maintained or improved with each change.

### Security and Performance Standards
Follow Rust memory safety guarantees. Use secure patterns for all code. Channel communication must be encrypted and authenticated. Input validation required for all user inputs (CLI args, channel messages). No known security vulnerabilities permitted in releases. Secrets must never be logged or exposed in error messages. Performance considerations required for real-time messaging and concurrent channel access.

### Error Handling Standards
Use thiserror for library errors, anyhow for application errors. Error messages must be user-friendly and actionable. Human-facing errors must be clear and provide recovery guidance. Internal errors must not expose sensitive information. Error types must implement std::error::Error and be properly documented. Channel communication errors must be gracefully handled with reconnection logic.

## Channel Architecture Standards

### Channel Registration and Validation
All channels must have valid configuration with security policies. Channel configurations must be validated before creation. Channel access controls must be properly enforced. Channel metadata must include security and compatibility information.

### Channel Communication Protocols
Channels use WebSocket-based communication with message framing. Message serialization follows JSON schema validation. Concurrent access to channels must be safely managed. Message ordering and delivery guarantees must be maintained. Channel disconnection and reconnection must be handled gracefully.

### Channel Security and Access Control
Channel access requires proper authentication and authorization. Public channels have restricted capabilities. Private channels support granular permissions. Channel encryption protects message confidentiality. Audit logging captures security-relevant events without exposing sensitive content.

## Human Interaction Standards

### User Experience Requirements
All user interactions must be intuitive and efficient. Command prompts must be clear and unambiguous. Response collection must handle various input formats. Timeout handling must respect user attention spans. Error recovery must guide users to successful completion.

### Authorization Workflow Standards
Critical actions require explicit authorization with clear context. Authorization requests must include sufficient information for informed decisions. Authorization responses must be securely recorded. Authorization workflows must support delegation and approval chains where appropriate.

### Accessibility and Inclusivity
Interface design must accommodate different user abilities and preferences. Text-based interfaces must support screen readers and accessibility tools. Command completion and validation must assist users in correct usage. Error messages must be understandable by users with varying technical expertise.

## Governance

### Constitution Authority
This constitution supersedes all other development practices and guides project governance. Amendments require documentation, community discussion, and clear rationale. Changes follow semantic versioning with impact assessment. Human-in-the-loop implications must be explicitly considered in all amendments.

### Compliance Verification
All PRs must verify constitution compliance. Reviews check adherence to principles, testing requirements, and quality standards. Human interaction changes require additional scrutiny for user experience and security implications. Complexity additions require justification. Breaking changes need explicit approval and migration plans.

### Amendment Process
Constitution amendments follow: propose change with rationale, community review, implementation, documentation update. Version increments based on change scope (major for principle changes, minor for additions, patch for clarifications). Human impact assessment required for all amendments.

**Version**: 1.0.0 | **Ratified**: 2025-01-27 | **Last Amended**: 2025-01-27