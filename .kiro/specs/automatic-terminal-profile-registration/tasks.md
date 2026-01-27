# Implementation Plan: Automatic Terminal Profile Registration

## Overview

This implementation plan breaks down the automatic terminal profile registration feature into discrete coding tasks. The approach follows an incremental strategy: create the profile provider, integrate it with the terminal interceptor, update the CLI integration manager, add comprehensive testing, and ensure backward compatibility.

## Tasks

- [x] 1. Create OpenDaemon Terminal Profile Provider
  - Create `extension/src/cli-integration/terminal-profile-provider.ts`
  - Implement `OpenDaemonTerminalProfileProvider` class that implements `vscode.TerminalProfileProvider`
  - Implement `provideTerminalProfile()` method with PATH injection logic
  - Handle platform-specific PATH separator (`;` for Windows, `:` for Unix)
  - Add logging for profile creation
  - _Requirements: 1.3, 1.4, 1.5, 3.1, 3.2, 3.3, 3.4_

- [x] 1.1 Write property test for profile provider PATH injection
  - **Property 1: Profile Provider Returns Valid Terminal Options**
  - **Validates: Requirements 1.4, 1.5**

- [x] 1.2 Write property test for platform-specific PATH formatting
  - **Property 3: Platform-Specific PATH Formatting**
  - **Validates: Requirements 3.1, 3.2, 3.3, 3.4**

- [x] 1.3 Write unit tests for profile provider
  - Test profile creation with valid CLI path
  - Test platform-specific PATH separators (Windows, macOS, Linux)
  - Test profile name and description
  - _Requirements: 1.4, 1.5, 3.1, 3.2, 3.3, 8.1, 8.2_

- [x] 2. Update Terminal Interceptor with Profile Registration
  - Update `extension/src/cli-integration/terminal-interceptor.ts`
  - Add `profileDisposable` and `previousDefaultProfile` fields
  - Implement profile registration in `start()` method
  - Implement `setDefaultProfile()` method to configure workspace settings
  - Implement `restoreDefaultProfile()` method for cleanup
  - Add binary existence verification before registration
  - Add error handling with logging (no unhandled exceptions)
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 2.4, 4.1, 4.3, 4.4, 7.1_

- [x] 2.7 Add Terminal Profile Declaration to package.json
  - Add `terminal.profiles` contribution to `extension/package.json`
  - Declare the `opendaemon.terminal` profile ID
  - This is REQUIRED for VS Code to recognize the profile
  - _Requirements: 1.1, 8.1, 8.3_

- [x] 2.1 Write property test for registration disposable storage
  - **Property 2: Registration Stores Disposable**
  - **Validates: Requirements 1.2**

- [x] 2.2 Write property test for workspace configuration scope
  - **Property 4: Workspace Configuration Scope**
  - **Validates: Requirements 2.2**

- [x] 2.3 Write property test for configuration preservation
  - **Property 5: Configuration Preservation**
  - **Validates: Requirements 2.4**

- [ ] 2.4 Write property test for error handling
  - **Property 6: Error Handling Without Exceptions**
  - **Validates: Requirements 4.1, 4.3, 4.4**

- [ ] 2.5 Write property test for binary verification
  - **Property 11: Binary Verification Before Registration**
  - **Validates: Requirements 7.1**

- [ ] 2.6 Write unit tests for terminal interceptor registration
  - Test successful profile registration
  - Test registration with non-existent binary (should skip)
  - Test default profile configuration update
  - Test workspace scope targeting
  - Test error handling during registration
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 4.1, 7.1, 7.2_

