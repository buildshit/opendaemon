# Implementation Plan: Terminal Auto-Creation and Logging

## Overview

This implementation plan breaks down the terminal auto-creation and logging feature into discrete coding tasks. The approach is incremental, with each task building on previous work. Testing tasks are included as sub-tasks to validate functionality early.

## Tasks

- [x] 1. Create ActivityLogger class
  - Create `extension/src/activity-logger.ts` with the ActivityLogger class
  - Implement log(), logServiceAction(), logTerminalAction(), logRpcAction(), logError(), show(), and dispose() methods
  - Add proper TypeScript types and JSDoc comments
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 4.11_

- [ ]* 1.1 Write unit tests for ActivityLogger
  - Test output channel creation
  - Test log message formatting
  - Test different log methods (service, terminal, RPC, error)
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9_

- [x] 2. Integrate ActivityLogger into extension lifecycle
  - Modify `extension/src/extension.ts` to create ActivityLogger instance on activation
  - Add activity logger to context subscriptions for proper disposal
  - Log extension activation and deactivation events
  - Pass activity logger instance to CommandManager and RpcClient constructors
  - _Requirements: 4.10, 4.11_

- [ ]* 2.1 Write unit tests for ActivityLogger integration
  - Test activity logger is created on activation
  - Test activation event is logged
  - Test deactivation event is logged
  - Test activity logger is disposed on deactivation
  - _Requirements: 4.10, 4.11_

- [x] 3. Enhance TerminalManager with activity logging
  - Modify `extension/src/terminal-manager.ts` to accept ActivityLogger in constructor
  - Add activity logging to getOrCreateTerminal() method for terminal creation
  - Implement writeLogLine() method to write single log lines with formatting
  - Add activity logging to writeLogLine() method
  - Add activity logging to closeTerminal() method
  - Update showTerminal() to log terminal shown action
  - _Requirements: 1.1, 1.2, 2.1, 2.3, 2.4, 2.5, 4.4, 4.5, 4.6_

- [ ]* 3.1 Write property test for terminal creation
  - **Property 1: Terminal Creation for Service Start**
  - **Validates: Requirements 1.1, 1.2, 1.5**
  - Test that for any service name, creating a terminal results in a terminal with the correct name format
  - Test that terminal is visible after creation
  - Test that focus is preserved (preserveFocus parameter)

- [ ]* 3.2 Write property test for log line formatting
  - **Property 6: Log Metadata Preservation**
  - **Validates: Requirements 2.3, 2.4, 2.5**
  - Test that for any log line (timestamp, content, stream), the formatted output contains all metadata
  - Test both stdout and stderr stream types

- [x] 4. Wire automatic terminal creation in CommandManager
  - Modify `extension/src/commands.ts` to accept ActivityLogger in constructor
  - Update startService() to create and show terminal immediately after RPC request
  - Update startAll() to create terminals for all services
  - Update restartService() to clear terminal before restarting
  - Add activity logging for all service actions (start, stop, restart)
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.3, 4.2, 4.3_

- [ ]* 4.1 Write property test for unique terminals per service
  - **Property 2: Unique Terminals per Service**
  - **Validates: Requirements 1.3**
  - Test that for any set of services started, each gets exactly one unique terminal

- [ ]* 4.2 Write property test for terminal reuse
  - **Property 3: Terminal Reuse**
  - **Validates: Requirements 1.4**
  - Test that starting a service twice reuses the existing terminal

- [ ]* 4.3 Write property test for terminal clearing on restart
  - **Property 9: Terminal Clearing on Restart**
  - **Validates: Requirements 3.3**
  - Test that restarting a service clears the terminal before new logs appear

- [x] 5. Implement real-time log streaming to terminals
  - Modify handleDaemonNotification() in `extension/src/extension.ts` to route 'logLine' notifications to TerminalManager
  - Keep existing routing to LogManager for backward compatibility
  - Add throttled activity logging for log line streaming (max once per second per service)
  - Implement shouldLogLine() helper function for throttling
  - _Requirements: 2.1, 2.2, 4.6, 6.3, 6.4_

- [ ]* 5.1 Write property test for real-time log streaming
  - **Property 4: Real-Time Log Streaming**
  - **Validates: Requirements 2.1**
  - Test that for any logLine notification, the content appears in the service's terminal

- [ ]* 5.2 Write property test for dual log routing
  - **Property 19: Dual Log View Support**
  - **Validates: Requirements 6.3, 6.4**
  - Test that log lines are routed to both LogManager and TerminalManager

- [x] 6. Implement historical log fetching for terminals
  - Modify showTerminal() command in `extension/src/commands.ts` to fetch historical logs
  - Add error handling for log fetch failures with activity and error logging
  - Ensure terminal remains open even if log fetch fails
  - _Requirements: 2.2, 5.5_

- [ ]* 6.1 Write property test for historical log fetching
  - **Property 5: Historical Log Fetching**
  - **Validates: Requirements 2.2**
  - Test that opening a terminal for a running service fetches and displays historical logs

- [x] 7. Implement terminal lifecycle management
  - Ensure terminals persist after service stops (verify existing behavior)
  - Verify terminal cleanup on manual close (verify existing onDidCloseTerminal handler)
  - Verify resource disposal on extension deactivation (verify existing dispose() method)
  - Add activity logging for terminal closure events
  - _Requirements: 3.1, 3.2, 3.4, 4.5_

