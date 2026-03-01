# Implementation Plan: VS Code Terminal CLI Integration

## Overview

This implementation plan breaks down the VS Code Terminal CLI Integration feature into discrete coding tasks. The approach follows a bottom-up strategy: building core utilities first (platform detection, binary resolution), then verification logic, then terminal integration, and finally user-facing features (notifications, commands). Each task builds on previous work and includes testing to validate functionality incrementally.

## Tasks

- [x] 1. Set up core utilities and platform detection
  - [x] 1.1 Create `src/cli-integration/platform-detector.ts` module
    - Implement `PlatformInfo` interface
    - Implement `detectPlatform()` function using Node.js `process.platform` and `process.arch`
    - Map platform strings to supported values ('win32', 'darwin', 'linux' and 'x64', 'arm64')
    - Throw descriptive error for unsupported platforms
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_
  
  - [x] 1.2 Write unit tests for platform detector
    - Test correct detection for each supported platform (Windows x64, macOS ARM64, macOS x64, Linux x64)
    - Test error thrown for unsupported platforms
    - Mock `process.platform` and `process.arch` for different scenarios
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 2. Implement binary resolution logic
  - [x] 2.1 Create `src/cli-integration/binary-resolver.ts` module
    - Implement `BinaryInfo` interface
    - Implement `resolveBinary(extensionPath, platform)` function
    - Construct binary name based on platform (e.g., 'dmn-win32-x64.exe', 'dmn-darwin-arm64')
    - Build full path using `path.join(extensionPath, 'bin', binaryName)`
    - Return `BinaryInfo` with name, fullPath, and binDir
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6_
  
  - [x] 2.2 Write property test for path construction pattern
    - **Property 3: Path Construction Pattern**
    - **Validates: Requirements 2.6**
    - Generate random valid extension paths and binary names
    - Verify constructed path matches pattern `{extensionPath}/bin/{binaryName}`
    - Use fast-check library with 100 iterations
    - _Requirements: 2.6_
  
  - [x] 2.3 Write unit tests for binary resolver
    - Test correct binary name for each platform
    - Test full path construction with various extension paths
    - Test bin directory extraction
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.6_

- [x] 3. Implement binary verification and permission handling
  - [x] 3.1 Create `src/cli-integration/binary-verifier.ts` module
    - Implement `VerificationResult` interface
    - Implement `verifyBinary(binaryPath)` async function
    - Check file existence using `fs.access()` with `fs.constants.F_OK`
    - On Unix-like systems, check execute permission with `fs.constants.X_OK`
    - Return verification result with exists, hasPermissions, and optional error
    - _Requirements: 6.1, 6.2, 6.3_
  
  - [x] 3.2 Implement permission fixing for Unix systems
    - Implement `fixPermissions(binaryPath)` async function
    - Use `fs.chmod(binaryPath, 0o755)` to set execute permissions
    - Return boolean indicating success/failure
    - Log all operations for debugging
    - _Requirements: 6.4, 6.5_
  
  - [x] 3.3 Write unit tests for binary verifier
    - Test verification succeeds when binary exists with permissions
    - Test verification fails when binary missing
    - Test verification fails when permissions missing (Unix)
    - Test permission fix succeeds
    - Test permission fix fails
    - Mock fs operations for different scenarios
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 4. Checkpoint - Ensure core utilities work correctly
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement terminal PATH injection logic
  - [x] 5.1 Create `src/cli-integration/terminal-interceptor.ts` module
    - Implement `TerminalInterceptor` class with constructor accepting binDir
    - Implement `start()` method to begin interception
    - Implement `stop()` method to cleanup and dispose listeners
    - Implement private `injectPath(existingEnv)` method
    - Determine PATH separator based on platform (';' for Windows, ':' for Unix)
    - Prepend bin directory to existing PATH: `binDir + separator + existingPath`
    - Handle case where PATH is undefined (create new PATH with just binDir)
    - _Requirements: 1.1, 1.5, 3.4, 3.5, 3.6_
  
  - [x] 5.2 Write property test for PATH preservation
    - **Property 2: PATH Preservation**
    - **Validates: Requirements 1.5**
    - Generate random PATH strings with various entries
    - Inject bin directory using `injectPath()`
    - Verify all original PATH entries remain present and in original order
    - Use fast-check library with 100 iterations
    - _Requirements: 1.5_
  
  - [x] 5.3 Write unit tests for PATH injection
    - Test PATH separator is ';' on Windows
    - Test PATH separator is ':' on Unix-like systems
    - Test bin directory is prepended to PATH
    - Test empty/undefined PATH is handled correctly
    - Mock platform detection for different scenarios
    - _Requirements: 1.1, 1.5, 3.4, 3.5_

