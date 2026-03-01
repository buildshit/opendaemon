# Design Document: VS Code Terminal CLI Integration

## Overview

This design implements automatic CLI availability for the OpenDaemon `dmn` command in VS Code's integrated terminal. The solution leverages VS Code's terminal API to inject the extension's bin directory into the PATH environment variable for all newly created terminals, enabling users to execute `dmn` commands without manual configuration.

The design follows VS Code's security model by only modifying terminal-specific environment variables rather than system-level PATH. This approach provides seamless CLI access within VS Code while maintaining system security boundaries.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    VS Code Extension                         │
│                                                              │
│  ┌──────────────────┐      ┌─────────────────────────┐     │
│  │  Extension       │      │  Terminal Interceptor    │     │
│  │  Activation      │─────▶│  - Listen for terminal   │     │
│  │                  │      │    creation events       │     │
│  └──────────────────┘      │  - Inject PATH           │     │
│           │                 └─────────────────────────┘     │
│           │                              │                   │
│           ▼                              │                   │
│  ┌──────────────────┐                   │                   │
│  │  Platform        │                   │                   │
│  │  Detector        │                   │                   │
│  └────────┬─────────┘                   │                   │
│           │                              │                   │
│           ▼                              │                   │
│  ┌──────────────────┐                   │                   │
│  │  Binary          │                   │                   │
│  │  Resolver        │                   │                   │
│  └────────┬─────────┘                   │                   │
│           │                              │                   │
│           ▼                              │                   │
│  ┌──────────────────┐                   │                   │
│  │  Binary          │                   │                   │
│  │  Verifier        │                   │                   │
│  └────────┬─────────┘                   │                   │
│           │                              │                   │
│           ▼                              ▼                   │
│  ┌──────────────────────────────────────────────┐          │
│  │         User Notification Manager            │          │
│  └──────────────────────────────────────────────┘          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
                  ┌─────────────────┐
                  │  VS Code        │
                  │  Terminal       │
                  │  (with PATH)    │
                  └─────────────────┘
```

### Component Interaction Flow

1. **Extension Activation**: Extension activates and initializes all components
2. **Platform Detection**: Detect OS and architecture
3. **Binary Resolution**: Determine correct binary path
4. **Binary Verification**: Verify binary exists and has permissions
5. **Terminal Interception Setup**: Register terminal creation listener
6. **User Notification**: Show first-time notification (if applicable)
7. **Terminal Creation**: When user creates terminal, inject PATH before initialization
8. **CLI Execution**: User can execute `dmn` commands in terminal

## Components and Interfaces

### 1. Platform Detector

**Purpose**: Identify the current operating system and architecture.

**Interface**:
```typescript
interface PlatformInfo {
  os: 'win32' | 'darwin' | 'linux';
  arch: 'x64' | 'arm64';
}

function detectPlatform(): PlatformInfo {
  // Returns current platform information
}
```

**Implementation Details**:
- Uses Node.js `process.platform` and `process.arch`
- Maps platform strings to supported values
- Throws error for unsupported platforms

### 2. Binary Resolver

**Purpose**: Determine the correct CLI binary path for the current platform.

**Interface**:
```typescript
interface BinaryInfo {
  name: string;        // e.g., "dmn-win32-x64.exe"
  fullPath: string;    // e.g., "/path/to/extension/bin/dmn-win32-x64.exe"
  binDir: string;      // e.g., "/path/to/extension/bin"
}

function resolveBinary(extensionPath: string, platform: PlatformInfo): BinaryInfo {
  // Returns binary information for the platform
}
```

**Binary Naming Convention**:
- Windows x64: `dmn-win32-x64.exe`
- macOS ARM64: `dmn-darwin-arm64`
- macOS x64: `dmn-darwin-x64`
- Linux x64: `dmn-linux-x64`

**Implementation Details**:
- Constructs binary name from platform info
- Builds full path using `path.join(extensionPath, 'bin', binaryName)`
- Returns all path components for flexibility

### 3. Binary Verifier

**Purpose**: Verify the CLI binary exists and has correct permissions.

**Interface**:
```typescript
interface VerificationResult {
  exists: boolean;
  hasPermissions: boolean;
  error?: string;
}

async function verifyBinary(binaryPath: string): Promise<VerificationResult> {
  // Checks if binary exists and has execute permissions
}

async function fixPermissions(binaryPath: string): Promise<boolean> {
  // Attempts to set execute permissions on Unix-like systems
}
```

**Implementation Details**:
- Uses `fs.access()` to check file existence
- On Unix-like systems, checks execute permission with `fs.constants.X_OK`
- On Windows, execute permission check is skipped (not applicable)
- Attempts `fs.chmod(binaryPath, 0o755)` to fix permissions
- Logs all verification steps for debugging

### 4. Terminal Interceptor

**Purpose**: Intercept terminal creation and inject PATH environment variable.

**Interface**:
```typescript
interface TerminalOptions {
  env?: { [key: string]: string };
}

