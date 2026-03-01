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

---

## Round 3: Service Status Synchronization Fix (Updated)

### Date: January 23, 2026

### Issues Reported

1. **Service status stuck at "Starting"** - Database service showed "Starting" in tree view even though it was running (outputting "DB heartbeat...")
2. **Duplicate serviceStopped notifications** - When stopping services, already-stopped services were getting duplicate notifications
3. **Terminals not closing** - Database terminal remained open after the service was stopped

### Root Causes

1. **Status Update Race Condition**: In `orchestrator.rs`, when a service's ready check takes longer than 100ms, the function returns without waiting for the status update. The spawned task sends the status via a channel, but nothing is listening anymore.

2. **Cascade Stop Logic Bug**: The cascade stopping logic in `stop_service` was checking `is_some()` on process manager status, which includes already-stopped services. This caused:
   - Unnecessary recursive calls for already-stopped services
   - Duplicate `ServiceStopped` events being emitted
   - Confusion in the extension about which services actually needed cleanup

3. **getStatus Inaccuracy**: The `getStatus` RPC handler only checked process manager status, which could be stale (stuck at "Starting" when the ready watcher knew the service was ready).

### Fixes Applied

#### 1. Fix Cascade Stop Logic (`orchestrator.rs`)

Only stop services that are actually running (status `Running` or `Starting`), and skip services that are already stopped:

```rust
pub async fn stop_service(&mut self, service_name: &str) -> Result<(), OrchestratorError> {
    // Check current status - only proceed if service is running/starting
    let current_status = self.process_manager.get_status(service_name);
    let is_already_stopped = matches!(
        current_status,
        Some(Stopped) | Some(Failed { .. }) | None
    );
    
    // If already stopped, just return - no need to cascade or emit events
    if is_already_stopped {
        return Ok(());
    }
    
    // Stop dependents first - only those that are actually running
    for dependent in dependents {
        let dep_needs_stop = matches!(
            dep_status,
            Some(Running) | Some(Starting)
        );
        
        if dep_needs_stop {
            self.stop_service(&dependent).await?;
        }
    }
    
    // Stop the service and emit event (only for services we actually stopped)
    self.process_manager.stop_service(service_name).await?;
    self.emit_event(OrchestratorEvent::ServiceStopped { service });
    Ok(())
}
```

#### 2. Fix getStatus to Check Ready Watcher (`rpc.rs`)

When process manager shows "Starting", check the ready watcher to see if the service is actually ready:

```rust
RpcRequest::GetStatus => {
    let orch = self.orchestrator.lock().await;
    let spawned_statuses = orch.process_manager.get_all_statuses();
    
    // Check ready watcher state for accurate status
    let ready_watcher = orch.ready_watcher().lock().await;
    
    let status_map = orch.config().services.keys()
        .map(|name| {
            let status_str = match spawned_statuses.get(name) {
                Some(Starting) => {
                    // If ready watcher says service is ready, report "running"
                    if ready_watcher.is_ready(name) {
                        "running"
                    } else {
                        "starting"
                    }
                }
                // ... other status handling
            };
            (name.clone(), status_str)
        })
        .collect();
}
```

#### 3. Add ready_watcher() Accessor (`orchestrator.rs`)

Added a public method to access the ready watcher for accurate status reporting:

```rust
pub fn ready_watcher(&self) -> &Arc<Mutex<ReadyWatcher>> {
    &self.ready_watcher
}
```

### Files Changed

| File | Changes |
|------|---------|
| `core/src/orchestrator.rs` | Fixed stop_service cascade logic; added ready_watcher() accessor |
| `core/src/rpc.rs` | getStatus now checks ready_watcher for accurate status |
| `extension/src/commands.ts` | Added refreshServices() after start/stop operations |

---

## Testing Checklist

- [ ] Start database service → Terminal shows logs with timestamps
- [ ] Start frontend service → Dependencies (backend-api, database) start first
- [ ] Service shows "Starting" then "Running" status
- [ ] Stop service → Terminal closes, status shows "NotStarted"
- [ ] **Stop All → ALL services show "NotStarted" status immediately**
- [ ] **Start All → ALL services show "Starting" then "Running" status**
- [ ] No RPC timeout errors in activity log
- [ ] getDependencies returns correct dependencies
- [ ] **Close terminal manually → Service stops and shows "NotStarted"**
- [ ] **Stop a dependency → All dependents stop, all terminals close**

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

