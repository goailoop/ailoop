# Tasks: ailoop Forward - Agent Message Streaming and Channeling

**Input**: Design documents from `/specs/002-ailoop-forward/`
**Prerequisites**: spec.md (required for user stories), architecture plan from ailoop_forward.md

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [x] T001 [P] Add `async-trait` dependency to `Cargo.toml`
- [x] T002 [P] Create `src/transport/` directory structure
- [x] T003 [P] Create `src/parser/` directory structure
- [x] T004 [P] Create `examples/web-ui/` directory structure

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T005 [P] Create transport trait in `src/transport/mod.rs` with `Transport` trait definition
- [x] T006 [P] Create transport factory in `src/transport/factory.rs` with `create_transport` function
- [x] T007 [P] Create parser trait in `src/parser/mod.rs` with `AgentParser` trait and `AgentEvent` types
- [x] T008 [P] Create parser factory in `src/parser/mod.rs` with `create_parser` function
- [x] T009 Extend Message model in `src/models/message.rs` - add optional `metadata` field (serde_json::Value)
- [x] T010 Update `src/models/mod.rs` to export new message functionality

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Stream Agent Output to Centralized Server (Priority: P1) 🎯 MVP

**Goal**: Enable developers to stream agent output to centralized server for monitoring

**Independent Test**: Pipe agent output through `ailoop forward` and verify messages appear in server

### Implementation for User Story 1

- [x] T011 [P] [US1] Implement Cursor parser in `src/parser/cursor.rs` - parse stream-json, json, text formats
- [x] T012 [P] [US1] Implement generic JSONL parser in `src/parser/jsonl.rs` - parse any JSONL with agent_type tags
- [x] T013 [US1] Create message converter in `src/cli/message_converter.rs` - convert AgentEvent to Message
- [x] T013a [US1] Preserve message metadata in `src/cli/message_converter.rs` - ensure session_id, client_id, and timestamp are stored in message.metadata field
- [x] T014 [P] [US1] Implement WebSocket transport in `src/transport/websocket.rs` - connect and send messages
- [x] T015 [P] [US1] Implement File transport in `src/transport/file.rs` - support stdin and file I/O
- [x] T016 [US1] Create forward command orchestrator in `src/cli/forward.rs` - parser → converter → transport
- [x] T017 [US1] Add Forward command to CLI in `src/main.rs` - add `Forward` variant to `Commands` enum
- [x] T018 [US1] Add handle_forward function in `src/cli/handlers.rs` - wire up forward command execution
- [x] T019 [US1] Add error handling for malformed input (skip with warnings) in `src/cli/forward.rs`
- [x] T020 [US1] Add channel name validation in `src/cli/handlers.rs` - reject invalid names with error
- [x] T021 [US1] Add connection retry logic with exponential backoff in `src/transport/websocket.rs`
- [x] T022 [US1] Add message buffering during disconnection in `src/transport/websocket.rs`

**Checkpoint**: At this point, User Story 1 should be fully functional - agents can stream output to server

---

## Phase 4: User Story 2 - View Agent Messages in Terminal UI (Priority: P1) 🎯 MVP

**Goal**: Enable administrators to view messages from different channels in terminal UI with channel switching

**Independent Test**: Start server, forward messages, verify they appear in terminal UI with formatting

### Implementation for User Story 2

- [x] T023 [P] [US2] Create message history storage in `src/server/history.rs` - per-channel message storage with FIFO eviction
- [x] T024 [US2] Integrate message history into server in `src/server/server.rs` - store messages as they arrive
- [x] T025 [US2] Enhance terminal UI in `src/server/terminal.rs` - add channel list display
- [x] T026 [US2] Add channel switching functionality in `src/server/terminal.rs` - Tab key to cycle channels
- [x] T027 [US2] Add formatted message display in `src/server/terminal.rs` - show agent type, timestamp, content
- [x] T028 [US2] Add message history display in `src/server/terminal.rs` - show recent messages when switching channels
- [x] T029 [US2] Add real-time message updates in `src/server/terminal.rs` - display new messages as they arrive
- [x] T030 [US2] Update server message processing in `src/server/server.rs` - add messages to history and display

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently - streaming and terminal viewing

---

## Phase 5: User Story 3 - Monitor Agents via Web Interface (Priority: P2)

**Goal**: Enable users to view agent messages in web browser with real-time updates

**Independent Test**: Open web UI, connect to server, verify channels and messages display correctly

### Implementation for User Story 3

