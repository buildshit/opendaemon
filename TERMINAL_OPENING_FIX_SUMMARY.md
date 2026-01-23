# Terminal Opening and Service Management Fix

## Issues Fixed

### 1. **Terminals Not Opening When Services Start**
**Problem:** When users clicked "Start Service", the terminal wouldn't appear or would appear empty.

**Root Cause:** Terminal creation was happening AFTER the RPC request completed, but there was no guarantee the daemon had actually started the process yet. The terminal was created too late in the flow.

**Fix:** Modified `startService()` in `extension/src/commands.ts` to:
- Create the terminal BEFORE sending the RPC request to start the service
- This ensures the terminal exists and is ready to receive log output immediately
- Show the terminal after the service starts

**Code Changes:**
```typescript
// Create terminal BEFORE starting service so logs appear immediately
this.terminalManager.getOrCreateTerminal(targetItem.serviceName);

await vscode.window.withProgress(..., async () => {
    await rpcClient.request('startService', { service: targetItem.serviceName });
});

// Show terminal after service starts
this.terminalManager.showTerminal(targetItem.serviceName, true);
```

### 2. **Services Showing "Already Running" Errors on Restart**
**Problem:** When trying to restart a service, users would get "Service already running: database" error.

**Root Cause:** The process manager kept stopped processes in its HashMap. When restarting:
1. `stop_service()` would mark the process as `Stopped` but leave it in the HashMap
2. `start_service()` would check if the service exists in the HashMap
3. If it found a stopped process, it would still return "AlreadyRunning" error

**Fix:** Modified `spawn_service()` in `core/src/process.rs` to:
- Check if a stopped or failed process exists
- Remove it from the HashMap before spawning a new one
- This allows the service to be properly restarted

**Code Changes:**
```rust
// If service is stopped or failed, remove it so we can spawn a new one
if matches!(process.status, ServiceStatus::Stopped | ServiceStatus::Failed { .. }) {
    self.processes.remove(service_name);
}
```

### 3. **Inability to Stop Services**
**Problem:** Stopping services would fail with "NotRunning" error even when the service was running.

**Root Cause:** The `stop_service()` method in `core/src/process.rs` was returning an error if the process was already stopped, instead of just returning success.

**Fix:** Modified `stop_service()` in `core/src/process.rs` to:
- Return `Ok(())` if the service is already stopped
- This allows idempotent stop operations and prevents errors on cascade stopping

**Code Changes:**
```rust
// Already stopped, just return success
if matches!(process.status, ServiceStatus::Stopped | ServiceStatus::Failed { .. }) {
    return Ok(());
}
```

### 4. **Restart Service Not Working Properly**
**Problem:** Restarting services would fail or not properly clear the terminal.

**Root Cause:** The restart logic was trying to clear the terminal instead of closing and recreating it, which didn't properly reset the terminal state.

**Fix:** Modified `restartService()` in `extension/src/commands.ts` to:
- Close the existing terminal completely
- Create a fresh terminal
- This ensures a clean slate for the restarted service

**Code Changes:**
```typescript
// Close and recreate terminal for clean restart
this.terminalManager.closeTerminal(targetItem.serviceName);
this.terminalManager.getOrCreateTerminal(targetItem.serviceName);
```

### 5. **Start All Services Not Creating Terminals**
**Problem:** When starting all services, terminals weren't being created for any of them.

**Root Cause:** Terminals were being created AFTER the RPC request, same as the single service issue.

**Fix:** Modified `startAll()` in `extension/src/commands.ts` to:
- Create terminals for all services BEFORE starting them
- Show all terminals after the services start

**Code Changes:**
```typescript
// Create terminals for all services BEFORE starting
for (const service of services) {
    this.terminalManager.getOrCreateTerminal(service.name);
}

await vscode.window.withProgress(..., async () => {
    await rpcClient.request('startAll');
});

// Show terminals for all services
for (const service of services) {
    this.terminalManager.showTerminal(service.name, true);
}
```

## Files Modified

1. **core/src/process.rs**
   - Fixed `spawn_service()` to remove stopped processes from HashMap
   - Fixed `stop_service()` to return success for already-stopped services

2. **extension/src/commands.ts**
   - Fixed `startService()` to create terminal before starting
   - Fixed `restartService()` to close and recreate terminal
   - Fixed `startAll()` to create terminals before starting services

## Testing the Fixes

1. **Test Terminal Opening:**
   - Click "Start Service" on any service
   - Terminal should appear immediately
   - Service logs should stream into the terminal

2. **Test Service Restart:**
   - Start a service
   - Click "Restart Service"
   - Should not get "already running" error
   - Terminal should be cleared and recreated
   - Service should restart successfully

3. **Test Stop Service:**
   - Start a service
   - Click "Stop Service"
   - Service should stop without errors
   - Can restart it immediately after

4. **Test Start All:**
   - Click "Start All"
   - All services should start
   - Terminals should appear for all services
   - Logs should stream into each terminal

## Impact

These fixes resolve the core issues preventing proper terminal integration and service lifecycle management:
- Users can now see service output in real-time
- Services can be properly restarted without errors
- Service state is properly managed throughout the lifecycle
- The extension is now fully functional for service management
