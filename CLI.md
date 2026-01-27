# OpenDaemon CLI Integration

This document provides a complete reference for the OpenDaemon CLI integration, including architecture, implementation details, problems encountered, and solutions.

## Overview

The OpenDaemon CLI integration allows users to run `dmn` commands directly from the terminal within VS Code and Cursor IDE. The goal is a seamless, native-feeling experience where users can simply install the extension and immediately use the CLI without manual PATH configuration.

**Key Features:**
- Automatic PATH injection for new terminals
- Cross-platform support (Windows, macOS, Linux)
- Works in both VS Code and Cursor IDE
- No manual installation steps required
- Comprehensive logging for troubleshooting

---

## Architecture

### Directory Structure

```
extension/
├── src/
│   ├── cli-integration/
│   │   ├── cli-integration-manager.ts   # Main coordinator
│   │   ├── cli-logger.ts                # Logging utilities
│   │   ├── terminal-interceptor.ts      # PATH injection
│   │   ├── terminal-profile-provider.ts # Terminal profile
│   │   ├── platform-detector.ts         # OS/arch detection
│   │   ├── binary-resolver.ts           # Binary path resolution
│   │   ├── binary-verifier.ts           # Binary verification
│   │   └── notification-manager.ts      # User notifications
│   └── extension.ts                     # Extension entry point
├── bin/
│   ├── dmn.exe                          # Windows executable (copy)
│   ├── dmn-win32-x64.exe                # Windows platform binary
│   ├── dmn.cmd                          # Windows wrapper (backup)
│   └── dmn                              # Unix wrapper script
└── package.json                         # Extension manifest
```

### Component Overview

| Component | Purpose |
|-----------|---------|
| `CLIIntegrationManager` | Main coordinator for CLI lifecycle |
| `TerminalInterceptor` | Injects PATH via terminal settings |
| `TerminalProfileProvider` | Provides selectable terminal profile |
| `CLILogger` | Outputs logs to "OpenDaemon CLI" channel |
| `PlatformDetector` | Detects OS and architecture |
| `BinaryResolver` | Resolves platform-specific binary path |
| `BinaryVerifier` | Verifies binary existence/permissions |
| `NotificationManager` | Shows user notifications |

---

## Files Reference

### 1. `cli-integration-manager.ts`

**Purpose:** Main coordinator that orchestrates the CLI integration lifecycle.

**Responsibilities:**
- Initialize platform detection
- Resolve binary paths
- Verify binary exists and has permissions
- Start terminal interceptor
- Show notifications
- Provide diagnostic functionality

**Key Methods:**
```typescript
async activate(): Promise<void>        // Activates CLI integration
async createTerminalWithCLI(): Promise<vscode.Terminal>  // Creates terminal with CLI
async runDiagnostics(): Promise<void>  // Runs diagnostic checks
async deactivate(): Promise<void>      // Cleans up resources
```

### 2. `terminal-interceptor.ts`

**Purpose:** Manages PATH injection into VS Code/Cursor terminals.

**Key Mechanisms:**
1. **Primary:** Modifies `terminal.integrated.env.*` settings
2. **Secondary:** Registers terminal profile provider as fallback

**Critical Implementation Details:**

```typescript
// Writes to BOTH user and workspace settings for maximum compatibility
await config.update(this.envConfigKey, newEnv, vscode.ConfigurationTarget.Global);
await config.update(this.envConfigKey, newEnv, vscode.ConfigurationTarget.Workspace);
```

**Why Both Settings?**
- Cursor IDE may only read user-level settings
- VS Code typically reads workspace settings
- Writing to both ensures compatibility across both IDEs

### 3. `cli-logger.ts`

**Purpose:** Provides a dedicated output channel for CLI-related logs.

**Output Channel:** "OpenDaemon CLI"

**Log Levels:**
- `info()` - Informational messages
- `warn()` - Warnings
- `error()` - Errors
- `debug()` - Debug information

**Special Methods:**
- `logSystemInfo()` - Logs platform, arch, Node version
- `logWorkspaceInfo()` - Logs workspace folders
- `logTerminalSettings()` - Logs terminal configuration

### 4. `terminal-profile-provider.ts`

**Purpose:** Provides an "OpenDaemon CLI" terminal profile in the terminal dropdown.

**Registration:**
```typescript
vscode.window.registerTerminalProfileProvider('opendaemon.terminal', provider);
```

This creates a selectable option in the terminal dropdown menu as a fallback mechanism.

### 5. `platform-detector.ts`

**Purpose:** Detects the current platform and architecture.

**Returns:**
```typescript
interface PlatformInfo {
  os: 'win32' | 'darwin' | 'linux';
  arch: 'x64' | 'arm64';
}
```

### 6. `binary-resolver.ts`

**Purpose:** Resolves the platform-specific binary path.

**Binary Naming Convention:**
- Windows: `dmn-win32-x64.exe`
- macOS Intel: `dmn-darwin-x64`
- macOS Apple Silicon: `dmn-darwin-arm64`
- Linux x64: `dmn-linux-x64`
- Linux ARM64: `dmn-linux-arm64`

