import * as vscode from 'vscode';
import * as path from 'path';

/**
 * Terminal Profile Provider for OpenDaemon CLI
 * Provides terminal profiles with automatic PATH injection for the dmn command
 */
export class OpenDaemonTerminalProfileProvider implements vscode.TerminalProfileProvider {
    private binDir: string;

    /**
     * Creates a new OpenDaemonTerminalProfileProvider
     * @param binDir The directory containing the CLI binary
     */
    constructor(binDir: string) {
        this.binDir = binDir;
    }

    /**
     * Provides a terminal profile with PATH injection
     * @param token Cancellation token
     * @returns Terminal profile with injected PATH
     */
    provideTerminalProfile(
        token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.TerminalProfile> {
        // Determine PATH separator based on platform
        const pathSeparator = this.getPathSeparator();
        
        // Get current PATH
        const currentPath = process.env.PATH || process.env.Path || '';
        
        // Prepend bin directory to PATH
        const newPath = currentPath 
            ? `${this.binDir}${pathSeparator}${currentPath}`
            : this.binDir;

        // Create terminal options with injected PATH
        const options: vscode.TerminalOptions = {
            name: 'OpenDaemon CLI',
            iconPath: new vscode.ThemeIcon('terminal'),
            env: {
                PATH: newPath
            }
        };

        // On Windows, also set Path for compatibility
        if (process.platform === 'win32') {
            options.env!.Path = newPath;
        }

        console.log('[OpenDaemon] Terminal profile provider: Creating terminal with PATH injection');
        console.log(`[OpenDaemon] Bin directory: ${this.binDir}`);

        return new vscode.TerminalProfile(options);
    }

    /**
     * Gets the PATH separator for the current platform
     * @returns ';' for Windows, ':' for Unix-like systems
     */
    private getPathSeparator(): string {
        return process.platform === 'win32' ? ';' : ':';
    }
}
