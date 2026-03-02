# Publishing Log (OpenDaemon Extension)

This document records the complete publishing work done for OpenDaemon extension distribution across:

- VS Code Marketplace
- Open VSX
- GitHub Releases (manual VSIX download)

Date: 2026-03-02

---

## Goal

Ship OpenDaemon extension in a real release flow with:

- Cross-platform runtime binaries (Windows, macOS x64/arm64, Linux x64/arm64)
- Automated CI packaging/publishing
- AGPL licensing metadata
- Manual VSIX download path through GitHub Releases

---

## Major Changes Implemented

## 1) Licensing and metadata

- Added AGPL metadata:
  - `extension/package.json` -> `"license": "AGPL-3.0-only"`
  - `Cargo.toml` workspace license -> `AGPL-3.0-only`
- Added license files:
  - `LICENSE`
  - `extension/LICENSE`
- Added marketplace metadata to extension manifest:
  - `homepage`
  - `bugs`
  - `keywords`

## 2) Cross-platform binary handling

Updated extension runtime resolution to support all intended platforms:

- `extension/src/cli-integration/binary-resolver.ts`
- `extension/src/binary-path.ts`
- `extension/src/cli-integration/terminal-interceptor.ts`
- `extension/src/cli-integration/notification-manager.ts`

Key behavior:

- Windows uses `dmn-win32-x64.exe`
- macOS uses `dmn-darwin-x64` or `dmn-darwin-arm64`
- Linux uses `dmn-linux-x64` or `dmn-linux-arm64`
- No hardcoded `F:\` or fixed drive assumptions in runtime logic

## 3) Packaging and script updates

Updated scripts to bundle/validate all platform binaries and wrappers:

- `scripts/build-all.ps1`, `scripts/build-all.sh`
- `scripts/bundle-extension.ps1`, `scripts/bundle-extension.sh`
- `scripts/package-extension.ps1`, `scripts/package-extension.sh`
- `scripts/package-all-platforms.sh`
- `scripts/test-package.ps1`, `scripts/test-package.sh`
- `scripts/test-binary-selection.js`

Also tightened package contents:

- `extension/.vscodeignore` updated to exclude local test/dev artifacts from VSIX.

## 4) CI workflow automation

Created and refined:

- `.github/workflows/extension-release.yml`

Workflow behavior:

1. Build binaries on native runners:
   - windows-x64
   - linux-x64
   - linux-arm64
   - macos-x64
   - macos-arm64
2. Package one VSIX containing all platform binaries
3. Publish to VS Code Marketplace + Open VSX (when `publish=true` or tag trigger)

---

## Issues Encountered and Fixes

## Issue A: macOS jobs failed (missing custom linkers)

Failure seen:

- `linker 'x86_64-apple-darwin-clang' not found`
- `linker 'aarch64-apple-darwin-clang' not found`

Fix:

- Updated `.cargo/config.toml` for Apple targets to use host `clang`.

## Issue B: deprecated/unsupported macOS runner labels

Fix:

- Switched workflow to supported labels:
  - `macos-15-intel` for x64
  - `macos-14` for arm64

## Issue C: Windows artifact export shell mismatch

Cause:

- `mkdir -p` step executed under PowerShell in Windows job.

Fix:

- Forced `shell: bash` on build/export steps where needed.

## Issue D: publish attempted old VSIX version

Failure seen in publish job:

- `opendaemon.opendaemon v0.1.0 already exists`

Root cause:

- Artifact contained multiple VSIX files; publish step selected first by plain `ls`.

Fix:

- Added workflow step to remove stale VSIX files before packaging:
  - `rm -f dist/*.vsix`
- Updated publish selection to newest VSIX:
  - `ls -t dist/*.vsix | head -n 1`

---

## Release and Publish Execution Record

## Validation run (build/package only)

- Run ID: `22570089469`
- URL: `https://github.com/buildshit/opendaemon/actions/runs/22570089469`
- Result: success
- Purpose: confirm full cross-platform build + VSIX package generation

## First publish attempt

- Run ID: `22571627136`
- Result: failure at marketplace publish step
- Reason: stale VSIX selection published `0.1.0`

## Successful publish run

- Run ID: `22571988629`
- URL: `https://github.com/buildshit/opendaemon/actions/runs/22571988629`
- Result: success
- `Publish to VS Code Marketplace`: success
- `Publish to Open VSX`: success

---

## Version and Release

- Extension version bumped to: `0.1.1`
- GitHub Release created: `v0.1.1`
- Release URL:
  - `https://github.com/buildshit/opendaemon/releases/tag/v0.1.1`
- VSIX asset URL:
  - `https://github.com/buildshit/opendaemon/releases/download/v0.1.1/opendaemon-0.1.1.vsix`

---

## IDE Verification Performed

- VS Code CLI:
  - `opendaemon.opendaemon@0.1.1` installed and confirmed
- Cursor:
  - direct marketplace ID install did not resolve in CLI
  - VSIX manual install succeeded
  - `opendaemon.opendaemon@0.1.1` confirmed after VSIX install

---

## Secrets/Prerequisites Used

Configured in repository Actions secrets:

- `VSCE_PAT`
- `OVSX_PAT`

Both were validated by workflow before publish steps.

---

## Safe workflow notes

- Temporary git worktrees were used during release/publish operations to avoid touching local runtime state files in active development workspace.
- Main repo development state remained intact while release tasks ran on clean checkouts.

---

## Next Release Checklist

1. Bump `extension/package.json` version.
2. Commit and push release changes to `main`.
3. Let `extension-release-main` auto-create and push `v<extension-version>` tag.
4. Let `extension-release` run from tag:
   - build binaries
   - package VSIX
   - publish VS Code Marketplace + Open VSX
   - create/update GitHub Release with VSIX asset
5. Verify both workflows succeed in Actions.
6. Smoke-test install/update in VS Code and Cursor.

---

## Main branch automation update (2026-03-02)

Added `main`-first release automation so stable releases are driven from `main` only:

- New workflow: `.github/workflows/extension-release-main.yml`
  - Trigger: push to `main` when `extension/package.json` changes
  - Reads extension version and creates/pushes matching tag (`v<version>`) if it does not already exist
- Updated workflow: `.github/workflows/extension-release.yml`
  - Added `create-github-release` job on tag runs
  - Creates (or updates) GitHub Release and uploads the packaged VSIX

Resulting behavior:

- Test and iterate in `cli`
- Merge/cherry-pick release commit to `main`
- Push `main` once
- Tag, marketplace publish, and GitHub Release happen automatically via workflows