**Returns:**
```typescript
interface BinaryInfo {
  name: string;      // e.g., "dmn-win32-x64.exe"
  fullPath: string;  // Full path to binary
  binDir: string;    // Directory containing binary
}
```

### 7. `binary-verifier.ts`

**Purpose:** Verifies the binary exists and has execute permissions.

**Checks:**
1. File existence
2. Execute permissions (important on Unix)

**Auto-fix:** Attempts to set execute permissions if missing (Unix only).

### 8. `notification-manager.ts`

**Purpose:** Manages user-facing notifications.

**Notifications:**
- First-time setup notification
- Error notifications
- CLI info notification
- Global installation instructions

---

## Extension Activation Flow

```
1. Extension activates (workspaceContains:dmn.json)
   │
2. Initialize CLI Integration
   │
   ├─ 2.1 Detect platform (OS, architecture)
   │
   ├─ 2.2 Resolve binary path
   │
   ├─ 2.3 Verify binary exists and has permissions
   │
   ├─ 2.4 Initialize TerminalInterceptor
   │
   ├─ 2.5 Inject PATH via terminal settings
   │      ├─ Write to user settings (Global)
   │      └─ Write to workspace settings
   │
   ├─ 2.6 Register terminal profile provider
   │
   └─ 2.7 Show first-time notification
   │
3. Find dmn.json in workspace
   │
4. Initialize daemon with config
   │
5. Start RPC client and services UI
```

---

## PATH Injection Mechanism

### How It Works

1. **Get current system PATH:**
   ```typescript
   const systemPath = process.env.PATH || process.env.Path || '';
   ```

2. **Prepend extension bin directory:**
   ```typescript
   const newPath = `${this.binDir}${pathSeparator}${systemPath}`;
   ```

3. **Update terminal settings:**
   ```typescript
   // User-level settings (more reliable for Cursor)
   await config.update('env.windows', newEnv, ConfigurationTarget.Global);
   
   // Workspace settings (backup)
   await config.update('env.windows', newEnv, ConfigurationTarget.Workspace);
   ```

### Why Full PATH Instead of Variable Substitution?

**DOES NOT WORK:**
```json
{
  "terminal.integrated.env.windows": {
    "Path": "C:\\path\\to\\bin;${env:Path}"
  }
}
```
The `${env:Path}` is NOT expanded - terminals literally see the string `${env:Path}`.

**WORKS:**
```json
{
  "terminal.integrated.env.windows": {
    "Path": "C:\\path\\to\\bin;C:\\Windows\\system32;..."
  }
}
```
Using the full PATH value ensures it's applied correctly.

---

## Windows-Specific Implementation

### The dmn.exe Solution

**Problem:** Windows command resolution relies on `.exe` files. Using only `dmn.cmd` wrapper was unreliable because:
1. PowerShell doesn't always resolve `.cmd` files from PATH
2. Cursor's terminal settings weren't being applied consistently

**Solution:** Create `dmn.exe` as a direct copy of `dmn-win32-x64.exe`:
```powershell
# In bundle-extension.ps1
Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe
```

**Result:** Windows finds `dmn.exe` directly without wrapper resolution.

### Files in bin/ for Windows

| File | Purpose |
|------|---------|
| `dmn.exe` | **Primary** - Direct executable users call |
| `dmn-win32-x64.exe` | Original platform binary |
| `dmn.cmd` | Backup wrapper script |

---

## Problems Encountered and Solutions

### Problem 1: `${env:PATH}` Variable Substitution Doesn't Work

**Symptom:** Terminal couldn't find `dmn` command even though PATH was set in settings.

**Root Cause:** VS Code's `terminal.integrated.env.*` doesn't expand `${env:PATH}` - it's used literally.

**Solution:** Use the full PATH value from `process.env.PATH` instead of variable substitution.

---

### Problem 2: Workspace Settings Not Applied in Cursor

**Symptom:** CLI worked in VS Code but not in Cursor IDE. Services panel didn't appear.

**Root Cause:** Cursor IDE wasn't consistently reading `terminal.integrated.env.windows` from workspace settings.

**Solution:** Write to BOTH user-level (Global) AND workspace settings:
```typescript
await config.update(envConfigKey, newEnv, ConfigurationTarget.Global);
await config.update(envConfigKey, newEnv, ConfigurationTarget.Workspace);
```

---

### Problem 3: PowerShell Didn't Find dmn.cmd

**Symptom:** Even with PATH set, `dmn` command wasn't found in PowerShell.

**Root Cause:** PowerShell's command resolution prioritizes `.exe` files. The `dmn.cmd` wrapper wasn't being found reliably.

**Solution:** Create `dmn.exe` as a direct copy of the platform binary:
```powershell
Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe
```

---

### Problem 4: Services Panel Not Showing in Cursor

**Symptom:** CLI logs showed activation complete, but no services appeared in the tree view.

**Root Cause:** Diagnostic logs were using `console.log()` which goes to Developer Tools, not the Output panel. Couldn't see what was failing.

