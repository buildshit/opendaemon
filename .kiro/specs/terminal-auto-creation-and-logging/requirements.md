# Requirements Document

## Introduction

This specification addresses the terminal integration feature in the OpenDaemon VSCode extension. Currently, the `TerminalManager` class exists but is not wired up to automatically create terminals and stream logs when services start. Users must manually right-click and select "Open Terminal" to see logs. This specification defines requirements to enable automatic terminal creation with real-time log streaming, along with comprehensive logging throughout the extension for debugging purposes.

## Glossary

- **Extension**: The OpenDaemon VSCode extension that manages services
- **TerminalManager**: The class responsible for creating and managing service terminals
- **CommandManager**: The class that handles user commands and service operations
- **LogManager**: The class that manages log storage and display in editor tabs
- **Service**: A process defined in dmn.json that the extension manages
- **Terminal**: A VSCode integrated terminal tab that displays service output
- **RPC_Client**: The client that communicates with the OpenDaemon daemon
- **Daemon**: The OpenDaemon background process that manages service lifecycle
- **Output_Channel**: A VSCode output panel for displaying extension logs
- **Log_Line**: A single line of output from a service with timestamp and stream type

## Requirements

### Requirement 1: Automatic Terminal Creation

**User Story:** As a developer, I want terminals to automatically appear when I start services, so that I can immediately see service output without manual steps.

#### Acceptance Criteria

1. WHEN a service starts, THE Extension SHALL create a terminal with the name "dmn: <service-name>"
2. WHEN a terminal is created for a service, THE Extension SHALL make the terminal visible in the terminal panel
3. WHEN multiple services start, THE Extension SHALL create a separate terminal for each service
4. WHEN a service that already has a terminal starts, THE Extension SHALL reuse the existing terminal if it is still open
5. WHEN a terminal is created, THE Extension SHALL not steal focus from the current editor

### Requirement 2: Real-Time Log Streaming

**User Story:** As a developer, I want service logs to stream to terminals in real-time, so that I can monitor service behavior as it happens.

#### Acceptance Criteria

1. WHEN the Daemon sends a 'logLine' notification, THE Extension SHALL write the log content to the service's terminal
2. WHEN a terminal is first opened for a running service, THE Extension SHALL fetch and display historical logs
3. WHEN log lines are written to a terminal, THE Extension SHALL preserve the timestamp and stream type information
4. WHEN a service produces stderr output, THE Extension SHALL display it in the terminal with appropriate formatting
5. WHEN a service produces stdout output, THE Extension SHALL display it in the terminal with appropriate formatting

### Requirement 3: Terminal Lifecycle Management

**User Story:** As a developer, I want terminals to be properly managed throughout the service lifecycle, so that I have a clean and organized workspace.

#### Acceptance Criteria

1. WHEN a service stops, THE Extension SHALL keep the terminal open to preserve log history
2. WHEN a user manually closes a terminal, THE Extension SHALL remove it from the TerminalManager's tracking
3. WHEN a service restarts, THE Extension SHALL clear the terminal before displaying new logs
4. WHEN the extension deactivates, THE Extension SHALL properly dispose of all terminal resources
5. WHEN a terminal is closed and the user opens it again, THE Extension SHALL create a new terminal and fetch recent logs

### Requirement 4: Extension Activity Logging

**User Story:** As a developer troubleshooting extension issues, I want comprehensive activity logs, so that I can understand what the extension is doing and diagnose problems.

#### Acceptance Criteria

1. THE Extension SHALL create an output channel named "OpenDaemon Activity"
2. WHEN a service starts, THE Extension SHALL log the service name and start action to the activity channel
3. WHEN a service stops, THE Extension SHALL log the service name and stop action to the activity channel
4. WHEN a terminal is created, THE Extension SHALL log the service name and terminal creation to the activity channel
5. WHEN a terminal is closed, THE Extension SHALL log the service name and terminal closure to the activity channel
6. WHEN log lines are streamed to a terminal, THE Extension SHALL log the service name and line count to the activity channel
7. WHEN an RPC request is sent, THE Extension SHALL log the method name and parameters to the activity channel
8. WHEN an RPC response is received, THE Extension SHALL log the method name and response status to the activity channel
9. WHEN a daemon notification is received, THE Extension SHALL log the notification type and service name to the activity channel
10. WHEN the extension activates, THE Extension SHALL log the activation event to the activity channel
11. WHEN the extension deactivates, THE Extension SHALL log the deactivation event to the activity channel

### Requirement 5: Error Logging Enhancement

**User Story:** As a developer troubleshooting extension errors, I want detailed error information, so that I can identify and fix issues quickly.

#### Acceptance Criteria

1. WHEN a terminal creation fails, THE Extension SHALL log the error with service name and error details to both activity and error channels
2. WHEN log streaming fails, THE Extension SHALL log the error with service name and error details to both activity and error channels
3. WHEN an RPC call fails, THE Extension SHALL log the error with method name and error details to both activity and error channels
4. WHEN a daemon notification cannot be processed, THE Extension SHALL log the error with notification type and error details to both activity and error channels
5. WHEN historical logs cannot be fetched, THE Extension SHALL log the error with service name and error details to both activity and error channels

### Requirement 6: Integration with Existing Features

**User Story:** As a developer, I want the automatic terminal feature to work seamlessly with existing manual terminal commands, so that I have flexibility in how I view logs.

#### Acceptance Criteria

1. WHEN a user manually opens a terminal via "Open Terminal" command, THE Extension SHALL use the same terminal that was automatically created if it exists
2. WHEN a user manually opens a terminal and no automatic terminal exists, THE Extension SHALL create a new terminal and fetch historical logs
3. WHEN automatic terminal creation is enabled, THE Extension SHALL continue to support the existing "Show Logs" command for editor-based log viewing
4. WHEN a service has both a terminal and an editor log view open, THE Extension SHALL update both views with new log lines
5. WHEN a user clears a terminal manually, THE Extension SHALL not interfere with the terminal state
