# Implementation Tasks: ailoop - Human-in-the-Loop CLI Tool

**Feature Branch**: `001-ailoop-hitl-tool`
**Created**: 2025-01-27
**Tasks**: 45 total
**Estimated Duration**: 8-12 weeks (part-time)

## Overview

This task breakdown implements ailoop as a Rust-based CLI tool with WebSocket server capabilities. Tasks are organized by user story to enable independent implementation and testing. Each user story represents a complete, testable feature increment.

**Key Architecture Decisions:**
- CLI-first design using clap
- Tokio async runtime for WebSocket server
- Channel-based message isolation
- TOML configuration with XDG compliance
- In-memory message queuing with size limits

## Phase 1: Setup (Project Initialization)

- [x] T001 Create Rust project structure with Cargo.toml
- [x] T002 Set up core dependencies (tokio, clap, serde, tokio-tungstenite, uuid, crossterm, ratatui)
- [x] T003 Configure cargo build settings for binary size optimization (<5MB)
- [ ] T004 Set up CI/CD pipeline with GitHub Actions for Linux/Windows testing (macOS deferred to future release)
- [x] T005 Create source code directory structure per implementation plan
- [x] T006 Initialize test framework with cargo test configuration
- [x] T007 Set up rustfmt and clippy configuration (100-char line limits)
- [x] T008 Create basic error handling infrastructure (thiserror + anyhow)

## Phase 2: Foundational (Core Infrastructure)

- [x] T009 Implement basic CLI framework with clap in src/cli/mod.rs
- [x] T010 Create core data models (Message, Channel, Authorization) in src/models/
- [x] T011 Implement channel validation and naming rules in src/channel/validation.rs
- [x] T012 Create configuration system with TOML support in src/models/configuration.rs
- [x] T013 Set up logging infrastructure with configurable levels in src/services/logging.rs
- [x] T014 Implement basic error types and handling patterns in src/lib.rs
- [x] T015 Create utility functions for XDG directory handling in src/lib.rs

## Phase 3: User Story 1 - AI Agent Collects Human Decision

**Goal**: Enable AI agents to ask questions and receive human responses
**Independent Test**: Execute `ailoop ask "test question"` and verify response collection
**Priority**: P1 (Core functionality)

- [x] T016 [US1] Implement CLI ask command handler in src/cli/commands.rs
- [x] T017 [US1] Create question message processing in src/services/interaction.rs
- [x] T018 [US1] Implement terminal question display in src/cli/handlers.rs
- [x] T019 [US1] Add response capture and return logic in src/services/interaction.rs
- [x] T020 [US1] Implement timeout handling for question responses in src/services/interaction.rs
- [x] T021 [US1] Add cancellation (Ctrl+C) handling in src/cli/handlers.rs
- [ ] T022 [US1] Create unit tests for ask command in tests/cli/ask_test.rs
- [ ] T023 [US1] Implement integration tests for question flow in tests/integration/question_flow_test.rs

## Phase 4: User Story 2 - AI Agent Requests Authorization

**Goal**: Enable AI agents to request and receive human authorization decisions
**Independent Test**: Execute `ailoop authorize "test action"` and verify approval/denial handling
**Priority**: P1 (Safety critical)

- [x] T024 [US2] Implement CLI authorize command handler in src/cli/commands.rs
- [x] T025 [US2] Create authorization message processing in src/services/interaction.rs
- [x] T026 [US2] Implement terminal authorization display in src/cli/handlers.rs
- [x] T027 [US2] Add authorization decision recording in src/models/authorization.rs
- [x] T028 [US2] Implement authorization timeout with denial default in src/services/interaction.rs
- [x] T029 [US2] Add invalid response validation and prompting in src/services/interaction.rs
- [ ] T030 [US2] Create unit tests for authorize command in tests/cli/authorize_test.rs
- [ ] T031 [US2] Implement integration tests for authorization flow in tests/integration/authorization_flow_test.rs

## Phase 5: User Story 3 - AI Agent Sends Notifications

**Goal**: Enable AI agents to send informational messages to humans
**Independent Test**: Execute `ailoop say "test message"` and verify message display
**Priority**: P2 (User feedback)

- [x] T032 [US3] Implement CLI say command handler in src/cli/commands.rs
- [x] T033 [US3] Create notification message processing in src/services/interaction.rs
- [x] T034 [US3] Add priority level support for notifications in src/models/message.rs
- [x] T035 [US3] Implement terminal notification display in src/cli/handlers.rs
- [ ] T036 [US3] Create unit tests for say command in tests/cli/say_test.rs

## Phase 6: User Story 4 - System Admin Deploys Server

**Goal**: Enable administrators to run a persistent server for multi-agent communication
**Independent Test**: Execute `ailoop serve` and verify server starts with terminal UI
**Priority**: P2 (Multi-agent support)