class TerminalInterceptor {
  private binDir: string;
  private disposable: vscode.Disposable | null;

  constructor(binDir: string) {
    // Initialize with bin directory path
  }

  start(): void {
    // Begin intercepting terminal creation
  }

  stop(): void {
    // Stop intercepting terminal creation
  }

  private injectPath(existingEnv: { [key: string]: string }): { [key: string]: string } {
    // Returns new env object with injected PATH
  }
}
```

**Implementation Details**:
- Uses `vscode.window.onDidOpenTerminal` event (Note: This fires AFTER terminal is created, so we need a different approach)
- **Correction**: Uses `vscode.window.createTerminal` wrapper approach
- Overrides or wraps terminal creation to inject environment
- Determines PATH separator based on platform (`;` for Windows, `:` for Unix)
- Prepends bin directory to existing PATH: `binDir + separator + existingPath`
- Preserves all other environment variables
- Stores disposable for cleanup on deactivation

**Alternative Approach** (More Reliable):
Since VS Code doesn't provide a pre-creation hook, we'll use a different strategy:
- Register a custom terminal profile that includes the modified PATH
- Set this as the default profile when extension activates
- Provide commands that create terminals with injected PATH
- Document that users should use "OpenDaemon: New Terminal" command or the extension will modify the default profile

**Revised Approach** (Best Solution):
- Use `vscode.window.createTerminal()` with custom `env` option whenever we need to create terminals
- Provide a command "OpenDaemon: New Terminal with CLI" that creates a terminal with PATH injected
- Optionally, modify the user's terminal profile settings to include the PATH (requires user permission)
- Show notification explaining users can use the command or manually add to their shell profile

### 5. User Notification Manager

**Purpose**: Display notifications to users about CLI availability and handle user interactions.

**Interface**:
```typescript
interface NotificationConfig {
  message: string;
  actions: Array<{ title: string; handler: () => void }>;
}

class NotificationManager {
  private context: vscode.ExtensionContext;

  constructor(context: vscode.ExtensionContext) {
    // Initialize with extension context for state persistence
  }

  async showFirstTimeNotification(binDir: string): Promise<void> {
    // Show notification on first activation
  }

  async showErrorNotification(error: string): Promise<void> {
    // Show error notification with troubleshooting info
  }

  async showGlobalInstallInstructions(platform: PlatformInfo, binDir: string): Promise<void> {
    // Show platform-specific global installation instructions
  }

  private hasShownFirstTime(): boolean {
    // Check if first-time notification was already shown
  }

  private markFirstTimeShown(): void {
    // Mark first-time notification as shown
  }
}
```

**Implementation Details**:
- Uses `vscode.window.showInformationMessage()` for notifications
- Uses `context.globalState` to persist notification state
- First-time notification includes:
  - Message: "OpenDaemon CLI is now available in VS Code terminals! Type 'dmn --help' to get started."
  - Actions: "Open Terminal", "View Documentation", "Don't Show Again"
- Error notifications include:
  - Clear error description
  - Troubleshooting steps
  - Link to documentation
- Global install instructions include:
  - Platform-specific commands
  - "Copy Path" button to copy bin directory to clipboard

### 6. CLI Integration Manager (Main Coordinator)

**Purpose**: Coordinate all components and manage the CLI integration lifecycle.

**Interface**:
```typescript
class CLIIntegrationManager {
  private context: vscode.ExtensionContext;
  private interceptor: TerminalInterceptor | null;
  private notificationManager: NotificationManager;
  private binaryInfo: BinaryInfo | null;

  constructor(context: vscode.ExtensionContext) {
    // Initialize manager
  }

  async activate(): Promise<void> {
    // Main activation logic
  }

  deactivate(): void {
    // Cleanup logic
  }

  async createTerminalWithCLI(name?: string): Promise<vscode.Terminal> {
    // Create a terminal with PATH injected
  }

