import * as fs from 'fs';
import * as path from 'path';
import * as vscode from 'vscode';

/**
 * Resolve the best available OpenDaemon binary path.
 *
 * During extension development, prefer the newest local workspace build.
 * In published extensions, fall back to the bundled platform binary.
 */
export function resolveDmnBinaryPath(context: vscode.ExtensionContext): string {
    const platform = process.platform;
    const arch = process.arch;
    if (arch !== 'x64' && arch !== 'arm64') {
        throw new Error(`Unsupported architecture: ${arch}`);
    }

    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (workspaceFolders && workspaceFolders.length > 0) {
        const workspaceRoot = workspaceFolders[0].uri.fsPath;
        const executableName = platform === 'win32' ? 'dmn.exe' : 'dmn';
        const localCandidates = [
            path.join(workspaceRoot, 'target', 'release', executableName),
            path.join(workspaceRoot, 'target', 'debug', executableName),
            path.join(workspaceRoot, 'target', 'build-current', 'release', executableName),
            path.join(workspaceRoot, 'target', 'build-current', 'debug', executableName)
        ];

        let selectedPath: string | null = null;
        let selectedMtime = -1;
        for (const candidate of localCandidates) {
            if (!fs.existsSync(candidate)) {
                continue;
            }
            const mtime = fs.statSync(candidate).mtimeMs;
            if (mtime > selectedMtime) {
                selectedMtime = mtime;
                selectedPath = candidate;
            }
        }

        if (selectedPath) {
            console.log(`[OpenDaemon] Using local binary: ${selectedPath}`);
            return selectedPath;
        }
    }

    let binaryName: string;
    if (platform === 'win32') {
        binaryName = 'dmn-win32-x64.exe';
    } else if (platform === 'darwin') {
        binaryName = arch === 'arm64' ? 'dmn-darwin-arm64' : 'dmn-darwin-x64';
    } else if (platform === 'linux') {
        binaryName = arch === 'arm64' ? 'dmn-linux-arm64' : 'dmn-linux-x64';
    } else {
        throw new Error(`Unsupported platform: ${platform}`);
    }

    const bundledPath = path.join(context.extensionPath, 'bin', binaryName);
    console.log(`[OpenDaemon] Using bundled binary: ${bundledPath}`);
    return bundledPath;
}