- [x] T037 [US4] Implement CLI serve command handler in src/cli/commands.rs
- [x] T038 [US4] Create WebSocket server with tokio-tungstenite in src/server/websocket.rs
- [x] T039 [US4] Implement interactive terminal UI with ratatui in src/server/terminal.rs
- [x] T040 [US4] Add message queuing system in src/server/queue.rs
- [x] T041 [US4] Implement channel isolation for server mode in src/channel/manager.rs
- [x] T042 [US4] Add server startup validation and error handling in src/server/mod.rs
- [ ] T043 [US4] Create server integration tests in tests/integration/server_test.rs

## Phase 7: User Story 5 - Developer Integrates ailoop

**Goal**: Enable developers to integrate ailoop commands into AI agent codebases
**Independent Test**: Execute ailoop commands programmatically and verify integration works
**Priority**: P2 (Adoption enablement)

- [ ] T044 [US5] Implement JSON output format for all commands in src/cli/handlers.rs
- [ ] T045 [US5] Add server mode support (--server flag) to all commands in src/cli/commands.rs
- [ ] T046 [US5] Create integration examples in quickstart.md (Python, JavaScript)
- [ ] T047 [US5] Implement channel specification support in all commands in src/cli/commands.rs
- [ ] T048 [US5] Add timeout configuration support in src/models/configuration.rs
- [ ] T049 [US5] Create integration tests for programmatic usage in tests/integration/api_test.rs

## Phase 8: User Story 6 - Admin Configures System

**Goal**: Enable administrators to customize ailoop behavior through configuration
**Independent Test**: Execute `ailoop config --init` and verify configuration creation
**Priority**: P3 (Deployment readiness)

- [x] T050 [US6] Implement CLI config --init command handler in src/cli/commands.rs
- [x] T051 [US6] Create interactive configuration prompts in src/cli/handlers.rs
- [x] T052 [US6] Add configuration validation logic in src/models/configuration.rs
- [x] T053 [US6] Implement TOML file creation and writing in src/services/config.rs
- [ ] T054 [US6] Add configuration testing capability in src/cli/commands.rs
- [ ] T055 [US6] Create configuration unit tests in tests/unit/config_test.rs

## Final Phase: Polish & Cross-Cutting Concerns

- [ ] T056 Implement accessibility features (screen reader support) in src/cli/handlers.rs
- [ ] T057 Add comprehensive logging with timestamps in src/services/logging.rs
- [ ] T058 Implement log level configuration in src/models/configuration.rs
- [ ] T059 Add performance monitoring and metrics in src/lib.rs
- [ ] T060 Create comprehensive documentation updates in README.md
- [ ] T061 Implement final security audit and penetration testing
- [ ] T062 Add cross-platform compatibility testing for Windows (macOS deferred to future release)
- [ ] T063 Create performance benchmarks and optimization
- [ ] T064 Implement final integration testing across all user stories
- [ ] T065 Prepare release artifacts and deployment documentation

## Dependencies & Execution Order

### User Story Dependencies
- US1 (P1): No dependencies - can be implemented first
- US2 (P1): No dependencies - can be implemented in parallel with US1
- US3 (P2): Depends on US1/US2 (shared CLI infrastructure)
- US4 (P2): Depends on US1/US2/US3 (server needs message types)
- US5 (P2): Depends on US1/US2/US3/US4 (integration needs all features)
- US6 (P3): Depends on US1/US2/US3/US4/US5 (config affects all features)

### Parallel Execution Opportunities
- **Phase 1-2**: Fully sequential (setup and foundational work)
- **Phase 3**: US1 and US2 can be developed in parallel (separate commands)
- **Phase 4**: US3, US4, US5 can be developed in parallel (different concerns)
- **Phase 5**: US6 can be developed independently (configuration system)
- **Final Phase**: All tasks can be parallelized (polish work)

### Critical Path
1. Phase 1-2: 2 weeks (setup and core infrastructure)
2. Phase 3: 2 weeks (US1 + US2 in parallel)
3. Phase 4: 2 weeks (US3 + US4 + US5 in parallel)
4. Phase 5: 1 week (US6 configuration)
5. Final Phase: 1-2 weeks (polish and testing)

## MVP Scope Recommendation

**Recommended MVP**: Complete Phase 1-3 (Setup + US1 + US2)
- **Duration**: ~4 weeks
- **Deliverable**: Core ask and authorize functionality
- **Value**: Essential human-in-the-loop capabilities for AI safety
- **Testable**: Independent command execution and response handling

## Quality Gates

- **Unit Test Coverage**: >80% for all new code
- **Integration Tests**: Pass for all user stories
- **Performance**: <5MB binary size, <1 second response times
- **Security**: No sensitive data exposure, input validation
- **Accessibility**: Screen reader compatible
- **Cross-Platform**: Works on Linux and Windows (macOS deferred to future release, constitution requirement II.21 partially satisfied)

## Implementation Strategy

**MVP First**: Focus on core ask/authorize functionality before advanced features
**Incremental Delivery**: Each user story is independently deployable
**Test-Driven**: Write tests before implementation where possible
**Documentation**: Update docs with each completed user story
**Security First**: Implement security measures early and test thoroughly

---

*Total Tasks: 65 | Setup: 8 | Foundational: 7 | User Stories: 42 | Polish: 8*

*This task breakdown ensures systematic implementation with clear dependencies, parallel execution opportunities, and measurable quality gates.*