  async showGlobalInstallInstructions(): Promise<void> {
    // Show global installation instructions
  }
}
```

**Activation Flow**:
1. Detect platform
2. Resolve binary path
3. Verify binary exists and has permissions
4. If verification fails, show error and return early
5. Initialize terminal interceptor
6. Start intercepting terminals
7. Show first-time notification (if applicable)
8. Register commands

**Commands to Register**:
- `opendaemon.newTerminalWithCLI`: Create new terminal with CLI available
- `opendaemon.showCLIInfo`: Show CLI availability notification
- `opendaemon.installCLIGlobally`: Show global installation instructions

## Data Models

### Platform Information
```typescript
interface PlatformInfo {
  os: 'win32' | 'darwin' | 'linux';
  arch: 'x64' | 'arm64';
}
```

### Binary Information
```typescript
interface BinaryInfo {
  name: string;        // Binary filename
  fullPath: string;    // Absolute path to binary
  binDir: string;      // Directory containing binary
}
```

### Verification Result
```typescript
interface VerificationResult {
  exists: boolean;           // Binary file exists
  hasPermissions: boolean;   // Has execute permissions (Unix only)
  error?: string;           // Error message if verification failed
}
```

### Terminal Environment
```typescript
interface TerminalEnvironment {
  [key: string]: string;  // Environment variables
}
```

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: PATH Injection Universality

*For any* terminal created through the extension's terminal creation mechanism, the terminal environment's PATH variable should contain the extension's bin directory.

**Validates: Requirements 1.1, 3.1, 3.2, 3.3**

### Property 2: PATH Preservation

*For any* existing PATH value, after PATH injection, all original PATH entries should remain present and in their original order, with only the bin directory prepended.

**Validates: Requirements 1.5**

### Property 3: Path Construction Pattern

*For any* valid extension path and platform-specific binary name, the Binary_Resolver should construct a full path following the pattern `{extensionPath}/bin/{binaryName}`.

**Validates: Requirements 2.6**

### Property 4: Notification Dismissal Persistence

*For any* user dismissal of the first-time notification, all subsequent extension activations should not display the notification again until the extension state is reset.

**Validates: Requirements 4.4**

## Error Handling

### Binary Verification Errors

**Missing Binary File**:
- **Detection**: Check file existence using `fs.access()` during activation
- **Response**: 
  - Log error with expected path
  - Display error notification: "OpenDaemon CLI binary not found. Expected at: {path}"
  - Provide action to reinstall extension
  - Disable terminal integration but allow other features to work
- **Recovery**: User can reinstall extension or manually place binary

**Permission Errors (Unix-like systems)**:
- **Detection**: Check execute permission using `fs.access()` with `X_OK` flag
- **Response**:
  - Attempt automatic fix with `fs.chmod(binaryPath, 0o755)`
  - If fix succeeds, log success and continue
  - If fix fails, display error notification with manual instructions
  - Provide command to copy: `chmod +x {binaryPath}`
- **Recovery**: User runs chmod command manually

**Unsupported Platform**:
- **Detection**: Platform detection returns unsupported OS/arch combination
- **Response**:
  - Log platform information
  - Display error notification listing supported platforms
  - Disable terminal integration
  - Allow other extension features to work
- **Recovery**: User reports issue or waits for platform support

### Terminal Creation Errors

**PATH Injection Failure**:
- **Detection**: Exception during environment variable manipulation
- **Response**:
  - Log error with details
  - Fall back to creating terminal without PATH injection
  - Display warning notification
  - Suggest manual PATH configuration or global installation
- **Recovery**: User uses global installation or manual PATH setup

**Terminal API Unavailable**:
- **Detection**: VS Code terminal API returns undefined or throws error
- **Response**:
  - Log error
  - Disable terminal integration
  - Display error notification suggesting VS Code update
- **Recovery**: User updates VS Code

### State Persistence Errors

**GlobalState Access Failure**:
- **Detection**: Exception when reading/writing to `context.globalState`
- **Response**:
  - Log error
  - Continue without state persistence (notification may show multiple times)
  - Display warning in output channel
- **Recovery**: Graceful degradation, feature continues to work

## Testing Strategy

### Dual Testing Approach

This feature requires both unit tests and property-based tests to ensure comprehensive coverage:

- **Unit tests**: Verify specific platform behaviors, error conditions, and UI interactions
- **Property tests**: Verify universal properties across all inputs (PATH preservation, path construction patterns)

### Unit Testing

**Platform Detection Tests**:
- Test detection returns correct values for Windows x64
- Test detection returns correct values for macOS ARM64
- Test detection returns correct values for macOS x64
- Test detection returns correct values for Linux x64
- Test detection throws error for unsupported platforms

**Binary Resolution Tests**:
- Test correct binary name for each platform
- Test full path construction
- Test bin directory extraction

**Binary Verification Tests**:
- Test verification succeeds when binary exists with permissions
- Test verification fails when binary missing
- Test verification fails when permissions missing (Unix)
- Test permission fix succeeds
- Test permission fix fails

**PATH Injection Tests**:
- Test PATH separator is `;` on Windows
- Test PATH separator is `:` on Unix-like systems
- Test bin directory is prepended to PATH
- Test empty PATH is handled correctly

**Notification Tests**:
- Test first-time notification displays on first activation
- Test first-time notification does not display on subsequent activations
- Test notification state persistence
- Test error notifications display correct messages
- Test global install instructions show platform-specific content

**Command Tests**:
- Test "New Terminal with CLI" command creates terminal
- Test "Show CLI Info" command displays notification
- Test "Install CLI Globally" command shows instructions

**Integration Tests**:
- Test full activation flow with valid binary
- Test activation flow with missing binary
- Test activation flow with permission issues
- Test terminal creation with PATH injection
- Test extension deactivation cleanup

### Property-Based Testing

We will use a property-based testing library for TypeScript (such as `fast-check`) to implement property tests. Each property test should run a minimum of 100 iterations.

**Property Test 1: PATH Injection Universality**
- **Tag**: `Feature: vscode-terminal-cli-integration, Property 1: For any terminal created through the extension's terminal creation mechanism, the terminal environment's PATH variable should contain the extension's bin directory`
- **Test**: Generate random terminal names and options, create terminals, verify PATH contains bin directory
- **Iterations**: 100