- [ ] 3. Implement Cleanup and Resource Management
  - Update `stop()` method in `terminal-interceptor.ts`
  - Dispose profile registration disposable
  - Restore previous default terminal profile
  - Add error handling for cleanup failures (log but don't block)
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 3.1 Write property test for cleanup disposal
  - **Property 7: Cleanup Disposes Registration**
  - **Validates: Requirements 5.1, 5.3**

- [ ] 3.2 Write property test for configuration restoration round-trip
  - **Property 8: Configuration Restoration Round-Trip**
  - **Validates: Requirements 5.2**

- [ ] 3.3 Write property test for cleanup error handling
  - **Property 9: Cleanup Error Handling**
  - **Validates: Requirements 5.4**

- [ ] 3.4 Write unit tests for cleanup
  - Test disposable disposal on deactivation
  - Test configuration restoration
  - Test cleanup with no previous configuration
  - Test error handling during cleanup
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

- [ ] 4. Update CLI Integration Manager
  - Update `extension/src/cli-integration/cli-integration-manager.ts`
  - Ensure `initialize()` calls `terminalInterceptor.start()` with CLI path
  - Ensure `dispose()` calls `terminalInterceptor.stop()`
  - Add error handling for profile registration failures
  - _Requirements: 1.1, 4.1, 4.2, 7.4_

- [ ] 4.1 Write unit tests for CLI integration manager
  - Test initialization with profile registration
  - Test disposal with cleanup
  - Test fallback when registration fails
  - _Requirements: 1.1, 4.1, 4.2_

- [ ] 5. Add User Notifications for Registration Failures
  - Update `terminal-interceptor.ts` to show non-intrusive notifications
  - Use `vscode.window.showInformationMessage()` for registration failures
  - Include message about fallback to manual command
  - _Requirements: 4.5_

- [ ] 5.1 Write unit tests for user notifications
  - Test notification shown on registration failure
  - Test notification content
  - _Requirements: 4.5_

- [ ] 6. Ensure Backward Compatibility
  - Verify "OpenDaemon: New Terminal with CLI" command still works
  - Ensure manual terminal creation uses same PATH injection logic
  - Test both automatic and manual terminal creation paths
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [ ] 6.1 Write property test for manual terminal creation
  - **Property 10: Manual Terminal Creation PATH Injection**
  - **Validates: Requirements 6.2**

- [ ] 6.2 Write unit tests for backward compatibility
  - Test "New Terminal with CLI" command exists
  - Test manual terminal creation with PATH injection
  - Test fallback when automatic registration disabled
  - _Requirements: 6.1, 6.2, 6.4_

- [ ] 7. Implement Dynamic Profile Updates
  - Add method to update profile provider when CLI path changes
  - Re-register profile provider with new path
  - Add file watcher or event listener for binary path changes
  - _Requirements: 7.3_

- [ ] 7.1 Write property test for profile provider updates
  - **Property 12: Profile Provider Update on Path Change**
  - **Validates: Requirements 7.3**

- [ ] 7.2 Write unit tests for dynamic updates
  - Test profile update when path changes
  - Test re-registration with new path
  - _Requirements: 7.3_

- [ ] 8. Add Profile Metadata and Identification
  - Update profile provider to include profile name "OpenDaemon CLI"
  - Add description: "Terminal with dmn CLI command available"
  - Add terminal icon using `vscode.ThemeIcon('terminal')`
  - _Requirements: 8.1, 8.2, 8.4_

- [ ] 8.1 Write property test for profile description
  - **Property 13: Profile Description Presence**
  - **Validates: Requirements 8.2**

- [ ] 8.2 Write unit tests for profile metadata
  - Test profile name is "OpenDaemon CLI"
  - Test description is present
  - Test icon is set
  - _Requirements: 8.1, 8.2, 8.4_

- [ ] 9. Checkpoint - Ensure all tests pass
  - Run all unit tests and property tests
  - Verify no regressions in existing functionality
  - Ensure all tests pass, ask the user if questions arise

- [ ] 10. Integration Testing
  - [ ] 10.1 Create integration test for end-to-end flow
    - Test extension activation → profile registration → terminal creation
    - Mock VS Code API calls
    - Verify profile provider is called when terminal is created
    - _Requirements: 1.1, 1.4, 2.1_

  - [ ] 10.2 Write integration tests for error scenarios
    - Test registration failure handling
    - Test cleanup failure handling
    - Test binary not found scenario
    - _Requirements: 4.1, 4.2, 5.4, 7.2_

- [ ] 11. Update Documentation
  - Update `extension/docs/cli-integration.md` with profile registration details
  - Document automatic vs manual terminal creation
  - Add troubleshooting section for profile registration issues
  - Document platform-specific behavior
  - _Requirements: All_

- [ ] 12. Final Checkpoint - Comprehensive Testing
  - Run full test suite (unit + property + integration)
  - Test on all platforms (Windows, macOS, Linux) if possible
  - Verify backward compatibility
  - Ensure all tests pass, ask the user if questions arise

## Notes

- Each task references specific requirements for traceability
- Property tests validate universal correctness properties with 100+ iterations
- Unit tests validate specific examples and edge cases
- Integration tests verify end-to-end flows
- The implementation maintains backward compatibility with existing terminal creation command