- [x] T031 [P] [US3] Create broadcast manager in `src/server/broadcast.rs` - track WebSocket viewer connections
- [x] T032 [US3] Integrate broadcast manager into server in `src/server/server.rs` - broadcast messages to viewers
- [x] T033 [US3] Add HTTP API server in `src/server/api.rs` - REST endpoints for channels and messages
- [x] T034 [US3] Implement GET /api/channels endpoint in `src/server/api.rs` - list all active channels
- [x] T035 [US3] Implement GET /api/channels/{channel}/messages endpoint in `src/server/api.rs` - get message history
- [x] T036 [US3] Implement GET /api/channels/{channel}/stats endpoint in `src/server/api.rs` - get channel statistics
- [x] T037 [US3] Add WebSocket viewer connection handling in `src/server/server.rs` - distinguish Agent vs Viewer connections
- [x] T038 [US3] Add channel subscription protocol in `src/server/broadcast.rs` - handle subscribe/unsubscribe messages
- [x] T039 [US3] Add automatic reconnection logic in `src/server/broadcast.rs` - restore subscriptions on reconnect
- [x] T040 [P] [US3] Create sample web UI HTML in `examples/web-ui/index.html` - basic page structure
- [x] T041 [P] [US3] Create web UI JavaScript client in `examples/web-ui/app.js` - WebSocket connection and message handling
- [x] T042 [P] [US3] Create web UI styles in `examples/web-ui/styles.css` - responsive design
- [x] T043 [US3] Add channel list display in `examples/web-ui/app.js` - fetch and display channels
- [x] T044 [US3] Add message display panel in `examples/web-ui/app.js` - formatted message rendering
- [x] T045 [US3] Add real-time message updates in `examples/web-ui/app.js` - handle WebSocket message events
- [x] T046 [US3] Add channel selection in `examples/web-ui/app.js` - switch between channels
- [x] T047 [US3] Create web UI README in `examples/web-ui/README.md` - usage instructions

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - Import Historical Agent Data (Priority: P2)

**Goal**: Enable developers to import historical agent data from files for testing

**Independent Test**: Import JSONL file with agent events, verify messages processed and displayed correctly

### Implementation for User Story 4

**Note**: This story builds on User Story 1 infrastructure (forward command and file transport)

- [ ] T048 [US4] Add file input support to forward command in `src/cli/forward.rs` - read from file path
- [ ] T049 [US4] Add input source detection in `src/cli/forward.rs` - distinguish stdin vs file input
- [ ] T050 [US4] Preserve original timestamps in message converter in `src/cli/message_converter.rs` - maintain historical accuracy
- [ ] T051 [US4] Add file transport input mode in `src/transport/file.rs` - read from file for import

**Checkpoint**: At this point, historical data import should work alongside live streaming

---

## Phase 7: User Story 5 - Support Multiple Agent Types (Priority: P2)

**Goal**: Enable system to support multiple agent output formats with agent type identification

**Independent Test**: Forward output from different agent types, verify each parsed correctly with agent identification

### Implementation for User Story 5

**Note**: This story builds on User Story 1 infrastructure (parser system and message converter)

- [ ] T052 [US5] Add agent type auto-detection in `src/parser/mod.rs` - detect agent type from output format
- [ ] T053 [US5] Preserve agent type in message metadata in `src/cli/message_converter.rs` - store in message.metadata
- [ ] T054 [US5] Display agent type in formatted messages in `src/server/terminal.rs` - show in message display
- [ ] T055 [US5] Display agent type in web UI in `examples/web-ui/app.js` - show in message rendering

**Checkpoint**: At this point, all user stories should work with multiple agent types

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T056 [P] Add comprehensive error handling and logging across all components
- [ ] T057 [P] Add unit tests for parser implementations in `tests/parser/`
- [ ] T058 [P] Add unit tests for message converter in `tests/cli/message_converter.rs`
- [ ] T059 [P] Add unit tests for transport implementations in `tests/transport/`
- [ ] T060 Add integration tests for forward command in `tests/integration/forward.rs`
- [ ] T061 Add integration tests for server broadcasting in `tests/integration/broadcast.rs`
- [ ] T062 Add integration tests for web UI in `tests/integration/web_ui.rs`
- [ ] T063 [P] Update main README.md with forward command usage examples
- [ ] T064 [P] Add documentation for transport architecture in `docs/transport.md`
- [ ] T065 [P] Add documentation for parser system in `docs/parser.md`
- [ ] T066 Code cleanup and refactoring - remove unused code, improve error messages
- [ ] T067 Performance optimization - verify message processing meets success criteria
- [ ] T068 Validate all success criteria from spec.md are met

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2)
- **Polish (Final Phase)**: Depends on all desired user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Depends on US1 for message flow
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Depends on US1 for message flow, US2 for history
- **User Story 4 (P2)**: Can start after US1 - Builds on forward command infrastructure
- **User Story 5 (P2)**: Can start after US1 - Builds on parser infrastructure

### Within Each User Story

- Models/traits before implementations
- Core functionality before error handling
- Basic features before advanced features
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, US1 and US2 can start (US2 needs US1 messages)
- Parser implementations (T011, T012) can run in parallel
- Transport implementations (T014, T015) can run in parallel
- Web UI files (T040, T041, T042) can be created in parallel
- Test files can be written in parallel

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Stream Agent Output)
4. Complete Phase 4: User Story 2 (Terminal UI)
5. **STOP and VALIDATE**: Test both stories independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Basic streaming!)
3. Add User Story 2 → Test independently → Deploy/Demo (Terminal viewing!)
4. Add User Story 3 → Test independently → Deploy/Demo (Web UI!)
5. Add User Story 4 → Test independently → Deploy/Demo (History import!)
6. Add User Story 5 → Test independently → Deploy/Demo (Multi-agent!)
7. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (forward command, parsers, transports)
   - Developer B: User Story 2 (terminal UI, history) - waits for US1 messages
3. Once US1 and US2 complete:
   - Developer A: User Story 3 (web UI, broadcasting, HTTP API)
   - Developer B: User Story 4 (file import)
   - Developer C: User Story 5 (multi-agent support)
4. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
- Message history uses FIFO eviction when limit (1000) is reached
- Malformed input is skipped with warnings, processing continues
- Network disconnections buffer messages and send on reconnect
- Channel names are validated at forward command with clear error messages
