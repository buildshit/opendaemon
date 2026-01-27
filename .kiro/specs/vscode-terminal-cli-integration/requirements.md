# Requirements Document

## Introduction

This document specifies the requirements for automatically making the OpenDaemon CLI (`dmn` command) available in VS Code's integrated terminal when the extension is installed. The feature enables users to execute `dmn` commands directly in VS Code terminals without manual PATH configuration, while respecting VS Code's security model that prevents extensions from modifying system-level environment variables.

## Glossary

- **Extension**: The OpenDaemon VS Code extension
- **CLI_Binary**: The platform-specific `dmn` executable bundled with the Extension
- **Terminal**: VS Code's integrated terminal instance
- **Terminal_Environment**: The environment variables available within a Terminal
- **PATH_Injection**: The process of adding the CLI_Binary location to the Terminal_Environment PATH variable
- **Platform_Detector**: Component that identifies the operating system and architecture
- **Binary_Resolver**: Component that determines the correct CLI_Binary path for the current platform
- **Terminal_Interceptor**: Component that intercepts Terminal creation to inject PATH
- **User_Notification**: Visual feedback shown to users about CLI availability
- **Global_Installation**: Optional process to add CLI_Binary to system PATH for use outside VS Code

## Requirements

### Requirement 1: Automatic CLI Availability

**User Story:** As a user, I want the `dmn` command to work immediately in VS Code terminals after installing the extension, so that I can manage services without manual configuration.

#### Acceptance Criteria

1. WHEN a user opens a new Terminal after installing the Extension, THE Terminal_Interceptor SHALL inject the CLI_Binary path into the Terminal_Environment PATH variable
2. WHEN a user types `dmn` followed by any valid command in the Terminal, THE Terminal SHALL execute the CLI_Binary successfully
3. WHEN the Extension activates, THE Binary_Resolver SHALL verify the CLI_Binary exists and has execute permissions
4. WHEN the CLI_Binary is missing or lacks permissions, THE Extension SHALL log an error and display a User_Notification with troubleshooting guidance
5. WHEN PATH_Injection occurs, THE Terminal SHALL preserve all existing PATH entries without modification

### Requirement 2: Cross-Platform Binary Resolution

**User Story:** As a user on any supported platform, I want the correct CLI binary to be used automatically, so that the CLI works regardless of my operating system or architecture.

#### Acceptance Criteria

1. WHEN the Platform_Detector runs on Windows x64, THE Binary_Resolver SHALL select `dmn-win32-x64.exe`
2. WHEN the Platform_Detector runs on macOS ARM64, THE Binary_Resolver SHALL select `dmn-darwin-arm64`
3. WHEN the Platform_Detector runs on macOS x64, THE Binary_Resolver SHALL select `dmn-darwin-x64`
4. WHEN the Platform_Detector runs on Linux x64, THE Binary_Resolver SHALL select `dmn-linux-x64`
5. WHEN the Platform_Detector encounters an unsupported platform, THE Extension SHALL display an error User_Notification listing supported platforms
6. THE Binary_Resolver SHALL construct the full path as `{extensionPath}/bin/{binaryName}`

### Requirement 3: Terminal Environment Injection

**User Story:** As a user, I want the CLI to work in all terminals I create in VS Code, so that I have a consistent experience across terminal instances.

#### Acceptance Criteria

1. WHEN a Terminal is created via the VS Code UI, THE Terminal_Interceptor SHALL inject PATH before the Terminal initializes
2. WHEN a Terminal is created via the Command Palette, THE Terminal_Interceptor SHALL inject PATH before the Terminal initializes
3. WHEN a Terminal is created programmatically by another extension, THE Terminal_Interceptor SHALL inject PATH before the Terminal initializes
4. WHEN PATH_Injection occurs on Windows, THE Terminal_Interceptor SHALL use semicolon (`;`) as the PATH separator
5. WHEN PATH_Injection occurs on Unix-like systems, THE Terminal_Interceptor SHALL use colon (`:`) as the PATH separator
6. WHEN the Extension deactivates, THE Terminal_Interceptor SHALL stop intercepting new Terminal creation

### Requirement 4: User Notification and Guidance

**User Story:** As a user, I want to be informed that the CLI is available and how to use it, so that I can discover and utilize the terminal commands.

#### Acceptance Criteria

1. WHEN the Extension activates for the first time after installation, THE Extension SHALL display a User_Notification explaining CLI availability
2. WHEN the User_Notification is displayed, THE Extension SHALL include an action button to open documentation
3. WHEN the User_Notification is displayed, THE Extension SHALL include an action button to open a new Terminal
4. WHEN the user dismisses the User_Notification, THE Extension SHALL not display it again on subsequent activations
5. THE Extension SHALL provide a command in the Command Palette to manually show the CLI availability notification

### Requirement 5: Global Installation Option

**User Story:** As a user, I want the option to install the CLI globally for use outside VS Code, so that I can use `dmn` commands in any terminal application.

#### Acceptance Criteria

1. THE Extension SHALL provide a Command Palette command "OpenDaemon: Install CLI Globally"
2. WHEN the global installation command is invoked, THE Extension SHALL display instructions for the user's platform
3. WHEN on Windows, THE Extension SHALL provide instructions to add the bin directory to system PATH via System Properties
4. WHEN on Unix-like systems, THE Extension SHALL provide a shell command to copy the binary to `/usr/local/bin` or add to PATH
5. WHEN global installation instructions are shown, THE Extension SHALL include a button to copy the bin directory path to clipboard

### Requirement 6: Binary Verification and Error Handling

**User Story:** As a user, I want clear error messages when the CLI cannot be made available, so that I can troubleshoot or report issues effectively.

#### Acceptance Criteria

1. WHEN the Extension activates, THE Binary_Resolver SHALL check if the CLI_Binary file exists
2. WHEN the CLI_Binary file does not exist, THE Extension SHALL log the expected path and display an error User_Notification
3. WHEN on Unix-like systems, THE Binary_Resolver SHALL verify the CLI_Binary has execute permissions
4. WHEN the CLI_Binary lacks execute permissions, THE Extension SHALL attempt to set execute permissions using `chmod +x`
5. WHEN permission modification fails, THE Extension SHALL display an error User_Notification with manual chmod instructions
6. WHEN binary verification fails, THE Extension SHALL still activate other features (UI, commands) but disable Terminal integration

### Requirement 7: Testing and Validation

**User Story:** As a developer, I want automated tests to verify CLI integration works correctly, so that I can confidently release updates without breaking terminal functionality.

#### Acceptance Criteria

1. THE Extension SHALL include unit tests that verify Binary_Resolver returns correct paths for each supported platform
2. THE Extension SHALL include unit tests that verify PATH_Injection uses correct separators for each platform
3. THE Extension SHALL include integration tests that verify Terminal creation with injected PATH
4. THE Extension SHALL include tests that verify error handling when CLI_Binary is missing
5. THE Extension SHALL include tests that verify User_Notification display logic and dismissal persistence

### Requirement 8: Documentation

**User Story:** As a user, I want clear documentation about the CLI integration, so that I understand how it works and how to troubleshoot issues.

#### Acceptance Criteria

1. THE Extension SHALL provide documentation explaining automatic CLI availability in VS Code terminals
2. THE Extension SHALL provide documentation listing all available `dmn` commands
3. THE Extension SHALL provide troubleshooting documentation for common CLI issues
4. THE Extension SHALL provide documentation explaining the difference between VS Code terminal integration and global installation
5. THE Extension SHALL include documentation about platform-specific binary names and locations
