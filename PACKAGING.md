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
2. Push a tag (for example `v0.1.1`) or run the release workflow manually.
3. Let `.github/workflows/extension-release.yml` build and package the VSIX.
4. Publish using workflow automation (requires secrets) or publish manually.

## Required Publish Secrets

Configure repository secrets:

- `VSCE_PAT`
- `OVSX_PAT`

## GitHub Releases (VSIX Downloads)

To make VSIX downloads available from your repository page:

1. Run the workflow with `publish=false` or `publish=true` to generate `extension-vsix`.
2. Download the VSIX artifact from the run.
3. Create a GitHub release and attach the VSIX:

```bash
gh release create v<version> dist/opendaemon-<version>.vsix \
  --repo buildshit/opendaemon \
  --title "OpenDaemon v<version>" \
  --notes "Cross-platform VSIX release."
```

This gives users a direct installable package even if they do not use a marketplace.

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
