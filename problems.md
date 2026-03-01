# OpenDaemon Problem Tracker

This file tracks active reliability, UX, and performance problems we are fixing.

## Working Rules

- Do not check off any item until you confirm the fix works in your environment.
- Keep items here as "open" or "implemented-awaiting-confirmation" while testing.
- After confirmation, move the finalized notes to the appropriate long-term docs.

## Active Problems

- [ ] **P1: RPC line framing drops partial JSON messages**
  - Status: implemented-awaiting-confirmation
  - Impact: Random timeouts/missing notifications when stop/start responses are split across chunks.
  - Target files: `extension/src/rpc-client.ts`, `core/src/rpc.rs`
  - Implementation note: In addition to client-side chunk buffering, daemon-side RPC writes are now serialized through a shared stdout lock so JSON-RPC notifications and responses cannot interleave/corrupt each other under concurrency.
  - Verify:
    - Stop/start requests do not randomly timeout when daemon is responsive.
    - No parse failures from chunked JSON lines.

- [x] **P2: Stop All closes terminals too late**
  - Status: confirmed-done (2026-03-01)
  - Impact: Terminals remain visible until RPC timeout when `stopAll` hangs or response is delayed.
  - Target files: `extension/src/commands.ts`
  - Implementation note: `Stop All` now uses the same per-service, non-blocking stop flow as individual stop actions (background `stopService` requests), instead of relying on a single `stopAll` RPC response path.
  - Confirmation note: User confirmed Stop All now matches individual stop UX in real usage.
  - Verified:
    - `Stop All` closes all service terminals immediately.
    - UI remains responsive even if daemon response is slow.

- [ ] **P3: Programmatic terminal close tracking can leak/stale**
  - Status: implemented-awaiting-confirmation
  - Impact: Rare user-close misclassification and edge-case sync bugs.
  - Target files: `extension/src/terminal-manager.ts`
  - Verify:
    - Manual close still stops service.
    - Programmatic close does not trigger duplicate stop attempts.

- [ ] **P4: Config watcher can duplicate behavior on repeated restarts**
  - Status: implemented-awaiting-confirmation
  - Impact: Multiple reload handlers can stack and cause noisy/redundant daemon reloads.
  - Target files: `extension/src/file-watcher.ts`
  - Verify:
    - Repeated config edits trigger one stable reload cycle.
    - No repeated duplicate notifications from one save.

- [ ] **P5: Background status refresh can overlap in-flight RPC calls**
  - Status: implemented-awaiting-confirmation
  - Impact: Request pileups and avoidable timeout pressure under load.
  - Target files: `extension/src/extension.ts`
  - Verify:
    - No concurrent overlapping `getStatus` polling calls.
    - Service status remains fresh without increased timeout errors.

- [ ] **P6: Circular dependency validation can false-positive on valid DAGs**
  - Status: implemented-awaiting-confirmation
  - Impact: Valid shared-dependency configs may be rejected.
  - Target files: `core/src/config.rs`
  - Verify:
    - Diamond dependencies validate correctly.
    - Real cycles still fail validation.

- [ ] **P7: Ready-condition log polling is heavier than needed**
  - Status: implemented-awaiting-confirmation
  - Impact: Extra CPU and lock pressure from fast full-buffer polling.
  - Target files: `core/src/orchestrator.rs`, `core/src/logs.rs`
  - Verify:
    - Reduced polling overhead while preserving readiness correctness.
    - No regressions in ready detection timing.

- [ ] **P8: Ready timeout errors can hide service identity**
  - Status: implemented-awaiting-confirmation
  - Impact: Troubleshooting is harder when timeout reports `unknown` service.
  - Target files: `core/src/orchestrator.rs`
  - Verify:
    - Timeout errors include the correct service name.

- [ ] **P9: README/examples mismatch config parser format**
  - Status: implemented-awaiting-confirmation
  - Impact: New users can hit avoidable config errors.
  - Target files: `core/src/config.rs` (compat), docs staged later after your confirmation.
  - Verify:
    - Both legacy and current `ready_when` formats are accepted.
    - Existing configs continue to work.

- [ ] **P10: Naturally exited processes are not reconciled into status/events**
  - Status: implemented-awaiting-confirmation
  - Impact: Services that exit/crash on their own can appear stale until unrelated refresh paths run.
  - Target files: `core/src/process.rs`, `core/src/orchestrator.rs`, `core/src/rpc.rs`
  - Verify:
    - Unexpected service exits emit `serviceFailed`/`serviceStopped` events promptly.
    - Tree status and terminal cleanup stay in sync without waiting for manual actions.
