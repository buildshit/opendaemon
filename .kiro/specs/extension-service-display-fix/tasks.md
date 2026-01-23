# Implementation Plan

- [x] 1. Add enhanced logging to service loading
  - Add console.log statements throughout `loadServicesFromConfig()` to track execution
  - Log the number of services found and their names
  - Log any errors during config parsing
  - _Requirements: 1.1, 1.2, 1.3, 4.5_

- [x] 2. Improve error handling in loadServicesFromConfig
  - Wrap config loading in try-catch with detailed error messages

  - Display errors using errorDisplayManager with CONFIG category
  - Handle case where services object is missing or empty
  - _Requirements: 1.4, 1.5, 4.1_

- [x] 3. Add enhanced logging to initialization sequence
  - Add step-by-step logging in `initializeDaemon()` function
  - Log tree view state after loading services
  - Log daemon startup status
  - _Requirements: 4.5_
    alization
  - _Requirements: 4.5_

- [x] 4. Improve command palette error messages
  - Update `getServiceItem()` to provide actionable error messages
  - Add "Create dmn.json" option when no config found
  - Add "Open dmn.json" option when no services found
  - Add "Reload Window" option when tree view not initialized
  - _Requirements: 1.4, 1.5, 5.4, 5.5_

- [x] 5. Add timeout_seconds field to ReadyCondition enum
  - Modify `ReadyCondition::LogContains` to include optional `timeout_seconds` field

  - Modify `ReadyCondition::UrlResponds` to include optional `timeout_seconds` field
  - Add serde default attribute for backward compatibility
  - _Requirements: 3.2_

- [x] 6. Update Orchestrator to use custom timeouts
  - Increase default timeout from 30 to 60 seconds in `Orchestrator::new()`
  - Extract timeout value from service config's ready_when condition
  - Pass custom timeout to `ready_watcher.watch_service_with_timeout()`
  - _Requirements: 3.1, 3.2, 3.4, 3.5_

- [x] 7. Improve timeout error messages
  - Update timeout error to include service name and condition details

  - Include last few log lines in timeout error for log_contains conditions
  - Add troubleshooting suggestions to timeout errors
  - _Requirements: 3.3, 4.3_

- [x] 8. Add service status synchronization check
  - Verify `loadServices()` is called after daemon starts
  - Add error handling for RPC failures during status loading
  - Log successful status updates
  - _Requirements: 2.1, 2.2, 4.4_

- [x] 9. Test service discovery with valid dmn.json
  - Create test case that verifies services are loaded from config
  - Verify tree view is populated before daemon starts
  - Verify services have NotStarted status initially
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 10. Test command palette with no services
  - Create test case for command execution when tree view is empty

  - Verify appropriate error message is displayed
  - Verify actionable options are provided
  - _Requirements: 5.4, 5.5_

- [x] 11. Test custom timeout configuration
  - Create test dmn.json with custom timeout values
  - Verify custom timeouts are respected
  - Verify default timeout is used when not specified
  - _Requirements: 3.2, 3.5_

- [x] 12. Update documentation
  - Document the timeout_seconds configuration option

  - Add troubleshooting section for "No services found" error
  - Add troubleshooting section for timeout errors
  - _Requirements: 3.2, 4.5_
