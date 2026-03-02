# OpenDaemon VS Code Extension

The extension provides visual service orchestration for the same OpenDaemon runtime used by the CLI and MCP server.

## Features

- `OpenDaemon Services` tree view in Explorer
- Start/stop/restart actions per service
- `Start All Services` and `Stop All Services`
- Service logs and dedicated service terminals (`dmn: <service>`)
- Automatic CLI availability in new integrated terminals
- Config creation flow when `dmn.json` is missing

## Command Palette

Use `OpenDaemon: ...` commands, including:

- `Start All Services`
- `Stop All Services`
- `Start Service`
- `Stop Service`
- `Restart Service`
- `Show Logs`
- `Open Terminal`
- `New Terminal with CLI`
- `Show CLI Logs`
- `Run CLI Diagnostics`

## Usage

1. Open a workspace that contains `dmn.json`.
2. Open the `OpenDaemon Services` view.
3. Start services from the view title buttons or service context menu.
4. Inspect logs and terminals while services run.

## Development

```bash
cd extension
npm install
npm run compile
npm test
```

For watch mode:

```bash
cd extension
npm run watch
```

## Packaging and Install (Windows Maintainer Loop)

From repo root:

```powershell
.\scripts\package-and-install-extension.ps1
```

This quick loop builds a Windows binary, bundles it, and installs the generated VSIX into supported editors on your machine.

## Cross-Platform Release Packaging

For a release-ready VSIX (Windows + macOS + Linux binaries), use CI and the release workflow:

- Workflow: `.github/workflows/extension-release.yml`
- Builds binaries for:
  - Windows x64
  - macOS x64 + arm64
  - Linux x64 + arm64
- Bundles all binaries into one VSIX and can publish to both marketplaces.

See [PACKAGING.md](PACKAGING.md) for the full release process and token setup.

## License

This extension is licensed under GNU AGPL v3 (`AGPL-3.0-only`).

## Troubleshooting

- Open a **new** integrated terminal after activation to get injected CLI PATH changes.
- Use `OpenDaemon: Show CLI Logs` for integration diagnostics.
- Use `OpenDaemon: Run CLI Diagnostics` to verify binary path and terminal env setup.

## Related Docs

- [Project README](../README.md)
- [CLI Guide](../CLI.md)
- [MCP Guide](../MCP.md)
- [Extension Packaging Guide](PACKAGING.md)
