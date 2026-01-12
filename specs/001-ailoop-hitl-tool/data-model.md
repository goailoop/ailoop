# Data Model: ailoop

## Overview
Data structures and relationships for the ailoop human-in-the-loop CLI tool. All models are designed for in-memory operation with optional file-based persistence for configuration and logging.

## Core Entities

### Message
Represents a communication unit between AI agents and humans.

**Fields:**
- `id: Uuid` - Unique message identifier
- `channel: String` - Channel name (validated, lowercase, 1-64 chars)
- `sender_type: SenderType` - AGENT or HUMAN
- `content: MessageContent` - Message payload
- `timestamp: DateTime<Utc>` - Creation timestamp
- `correlation_id: Option<Uuid>` - Links related messages

**Validation Rules:**
- Channel name matches pattern: `^[a-z0-9][a-z0-9_-]{0,63}$`
- Content size limited to 10KB
- Timestamp always in UTC

**Relationships:**
- Belongs to Channel
- May reference Authorization (for authorization messages)

### Authorization
Records human approval decisions with full audit trail.

**Fields:**
- `id: Uuid` - Unique authorization identifier
- `channel: String` - Channel context
- `action: String` - Description of action requiring approval
- `requester: String` - AI agent identifier
- `decision: AuthorizationDecision` - APPROVED, DENIED, TIMEOUT
- `human_user: Option<String>` - Human who made decision (if applicable)
- `request_timestamp: DateTime<Utc>` - When authorization was requested
- `decision_timestamp: DateTime<Utc>` - When decision was made
- `metadata: HashMap<String, Value>` - Additional context

**Validation Rules:**
- Action description required and non-empty
- Decision timestamp must be after request timestamp
- Metadata size limited to 1KB

**State Transitions:**
- REQUESTED → APPROVED (human approval)
- REQUESTED → DENIED (human denial)
- REQUESTED → TIMEOUT (no timely response)

### Channel
Represents an isolated communication pathway.

**Fields:**
- `name: String` - Channel identifier (primary key)
- `created_at: DateTime<Utc>` - Creation timestamp
- `metadata: ChannelMetadata` - Configuration and properties
- `message_queue: VecDeque<Message>` - Pending messages (FIFO)
- `active_connections: HashSet<ConnectionId>` - Active WebSocket connections

**Validation Rules:**
- Name matches channel naming convention
- Maximum 1000 queued messages per channel
- Connection limit: 10 per channel

**Relationships:**
- Has many Messages
- Has many Authorizations
- Referenced by Configuration (default channel settings)

### Configuration
User-defined settings controlling application behavior.

**Fields:**
- `timeout_seconds: Option<u32>` - Default timeout (0 = disabled)
- `default_channel: String` - Default channel name
- `log_level: LogLevel` - Logging verbosity
- `server_host: String` - Server bind address
- `server_port: u16` - Server port number
- `max_connections: u32` - Maximum concurrent connections
- `max_message_size: usize` - Maximum message size in bytes

**Validation Rules:**
- Timeout between 0-3600 seconds
- Port between 1024-65535
- Max connections ≤ 1000
- Message size ≤ 10KB

**Relationships:**
- Referenced by Channel (for default channel)

## Data Flow

### Message Processing Flow
1. AI Agent creates Message with channel and content
2. Message validated against channel rules and size limits
3. Message added to channel's queue (FIFO)
4. Server broadcasts to all active human connections on channel
5. Human response creates new Message with correlation_id
6. Response routed back to requesting AI agent

### Authorization Flow
1. AI Agent requests authorization via Message
2. Authorization record created with REQUESTED state
3. Human decision updates authorization state
4. Authorization outcome communicated back to AI agent
5. Authorization record persisted for audit trail

### Channel Management Flow
1. Channel created on-demand when first message sent
2. Channel metadata initialized with defaults
3. Messages queued until human connections available
4. Channel automatically cleaned up when inactive
5. Channel validation enforced on all operations

## Persistence Strategy

### Configuration
- Stored as TOML file in XDG-compliant location
- Loaded at startup, reloaded on SIGHUP
- Defaults applied for missing values

### Runtime State
- Messages: In-memory only (no persistence)
- Authorizations: In-memory with optional file logging
- Channels: In-memory with automatic cleanup

### Logging
- Structured JSON logs to configurable file location
- Log levels: ERROR, WARN, INFO, DEBUG, TRACE
- Sensitive data automatically sanitized
- Rotation based on size/time limits

## Security Considerations

### Data Sanitization
- All user inputs validated against injection patterns
- Sensitive data automatically masked in logs
- Channel isolation prevents cross-channel data access

### Access Control
- No authentication required (per requirements)
- Channel-based isolation as primary security boundary
- Connection validation prevents unauthorized WebSocket access

### Audit Trail
- All authorizations logged with timestamps
- Message metadata preserved for debugging
- No sensitive content logged

## Performance Characteristics

### Memory Usage
- Base footprint: ~5MB
- Per channel: ~1KB
- Per message: ~2KB
- Per connection: ~8KB

### Scaling Limits
- Maximum channels: 1000
- Maximum messages per channel: 1000
- Maximum connections: 1000
- Message throughput: 1000/second

### Resource Management
- Automatic cleanup of inactive channels
- Message queue size limits prevent memory exhaustion
- Connection timeouts prevent resource leaks