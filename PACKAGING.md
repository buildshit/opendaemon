# Packaging and Publishing

This repository contains multiple components (core, extension, MCP).  
This guide focuses on shipping the VS Code extension as a real multi-platform release.

## What Changed

The extension packaging flow now bundles binaries for:

- Windows x64
- macOS x64 + arm64
- Linux x64 + arm64

and supports publishing to:

- VS Code Marketplace
- Open VSX (for VS Code forks)

CI builds compile each binary on a native runner for that OS/architecture pair.  
For macOS targets, the build uses the standard host `clang` toolchain on GitHub Actions runners.

## Recommended Release Flow

1. Bump extension version in `extension/package.json`.
2. Commit and push the release commit to `main`.
3. Let `.github/workflows/extension-release-main.yml` create and push the matching `v*` tag.
4. Let `extension-release-main` trigger `.github/workflows/extension-release.yml` on the new tag ref to build binaries, package VSIX, publish to both marketplaces, and create/update the GitHub Release with the VSIX asset.
5. Verify both workflows are successful in Actions.

### CLI/testing branch flow

Use `cli` for validation and iteration, then merge/cherry-pick release-ready changes into `main`.
Only `main` pushes should drive automated release tags and GitHub Release publication.

## Required Publish Secrets

Configure repository secrets:

- `VSCE_PAT`
- `OVSX_PAT`

## One-Time Marketplace Setup

Before first publish:

1. Create/verify the `opendaemon` publisher on VS Code Marketplace.
2. Create/verify the `opendaemon` namespace on Open VSX.
3. Ensure `extension/package.json` publisher stays `opendaemon` so both marketplaces map correctly.

## Local Commands

Quick Windows maintainer loop:

```powershell
.\scripts\package-and-install-extension.ps1
```

Package from existing multi-platform binaries in `dist/`:

```powershell
.\scripts\package-extension.ps1
```

See `extension/PACKAGING.md` for extension-specific details.
