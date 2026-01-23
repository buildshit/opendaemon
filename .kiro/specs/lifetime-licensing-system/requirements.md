# Requirements Document

## Introduction

The lifetime licensing system enables users to install the OpenDaemon VS Code extension for free from the marketplace while requiring a lifetime license purchase to unlock full functionality. The system provides a seamless authentication flow that connects a web application for license management with local IDE token validation.

## Glossary

- **Extension**: The OpenDaemon VS Code extension installed from the marketplace
- **Webapp**: Web application for account creation and license purchase
- **CLI_Tool**: The `dmn` binary command-line interface
- **License_Token**: Cryptographically signed token proving license ownership
- **Auth_Flow**: Browser-based authentication process connecting webapp to local IDE
- **License_Server**: Backend service managing license validation and token generation

## Requirements

### Requirement 1: Free Extension Installation

**User Story:** As a developer, I want to install the OpenDaemon extension from the VS Code marketplace for free, so that I can evaluate the basic functionality before purchasing.

#### Acceptance Criteria

1. THE Extension SHALL be available for free download from the VS Code marketplace
2. WHEN a user installs the extension, THE Extension SHALL provide basic functionality without requiring immediate payment
3. WHEN the extension runs with basic functionality, THE Extension SHALL display licensing prompts to inform users about full features
4. THE Extension SHALL clearly indicate which features require a license and which are available in the free tier

### Requirement 2: License Purchase and Account Management

**User Story:** As a user, I want to create an account and purchase a lifetime license through a web application, so that I can unlock the full functionality of the extension.

#### Acceptance Criteria

1. THE Webapp SHALL provide user account creation functionality
2. THE Webapp SHALL process lifetime license purchases securely
3. WHEN a user completes a purchase, THE License_Server SHALL generate a valid license associated with their account
4. THE Webapp SHALL provide a user dashboard showing license status and purchase history
5. THE License_Server SHALL maintain persistent records of all license purchases

### Requirement 3: CLI Authentication Command

**User Story:** As a user, I want to authenticate my license through a terminal command, so that I can easily connect my purchased license to my local development environment.

#### Acceptance Criteria

1. THE CLI_Tool SHALL provide an authentication command that initiates the license validation process
2. WHEN the authentication command is executed, THE CLI_Tool SHALL open the user's default browser to the webapp
3. THE CLI_Tool SHALL generate a unique session identifier for the authentication request
4. THE CLI_Tool SHALL wait for the authentication flow to complete before proceeding
5. IF the browser cannot be opened, THEN THE CLI_Tool SHALL provide alternative authentication instructions

### Requirement 4: Secure Token Exchange

**User Story:** As a system administrator, I want secure token exchange between the webapp and local IDE, so that license validation cannot be compromised or forged.

#### Acceptance Criteria

1. THE Webapp SHALL validate user credentials and license status before issuing tokens
2. WHEN authentication is successful, THE License_Server SHALL generate a cryptographically signed License_Token
3. THE License_Token SHALL contain user identification, license expiration (if applicable), and feature permissions
4. THE Auth_Flow SHALL securely transfer the License_Token from webapp to the local CLI_Tool
5. THE License_Token SHALL use industry-standard cryptographic signatures to prevent forgery
6. THE License_Server SHALL maintain a revocation list for invalidated tokens

### Requirement 5: Local Token Storage and Validation

**User Story:** As a user, I want my license token stored securely on my local machine, so that I don't need to re-authenticate frequently while maintaining security.

#### Acceptance Criteria

1. THE CLI_Tool SHALL store the License_Token in a secure local location appropriate for the operating system
2. THE Extension SHALL validate the stored License_Token on startup
3. WHEN validating tokens, THE Extension SHALL verify cryptographic signatures without requiring network access
4. THE Extension SHALL handle token storage consistently across Windows, macOS, and Linux platforms
5. IF a token is invalid or corrupted, THEN THE Extension SHALL gracefully fall back to basic functionality
6. THE Extension SHALL provide a mechanism to clear stored tokens for troubleshooting

### Requirement 6: License Status Checking

**User Story:** As a user, I want the extension to automatically check my license status, so that I can access full functionality immediately after authentication without manual intervention.

#### Acceptance Criteria

1. THE Extension SHALL check license status during VS Code startup
2. WHEN a valid License_Token is found, THE Extension SHALL unlock all licensed features immediately
3. WHEN no valid token is found, THE Extension SHALL operate in basic mode with appropriate user notifications
4. THE Extension SHALL provide visual indicators of current license status in the VS Code interface
5. THE Extension SHALL periodically validate token integrity during operation
6. IF token validation fails during operation, THEN THE Extension SHALL notify the user and provide re-authentication options

### Requirement 7: Error Handling and Recovery

**User Story:** As a user, I want graceful handling of authentication errors, so that I can resolve licensing issues without losing productivity.

#### Acceptance Criteria

1. WHEN authentication fails, THE System SHALL provide clear error messages indicating the specific problem
2. WHEN network connectivity is unavailable during authentication, THE System SHALL provide offline guidance
3. IF a License_Token expires or becomes invalid, THEN THE Extension SHALL notify the user and provide re-authentication options
4. THE System SHALL handle browser compatibility issues gracefully during the Auth_Flow
5. WHEN token storage fails, THE System SHALL provide alternative authentication methods
6. THE System SHALL log authentication errors for troubleshooting while protecting sensitive information

### Requirement 8: Cross-Platform Compatibility

**User Story:** As a developer working across multiple operating systems, I want the licensing system to work consistently on Windows, macOS, and Linux, so that I can use the same license across all my development environments.

#### Acceptance Criteria

1. THE CLI_Tool SHALL execute authentication commands consistently across Windows, macOS, and Linux
2. THE Extension SHALL store and retrieve License_Tokens using platform-appropriate secure storage mechanisms
3. THE Auth_Flow SHALL handle browser launching correctly on all supported platforms
4. THE System SHALL use cross-platform compatible file paths and system calls
5. THE License_Token format SHALL be platform-independent to enable license portability
6. THE System SHALL handle platform-specific security requirements for token storage

### Requirement 9: Integration with Existing Architecture

**User Story:** As a system maintainer, I want the licensing system to integrate seamlessly with the existing Rust core and TypeScript extension architecture, so that implementation is efficient and maintainable.

#### Acceptance Criteria

1. THE licensing system SHALL extend the existing `pro/src/auth.rs` module without breaking current functionality
2. THE CLI_Tool SHALL integrate authentication commands into the existing `dmn` binary interface
3. THE Extension SHALL incorporate license checking into the existing TypeScript codebase
4. THE System SHALL reuse existing configuration and error handling patterns
5. THE licensing implementation SHALL maintain the current cross-platform build and deployment processes
6. THE System SHALL preserve existing API contracts while adding license validation