# Web Command Specification

## Overview
The `web` command provides a web-based user interface for managing ailoop events and interactions. It complements the `serve` command by offering a visual interface for users to view, respond to, and manage various types of events and notifications.

## Functional Requirements

### Core Features

#### 1. Event Dashboard
- **Event Queue Visualization**: Display incoming events in organized queues (questions, approvals, notifications, errors)
- **Real-time Updates**: Automatically refresh event lists as new events arrive
- **Event Categorization**: Group events by type (user questions, system notifications, approval requests, error alerts)
- **Event Status Tracking**: Show processing status for each event (pending, answered, approved, denied, expired)

#### 2. Interactive Event Management
- **Question Answering**: Allow users to respond to questions sent by connected clients
- **Approval Interface**: Provide accept/deny buttons for approval requests
- **Notification Acknowledgment**: Mark notifications as read/unread
- **Bulk Actions**: Support selecting multiple events for batch operations

#### 3. Service Integration
- **Serve Command Compatibility**: Work alongside running `serve` command instances
- **Multi-client Support**: Display events from all connected clients in unified interface
- **Service Status Monitoring**: Show connection status and health of backend services

#### 4. User Interface Components

##### Navigation
- **Queue Tabs**: Separate tabs for different event types (Questions, Approvals, Notifications, System)
- **Search and Filter**: Find events by content, sender, or timestamp
- **Sort Options**: Order events by time, priority, or type

##### Event Details View
- **Full Event Content**: Display complete event information including metadata
- **Response History**: Show previous interactions with the same client/event type
- **Quick Actions**: One-click responses for common actions

##### Response Interface
- **Text Input**: Rich text editor for detailed responses
- **Template Responses**: Pre-defined response templates for common scenarios
- **File Attachments**: Support for attaching files to responses

### User Workflows

#### Scenario 1: Managing Client Questions
1. User starts ailoop serve command to enable service
2. User runs `ailoop web` to open management interface
3. Connected clients send questions that appear in Questions queue
4. User reviews questions, provides answers through web interface
5. Responses are sent back to clients automatically

#### Scenario 2: Approval Workflow
1. System generates approval requests for sensitive operations
2. Requests appear in Approvals queue with details
3. User reviews request details and context
4. User approves or denies with optional comments
5. Decision is communicated back to requesting service

#### Scenario 3: Notification Management
1. Various services send status updates and alerts
2. Notifications appear in real-time dashboard
3. User acknowledges important notifications
4. System tracks acknowledgment status

## Command Line Interface

### Basic Usage
```bash
ailoop web [options]
```

### Options
  -h, --help        show help                                                              [boolean]
  -v, --version     show version number                                                    [boolean]
      --print-logs  print logs to stderr                                                   [boolean]
      --log-level   log level                   [string] [choices: "DEBUG", "INFO", "WARN", "ERROR"]
      --port        port to listen on                                          [number] [default: 0]
      --hostname    hostname to listen on                            [string] [default: "127.0.0.1"]
      --mdns        enable mDNS service discovery (defaults hostname to 0.0.0.0)
                                                                          [boolean] [default: false]
      --cors        additional domains to allow for CORS                       [array] [default: []]
      --serve-url   URL of running serve command instance                      [string]
      --auto-open   automatically open web interface in default browser       [boolean] [default: true]
