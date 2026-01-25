# Status Display Fix Summary

## Issue Description

Services were running correctly but their statuses were not being displayed properly in real-time in the VS Code extension's tree view. Specifically:

1. **Frontend service stuck at "Starting"**: Even after the frontend service was ready and running (outputting "Frontend available at http://localhost:3000"), the tree view continued to show "Starting" instead of "Running".

2. **Inconsistent status after stopAll**: After stopping all services, some services showed incorrect states.

## Root Cause Analysis

After examining the logs and code, the following issues were identified:

### Issue 1: Mutex Lock Held Too Long (Daemon)

In `core/src/orchestrator.rs`, when a service had a `ready_when` condition, the ready watching task would:

1. Acquire the `ready_watcher` mutex lock
2. Hold the lock for the **entire duration** of watching (up to 10 seconds timeout)
3. Only release the lock after the watch completed

This caused problems because:
- `getStatus` RPC calls also needed to acquire the `ready_watcher` lock to check ready states
- If `getStatus` was called while a service was being watched, it would block until the watch completed
- This blocked the extension from getting accurate status updates

### Issue 2: Status Channel Dropped Prematurely (Daemon)

After 100ms timeout in `start_service_with_deps`, the status receiver channel (`status_rx`) was dropped. When the ready watcher later tried to send status updates, the send would fail silently.

### Issue 3: No Fallback Status Refresh (Extension)

The extension relied entirely on real-time notifications (`serviceStarting`, `serviceReady`, `serviceStopped`). If any notification was missed or delayed, the tree view would not update.

## Fixes Implemented

### Fix 1: Restructured Ready Watching (Daemon)

**File**: `core/src/orchestrator.rs`

Changed the ready watching approach to:
1. Only briefly lock the `ready_watcher` mutex to register the condition
2. Perform the actual watching **without holding the mutex** using a new standalone function `watch_condition_standalone`
3. Only briefly lock again to mark the service as ready

This allows `getStatus` calls to proceed without being blocked by long-running ready watches.

**Key changes**:
- Added `watch_condition_standalone` helper function that performs log pattern matching or URL checking without needing mutex access
- Removed the 100ms wait that was dropping the status channel
- Added explicit error logging when event sending fails

### Fix 2: Added register_condition Method (Ready Watcher)

**File**: `core/src/ready.rs`

Added a new method `register_condition` that allows registering a ready condition without starting to watch. This enables the separation of condition registration (needs mutex) from actual watching (doesn't need mutex).

### Fix 3: Added Periodic Status Refresh (Extension)

**File**: `extension/src/extension.ts`

Added a periodic status refresh mechanism as a fallback:
- Polls daemon status every 2 seconds via `getStatus` RPC call
- Only updates tree view if status actually changed (to avoid UI flicker)
- Logs status changes detected via polling for debugging
- Properly starts/stops with daemon lifecycle

**Key additions**:
- `startPeriodicStatusRefresh()` - Starts the periodic refresh interval
- `stopPeriodicStatusRefresh()` - Stops the interval
- `refreshStatusFromDaemon()` - Silent status refresh that doesn't log errors prominently
- Updated `deactivate()`, `handleConfigChanged()`, and `handleConfigDeleted()` to properly manage the refresh interval

## Testing

To verify the fixes:

1. Start the extension with a dmn.json that has services with `ready_when` conditions
2. Start a service (e.g., frontend with dependencies)
3. Verify that:
   - Status changes from "Starting" to "Running" when the ready condition is met
   - `getStatus` calls don't block while services are starting
   - Tree view stays in sync even if notifications are delayed

## Files Changed

1. `core/src/orchestrator.rs` - Restructured ready watching to not hold mutex during watch
2. `core/src/ready.rs` - Added `register_condition` method
3. `extension/src/extension.ts` - Added periodic status refresh fallback

## Date

January 25, 2026
