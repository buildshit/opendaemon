# Service Actions Bug Fixes - Complete Summary

## Date: January 23, 2026 (Updated)

## Issues Fixed

### 1. Terminal Not Showing Service Logs (FIXED)

**Problem:** The terminal was created but showed only an empty PowerShell prompt. Service logs from the daemon were not displayed in the terminal.

**Root Cause:** The previous implementation used "real" VS Code terminals, but since services run in the daemon (not in the terminal), logs weren't being injected into the terminal.

**Solution:** Rewrote `terminal-manager.ts` to use **pseudoterminals** that can receive and display log output from the daemon:
- Created `ServicePseudoterminal` class implementing `vscode.Pseudoterminal`
- Pseudoterminal displays service name header and "Waiting for service output..." message
- Log lines are formatted with timestamps and color-coded (stderr in red)
- Supports stdin forwarding to daemon via `writeStdin` RPC
- Updated `extension.ts` to route `logLine` notifications to terminal manager

**Files Changed:**
- `extension/src/terminal-manager.ts` - Complete rewrite with pseudoterminal implementation
- `extension/src/extension.ts` - Added log line routing to terminals, set up stdin writer

### 2. RPC Timeouts on startService/stopService (FIXED)

**Problem:** `startService` and `stopService` RPC calls were timing out after 2 minutes. The logs showed "Request timeout: startService" errors.

**Root Cause:** The daemon's `start_service_with_deps` method blocked synchronously while waiting for the service's ready condition to be met. If the ready check took longer than the extension's RPC timeout, the request would fail.

**Solution:** Made `start_service_with_deps` non-blocking:
- Service spawning returns immediately after the process starts
- Ready check runs asynchronously in a separate tokio task
- Status updates happen via `serviceReady`/`serviceFailed` notifications
- Dependencies still wait for ready state before starting dependents
- Reduced log polling interval from 50ms to 10ms for faster ready detection

**Files Changed:**
- `core/src/orchestrator.rs` - Made start_service_with_deps non-blocking, async ready checking

### 3. Terminal Not Closing When Service Stops (FIXED)

**Problem:** When a service was stopped, the terminal would remain open instead of being closed.

**Root Cause:** If the `stopService` RPC timed out, the `serviceStopped` notification was never sent (or not received in time), so the terminal wasn't closed.

**Solution:** 
- Updated `commands.ts` `stopService` method to use `try/finally` pattern
- Terminal is now closed in `finally` block regardless of RPC success/failure
- Tree view status is also updated to "Stopped" in the finally block
- The `serviceStopped` notification handler still works as a backup

**Files Changed:**
- `extension/src/commands.ts` - Added finally block for guaranteed terminal cleanup

### 4. getDependencies RPC Method Not Found (FIXED)

**Problem:** Logs showed `"Method not found: getDependencies"` error.

**Root Cause:** The daemon binary was outdated. The `getDependencies` method was already implemented in `rpc.rs` but the binary hadn't been rebuilt.

**Solution:** Rebuilt the daemon with `cargo build --release`. The method was already correctly implemented:
```rust
RpcRequest::GetDependencies { service } => {
    // Returns direct dependencies for a service from the graph
    match orch.graph().get_dependencies(&service) {
        Ok(deps) => JsonRpcResponse::success(id, json!({
            "service": service,
            "dependencies": deps
        })),
        ...
    }
}
```

### 5. Status Not Updating Accurately (FIXED)

**Problem:** Service status would show "Starting" forever instead of transitioning to "Running" or "Failed".

**Root Cause:** The status updates depended on the RPC response which was timing out. Notifications were being sent but the tree view wasn't being updated properly.

**Solution:** 
- Tree view now updates based on notifications (`serviceStarting`, `serviceReady`, `serviceFailed`, `serviceStopped`)
- The `startService` RPC returns quickly so no timeout
- Status is updated immediately when notifications are received
- Changed success message from "started" to "is starting" to reflect async nature

**Files Changed:**
- `extension/src/commands.ts` - Changed message wording
- `extension/src/extension.ts` - Notification handlers already update tree view

## Architecture After Fixes

### Terminal Flow
```
User clicks "Start Service"
    → Extension creates pseudoterminal
    → Extension sends startService RPC
    → Daemon spawns process, returns immediately
    → Daemon emits serviceStarting notification
    → Extension updates tree view to "Starting"
    → Daemon streams logLine notifications
    → Extension writes log lines to pseudoterminal
    → Daemon emits serviceReady notification
    → Extension updates tree view to "Running"
```

### Stop Service Flow
```
User clicks "Stop Service"
    → Extension sends stopService RPC
    → Daemon stops process, emits serviceStopped
    → RPC returns (or times out)
    → Extension closes terminal (in finally block)
    → Extension updates tree view to "Stopped"
```

### Pseudoterminal Features
- Displays service name header
- Shows timestamps for each log line
- Color-codes stderr (red) vs stdout
- Supports stdin input forwarding to daemon
- Shows "Service stopped" message when terminated

## Files Modified

### Extension (TypeScript)
1. `extension/src/terminal-manager.ts` - Complete rewrite with pseudoterminal
2. `extension/src/extension.ts` - Log routing to terminals, stdin writer setup
3. `extension/src/commands.ts` - Guaranteed terminal cleanup, message updates
4. `extension/src/test/suite/terminal-manager.test.ts` - Updated for new API

### Daemon (Rust)
1. `core/src/orchestrator.rs` - Non-blocking start_service_with_deps

## How to Test

1. **Reload the VS Code window** to pick up the new extension
2. **Verify terminal shows logs:**
   - Start the `database` service
   - Terminal should show "=== Service: database ===" header
   - Log lines should appear with timestamps
3. **Verify service starts without timeout:**
   - Status should change from "Starting" to "Running"
   - No "Request timeout" errors in output
4. **Verify terminal closes on stop:**
   - Stop the service
   - Terminal should close automatically
   - Status should show "Stopped"
5. **Verify dependencies work:**
   - Start `frontend` (depends on `backend-api` which depends on `database`)
   - All three services should start in correct order

## Known Behaviors

1. **Async Ready Check:** The `startService` message now says "is starting" instead of "started" because the ready check is async. The `serviceReady` notification will update status when actually ready.

2. **Quick Start Returns:** Services without `ready_when` conditions will show as "Running" immediately after spawn. Services with `ready_when` will show "Starting" until the condition is met.

3. **Terminal Persists Until Stop:** Terminals remain open until the service is stopped or the user closes them manually. This allows viewing historical logs.
