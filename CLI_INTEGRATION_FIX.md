# CLI Integration Fix - Complete History

This document tracks all attempts to make the `dmn` CLI command work natively in VS Code terminals.

## Goal

Allow users to type `dmn` in any VS Code terminal after installing the OpenDaemon extension, without any manual PATH configuration.

---

## Attempt 1: Terminal Profile Provider + Default Profile

**Date**: Initial implementation

**Approach**:
1. Register a terminal profile provider using `vscode.window.registerTerminalProfileProvider()`
2. Set `terminal.integrated.defaultProfile.windows` to the profile ID

**Code**:
```typescript
this.profileDisposable = vscode.window.registerTerminalProfileProvider(
    'opendaemon.terminal',
    provider
);
await config.update('defaultProfile.windows', 'opendaemon.terminal', ConfigurationTarget.Workspace);
```

**Result**: ❌ Failed

**Why**: VS Code's `defaultProfile.*` setting expects a profile name that matches built-in profiles (like "PowerShell", "Command Prompt") or profiles defined in `terminal.integrated.profiles.*`. Extension-contributed profile providers only create a selectable option in the dropdown - they cannot be set as the default this way.

---

## Attempt 2: Add Profile Declaration to package.json

**Date**: After Attempt 1

**Approach**:
Added terminal profile declaration to `package.json`:

```json
"contributes": {
  "terminal": {
    "profiles": [
      {
        "id": "opendaemon.terminal",
        "title": "OpenDaemon CLI"
      }
    ]
  }
}
```

**Result**: ❌ Still failed

**Why**: Even with the declaration, setting `defaultProfile.windows` to `"opendaemon.terminal"` doesn't make VS Code use our profile provider for new terminals.

---

## Attempt 3: terminal.integrated.env.* with Variable Substitution

**Date**: 2026-01-27

**Approach**:
Use VS Code's official `terminal.integrated.env.*` settings to inject PATH:

```json
{
  "terminal.integrated.env.windows": {
    "Path": "c:\\path\\to\\bin;${env:Path}",
    "PATH": "c:\\path\\to\\bin;${env:PATH}"
  }
}
```

**Result**: ❌ Failed

**Why**: VS Code's `${env:Path}` variable substitution in terminal environment settings **doesn't actually work**. The terminal literally received the string `${env:Path}` instead of the expanded PATH value.

**Evidence from logs**:
```
"Path": "c:\\...\\bin;${env:Path}"  ← Not expanded!
```

---

## Attempt 4: terminal.integrated.env.* with Full PATH Value

**Date**: 2026-01-27

**Approach**:
Instead of variable substitution, use the full PATH value from `process.env.PATH`:

```typescript
const systemPath = process.env.PATH || process.env.Path || '';
const newPath = `${this.binDir}${pathSeparator}${systemPath}`;
newEnv['Path'] = newPath;
newEnv['PATH'] = newPath;
```

**Result**: ✅ PATH injection works!

**Diagnostics**:
```powershell
PS> $env:Path.Split(';')[0..5]
C:\Program Files\PowerShell\7
...
c:\Users\...\.vscode\extensions\opendaemon.opendaemon-0.1.0\bin  ← SUCCESS!
...

PS> $env:Path -match 'opendaemon'
True  ← SUCCESS!

PS> & "c:\...\bin\dmn-win32-x64.exe" --version
dmn 0.1.0  ← BINARY WORKS!
```

**BUT**: `dmn --version` still fails!

**Why**: The binary is named `dmn-win32-x64.exe`, not `dmn.exe`. PowerShell can't find `dmn` because there's no file with that name.

---

## Attempt 5: Wrapper Scripts

**Date**: 2026-01-27

**Approach**:
Create wrapper scripts that call the platform-specific binary:

**Windows (`dmn.cmd`)**:
```cmd
@echo off
REM OpenDaemon CLI wrapper for Windows
REM This script allows users to type 'dmn' instead of 'dmn-win32-x64.exe'
"%~dp0dmn-win32-x64.exe" %*
```

**Result**: ⚠️ Partial - Works with `cmd.exe` but unreliable with PowerShell

**Why**: PowerShell's command resolution can be inconsistent with `.cmd` files, especially when workspace-level settings aren't being picked up by Cursor IDE terminals.

---

## Attempt 6: dmn.exe Direct Copy + User-Level Settings (CURRENT FIX)

**Date**: 2026-01-27

**Root Cause Analysis**:
Investigation revealed multiple issues:

1. **Workspace settings not applied in Cursor**: The `terminal.integrated.env.windows` workspace settings were being written correctly to `.vscode/settings.json`, but Cursor IDE terminals weren't picking them up.

2. **PowerShell `.cmd` resolution**: While PowerShell should find `.cmd` files in PATH, this depends on the PATH actually being set - which wasn't happening with workspace settings alone.

3. **Missing `dmn.exe`**: Windows command resolution prioritizes `.exe` files. Having only `dmn.cmd` meant relying on the PATH being correctly configured AND PowerShell properly resolving `.cmd` files.

**Solution - Three-Pronged Approach**:

### 1. Create `dmn.exe` directly (Most Important)
Copy the platform-specific binary to `dmn.exe`:
```powershell
Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe
```

This ensures Windows finds `dmn.exe` without any wrapper script resolution issues.

**Files in extension/bin after fix**:
- `dmn.exe` (4.5 MB) - Direct copy of the binary
- `dmn-win32-x64.exe` (4.5 MB) - Original platform binary
- `dmn.cmd` (155 B) - Backup wrapper script
- `dmn` (739 B) - Unix wrapper script