- [ ]* 7.1 Write property test for terminal persistence
  - **Property 7: Terminal Persistence After Service Stop**
  - **Validates: Requirements 3.1**
  - Test that stopping a service keeps the terminal open

- [ ]* 7.2 Write property test for terminal tracking cleanup
  - **Property 8: Terminal Tracking Cleanup**
  - **Validates: Requirements 3.2**
  - Test that manually closing a terminal removes it from tracking

- [ ]* 7.3 Write property test for resource disposal
  - **Property 10: Resource Disposal on Deactivation**
  - **Validates: Requirements 3.4**
  - Test that extension deactivation disposes all terminal resources

- [ ]* 7.4 Write property test for terminal recreation
  - **Property 11: Terminal Recreation After Closure**
  - **Validates: Requirements 3.5**
  - Test that reopening a closed terminal creates a new terminal with logs

- [x] 8. Add RPC activity logging
  - Modify `extension/src/rpc-client.ts` to accept ActivityLogger in constructor
  - Add activity logging to request() method for outgoing requests
  - Add activity logging to handleResponse() method for responses
  - Add activity logging to timeout errors
  - _Requirements: 4.7, 4.8, 5.3_

- [ ]* 8.1 Write property test for RPC action logging
  - **Property 15: RPC Action Logging**
  - **Validates: Requirements 4.7, 4.8**
  - Test that for any RPC request/response, activity is logged

- [x] 9. Add comprehensive notification logging
  - Enhance handleDaemonNotification() in `extension/src/extension.ts` to log all notification types
  - Add activity logging for ServiceStatusChanged, serviceStarting, serviceReady, serviceFailed, serviceStopped
  - Add error logging for notification processing failures
  - _Requirements: 4.9, 5.4_

- [ ]* 9.1 Write property test for notification logging
  - **Property 16: Notification Logging**
  - **Validates: Requirements 4.9**
  - Test that for any daemon notification, activity is logged

- [x] 10. Implement comprehensive error logging
  - Add try-catch blocks with dual logging (activity + error channels) to terminal creation in TerminalManager
  - Add error handling with dual logging to writeLogLine() in TerminalManager
  - Add error handling with dual logging to historical log fetching in CommandManager
  - Add error handling with dual logging to notification processing in extension.ts
  - Ensure all errors include context (service name, method name, etc.)
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5_

- [ ]* 10.1 Write property test for comprehensive error logging
  - **Property 17: Comprehensive Error Logging**
  - **Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5**
  - Test that for any error type, logging occurs to both activity and error channels

- [x] 11. Implement manual terminal command integration
  - Verify showTerminal() command reuses existing terminals (already implemented)
  - Verify showTerminal() creates new terminal with logs if none exists (already implemented)
  - Add activity logging to showTerminal() command
  - _Requirements: 6.1, 6.2_

- [ ]* 11.1 Write property test for manual terminal integration
  - **Property 18: Manual Terminal Command Integration**
  - **Validates: Requirements 6.1, 6.2**
  - Test that manual "Open Terminal" command reuses auto-created terminals
  - Test that manual command creates new terminal with logs if none exists

- [x] 12. Add activity logging for service actions
  - Add activity logging to all service status change notifications
  - Ensure service name and new status are logged
  - _Requirements: 4.2, 4.3_

- [ ]* 12.1 Write property test for service action logging
  - **Property 12: Service Action Logging**
  - **Validates: Requirements 4.2, 4.3**
  - Test that for any service start/stop action, activity is logged

- [ ]* 12.2 Write property test for terminal action logging
  - **Property 13: Terminal Action Logging**
  - **Validates: Requirements 4.4, 4.5**
  - Test that for any terminal creation/closure, activity is logged

- [ ]* 12.3 Write property test for log streaming activity logging
  - **Property 14: Log Streaming Activity Logging**
  - **Validates: Requirements 4.6**
  - Test that log streaming activity is logged (with throttling)

- [x] 13. Checkpoint - Ensure all tests pass
  - Run all unit tests and property tests
  - Verify no regressions in existing functionality
  - Test manually with real services
  - Ensure all tests pass, ask the user if questions arise

- [ ] 14. Update package.json and documentation
  - Add fast-check as a dev dependency for property-based testing
  - Update extension README with information about activity logging
  - Add JSDoc comments to all new public methods
  - _Requirements: All_

- [ ] 15. Final integration testing
  - Test complete flow: start service → terminal appears → logs stream → stop service → terminal persists
  - Test error scenarios: daemon crash, RPC timeout, terminal creation failure
  - Test activity log output for completeness
  - Verify backward compatibility with existing "Show Logs" command
  - Test with multiple services running simultaneously
  - _Requirements: All_

- [ ] 16. Final checkpoint - Ensure all tests pass
  - Run full test suite
  - Verify all property tests pass with 100+ iterations
  - Check code coverage meets goals (>80% line coverage)
  - Ensure all tests pass, ask the user if questions arise

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Property tests validate universal correctness properties with 100+ iterations
- Unit tests validate specific examples and edge cases
- Checkpoints ensure incremental validation
- The implementation maintains backward compatibility with existing log viewing features
- Activity logging is throttled for log streaming to avoid performance impact
- All error handling includes dual logging to both activity and error channels
