# Feature Specification: ailoop Forward - Agent Message Streaming and Channeling

**Feature Branch**: `002-ailoop-forward`  
**Created**: 2025-01-27  
**Status**: Draft  
**Input**: User description: "create a specification for ailoop forward new features"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Stream Agent Output to Centralized Server (Priority: P1)

As a developer running headless AI agents in containers, I need to stream agent output to a centralized server so I can monitor all agent activity from a single location.

**Why this priority**: This is the core value proposition - enabling centralized monitoring of multiple AI agents, which is essential for production deployments where agents run in isolated environments.

**Independent Test**: Can be fully tested by piping agent output through the forward command and verifying messages appear in the server, delivering immediate value for agent monitoring.

**Acceptance Scenarios**:

1. **Given** a developer has agent output (e.g., from Cursor CLI), **When** they pipe it through `ailoop forward`, **Then** messages are sent to the ailoop server and appear in the server's channel view.
2. **Given** multiple agents are running simultaneously, **When** each pipes output through forward with different channels, **Then** messages are organized by channel and can be viewed separately.
3. **Given** an agent outputs events in JSONL format, **When** the forward command processes the input, **Then** events are parsed and converted to messages regardless of agent type.

---

### User Story 2 - View Agent Messages in Terminal UI (Priority: P1)

As a system administrator monitoring AI agents, I need to view messages from different channels in the terminal UI so I can track what each agent is doing in real-time.

**Why this priority**: Terminal UI is the primary interface for server operators - without channel switching and formatted display, the feature cannot deliver its core value.

**Independent Test**: Can be fully tested by starting the server, forwarding messages, and verifying they appear in the terminal UI with proper formatting and channel organization.

**Acceptance Scenarios**:

1. **Given** the ailoop server is running with multiple active channels, **When** an administrator views the terminal UI, **Then** they can see a list of all channels and switch between them.
2. **Given** messages arrive in a channel, **When** the administrator is viewing that channel, **Then** messages appear in real-time with formatted display showing agent type, timestamp, and content.
3. **Given** an administrator wants to review message history, **When** they switch to a channel, **Then** they can see recent messages from that channel with proper formatting.

---

### User Story 3 - Monitor Agents via Web Interface (Priority: P2)

As a developer or operator, I need to view agent messages in a web browser so I can monitor agents remotely without terminal access.

**Why this priority**: Web interface enables remote monitoring and provides a more accessible interface for non-technical stakeholders.

**Independent Test**: Can be fully tested by opening the web UI, connecting to the server, and verifying channels and messages are displayed correctly.

**Acceptance Scenarios**:

1. **Given** the ailoop server is running, **When** a user opens the web UI in a browser, **Then** they can see a list of active channels and connect to the server.
2. **Given** a user is viewing a channel in the web UI, **When** new messages arrive, **Then** they appear in real-time without page refresh.
3. **Given** a user wants to review message history, **When** they select a channel, **Then** they can see recent messages with proper formatting and metadata.

---

### User Story 4 - Import Historical Agent Data (Priority: P2)

As a developer testing agent integrations, I need to import historical agent output from files so I can test the system with past data and verify message processing.

**Why this priority**: History import enables testing and debugging without requiring live agent output, improving development workflow.

**Independent Test**: Can be fully tested by importing a JSONL file with agent events and verifying messages are processed and displayed correctly.

**Acceptance Scenarios**:

1. **Given** a developer has a JSONL file with agent events, **When** they import it using the forward command, **Then** messages are processed and sent to the server as if they were live.
2. **Given** historical data is imported, **When** it is processed, **Then** messages maintain their original timestamps and metadata for accurate historical representation.

---

### User Story 5 - Support Multiple Agent Types (Priority: P2)

As a developer using different AI agent frameworks, I need the system to support multiple agent output formats so I can monitor agents from different sources in the same interface.

**Why this priority**: Multi-agent support is essential for real-world deployments where organizations use multiple AI tools and frameworks.

**Independent Test**: Can be fully tested by forwarding output from different agent types and verifying each is parsed correctly and displayed with appropriate agent identification.

**Acceptance Scenarios**:

1. **Given** an agent outputs events in a standard JSONL format with agent type tags, **When** the forward command processes it, **Then** the agent type is identified and preserved in messages.
2. **Given** messages from different agent types, **When** they are displayed, **Then** each message clearly shows which agent type it came from.

---

### Edge Cases