### 2. Use User-Level Settings (Global) Instead of Just Workspace
Write PATH settings to BOTH user-level AND workspace settings for maximum compatibility:

```typescript
// User-level (Global) - more reliable for Cursor
await config.update(this.envConfigKey, newEnv, vscode.ConfigurationTarget.Global);

// Also workspace as backup
await config.update(this.envConfigKey, newEnv, vscode.ConfigurationTarget.Workspace);
```

### 3. Add Diagnostic Command
Added `OpenDaemon: Run CLI Diagnostics` command to help users troubleshoot PATH issues.

**Result**: ✅ Should work reliably now

**Updated Bundle Script** (`scripts/bundle-extension.ps1`):
```powershell
# IMPORTANT: Create dmn.exe copy for Windows
if (Test-Path dist/dmn-win32-x64.exe) {
    Copy-Item dist/dmn-win32-x64.exe extension/bin/dmn.exe
    Write-Host "  Created dmn.exe (copy of dmn-win32-x64.exe)" -ForegroundColor Green
}
```

---

## Summary of What Works

| Component | Status | Notes |
|-----------|--------|-------|
| Binary packaging | ✅ | Binary exists in `extension/bin/` |
| Binary execution | ✅ | Works when called with full path |
| PATH injection (user-level) | ✅ | Bin directory added to user terminal settings |
| PATH injection (workspace) | ✅ | Also writes to workspace as backup |
| `dmn.exe` on Windows | ✅ | Direct executable, no wrapper needed |
| Wrapper scripts | ✅ | `dmn.cmd` (backup) and `dmn` (Unix) |
| Diagnostic command | ✅ | "OpenDaemon: Run CLI Diagnostics" |
| `dmn` command | ✅ | Should now work reliably |

---

## Files Modified

### Core Implementation
- `extension/src/cli-integration/terminal-interceptor.ts` - PATH injection (now writes to both user AND workspace settings), diagnostic method
- `extension/src/cli-integration/cli-integration-manager.ts` - Activation flow, diagnostic command support
- `extension/src/cli-integration/terminal-profile-provider.ts` - Profile provider (secondary)
- `extension/src/cli-integration/cli-logger.ts` - Debug output channel
- `extension/src/extension.ts` - Register diagnostic command

### Build/Package
- `scripts/bundle-extension.ps1` - Now creates `dmn.exe` copy + wrapper scripts

### Configuration
- `extension/package.json` - Added `runCLIDiagnostics` command
- `.vscode/settings.json` - Terminal environment settings (auto-generated)
- User settings (`%APPDATA%\Cursor\User\settings.json`) - Terminal env settings now also written here

### Documentation
- `AUTOMATIC_TERMINAL_PROFILE_IMPLEMENTATION.md` - Implementation history
- `CLI_INTEGRATION_FIX.md` - This file

---

## Debug Output Channel

Added "OpenDaemon CLI" output channel for debugging. Access via:
- Command Palette → "OpenDaemon: Show CLI Logs"

Shows:
- System information (platform, architecture, VS Code version)
- Binary verification status
- PATH injection details
- Settings before/after changes

---

## Diagnostic Command

Added `OpenDaemon: Run CLI Diagnostics` command that checks:
1. Binary directory existence
2. Files in bin directory (especially `dmn.exe`)
3. Terminal settings configuration
4. Troubleshooting instructions

Access via Command Palette (Ctrl+Shift+P) → "OpenDaemon: Run CLI Diagnostics"

---

## Next Steps

1. ✅ **Create wrapper scripts** - DONE
2. ✅ **Create dmn.exe directly** - DONE (most reliable)
3. ✅ **Use user-level settings** - DONE (more reliable than workspace-only)
4. ✅ **Add diagnostic command** - DONE
5. ✅ **Update bundle script** - DONE
6. ✅ **Repackage and install** - DONE

## Testing Instructions

1. **Reload Cursor** (Ctrl+Shift+P → "Developer: Reload Window")
2. **Open a NEW terminal** (Ctrl+Shift+` or Terminal → New Terminal)
3. **Run**:
   ```powershell
   dmn --version
   ```
4. **Expected output**: `dmn 0.1.0`

### If Still Not Working

1. Run diagnostic command: Ctrl+Shift+P → "OpenDaemon: Run CLI Diagnostics"
2. Check if PATH contains the extension bin directory:
   ```powershell
   $env:PATH -split ';' | Select-String 'opendaemon'
   ```
3. Manually test the binary:
   ```powershell
   & "$env:USERPROFILE\.vscode\extensions\opendaemon.opendaemon-0.1.0\bin\dmn.exe" --version
   ```
4. Check user settings contain terminal env config:
   ```powershell
   cat "$env:APPDATA\Cursor\User\settings.json" | Select-String 'terminal.integrated.env'
   ```

---

## Lessons Learned

1. **VS Code's `${env:VAR}` doesn't work in terminal.integrated.env**: Must use full values
2. **defaultProfile.* only works with built-in profiles**: Can't set extension profile as default
3. **Binary naming matters**: `dmn.exe` is more reliable than `dmn.cmd` on Windows
4. **User-level settings more reliable than workspace**: Cursor may not pick up workspace-level terminal.integrated.env settings
5. **Debug logging is essential**: The CLI output channel helped identify issues quickly
6. **Direct executable copy is most reliable**: Creating `dmn.exe` as a copy of the platform binary is more robust than relying on `.cmd` wrapper resolution
