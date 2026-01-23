# Requirements Document

## Introduction

The OpenDaemon VS Code extension currently has issues where services defined in `dmn.json` are not being displayed in the tree view, and when attempting to start services, users encounter "No services found" errors and timeout errors. This spec addresses the bugs preventing proper service discovery, display, and startup in the extension.

## Requirements

### Requirement 1: Service Discovery and Display

**User Story:** As a developer, I want to see all services from my `dmn.json` file in the VS Code sidebar immediately when the extension activates, so that I can interact with them.

#### Acceptance Criteria

1. WHEN the extension activates and finds a `dmn.json` file THEN it SHALL immediately load and display all services in the tree view
2. WHEN `loadServicesFromConfig()` is called THEN it SHALL successfully parse the `dmn.json` file and populate the tree view with service names
3. WHEN the tree view is populated THEN each service SHALL be displayed with an initial status of "NotStarted"
4. WHEN a user opens the command palette and selects "Start Service" THEN the system SHALL show a list of available services from the tree view
5. IF the tree view has no services THEN the system SHALL display an appropriate message explaining why (e.g., "No dmn.json found" or "dmn.json contains no services")

### Requirement 2: Service Status Synchronization

**User Story:** As a developer, I want the tree view to accurately reflect the current status of my services, so that I know what's running and what's not.

#### Acceptance Criteria

1. WHEN the daemon starts successfully THEN it SHALL send the current status of all services to the extension
2. WHEN the extension receives service status updates THEN it SHALL update the tree view to reflect the new statuses
3. WHEN a service transitions from "Starting" to "Running" THEN the tree view SHALL update the icon and description accordingly
4. WHEN a service fails THEN the tree view SHALL display the failure status with the exit code if available
5. WHEN the daemon is not running THEN the tree view SHALL still display services with "NotStarted" status

### Requirement 3: Ready Condition Timeout Configuration

**User Story:** As a developer, I want services to have adequate time to start up before timing out, so that slow-starting services don't fail unnecessarily.

#### Acceptance Criteria

1. WHEN a service has a `ready_when` condition THEN the system SHALL wait for a reasonable timeout period before declaring failure
2. WHEN the default timeout is too short for a service THEN the user SHALL be able to configure a custom timeout in the `dmn.json` file
3. WHEN a service times out THEN the system SHALL provide a clear error message indicating which service timed out and what condition it was waiting for
4. WHEN a service's ready condition is met before the timeout THEN the system SHALL immediately mark it as ready and proceed
5. IF no timeout is specified THEN the system SHALL use a default timeout of at least 30 seconds

### Requirement 4: Error Reporting and Debugging

**User Story:** As a developer, I want clear error messages when services fail to start or display, so that I can quickly diagnose and fix issues.

#### Acceptance Criteria

1. WHEN the extension fails to load services from `dmn.json` THEN it SHALL display an error message with details about what went wrong
2. WHEN the daemon fails to start THEN the extension SHALL display the daemon's error output in the error display panel
3. WHEN a service fails to meet its ready condition THEN the system SHALL show the last few log lines from that service
4. WHEN the RPC communication between extension and daemon fails THEN the system SHALL log the error and provide troubleshooting guidance
5. WHEN debugging is needed THEN the extension SHALL log key events (config loaded, daemon started, services discovered) to the console

### Requirement 5: Command Palette Integration

**User Story:** As a developer, I want to use the command palette to start, stop, and manage services, so that I have keyboard-driven access to all functionality.

#### Acceptance Criteria

1. WHEN a user opens the command palette and types "OpenDaemon" THEN all available commands SHALL be listed
2. WHEN a user selects "Start All Services" THEN the system SHALL start all services if the tree view has services loaded
3. WHEN a user selects "Start Service" THEN the system SHALL show a quick pick menu with all available services
4. WHEN no services are available THEN the command SHALL display "No services found" with a link to create or check the `dmn.json` file
5. WHEN a command fails THEN the system SHALL display an error notification with actionable next steps