- [x] 6. Implement user notification system
  - [x] 6.1 Create `src/cli-integration/notification-manager.ts` module
    - Implement `NotificationManager` class with constructor accepting ExtensionContext
    - Implement `showFirstTimeNotification(binDir)` async method
    - Check if notification was already shown using `context.globalState.get()`
    - Display notification with message about CLI availability
    - Include action buttons: "Open Terminal", "View Documentation", "Don't Show Again"
    - Mark notification as shown using `context.globalState.update()`
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_
  
  - [x] 6.2 Implement error and instruction notifications
    - Implement `showErrorNotification(error)` async method
    - Implement `showGlobalInstallInstructions(platform, binDir)` async method
    - Show platform-specific instructions for Windows (System Properties PATH)
    - Show platform-specific instructions for Unix (copy to /usr/local/bin or add to PATH)
    - Include "Copy Path" button to copy bin directory to clipboard
    - _Requirements: 1.4, 5.2, 5.3, 5.4, 5.5, 6.2, 6.5_
  
  - [x] 6.3 Write property test for notification dismissal persistence
    - **Property 4: Notification Dismissal Persistence**
    - **Validates: Requirements 4.4**
    - Simulate notification dismissal
    - Activate extension multiple times
    - Verify notification not shown after dismissal
    - Use fast-check library with 100 iterations
    - _Requirements: 4.4_
  
  - [x] 6.4 Write unit tests for notification manager
    - Test first-time notification displays on first activation
    - Test notification does not display on subsequent activations
    - Test notification state persistence
    - Test error notifications display correct messages
    - Test global install instructions show platform-specific content
    - Mock ExtensionContext.globalState for testing
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.2, 5.3, 5.4, 5.5_

- [x] 7. Checkpoint - Ensure notification system works correctly
  - Ensure all tests pass, ask the user if questions arise.

- [x] 8. Implement main CLI integration manager
  - [x] 8.1 Create `src/cli-integration/cli-integration-manager.ts` module
    - Implement `CLIIntegrationManager` class with constructor accepting ExtensionContext
    - Implement `activate()` async method with full activation flow
    - Detect platform using PlatformDetector
    - Resolve binary path using BinaryResolver
    - Verify binary using BinaryVerifier
    - Handle verification failures gracefully (log error, show notification, return early)
    - Initialize TerminalInterceptor with bin directory
    - Show first-time notification if applicable
    - Store references to components for cleanup
    - _Requirements: 1.3, 1.4, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_
  
  - [x] 8.2 Implement terminal creation with CLI
    - Implement `createTerminalWithCLI(name?)` async method
    - Use `vscode.window.createTerminal()` with custom env option
    - Inject PATH using TerminalInterceptor's injectPath logic
    - Return created terminal instance
    - _Requirements: 1.1, 1.2, 3.1, 3.2, 3.3_
  
  - [x] 8.3 Implement deactivation and cleanup
    - Implement `deactivate()` method
    - Stop TerminalInterceptor
    - Dispose of any registered disposables
    - _Requirements: 3.6_
  
  - [x] 8.4 Implement global installation instructions command
    - Implement `showGlobalInstallInstructions()` async method
    - Delegate to NotificationManager
    - _Requirements: 5.1, 5.2_
  
  - [x] 8.5 Write property test for PATH injection universality
    - **Property 1: PATH Injection Universality**
    - **Validates: Requirements 1.1, 3.1, 3.2, 3.3**
    - Generate random terminal names and options
    - Create terminals using `createTerminalWithCLI()`
    - Verify PATH contains bin directory in all cases
    - Use fast-check library with 100 iterations
    - _Requirements: 1.1, 3.1, 3.2, 3.3_
  
  - [x] 8.6 Write integration tests for CLI integration manager
    - Test full activation flow with valid binary
    - Test activation flow with missing binary
    - Test activation flow with permission issues
    - Test terminal creation with PATH injection
    - Test extension deactivation cleanup
    - Mock all dependencies for isolated testing
    - _Requirements: 1.3, 1.4, 6.1, 6.2, 6.3, 6.4, 6.5, 6.6_