---

## Round 4: Status Display and Terminal Sync Fixes

### Date: January 25, 2026

### Issues Reported

1. **Services showing "Failed" when intentionally stopped** - When stopping a service via UI, it would show "Failed" status instead of "NotStarted"
2. **Terminal closing doesn't stop service** - When user manually closes a terminal, the associated service should stop (two-way sync)
3. **Database terminal stays open after stopping** - When stopping database service, its terminal remained open even though the service was killed

### Root Causes

1. **Failed Status Bug (`process.rs`)**: When `stop_service()` was called, it would set the status based on the process exit code. On Windows (and Unix when using SIGKILL), killed processes have non-zero exit codes (e.g., 130 for SIGINT, 137 for SIGKILL), causing intentionally stopped services to be marked as `Failed` instead of `Stopped`.

2. **No Two-Way Terminal Sync**: The terminal manager only tracked when services were stopped to close terminals. When users manually closed a terminal, the service kept running.

3. **Status Mapping**: The extension mapped "stopped" status from daemon to `ServiceStatus.Stopped`, but from a user's perspective, a stopped service should appear the same as one that was never started (`NotStarted`).

### Fixes Applied

#### 1. Fix stop_service to Always Set Stopped Status (`process.rs`)

When `stop_service()` is called explicitly, always mark as `Stopped` regardless of exit code:

```rust
// BEFORE: Set status based on exit code (wrong for killed processes)
match wait_result {
    Ok(Ok(exit_status)) => {
        if exit_status.success() {
            process.status = ServiceStatus::Stopped;
        } else {
            process.status = ServiceStatus::Failed {
                exit_code: exit_status.code().unwrap_or(-1),
            };
        }
    }
}

// AFTER: Always mark as Stopped when stop_service is called
match wait_result {
    Ok(Ok(_exit_status)) => {
        // Always mark as Stopped when stop_service is called explicitly,
        // regardless of exit code. Killed processes often have non-zero
        // exit codes which doesn't mean they "failed".
        process.status = ServiceStatus::Stopped;
    }
}
```

#### 2. Two-Way Terminal Sync (`terminal-manager.ts`)

Added terminal close handler that stops the service when user closes terminal:

```typescript
// New type for handling terminal closes
export type TerminalCloseHandler = (serviceName: string) => Promise<void>;

// In TerminalManager constructor:
vscode.window.onDidCloseTerminal((terminal) => {
    // ... find service name ...
    
    // Check if this was a user-initiated close (not programmatic)
    const wasUserClose = !this.closingProgrammatically.has(serviceName);
    
    if (wasUserClose) {
        // Stop the service when user closes the terminal (two-way sync)
        if (this.terminalCloseHandler) {
            this.terminalCloseHandler(serviceName);
        }
    }
});

// Track programmatic closes to avoid stopping service when WE close it
closeTerminal(serviceName: string): void {
    this.closingProgrammatically.add(serviceName);
    terminal.dispose();
}
```

#### 3. Wire Up Terminal Close Handler (`extension.ts`)

```typescript
// Set up terminal close handler for two-way sync
terminalManager.setTerminalCloseHandler(async (serviceName: string) => {
    if (rpcClient) {
        await rpcClient.request('stopService', { service: serviceName });
        
        // Update tree view status to NotStarted
        if (treeDataProvider) {
            treeDataProvider.updateServiceStatus(serviceName, ServiceStatus.NotStarted);
        }
    }
});
```

#### 4. Map "Stopped" to "NotStarted" (`extension.ts`)

From user's perspective, stopped = not running:

```typescript
function parseServiceStatus(statusStr: string): ServiceStatus {
    switch (normalized) {
        case 'stopped':
            // Map stopped to NotStarted - from user's perspective, a stopped service
            // should appear the same as one that was never started
            return ServiceStatus.NotStarted;
        // ...
    }
}
```

#### 5. Update All Status Handlers to Use NotStarted

