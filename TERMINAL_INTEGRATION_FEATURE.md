# Terminal Integration Feature

## Overview

OpenDaemon now supports **real-time terminal integration** for service logs! Instead of creating log files, each service gets its own named terminal tab in VS Code where logs are streamed in real-time.

## What Changed

### Before
- Services logged to files or output channels
- Had to manually open and refresh log files
- Limited interactivity

### After
- Each service gets a dedicated terminal: `dmn: <service-name>`
- Real-time log streaming
- Full terminal interactivity - you can run commands in the same terminal!
- Named tabs for easy navigation

## Key Features

### 1. Named Terminals
Each service gets a terminal with a clear name:
- `dmn: database`
- `dmn: backend-api`
- `dmn: frontend`

### 2. Real-Time Streaming
Logs appear instantly as services produce output - no refresh needed!

### 3. Interactive
These are real VS Code terminals, so you can:
- Run additional commands
- Interact with services
- Copy/paste easily
- Search logs with Ctrl+F

### 4. Easy Navigation
- Click terminal tabs to switch between services
- Split terminals to view multiple services side-by-side
- Use the terminal dropdown for quick access

## How to Use

### Opening a Service Terminal

**Method 1: From Tree View**
1. Right-click on a service in the OpenDaemon Services view
2. Select "Open Terminal"

**Method 2: Command Palette**
1. Press `Ctrl+Shift+P`
2. Type "OpenDaemon: Open Terminal"
3. Select your service

**Method 3: Automatic**
- Terminals are automatically created when services start

### Managing Terminals

- **Switch Services:** Click terminal tabs at the bottom
- **Close Terminal:** Click the trash icon on the tab
- **Clear Output:** Type `cls` (Windows) or `clear` (Unix)
- **Split View:** Right-click tab → "Split Terminal"

## Implementation Details

### New Files Created

1. **`extension/src/terminal-manager.ts`**
   - Core terminal management logic
   - Creates and tracks terminals for each service
   - Handles log streaming to terminals
   - Manages terminal lifecycle

2. **`extension/docs/terminal-integration.md`**
   - User-facing documentation
   - Usage examples and tips
   - Troubleshooting guide

### Modified Files

1. **`extension/src/commands.ts`**
   - Added `TerminalManager` integration
   - New `showTerminal()` command
   - Terminal creation on service start

2. **`extension/package.json`**
   - Added `opendaemon.showTerminal` command
   - Added terminal icon to context menu
   - Registered command in menus

3. **`extension/src/test/suite/command-palette-no-services.test.ts`**
   - Fixed type casting for test compatibility

## API Reference

### TerminalManager Class

```typescript
class TerminalManager {
    // Get or create a terminal for a service
    getOrCreateTerminal(serviceName: string): vscode.Terminal
    
    // Show a service's terminal
    showTerminal(serviceName: string, preserveFocus?: boolean): void
    
    // Write lines to terminal
    writeLines(serviceName: string, lines: string[]): void
    
    // Clear terminal
    clearTerminal(serviceName: string): void
    
    // Close terminal
    closeTerminal(serviceName: string): void
    
    // Close all terminals
    closeAllTerminals(): void
    
    // Check if terminal exists
    hasTerminal(serviceName: string): boolean
    
    // Get active terminal names
    getActiveTerminals(): string[]
}
```

## Benefits

### For Users
✅ **Real-time visibility** - See logs as they happen
✅ **Better UX** - Familiar terminal interface
✅ **Interactive** - Run commands in service context
✅ **Easy navigation** - Named tabs and quick switching
✅ **No file clutter** - No log files to manage

### For Developers
✅ **Cleaner code** - No file I/O management
✅ **Better debugging** - Live output for troubleshooting
✅ **Resource efficient** - Logs stay in memory
✅ **VS Code native** - Uses built-in terminal API

## Example Workflow

1. **Start your services:**
   ```
   Click "Start All" in OpenDaemon view
   ```

2. **Terminals automatically open:**
   - `dmn: database` - Shows PostgreSQL startup
   - `dmn: backend-api` - Shows API server logs
   - `dmn: frontend` - Shows Vite dev server

3. **Monitor in real-time:**
   - Switch between tabs to check each service
   - Split terminals to view multiple at once
   - Search logs with Ctrl+F

4. **Debug issues:**
   - See errors immediately
   - Run diagnostic commands in the same terminal
   - Copy error messages for searching

## Configuration

No additional configuration needed! Works with your existing `dmn.json`:

```json
{
    "version": "1.0",
    "services": {
        "database": {
            "command": "docker run postgres",
            "ready_when": {
                "type": "log_contains",
                "pattern": "ready to accept connections"
            }
        },
        "backend-api": {
            "command": "npm run dev",
            "depends_on": ["database"]
        }
    }
}
```

## Future Enhancements

Potential improvements:
- [ ] Automatic log filtering by level (info/warn/error)
- [ ] Terminal persistence across VS Code restarts
- [ ] Custom terminal themes per service type
- [ ] Log export functionality
- [ ] Terminal grouping for related services

## Testing

To test the feature:

1. **Build the extension:**
   ```bash
   cd extension
   npm run compile
   ```

2. **Press F5** to launch Extension Development Host

3. **Open a workspace** with `dmn.json`

4. **Start a service** and watch the terminal appear!

5. **Right-click a service** → "Open Terminal" to manually open

## Troubleshooting

### Terminal not showing logs?
- Check service is running (tree view status)
- Verify service produces output
- Try restarting the service

### Too many terminals?
- Close unused terminals (trash icon)
- Use split view for active services
- Terminals auto-cleanup when closed

### Terminal closed accidentally?
- Right-click service → "Open Terminal"
- Recent logs will be fetched automatically

## Summary

This feature transforms OpenDaemon from a background service manager into an interactive development tool. Users can now see exactly what their services are doing in real-time, with the full power of VS Code's terminal at their fingertips!

**Key Takeaway:** Instead of hunting through log files, developers now have live, interactive terminals for each service - making debugging and monitoring significantly easier.