**Property Test 2: PATH Preservation**
- **Tag**: `Feature: vscode-terminal-cli-integration, Property 2: For any existing PATH value, after PATH injection, all original PATH entries should remain present and in their original order`
- **Test**: Generate random PATH strings with various entries, inject bin directory, verify all original entries remain
- **Iterations**: 100

**Property Test 3: Path Construction Pattern**
- **Tag**: `Feature: vscode-terminal-cli-integration, Property 3: For any valid extension path and platform-specific binary name, the Binary_Resolver should construct a full path following the pattern`
- **Test**: Generate random valid extension paths and binary names, verify constructed path matches pattern
- **Iterations**: 100

**Property Test 4: Notification Dismissal Persistence**
- **Tag**: `Feature: vscode-terminal-cli-integration, Property 4: For any user dismissal of the first-time notification, all subsequent extension activations should not display the notification again`
- **Test**: Simulate dismissal, activate extension multiple times, verify notification not shown
- **Iterations**: 100

### Test Configuration

- **Framework**: Mocha (already used in extension)
- **Property Testing Library**: fast-check
- **Minimum Iterations**: 100 per property test
- **Coverage Target**: 80% code coverage
- **CI Integration**: Run tests on all supported platforms

### Testing Challenges and Solutions

**Challenge**: Testing terminal creation in VS Code extension tests
- **Solution**: Mock VS Code terminal API, verify correct parameters passed

**Challenge**: Testing file system operations across platforms
- **Solution**: Use temporary directories and mock fs operations where needed

**Challenge**: Testing state persistence
- **Solution**: Mock `ExtensionContext.globalState` with in-memory implementation

**Challenge**: Testing platform-specific behavior
- **Solution**: Mock `process.platform` and `process.arch` to simulate different platforms

## Implementation Notes

### VS Code Terminal API Limitations

The VS Code terminal API does not provide a pre-creation hook to modify terminal environment before the terminal is created. The available approaches are:

1. **Custom Terminal Profile** (Recommended):
   - Register a custom terminal profile with modified environment
   - Set as default or provide as option to users
   - Most reliable approach

2. **Wrapper Command**:
   - Provide "OpenDaemon: New Terminal with CLI" command
   - Creates terminal with injected PATH
   - Requires user to use command instead of default terminal creation

3. **Shell Integration**:
   - Modify user's shell profile (.bashrc, .zshrc, etc.)
   - Requires user permission and is platform-specific
   - Most intrusive but works everywhere

**Chosen Approach**: Combination of #1 and #2
- Register custom terminal profile for automatic integration
- Provide command for explicit terminal creation
- Show notification explaining both options

### Platform-Specific Considerations

**Windows**:
- Binary has `.exe` extension
- PATH separator is `;`
- No execute permissions needed
- PowerShell and CMD have different environment variable syntax

**macOS**:
- Binary has no extension
- PATH separator is `:`
- Execute permissions required (`chmod +x`)
- May need to handle Gatekeeper security on first run

**Linux**:
- Binary has no extension
- PATH separator is `:`
- Execute permissions required (`chmod +x`)
- May need to handle different shell environments (bash, zsh, fish)

### Security Considerations

- Extension only modifies terminal-specific environment, not system PATH
- Binary is bundled with extension and verified before use
- No network requests or external dependencies
- User can inspect binary location and permissions
- Global installation is optional and requires explicit user action

### Performance Considerations

- Platform detection and binary resolution happen once during activation
- Binary verification is async and non-blocking
- Terminal interception has minimal overhead
- State persistence uses VS Code's built-in mechanisms

### Future Enhancements

- Auto-detect if user has manually added CLI to system PATH
- Provide shell completion scripts for dmn commands
- Add telemetry to track CLI usage patterns
- Support custom binary locations for development
- Integrate with VS Code's shell integration API when available