- [x] 9. Register commands and integrate with extension
  - [x] 9.1 Update `src/extension.ts` to integrate CLI manager
    - Import CLIIntegrationManager
    - Create instance in `activate()` function
    - Call `cliManager.activate()` during extension activation
    - Call `cliManager.deactivate()` in extension's `deactivate()` function
    - Handle activation errors gracefully
    - _Requirements: 1.3, 6.6_
  
  - [x] 9.2 Register VS Code commands in `package.json` and `extension.ts`
    - Register `opendaemon.newTerminalWithCLI` command
    - Register `opendaemon.showCLIInfo` command
    - Register `opendaemon.installCLIGlobally` command
    - Wire commands to CLIIntegrationManager methods
    - Add command titles and categories to package.json
    - _Requirements: 4.5, 5.1_
  
  - [x] 9.3 Implement command handlers
    - Implement handler for "New Terminal with CLI" command
    - Implement handler for "Show CLI Info" command
    - Implement handler for "Install CLI Globally" command
    - Handle errors in command execution
    - _Requirements: 4.5, 5.1_
  
  - [x] 9.4 Write unit tests for command registration
    - Test "New Terminal with CLI" command creates terminal
    - Test "Show CLI Info" command displays notification
    - Test "Install CLI Globally" command shows instructions
    - Mock VS Code API for testing
    - _Requirements: 4.5, 5.1_

- [x] 10. Checkpoint - Ensure full integration works correctly
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. Add documentation
  - [x] 11.1 Create `extension/docs/cli-integration.md` documentation file
    - Explain automatic CLI availability in VS Code terminals
    - List all available `dmn` commands with examples
    - Provide troubleshooting guide for common issues
    - Explain difference between VS Code terminal integration and global installation
    - Document platform-specific binary names and locations
    - Include screenshots or examples of terminal usage
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5_
  
  - [x] 11.2 Update main README.md with CLI integration information
    - Add section about terminal CLI integration
    - Link to detailed CLI documentation
    - Mention automatic availability in VS Code terminals
    - _Requirements: 8.1_

- [x] 12. Final validation and cleanup
  - [x] 12.1 Run full test suite
    - Execute all unit tests
    - Execute all property tests
    - Execute all integration tests
    - Verify test coverage meets 80% target
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_
  
  - [x] 12.2 Manual testing on each platform
    - Test on Windows x64
    - Test on macOS ARM64
    - Test on macOS x64
    - Test on Linux x64
    - Verify CLI works in terminals on each platform
    - Verify notifications display correctly
    - Verify commands work as expected
    - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.3, 2.4_
  
  - [x] 12.3 Code review and cleanup
    - Review all new code for consistency
    - Remove any debug logging
    - Ensure error messages are user-friendly
    - Verify TypeScript types are correct
    - Check for any TODO comments
    - _Requirements: All_

## Notes

- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties using fast-check library
- Unit tests validate specific examples and edge cases
- Integration tests validate end-to-end flows
- The implementation uses TypeScript and integrates with the existing VS Code extension structure
- All property tests should run a minimum of 100 iterations
- Test coverage target is 80% for all new code
