# Changelog

## 0.1.4 - 2026-03-04

- Hardened Windows service shutdown to terminate full process trees and reduce lingering background processes that can keep ports locked.
- Improved extension/daemon lifecycle shutdown behavior so `stopAll` is requested before daemon teardown and daemon exit performs a best-effort service stop.
- Updated extension marketplace metadata to the correct repository (`https://github.com/buildshit/opendaemon`) and official website (`https://opendaemon.com`).
- Fixed PowerShell packaging scripts to be encoding-safe on Windows and made `.vsix` verification extraction reliable via a temporary `.zip` copy.

## 0.1.3 - 2026-03-04

- Corrected extension marketplace repository metadata to `https://github.com/buildshit/opendaemon`.
- Updated extension homepage metadata to `https://opendaemon.com` so listing resources point to the official site.
- Fixed PowerShell packaging scripts to be encoding-safe on Windows by replacing Unicode status glyphs with ASCII markers.
- Fixed `scripts/test-package.ps1` to extract `.vsix` files via a temporary `.zip` copy so package verification runs reliably in PowerShell.

## 0.1.2 - 2026-03-03

- Fixed Windows service shutdown to terminate the full process tree (`taskkill /T /F`) so wrapper-launched children do not linger and hold ports.
- Added a Windows regression test that starts a real port listener through `cmd -> powershell`, stops the service, and verifies the port is released.
- Improved extension shutdown flow to request `stopAll` before daemon teardown and prefer graceful daemon EOF shutdown with forced-kill fallback.
- Updated daemon-mode exit behavior to stop all managed services when the RPC stdio session ends.
- Improved local packaging reliability by stopping stale processes running from `dist/dmn-win32-x64.exe` before overwriting the binary.
- Added a `local-dist` workflow service in `dmn.json` for repeatable local package+install validation before release.

## 0.1.1 - 2026-03-02

- Published cross-platform extension build pipeline validation (Windows, macOS x64/arm64, Linux x64/arm64).
- Prepared marketplace release update for `opendaemon.opendaemon`.
- Added GitHub Release distribution guidance so users can download the VSIX directly.