**Solution:** Route all diagnostic logs through the CLI logger:
```typescript
const logger = getCLILogger();
logger.info('Daemon Initialization Start');
// ... diagnostic logging
```

**Additional Fix:** The daemon initialization was failing silently. Added proper error handling and logging throughout the activation flow.

---

### Problem 5: Existing Terminals Don't Get Updated PATH

**Symptom:** Users had to open a NEW terminal for `dmn` to work.

**Root Cause:** Terminal environment is set when the terminal starts. Changing settings doesn't affect running terminals.

**Solution:** This is expected behavior. The extension:
1. Shows a notification about opening a new terminal
2. Logs: "NOTE: You must open a NEW terminal for changes to take effect"
3. Provides an "OpenDaemon CLI" terminal profile as an alternative

---

## Build and Bundle Process

### Bundle Script (bundle-extension.ps1)

```powershell
# 1. Create bin directory
New-Item -ItemType Directory -Force -Path extension/bin

# 2. Copy platform binaries
Copy-Item dist/dmn-win32-x64.exe extension/bin/

# 3. Create dmn.exe (CRITICAL for Windows)
Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe

# 4. Create wrapper scripts
# dmn.cmd for Windows (backup)
# dmn for Unix
```

### Package Extension

```bash
cd extension
npm run compile
npx vsce package --allow-missing-repository
```

### Install Extension

```bash
# VS Code
code --install-extension opendaemon-0.1.0.vsix

# Cursor IDE
cursor --install-extension opendaemon-0.1.0.vsix
```

---

## Configuration Files

### package.json Contributions

```json
{
  "contributes": {
    "terminal": {
      "profiles": [{
        "id": "opendaemon.terminal",
        "title": "OpenDaemon CLI"
      }]
    },
    "commands": [
      {
        "command": "opendaemon.newTerminalWithCLI",
        "title": "New Terminal with CLI",
        "category": "OpenDaemon"
      },
      {
        "command": "opendaemon.showCLILogs",
        "title": "Show CLI Logs",
        "category": "OpenDaemon"
      },
      {
        "command": "opendaemon.runCLIDiagnostics",
        "title": "Run CLI Diagnostics",
        "category": "OpenDaemon"
      }
    ]
  }
}
```

### .vscodeignore

```
# Ensure bin directory is included
!bin/**
```

---

## Diagnostic Tools

### Run CLI Diagnostics Command

Access via: `Ctrl+Shift+P` → "OpenDaemon: Run CLI Diagnostics"

**Checks performed:**
1. Binary directory existence
2. Files in bin directory
3. Presence of `dmn.exe` (Windows)
4. Terminal settings configuration
5. PATH contains bin directory

### Output Channels

| Channel | Purpose |
|---------|---------|
| OpenDaemon CLI | CLI integration logs, PATH injection, diagnostics |
| OpenDaemon Activity | Service lifecycle, RPC communication |

---

## Testing Checklist

### VS Code Testing
- [ ] Install extension from VSIX
- [ ] Open workspace with dmn.json
- [ ] Verify services panel appears
- [ ] Open new terminal
- [ ] Run `dmn --version`
- [ ] Start/stop services from UI

### Cursor IDE Testing
- [ ] Install extension from VSIX
- [ ] Open workspace with dmn.json
- [ ] Verify services panel appears
- [ ] Check CLI output channel for errors
- [ ] Open new terminal
- [ ] Run `dmn --version`
- [ ] Start/stop services from UI

---

## Troubleshooting

### dmn command not found

1. **Open a NEW terminal** (existing terminals won't have updated PATH)
2. Check CLI output channel for errors
3. Run diagnostic command: "OpenDaemon: Run CLI Diagnostics"
4. Verify PATH in terminal:
   ```powershell
   $env:PATH -split ';' | Select-String 'opendaemon'
   ```

### Services panel not showing

1. Check CLI output channel for "Daemon Initialization" logs
2. Verify dmn.json exists in workspace root
3. Check for errors in initialization:
   - `findDmnConfig: NOT FOUND` → dmn.json missing
   - `Error initializing daemon` → Check daemon logs

### Extension works in VS Code but not Cursor

1. Cursor may need a reload after extension install
2. Check that user settings contain PATH:
   ```powershell
   cat "$env:APPDATA\Cursor\User\settings.json" | Select-String 'terminal.integrated.env'
   ```

---

## Version History

### v0.1.0 (2026-01-27)
- Initial CLI integration implementation
- PATH injection via terminal.integrated.env.*
- Windows: dmn.exe direct copy solution
- Dual settings write (user + workspace) for Cursor compatibility
- Comprehensive logging to output channel
- Diagnostic command for troubleshooting

---

## Summary

The CLI integration achieves seamless `dmn` command availability through:

1. **Automatic PATH injection** - Adds extension bin directory to terminal PATH
2. **Multi-IDE support** - Works in both VS Code and Cursor IDE
3. **Robust Windows support** - `dmn.exe` copy eliminates wrapper issues
4. **Comprehensive logging** - All diagnostics visible in output channel
5. **Graceful degradation** - Terminal profile provider as fallback

Users simply install the extension, open a new terminal, and run `dmn` commands immediately.
