# Automatic Terminal Profile Registration - Implementation Summary

## Overview

Implemented automatic CLI availability for the `dmn` command in all VS Code terminals. The extension now injects the CLI binary path into the terminal environment automatically.

## Problem History

### Initial Problem
The README claimed that `dmn` would work automatically in all VS Code terminals after installing the extension, but the implementation only made it available when users explicitly used the "OpenDaemon: New Terminal with CLI" command.

### First Attempt (Terminal Profile Provider)
The first attempt used VS Code's `registerTerminalProfileProvider` API and tried to set it as the default profile:
- Created a terminal profile provider
- Registered it with `vscode.window.registerTerminalProfileProvider()`
- Tried to set `terminal.integrated.defaultProfile.*` to the profile ID

**Why it didn't work**: VS Code's `defaultProfile.*` setting expects a profile name that matches built-in profiles or profiles defined in `terminal.integrated.profiles.*`. Extension-contributed profile providers only create a selectable option in the dropdown - they cannot be set as the default this way.

### Second Attempt (Variable Substitution)
Tried using `terminal.integrated.env.*` with variable substitution:
```json
{
  "terminal.integrated.env.windows": {
    "Path": "c:\\path\\to\\bin;${env:Path}"
  }
}
```

**Why it didn't work**: VS Code's `${env:Path}` variable substitution in terminal environment settings **doesn't actually work**. The terminal literally received the string `${env:Path}` instead of the expanded PATH value.

### Third Attempt (Full PATH Value)
Fixed the variable substitution issue by using the full PATH value from `process.env.PATH`:
```json
{
  "terminal.integrated.env.windows": {
    "Path": "c:\\path\\to\\bin;C:\\Python313\\Scripts\\;...full system PATH..."
  }
}
```

**Status**: PATH injection now works! Diagnostics showed:
- ✅ `$env:Path -match 'opendaemon'` returns `True`
- ✅ Binary directory is in the PATH
- ✅ Binary works when called with full path

**BUT**: `dmn --version` still fails because the binary is named `dmn-win32-x64.exe`, not `dmn.exe`!

### Fourth Attempt (IMPLEMENTED) - Wrapper Script
**Root Cause Identified**: The binary filename is platform-specific (`dmn-win32-x64.exe`), but users expect to type just `dmn`.

**Solution**: Created wrapper scripts that call the actual binary:
- `dmn.cmd` for Windows - calls `dmn-win32-x64.exe`
- `dmn` (shell script) for Unix - detects platform and calls appropriate binary

**Status**: ✅ Implemented and packaged

## Current Implementation

### How It Works

1. **Extension Activation**:
   - CLI Integration Manager activates
   - Verifies binary exists
   - Creates Terminal Interceptor with bin directory
   - Calls `interceptor.start()`

2. **PATH Injection via Settings**:
   - Interceptor updates `terminal.integrated.env.windows` (or `env.osx`/`env.linux`)
   - Uses the FULL system PATH value (not variable substitution which doesn't work)
   - This affects ALL new terminals automatically

3. **Wrapper Scripts** (NEW):
   - `dmn.cmd` for Windows - calls `dmn-win32-x64.exe`
   - `dmn` (shell script) for Unix - calls the appropriate binary
   - These allow users to type just `dmn` instead of the full binary name

4. **Secondary: Profile Provider**:
   - Also registers a terminal profile provider as a fallback
   - Creates a selectable "OpenDaemon CLI" profile in the terminal dropdown

5. **Terminal Creation**:
   - User opens any new terminal
   - VS Code applies the environment settings automatically
   - Terminal starts with `dmn` command available in PATH

## Diagnostic Results (2026-01-27)

Testing in VS Code terminal revealed:

```powershell
PS> $env:Path.Split(';')[0..5]
C:\Program Files\PowerShell\7
c:\Users\...\Code\User\globalStorage\github.copilot-chat\debugCommand
c:\Users\...\Code\User\globalStorage\github.copilot-chat\copilotCli
c:\Users\...\.vscode\extensions\opendaemon.opendaemon-0.1.0\bin   # ← OUR BIN DIR!
C:\Python313\Scripts\
C:\Python313\

PS> $env:Path -match 'opendaemon'
True  # ← PATH INJECTION WORKS!

PS> & "c:\Users\...\.vscode\extensions\opendaemon.opendaemon-0.1.0\bin\dmn-win32-x64.exe" --version
dmn 0.1.0  # ← BINARY WORKS!

PS> dmn --version
dmn: The term 'dmn' is not recognized...  # ← FAILS - no dmn.exe, only dmn-win32-x64.exe
```

**Conclusion**: PATH injection works perfectly. The issue is the binary naming.

## Files Created

- `extension/src/cli-integration/terminal-profile-provider.ts` - Profile provider
- `extension/src/cli-integration/cli-logger.ts` - CLI debugging output channel
- `extension/src/test/suite/terminal-profile-provider.property.test.ts` - Property tests
- `extension/src/test/suite/terminal-profile-provider.test.ts` - Unit tests

## Files Modified

- `extension/src/cli-integration/terminal-interceptor.ts` - PATH injection via settings
- `extension/src/cli-integration/cli-integration-manager.ts` - Added logging
- `extension/src/extension.ts` - Added "Show CLI Logs" command
- `extension/package.json` - Added CLI log command

## Key Features

- **CLI Output Channel**: "OpenDaemon CLI" output for debugging PATH injection
- **Full PATH Injection**: Uses actual PATH values, not broken variable substitution
- **Cross-Platform**: Handles Windows (`;`) and Unix (`:`) PATH separators
- **Workspace-Scoped**: Only affects current workspace, not user settings
- **Graceful Degradation**: Falls back to manual command if injection fails
- **Backward Compatible**: "New Terminal with CLI" command still works

## Next Steps

1. ✅ **Create wrapper scripts** - DONE
   - `extension/bin/dmn.cmd` for Windows
   - `extension/bin/dmn` for Unix

2. ✅ **Update bundling scripts** - DONE
   - `scripts/bundle-extension.ps1` now creates wrapper scripts automatically

3. 🔄 **Test** that `dmn --version` works - PENDING USER VERIFICATION

## Testing Instructions

1. Reload VS Code (Ctrl+Shift+P → "Developer: Reload Window")
2. Delete `.vscode/settings.json` or clear `terminal.integrated.env.windows`
3. Open a NEW terminal (Ctrl+`)
4. Run: `dmn --version`
5. Expected: `dmn 0.1.0`

## Compilation Status

```
npm run compile
> tsc -p ./
Exit Code: 0
```

All TypeScript compiles successfully with no errors.
