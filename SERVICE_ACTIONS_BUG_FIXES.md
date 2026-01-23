# Service Actions Bug Fixes

## Date: January 23, 2026 (Updated - Round 2)

## Issues Reported

### From User Testing (Round 1)

1. **Terminal not showing service output** - Terminal showed empty PowerShell prompt instead of service logs
2. **RPC timeouts** - `startService` and `stopService` requests timing out after 2 minutes
3. **Terminal not closing** - Terminals remained open when services were stopped
4. **getDependencies not found** - RPC error "Method not found: getDependencies"
5. **Status not accurate** - Services stuck in "Starting" status forever

### From User Testing (Round 2)

6. **Dependencies not starting in order** - When starting frontend, database and backend-api don't start first
7. **Terminal not closing on failure** - When service fails, terminal should be closed
8. **Daemon binary not being used** - Extension was using bundled binary instead of locally built one

### From Activity Logs
```
[2026-01-23T14:47:35.181Z] RPC [getDependencies] request - error: Method not found: getDependencies
[2026-01-23T14:49:35.186Z] ERROR in RPC request startService: Request timeout: startService
[2026-01-23T15:04:31.497Z] ERROR in RPC request stopService: Request timeout: stopService
```

## Root Causes Identified

1. **Terminal Issue**: Used "real" VS Code terminals which can't receive injected log output. Services run in the daemon, not in the terminal shell.

2. **Timeout Issue**: `start_service_with_deps()` blocked synchronously waiting for ready conditions (up to 60 seconds per service). Combined with dependency chains, this easily exceeded the 2-minute RPC timeout.

3. **Terminal Cleanup Issue**: Terminal was only closed in the success path of `stopService`. If RPC timed out, cleanup never happened.

4. **getDependencies Issue**: Extension was loading the bundled binary from `extension/bin/` instead of the locally built one at `target/release/dmn.exe`. The bundled binary was outdated.

5. **Status Issue**: Tree view updates depended on RPC response. With timeouts, the response never came, leaving status stale.

6. **Dependency Fallback Issue**: When `getDependencies` RPC failed, no fallback existed to read dependencies from the config file.

## Fixes Applied

### 1. Pseudoterminal Implementation (`terminal-manager.ts`)

```typescript
class ServicePseudoterminal implements vscode.Pseudoterminal {
    private writeEmitter = new vscode.EventEmitter<string>();
    
    // Can write log lines directly
    writeLogLine(logLine: LogLine): void {
        const formattedLine = `${timestamp} ${content}\r\n`;
        this.write(formattedLine);
    }
    
    // Can handle stdin from user
    handleInput(data: string): void {
        this.stdinWriter(this.serviceName, data);
    }
}
```

### 2. Non-Blocking Service Start (`orchestrator.rs`)

Changed from:
```rust
// OLD: Blocking - waited for ready condition
let result = watcher.watch_service_with_timeout(...).await;
match result {
    Ok(_) => emit_ready(),
    Err(_) => emit_failed()
}
```

To:
```rust
// NEW: Non-blocking - spawn async task for ready check
tokio::spawn(async move {
    let result = watcher.watch_service_with_timeout(...).await;
    match result {
        Ok(_) => event_tx.send(ServiceReady),
        Err(_) => event_tx.send(ServiceFailed)
    }
});
// Return immediately
Ok(())
```

### 3. Guaranteed Terminal Cleanup (`commands.ts`)

```typescript
// OLD: Cleanup only in success path
try {
    await rpcClient.request('stopService', ...);
    this.terminalManager.closeTerminal(serviceName);
} catch (err) {
    // Terminal never closed on error!
}

// NEW: Cleanup in finally block
try {
    await rpcClient.request('stopService', ...);
} finally {
    this.terminalManager.closeTerminal(serviceName);
    treeDataProvider.updateServiceStatus(serviceName, Stopped);
}
```

### 4. Log Line Routing to Terminal (`extension.ts`)

```typescript
if (method === 'logLine') {
    // Route to terminal for real-time display
    const terminalManager = commandManager.getTerminalManager();
    terminalManager.writeLogLine(service, logLine);
    
    // Also route to LogManager for editor viewing
    logManager.appendLogLine(service, logLine);
}
```

### 5. Use Local Daemon Binary (`daemon.ts`)

```typescript
// OLD: Only used bundled binary
const binPath = path.join(this.context.extensionPath, 'bin', binaryName);

// NEW: Check for local build first, fall back to bundled
const releasePath = path.join(workspaceRoot, 'target', 'release', 'dmn.exe');
if (fs.existsSync(releasePath)) {
    return releasePath;  // Use local build
}
// Fall back to bundled binary
```

### 6. Dependency Fallback (`commands.ts`)

```typescript
// OLD: Only used RPC
const response = await rpcClient.request('getDependencies', { service: name });
deps = response?.dependencies || [];

// NEW: Fall back to config if RPC fails
try {
    const response = await rpcClient.request('getDependencies', { service: name });
    deps = response?.dependencies || [];
} catch {
    // RPC failed, read from config file
    if (configServices && configServices[name]) {
        deps = configServices[name].depends_on || [];
    }
}
```

### 7. Close Terminal on Start Failure (`commands.ts`)

```typescript
} catch (err) {
    // Close terminal since service failed to start
    this.terminalManager.closeTerminal(targetItem.serviceName);
    
    // Update status to failed
    treeDataProvider.updateServiceStatus(targetItem.serviceName, ServiceStatus.Failed);
}
```

## Files Changed

| File | Changes |
|------|---------|
| `extension/src/terminal-manager.ts` | Complete rewrite with `ServicePseudoterminal` |
| `extension/src/extension.ts` | Log routing to terminals, stdin writer setup |
| `extension/src/commands.ts` | try/finally for terminal cleanup, dependency fallback, close terminal on failure |
| `extension/src/daemon.ts` | Check for local build before using bundled binary |
| `core/src/orchestrator.rs` | Non-blocking `start_service_with_deps` |
| `extension/src/test/suite/terminal-manager.test.ts` | Updated for new API |

## Testing Checklist

- [ ] Start database service → Terminal shows logs with timestamps
- [ ] Start frontend service → Dependencies (backend-api, database) start first
- [ ] Service shows "Starting" then "Running" status
- [ ] Stop service → Terminal closes, status shows "Stopped"
- [ ] No RPC timeout errors in activity log
- [ ] getDependencies returns correct dependencies

## Build Commands

```powershell
# Rebuild daemon
cd "f:\test apps\opendaemon"
cargo build --release

# Rebuild extension
cd "f:\test apps\opendaemon\extension"
npm run compile

# Reload VS Code window to apply changes
# Press Ctrl+Shift+P → "Developer: Reload Window"
```
