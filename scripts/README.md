# Scripts Overview

## Packaging Scripts

- `package-and-install-extension.ps1`  
  Fast Windows loop: package extension and install into detected editors.

- `package-extension-quick.ps1`  
  Fast package for local Windows development.

- `package-extension.ps1` / `package-extension.sh`  
  Full extension packaging from `dist/` binaries (all platforms expected).

- `package-all-platforms.sh`  
  Alias for `package-extension.sh --build-all`.

## Build Scripts

- `build-current.ps1` / `build-current.sh`  
  Build the current host platform binary.

- `build-all.ps1` / `build-all.sh`  
  Attempt all-target Rust builds from one machine.

## Bundle Script

- `bundle-extension.ps1` / `bundle-extension.sh`  
  Copy binaries from `dist/` into `extension/bin/` and create wrappers:
  - `dmn` (Unix)
  - `dmn.exe` and `dmn.cmd` (Windows)

## Validation Scripts

- `test-package.ps1` / `test-package.sh`  
  Verify VSIX contents and expected included/excluded files.

- `test-install.ps1` / `test-install.sh`  
  Install and verify extension in supported editor CLIs.