Changed all places that set `ServiceStatus.Stopped` to use `ServiceStatus.NotStarted`:
- `extension.ts`: `serviceStopped` notification handler
- `commands.ts`: `stopService` finally block
- `commands.ts`: `stopAll` service status updates

### Files Changed

| File | Changes |
|------|---------|
| `core/src/process.rs` | Always set `Stopped` status in `stop_service()` regardless of exit code |
| `extension/src/terminal-manager.ts` | Added terminal close handler, programmatic close tracking |
| `extension/src/extension.ts` | Wire up terminal close handler, map "stopped" to NotStarted |
| `extension/src/commands.ts` | Use NotStarted instead of Stopped in stop handlers |

### Summary

- Services now show "NotStarted" instead of "Failed" when stopped
- Closing a terminal manually now stops the associated service (two-way sync)
- All terminals are properly closed when stopping services (including cascade stops)
- Distinction: "Failed" = service crashed/errored, "NotStarted" = service not running

---

## Round 5: Terminal Close Delay Fix

### Date: January 25, 2026

### Issue Reported

**Database terminal doesn't close immediately after stopping** - When stopping the database service via the UI, the terminal stays open until the RPC request times out (up to 120 seconds), showing the error "Failed to stop database: Request timeout: stopService".

### Symptoms from Logs

```
[09:32:02.111Z] Service [database]: Stopping service
[09:32:02.111Z] RPC [stopService] request - id: 24, params: {"service":"database"}
[09:32:02.116Z] RPC [serviceStopped] notification - {"service":"backend-api"}
[09:32:02.116Z] Terminal [backend-api]: Terminal closed programmatically
... (no serviceStopped notification for database, no RPC response)
[09:32:03.602Z] [Status Refresh] database: Running -> NotStarted
```

The daemon successfully stops all services (including dependents), but:
1. The `serviceStopped` notification for the directly stopped service (`database`) is never received
2. The RPC response for `stopService` request never arrives
3. Eventually the RPC times out after 120 seconds

### Root Cause

The extension was waiting for the RPC response before closing the terminal (in the `finally` block). Since the daemon wasn't sending a response for the `stopService` request, the terminal stayed open until the 120 second timeout.

The actual bug appears to be in the daemon's Windows process handling - the `stopService` request blocks indefinitely. However, the daemon IS stopping the service (as evidenced by status refresh showing "NotStarted"), just not sending the response/notification.

### Fix Applied

Close the terminal **immediately** when the user clicks stop, **before** waiting for the RPC response. This provides better UX - the user sees the terminal close right away, and we don't need to wait for daemon confirmation.

```typescript
// BEFORE: Terminal closed in finally block (after RPC completes/times out)
private async stopService(item?: ServiceTreeItem): Promise<void> {
    try {
        await rpcClient.request('stopService', ...);  // Could hang for 120s
    } finally {
        this.terminalManager.closeTerminal(serviceName);  // Too late!
    }
}

// AFTER: Terminal closed immediately when stop is requested
private async stopService(item?: ServiceTreeItem): Promise<void> {
    // Close terminal IMMEDIATELY - don't wait for RPC
    this.terminalManager.closeTerminal(serviceName);
    treeDataProvider.updateServiceStatus(serviceName, ServiceStatus.NotStarted);
    
    try {
        await rpcClient.request('stopService', ...);  // Still send the request
    } catch (err) {
        // Even if RPC fails, terminal is already closed
    }
}
```

### Files Changed

| File | Changes |
|------|---------|
| `extension/src/commands.ts` | Move terminal close to BEFORE RPC request in `stopService()` |

### Testing

1. Start database service
2. Click stop on database
3. Terminal should close **immediately** (not after timeout)
4. UI should update to "NotStarted" immediately
5. Even if RPC times out, user experience is good

### Note on Daemon Bug

The underlying daemon issue (not sending `serviceStopped` notification and RPC response for directly stopped services) still exists but is now masked by this extension-side fix. The daemon should be investigated separately to ensure:
1. `serviceStopped` notification is sent for the service that was directly stopped
2. RPC response is sent for `stopService` requests

---

## Round 5b: Remove Blocking Progress Notification

### Date: January 25, 2026

### Issue Reported

**"Stopping database..." notification persists** - After the terminal closes and status shows "NotStarted", a progress notification continues showing "Stopping database..." until the RPC times out.

