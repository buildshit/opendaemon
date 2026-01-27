# Design Document: Automatic Terminal Profile Registration

## Overview

This design implements automatic terminal profile registration for the OpenDaemon VS Code extension, enabling the `dmn` CLI command to work in all new terminals without requiring users to invoke a special command. The solution leverages VS Code's `registerTerminalProfileProvider` API to create a custom terminal profile with PATH injection, then sets it as the default profile for the workspace.

The key insight is that VS Code doesn't provide an API to intercept terminal creation before initialization, but it does allow extensions to register custom terminal profiles that control how terminals are created. By registering a profile provider and setting it as the default, we ensure all new terminals automatically include the CLI binary in their PATH.

## Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      Extension Activation                    │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  CLI Integration Manager                     │
│  - Coordinates CLI integration features                      │
│  - Manages binary verification                               │
│  - Delegates to Terminal Interceptor                         │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Terminal Interceptor                      │
│  - Registers terminal profile provider                       │
│  - Sets default workspace profile                            │
│  - Manages profile lifecycle                                 │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              OpenDaemon Terminal Profile Provider            │
│  - Implements TerminalProfileProvider interface              │
│  - Provides terminal options with PATH injection             │
│  - Handles platform-specific PATH formatting                 │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Extension Activation**: Extension activates and initializes CLI Integration Manager
2. **Binary Verification**: CLI Integration Manager verifies CLI binary exists
3. **Profile Registration**: Terminal Interceptor registers the profile provider
4. **Default Configuration**: Terminal Interceptor sets the profile as workspace default
5. **Terminal Creation**: User creates terminal → VS Code calls profile provider → Terminal created with PATH injection

## Components and Interfaces

### Terminal Profile Provider

The core component that implements VS Code's `TerminalProfileProvider` interface:

```typescript
interface OpenDaemonTerminalProfileProvider implements vscode.TerminalProfileProvider {
  provideTerminalProfile(
    token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.TerminalProfile>;
}
```

**Responsibilities:**
- Provide terminal options with injected PATH
- Handle platform-specific PATH formatting
- Return terminal profile configuration

**Implementation:**
```typescript
class OpenDaemonTerminalProfileProvider implements vscode.TerminalProfileProvider {
  constructor(
    private readonly cliPath: string,
    private readonly platform: NodeJS.Platform
  ) {}

  provideTerminalProfile(
    token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.TerminalProfile> {
    const pathSeparator = this.platform === 'win32' ? ';' : ':';
    const cliDir = path.dirname(this.cliPath);
    const currentPath = process.env.PATH || '';
    const newPath = `${cliDir}${pathSeparator}${currentPath}`;

    return new vscode.TerminalProfile({
      name: 'OpenDaemon CLI',
      iconPath: new vscode.ThemeIcon('terminal'),
      env: {
        PATH: newPath
      }
    });
  }
}
```

### Terminal Interceptor Updates

The `TerminalInterceptor` class will be updated to handle profile registration:

```typescript
class TerminalInterceptor {
  private profileDisposable: vscode.Disposable | undefined;
  private previousDefaultProfile: string | undefined;

  async start(cliPath: string): Promise<void> {
    // Verify CLI binary exists
    if (!fs.existsSync(cliPath)) {
      logger.warn('CLI binary not found, skipping profile registration');
      return;
    }

    try {
      // Register terminal profile provider
      const provider = new OpenDaemonTerminalProfileProvider(
        cliPath,
        process.platform
      );
      
      this.profileDisposable = vscode.window.registerTerminalProfileProvider(
        'opendaemon.terminal',
        provider
      );

      // Set as default workspace profile
      await this.setDefaultProfile('opendaemon.terminal');
      
      logger.info('Terminal profile registered successfully');
    } catch (error) {
      logger.error('Failed to register terminal profile', error);
      // Fall back to manual terminal creation
    }
  }

  async stop(): Promise<void> {
    // Dispose profile registration
    if (this.profileDisposable) {
      this.profileDisposable.dispose();
      this.profileDisposable = undefined;
    }

    // Restore previous default profile
    if (this.previousDefaultProfile) {
      await this.restoreDefaultProfile();
    }
  }

  private async setDefaultProfile(profileId: string): Promise<void> {
    const config = vscode.workspace.getConfiguration('terminal.integrated');
    
    // Store previous default for restoration
    this.previousDefaultProfile = config.get('defaultProfile.linux') ||
                                   config.get('defaultProfile.osx') ||
                                   config.get('defaultProfile.windows');

    // Set as default based on platform
    const platform = process.platform;
    const configKey = platform === 'win32' ? 'defaultProfile.windows' :
                      platform === 'darwin' ? 'defaultProfile.osx' :
                      'defaultProfile.linux';

    await config.update(
      configKey,
      profileId,
      vscode.ConfigurationTarget.Workspace
    );
  }

  private async restoreDefaultProfile(): Promise<void> {
    if (!this.previousDefaultProfile) {
      return;
    }

    const config = vscode.workspace.getConfiguration('terminal.integrated');
    const platform = process.platform;
    const configKey = platform === 'win32' ? 'defaultProfile.windows' :
                      platform === 'darwin' ? 'defaultProfile.osx' :
                      'defaultProfile.linux';

    await config.update(
      configKey,
      this.previousDefaultProfile,
      vscode.ConfigurationTarget.Workspace
    );
  }
}
```

