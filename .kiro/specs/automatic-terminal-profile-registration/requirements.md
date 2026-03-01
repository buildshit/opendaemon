# Requirements Document

## Introduction

This specification defines the requirements for automatic terminal profile registration in the OpenDaemon VS Code extension. Currently, the `dmn` CLI command only works when users explicitly use the "OpenDaemon: New Terminal with CLI" command. This feature will enable `dmn` to work automatically in ALL new terminals by registering a custom terminal profile with PATH injection and setting it as the default profile for the workspace.

## Glossary

- **Extension**: The OpenDaemon VS Code extension
- **Terminal_Profile**: A VS Code terminal configuration that defines how terminals are created
- **Terminal_Profile_Provider**: A VS Code API interface that provides terminal creation logic
- **PATH_Injection**: The process of adding the CLI binary path to the terminal's PATH environment variable
- **CLI_Binary**: The `dmn` command-line executable
- **Workspace**: The VS Code workspace where the extension is active
- **Terminal_Interceptor**: The component responsible for managing terminal profile registration
- **CLI_Integration_Manager**: The component that coordinates CLI integration features
- **Default_Profile**: The terminal profile that VS Code uses when creating new terminals

## Requirements

### Requirement 1: Terminal Profile Provider Registration

**User Story:** As a developer, I want the extension to register a custom terminal profile provider, so that all new terminals automatically have access to the `dmn` CLI command.

#### Acceptance Criteria

1. WHEN the extension activates, THE Extension SHALL register a terminal profile provider using the VS Code API
2. WHEN the terminal profile provider is registered, THE Extension SHALL store the registration disposable for cleanup
3. THE Terminal_Profile_Provider SHALL implement the VS Code TerminalProfileProvider interface
4. WHEN provideTerminalProfile is called, THE Terminal_Profile_Provider SHALL return terminal options with PATH injection
5. THE Terminal_Profile_Provider SHALL include the CLI_Binary path in the PATH environment variable

### Requirement 2: Default Profile Configuration

**User Story:** As a developer, I want the custom terminal profile to be set as the default for my workspace, so that I don't need to manually select it when creating terminals.

#### Acceptance Criteria

1. WHEN the terminal profile provider is registered, THE Extension SHALL set it as the default terminal profile for the workspace
2. THE Extension SHALL use workspace-level configuration to avoid affecting user-level settings
3. WHEN a user creates a new terminal using Ctrl+` or the Terminal menu, THE Extension SHALL use the custom profile automatically
4. THE Extension SHALL preserve any existing workspace terminal profile settings

### Requirement 3: Cross-Platform Compatibility

**User Story:** As a developer on any platform, I want the terminal profile registration to work correctly, so that I have a consistent experience regardless of my operating system.

#### Acceptance Criteria

1. WHEN running on Windows, THE Extension SHALL inject the CLI_Binary path using Windows PATH format
2. WHEN running on macOS, THE Extension SHALL inject the CLI_Binary path using Unix PATH format
3. WHEN running on Linux, THE Extension SHALL inject the CLI_Binary path using Unix PATH format
4. THE Extension SHALL detect the platform and apply the correct PATH separator
5. THE Extension SHALL handle platform-specific shell configurations (PowerShell, bash, zsh, etc.)

### Requirement 4: Error Handling and Graceful Degradation

**User Story:** As a developer, I want the extension to handle profile registration errors gracefully, so that the extension remains functional even if profile registration fails.

#### Acceptance Criteria

1. IF terminal profile registration fails, THEN THE Extension SHALL log the error and continue activation
2. IF terminal profile registration fails, THEN THE Extension SHALL fall back to the existing "New Terminal with CLI" command
3. WHEN an error occurs during profile provider execution, THE Extension SHALL log the error details
4. THE Extension SHALL not throw unhandled exceptions during profile registration
5. WHEN profile registration fails, THE Extension SHALL notify the user with a non-intrusive message

### Requirement 5: Resource Cleanup

**User Story:** As a developer, I want the extension to clean up terminal profile registrations when deactivated, so that it doesn't leave behind configuration changes.

#### Acceptance Criteria

1. WHEN the extension deactivates, THE Extension SHALL dispose of the terminal profile provider registration
2. THE Extension SHALL restore the previous default terminal profile configuration
3. THE Extension SHALL not leave orphaned terminal profile providers after deactivation
4. WHEN cleanup fails, THE Extension SHALL log the error without blocking deactivation

### Requirement 6: Backward Compatibility

**User Story:** As a developer who uses the existing "New Terminal with CLI" command, I want it to continue working, so that my workflow is not disrupted.

#### Acceptance Criteria

1. THE Extension SHALL maintain the existing "OpenDaemon: New Terminal with CLI" command
2. WHEN a user invokes the "New Terminal with CLI" command, THE Extension SHALL create a terminal with PATH injection
3. THE Extension SHALL support both automatic profile registration and manual terminal creation
4. WHEN automatic profile registration is disabled, THE Extension SHALL fall back to manual terminal creation

### Requirement 7: CLI Binary Verification

**User Story:** As a developer, I want the extension to verify the CLI binary exists before registering the terminal profile, so that terminals are not created with invalid PATH configurations.

#### Acceptance Criteria

1. WHEN registering the terminal profile provider, THE Extension SHALL verify the CLI_Binary exists
2. IF the CLI_Binary does not exist, THEN THE Extension SHALL not register the terminal profile provider
3. WHEN the CLI_Binary path changes, THE Extension SHALL update the terminal profile provider
4. THE Extension SHALL use the existing binary verification logic from the CLI integration manager

### Requirement 8: Terminal Profile Naming and Identification

**User Story:** As a developer, I want the custom terminal profile to have a clear name, so that I can identify it in the VS Code terminal profile list.

#### Acceptance Criteria

1. THE Terminal_Profile SHALL have the name "OpenDaemon CLI"
2. THE Terminal_Profile SHALL include a description indicating it provides `dmn` command access
3. WHEN viewing available terminal profiles, THE Extension SHALL display the custom profile with its name and description
4. THE Terminal_Profile SHALL use an appropriate icon to distinguish it from other profiles