### Root Cause

The code was using `vscode.window.withProgress()` to show a blocking progress notification while waiting for the RPC response. Since the daemon doesn't respond to the `stopService` RPC, the notification stayed visible indefinitely.

### Fix Applied

Send the RPC request in the background (fire-and-forget) without blocking the UI:

```typescript
// BEFORE: Blocking progress notification that waits for RPC
await vscode.window.withProgress(
    { title: `Stopping ${serviceName}...` },
    async () => {
        await rpcClient.request('stopService', ...);  // Blocks UI!
    }
);

// AFTER: Non-blocking background RPC
rpcClient.request('stopService', { service: serviceName })
    .then(() => {
        // Log success in background
    })
    .catch((err) => {
        // Log error in background - don't show to user
        // From user's perspective, service is already stopped
    });

// Show immediate feedback
vscode.window.showInformationMessage(`Service ${serviceName} stopped`);
```

### Rationale

Since we've already:
1. Closed the terminal (immediate)
2. Updated the UI status to "NotStarted" (immediate)
3. Shown the success message (immediate)

...there's no need to wait for daemon confirmation. The RPC request tells the daemon to stop, but we don't need to block the UI waiting for a response that may never come.

### Files Changed

| File | Changes |
|------|---------|
| `extension/src/commands.ts` | Replace blocking `withProgress` with background RPC call |

### Result

- Terminal closes immediately
- Status updates immediately  
- Success message shows immediately
- No lingering progress notification
- RPC still sent to daemon (in background)

---

## Round 6: Stop All UX Parity + Cross-IDE Packaging Reliability

### Date: March 1, 2026

### Issue Reported

**Stop All still timed out in practice (especially database service)** even after extension-side Stop All logic was changed to per-service background stop calls.

### Root Causes

1. **Daemon RPC stdout writes were not serialized**: JSON-RPC notifications and responses could be written concurrently by different async tasks, which can interleave lines and produce parse/timeouts on the client.
2. **Packaging/install flow could deploy stale code**:
   - `install-extension.ps1` only installed into `code`, leaving forks (Cursor/Antigravity/Kiro) potentially on older extension builds.
   - `package-extension-quick.ps1` only rebuilt the daemon binary when missing (not when stale), so new Rust daemon fixes might not be included in VSIX.
   - `build-current.ps1` did not fail-fast on cargo build failure and could continue with old binaries.

### Fixes Applied

#### 1. Serialize daemon RPC writes (`core/src/rpc.rs`)

- Introduced a shared stdout writer: `Arc<Mutex<tokio::io::Stdout>>`
- Added `write_json_line(...)` helper and routed both request responses and event notifications through it.
- Result: JSON lines are emitted atomically and in-order, eliminating response/notification interleaving corruption.

#### 2. Make daemon rebuilds reliable (`scripts/build-current.ps1`)

- Added explicit failure handling after `cargo build`.
- Switched to an isolated target directory (`--target-dir target/build-current`) to avoid lock contention with running `dmn.exe`.
- Copy step now force-updates `dist/dmn-win32-x64.exe`.

#### 3. Ensure quick package includes fresh daemon (`scripts/package-extension-quick.ps1`)

- Added stale-check logic: rebuild daemon when `core/src` or Cargo files are newer than `extension/bin/dmn-win32-x64.exe`.
- Force-copy rebuilt binary into `extension/bin`.

#### 4. Install to all detected VS Code-family editors (`scripts/install-extension.ps1`)

- Installer now targets detected CLIs from: `code`, `cursor`, `antigravity`, `kiro`.
- Prevents editor-specific stale extension installs during cross-IDE testing.

### Files Changed

| File | Changes |
|------|---------|
| `core/src/rpc.rs` | Serialize all JSON-RPC stdout writes via shared async mutex |
| `scripts/build-current.ps1` | Fail-fast build checks + isolated target dir build |
| `scripts/package-extension-quick.ps1` | Stale daemon detection and forced binary refresh |
| `scripts/install-extension.ps1` | Multi-editor install support (`code/cursor/antigravity/kiro`) |

### Validation Outcome

- User confirmed Stop All now behaves as intended in real usage.
- Stop behavior is now consistent with individual service stop UX.