- What happens when agent output contains malformed JSON or invalid event structures? (System skips malformed lines, logs warnings, and continues processing valid input)
- How does the system handle network disconnections between forward command and server? (Messages are buffered during disconnection and sent when connection is restored)
- What happens when the server is not running and forward command attempts to connect? (Forward command retries connection with exponential backoff up to a maximum timeout, then exits with error if server remains unavailable)
- How does the system handle very large message volumes or rapid message bursts? (Oldest messages are evicted when limit reached)
- What happens when a channel name is invalid or contains special characters? (Forward command rejects invalid channel names with clear error message, preventing invalid data from reaching the server)
- How does the system handle duplicate messages or message ordering issues? (System preserves message order by timestamp, detects duplicates by message ID if present, or by content hash + timestamp as fallback. Duplicate messages are skipped to prevent duplicate display while maintaining chronological ordering)
- What happens when the web UI loses connection to the server? (Web UI automatically reconnects and restores previous channel subscriptions, maintaining user's viewing context)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST accept agent output via standard input (stdin) and process it in real-time
- **FR-002**: System MUST support parsing agent output in multiple formats (JSON, JSONL, plain text), skipping malformed lines with logged warnings while continuing to process valid input
- **FR-003**: System MUST convert agent events to standardized message format regardless of source agent type
- **FR-004**: System MUST support forwarding messages via multiple transport mechanisms (WebSocket, file)
- **FR-005**: System MUST organize messages by channel, allowing multiple agents to use different channels, rejecting invalid channel names with clear error messages at the forward command
- **FR-006**: System MUST preserve agent type information in messages for display purposes
- **FR-007**: System MUST store message history per channel for retrieval and display, evicting oldest messages (FIFO) when the maximum limit is reached
- **FR-008**: System MUST allow terminal UI users to switch between channels to view different message streams
- **FR-009**: System MUST format messages for display showing agent type, timestamp, and content
- **FR-010**: System MUST broadcast new messages to connected WebSocket clients in real-time
- **FR-011**: System MUST provide HTTP API endpoints for querying channels and message history
- **FR-012**: System MUST support WebSocket connections for clients to receive real-time message updates, automatically reconnecting and restoring channel subscriptions when connection is lost
- **FR-013**: System MUST allow WebSocket clients to subscribe to specific channels or all channels
- **FR-014**: System MUST support importing historical agent data from files for testing purposes
- **FR-015**: System MUST handle transport errors gracefully without losing message processing capability, buffering messages during network disconnections and sending them when connection is restored
- **FR-019**: System MUST retry server connection with exponential backoff when server is unavailable at startup, exiting with error if connection cannot be established within maximum timeout period
- **FR-016**: System MUST support auto-detection of agent type from output format when not explicitly specified
- **FR-017**: System MUST maintain message metadata including session IDs, client IDs, and timestamps
- **FR-018**: System MUST provide a sample web UI demonstrating WebSocket integration and message display

### Key Entities

- **Message**: Represents a single communication from an agent, containing content, metadata, channel, and timestamp
- **Channel**: A logical grouping mechanism for organizing messages from different agents or workflows
- **Agent Event**: Raw output from an agent that needs to be parsed and converted to a message
- **Transport**: Mechanism for delivering messages from forward command to server (WebSocket, file, etc.)
- **Client Connection**: WebSocket connection from a viewer client to receive real-time message updates

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can forward agent output from stdin to server with messages appearing in server UI within 2 seconds of agent output
- **SC-002**: System can process and display messages from at least 10 concurrent agent streams without performance degradation
- **SC-003**: Terminal UI users can switch between channels and view messages within 1 second of channel selection
- **SC-004**: Web UI users can view all active channels and connect to server within 3 seconds of page load
- **SC-005**: System can import and process historical data files containing at least 1000 messages without errors
- **SC-006**: Messages from different agent types are correctly identified and displayed with agent type information in 100% of cases
- **SC-007**: WebSocket clients receive new messages within 500ms of message arrival at server
- **SC-008**: System maintains message history for at least 1000 messages per channel, automatically evicting oldest messages when limit is exceeded to prevent unbounded growth
- **SC-009**: Forward command handles network disconnections and reconnects automatically, buffering messages during disconnection and successfully delivering them upon reconnection without data loss
- **SC-010**: 95% of agent output formats (JSON, JSONL, text) are successfully parsed and converted to messages

## Assumptions

- Agent output formats follow standard patterns (JSON, JSONL, or plain text)
- Network connectivity between forward command and server is generally available
- Server has sufficient resources to handle expected message volume
- Channel names follow naming conventions (alphanumeric with hyphens/underscores)
- Users have appropriate permissions to access server and view messages
- Web UI will be served over HTTP/HTTPS with WebSocket support
- Historical data files are in valid format matching expected agent output structure

## Dependencies

- Existing ailoop server infrastructure (from feature 001)
- WebSocket support in server
- Message model and channel isolation mechanisms
- Terminal UI framework (already in use)

## Clarifications

### Session 2025-01-27

- Q: When a channel reaches the maximum message history limit (1000 messages), what should the system do with new incoming messages? → A: Evict oldest messages (FIFO) when limit reached
- Q: When the forward command encounters malformed JSON or invalid event structures in agent output, what should it do? → A: Skip malformed lines and continue processing (log warning)
- Q: When the forward command loses connection to the server, how should it handle messages that arrive during the disconnection period? → A: Buffer messages during disconnection, send when reconnected
- Q: When the forward command starts and the server is not running or unreachable, what should the forward command do? → A: Retry with backoff up to maximum timeout, then exit
- Q: When a user provides an invalid channel name (e.g., contains spaces, special characters, or violates length limits), what should the system do? → A: Reject invalid names with clear error message at forward command
- Q: When the web UI loses its WebSocket connection to the server, what should happen? → A: Automatically reconnect and restore channel subscriptions
- Q: How does the system handle duplicate messages or message ordering issues? → A: System preserves message order by timestamp, detects duplicates by message ID if present, or by content hash + timestamp as fallback. Duplicate messages are skipped to prevent duplicate display while maintaining chronological ordering

## Out of Scope

- Authentication and authorization for web UI (assumed to be handled by infrastructure)
- Message persistence beyond in-memory history (future enhancement)
- Advanced message filtering or search capabilities (future enhancement)
- Support for Kafka or Redis transports (future enhancement)
- Custom agent parser implementations beyond Cursor and generic JSONL (future enhancement)
- Message encryption or security features (assumed to be handled by infrastructure)