### CLI Integration Manager Updates

The `CLIIntegrationManager` will coordinate the profile registration:

```typescript
class CLIIntegrationManager {
  private terminalInterceptor: TerminalInterceptor;

  async initialize(): Promise<void> {
    // Verify binary exists
    const cliPath = await this.binaryResolver.resolve();
    if (!cliPath) {
      logger.warn('CLI binary not found');
      return;
    }

    // Start terminal interceptor with profile registration
    await this.terminalInterceptor.start(cliPath);
  }

  async dispose(): Promise<void> {
    // Stop terminal interceptor and clean up
    await this.terminalInterceptor.stop();
  }
}
```

## Data Models

### Terminal Profile Configuration

```typescript
interface TerminalProfileConfig {
  name: string;           // "OpenDaemon CLI"
  iconPath: vscode.ThemeIcon;
  env: {
    PATH: string;         // Injected PATH with CLI binary directory
  };
}
```

### Profile Registration State

```typescript
interface ProfileRegistrationState {
  profileDisposable: vscode.Disposable | undefined;
  previousDefaultProfile: string | undefined;
  isRegistered: boolean;
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: Profile Provider Returns Valid Terminal Options

*For any* call to `provideTerminalProfile()`, the returned terminal profile should include a valid PATH environment variable that contains the CLI binary directory.

**Validates: Requirements 1.4, 1.5**

### Property 2: Registration Stores Disposable

*For any* successful terminal profile provider registration, the extension should store the registration disposable for later cleanup.

**Validates: Requirements 1.2**

### Property 3: Platform-Specific PATH Formatting

*For any* platform (Windows, macOS, Linux), the PATH environment variable should use the correct platform-specific separator (`;` for Windows, `:` for Unix-like systems).

**Validates: Requirements 3.1, 3.2, 3.3, 3.4**

### Property 4: Workspace Configuration Scope

*For any* configuration update related to terminal profile defaults, the extension should target workspace-level configuration scope, not user-level.

**Validates: Requirements 2.2**

### Property 5: Configuration Preservation

*For any* existing workspace terminal profile configuration, after profile registration, the previous configuration value should be stored for restoration.

**Validates: Requirements 2.4**

### Property 6: Error Handling Without Exceptions

*For any* error during terminal profile registration or provider execution, the extension should log the error and continue without throwing unhandled exceptions.

**Validates: Requirements 4.1, 4.3, 4.4**

### Property 7: Cleanup Disposes Registration

*For any* registered terminal profile provider, when the extension deactivates, the registration disposable should be disposed.

**Validates: Requirements 5.1, 5.3**

### Property 8: Configuration Restoration Round-Trip

*For any* workspace terminal profile configuration, after profile registration and subsequent cleanup, the configuration should be restored to its original value.

**Validates: Requirements 5.2**

### Property 9: Cleanup Error Handling

*For any* error during cleanup, the extension should log the error and complete deactivation without blocking.

**Validates: Requirements 5.4**

### Property 10: Manual Terminal Creation PATH Injection

*For any* invocation of the "New Terminal with CLI" command, the created terminal should have the CLI binary directory in its PATH environment variable.

**Validates: Requirements 6.2**

### Property 11: Binary Verification Before Registration

*For any* terminal profile registration attempt, the extension should verify the CLI binary exists before proceeding with registration.

**Validates: Requirements 7.1**

### Property 12: Profile Provider Update on Path Change

*For any* change to the CLI binary path, the terminal profile provider should be updated to reflect the new path.

**Validates: Requirements 7.3**

### Property 13: Profile Description Presence

*For any* terminal profile created by the provider, it should include a description field.

**Validates: Requirements 8.2**

## Error Handling

### Registration Errors

**Error Scenario**: Terminal profile provider registration fails
- **Handling**: Log error, continue extension activation, fall back to manual terminal creation
- **User Impact**: User can still use "New Terminal with CLI" command
- **Recovery**: Retry registration on next activation

**Error Scenario**: CLI binary not found during registration
- **Handling**: Skip profile registration, log warning
- **User Impact**: Automatic profile not available, manual command still works
- **Recovery**: Registration will succeed once binary is available

### Runtime Errors

**Error Scenario**: Profile provider throws exception during `provideTerminalProfile()`
- **Handling**: Catch exception, log error, return undefined
- **User Impact**: Terminal creation falls back to default profile
- **Recovery**: Next terminal creation attempt will retry

**Error Scenario**: Configuration update fails
- **Handling**: Log error, continue with registration
- **User Impact**: Profile registered but not set as default
- **Recovery**: User can manually select profile from terminal dropdown

### Cleanup Errors

**Error Scenario**: Disposable disposal fails during deactivation
- **Handling**: Log error, continue deactivation
- **User Impact**: Profile may remain registered until VS Code restart
- **Recovery**: VS Code will clean up on restart

**Error Scenario**: Configuration restoration fails
- **Handling**: Log error, complete deactivation
- **User Impact**: Previous default profile not restored
- **Recovery**: User can manually reset terminal profile in settings

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests for comprehensive coverage:

- **Unit tests**: Verify specific examples, edge cases, and error conditions
- **Property tests**: Verify universal properties across all inputs

### Unit Testing Focus

Unit tests should cover:
- Extension activation flow with profile registration
- Profile registration with valid CLI binary path
- Profile registration failure when binary doesn't exist
- Configuration updates target workspace scope
- Cleanup disposes registration and restores configuration
- Backward compatibility with "New Terminal with CLI" command
- Platform-specific edge cases (Windows, macOS, Linux)
- Error scenarios (registration failure, runtime errors, cleanup errors)

### Property-Based Testing Configuration

**Library**: fast-check (TypeScript property-based testing library)

**Configuration**:
- Minimum 100 iterations per property test
- Each test tagged with: **Feature: automatic-terminal-profile-registration, Property {number}: {property_text}**

**Property Tests**:

1. **Profile Provider Returns Valid Terminal Options**
   - Generate: Random CLI binary paths
   - Verify: Returned profile contains PATH with CLI directory
   - Tag: **Feature: automatic-terminal-profile-registration, Property 1: Profile Provider Returns Valid Terminal Options**

2. **Registration Stores Disposable**
   - Generate: Random registration scenarios
   - Verify: Disposable is stored after registration
   - Tag: **Feature: automatic-terminal-profile-registration, Property 2: Registration Stores Disposable**

3. **Platform-Specific PATH Formatting**
   - Generate: Random platforms (win32, darwin, linux) and CLI paths
   - Verify: PATH uses correct separator for platform
   - Tag: **Feature: automatic-terminal-profile-registration, Property 3: Platform-Specific PATH Formatting**

4. **Workspace Configuration Scope**
   - Generate: Random configuration updates
   - Verify: All updates target workspace scope
   - Tag: **Feature: automatic-terminal-profile-registration, Property 4: Workspace Configuration Scope**

5. **Configuration Preservation**
   - Generate: Random existing configurations
   - Verify: Previous value stored before update
   - Tag: **Feature: automatic-terminal-profile-registration, Property 5: Configuration Preservation**

6. **Error Handling Without Exceptions**
   - Generate: Random error scenarios
   - Verify: No unhandled exceptions, errors logged
   - Tag: **Feature: automatic-terminal-profile-registration, Property 6: Error Handling Without Exceptions**

7. **Cleanup Disposes Registration**
   - Generate: Random registration states
   - Verify: Disposable disposed on cleanup
   - Tag: **Feature: automatic-terminal-profile-registration, Property 7: Cleanup Disposes Registration**

8. **Configuration Restoration Round-Trip**
   - Generate: Random configurations
   - Verify: Register → Cleanup → Configuration restored
   - Tag: **Feature: automatic-terminal-profile-registration, Property 8: Configuration Restoration Round-Trip**

9. **Cleanup Error Handling**
   - Generate: Random cleanup errors
   - Verify: Errors logged, deactivation completes
   - Tag: **Feature: automatic-terminal-profile-registration, Property 9: Cleanup Error Handling**

10. **Manual Terminal Creation PATH Injection**
    - Generate: Random CLI paths
    - Verify: Created terminal has PATH injection
    - Tag: **Feature: automatic-terminal-profile-registration, Property 10: Manual Terminal Creation PATH Injection**

11. **Binary Verification Before Registration**
    - Generate: Random binary paths (existing and non-existing)
    - Verify: Registration only proceeds if binary exists
    - Tag: **Feature: automatic-terminal-profile-registration, Property 11: Binary Verification Before Registration**

12. **Profile Provider Update on Path Change**
    - Generate: Random path changes
    - Verify: Provider updated with new path
    - Tag: **Feature: automatic-terminal-profile-registration, Property 12: Profile Provider Update on Path Change**

13. **Profile Description Presence**
    - Generate: Random profile creation scenarios
    - Verify: Description field present in profile
    - Tag: **Feature: automatic-terminal-profile-registration, Property 13: Profile Description Presence**

### Integration Testing

Integration tests should verify:
- End-to-end flow: Extension activation → Profile registration → Terminal creation
- Interaction with VS Code API (using mocks)
- Interaction with existing CLI integration components
- Backward compatibility with existing terminal creation command

### Test Coverage Goals

- Unit test coverage: >90% of new code
- Property test coverage: All 13 correctness properties
- Integration test coverage: All major user flows
- Edge case coverage: All platform-specific behaviors
