# OpenDaemon Extension Packaging

This document describes how OpenDaemon extension packages are built with platform binaries for:

- Windows (`dmn-win32-x64.exe`)
- macOS Intel (`dmn-darwin-x64`)
- macOS Apple Silicon (`dmn-darwin-arm64`)
- Linux x64 (`dmn-linux-x64`)
- Linux arm64 (`dmn-linux-arm64`)

## Local Maintainer Loop (Windows)

Use this when you want a quick local install/test cycle:

```powershell
.\scripts\package-and-install-extension.ps1
```

This path is intentionally fast and optimized for local Windows development.

## Release-Ready Packaging (All Platforms)

Use the GitHub Actions workflow:

- `.github/workflows/extension-release.yml`

The workflow:

1. Builds each platform binary on matching runners.
2. Downloads all binaries into `dist/`.
3. Bundles binaries and wrappers into `extension/bin/`.
4. Packages a single multi-platform VSIX.
5. Optionally publishes to both VS Code Marketplace and Open VSX.

Notes:
- macOS binaries are built natively on GitHub macOS runners using host `clang`.
- No local Windows drive letter assumptions are used in extension runtime path resolution.

## Required Secrets for Publish

Set these repository secrets before enabling publish:

- `VSCE_PAT` - Visual Studio Marketplace personal access token
- `OVSX_PAT` - Open VSX access token

Without these secrets, the publish step will fail by design.

## One-Time Publisher Setup

- VS Code Marketplace publisher: `opendaemon`
- Open VSX namespace: `opendaemon`
- Keep `publisher` in `extension/package.json` as `opendaemon`

## Local Full Package Validation

If you already have all platform binaries in `dist/`, run:

```powershell
.\scripts\package-extension.ps1
```

To attempt local all-target binary builds first:

```powershell
.\scripts\package-extension.ps1 -BuildAll
```

## Package Verification

`scripts/test-package.ps1` / `scripts/test-package.sh` validates:

- all required binaries are present in the VSIX
- command wrappers (`dmn`, `dmn.exe`, `dmn.cmd`) are included
- expected compiled extension files exist
- dev/source files are excluded from the package

## GitHub Release Distribution

After a successful packaging/publish run, create a GitHub release and upload the VSIX artifact:

```bash
gh release create v<version> dist/opendaemon-<version>.vsix \
  --repo buildshit/opendaemon \
  --title "OpenDaemon v<version>" \
  --notes "Cross-platform extension package."
```

This provides a direct manual installation download in addition to marketplace installs.
