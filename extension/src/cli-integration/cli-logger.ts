import * as vscode from 'vscode';

/**
 * Logger for CLI Integration debugging
 * Provides an output channel for tracking CLI-related operations
 */
export class CLILogger {
    private static instance: CLILogger | null = null;
    private outputChannel: vscode.OutputChannel;

    private constructor() {
        this.outputChannel = vscode.window.createOutputChannel('OpenDaemon CLI');
    }

    /**
     * Gets the singleton instance of CLILogger
     */
    static getInstance(): CLILogger {
        if (!CLILogger.instance) {
            CLILogger.instance = new CLILogger();
        }
        return CLILogger.instance;
    }

    /**
     * Logs an informational message
     */
    info(message: string, ...args: unknown[]): void {
        this.log('INFO', message, ...args);
    }

    /**
     * Logs a warning message
     */
    warn(message: string, ...args: unknown[]): void {
        this.log('WARN', message, ...args);
    }

    /**
     * Logs an error message
     */
    error(message: string, ...args: unknown[]): void {
        this.log('ERROR', message, ...args);
    }

    /**
     * Logs a debug message
     */
    debug(message: string, ...args: unknown[]): void {
        this.log('DEBUG', message, ...args);
    }

    /**
     * Logs a message with the specified level
     */
    private log(level: string, message: string, ...args: unknown[]): void {
        const timestamp = new Date().toISOString();
        let formattedMessage = `[${timestamp}] [${level}] ${message}`;
        
        if (args.length > 0) {
            const argsStr = args.map(arg => {
                if (typeof arg === 'object') {
                    try {
                        return JSON.stringify(arg, null, 2);
                    } catch {
                        return String(arg);
                    }
                }
                return String(arg);
            }).join(' ');
            formattedMessage += `\n  Details: ${argsStr}`;
        }
        
        this.outputChannel.appendLine(formattedMessage);
        
        // Also log to console for development
        console.log(`[OpenDaemon CLI] ${formattedMessage}`);
    }

    /**
     * Shows the output channel
     */
    show(): void {
        this.outputChannel.show();
    }

    /**
     * Logs system information for debugging
     */
    logSystemInfo(): void {
        this.info('=== System Information ===');
        this.info(`Platform: ${process.platform}`);
        this.info(`Architecture: ${process.arch}`);
        this.info(`Node Version: ${process.version}`);
        this.info(`VS Code Version: ${vscode.version}`);
        this.info(`Current PATH: ${process.env.PATH || process.env.Path || '(not set)'}`);
        this.info('========================');
    }

    /**
     * Logs workspace information
     */
    logWorkspaceInfo(): void {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        this.info('=== Workspace Information ===');
        if (workspaceFolders && workspaceFolders.length > 0) {
            workspaceFolders.forEach((folder, index) => {
                this.info(`Workspace ${index + 1}: ${folder.uri.fsPath}`);
            });
        } else {
            this.info('No workspace folders open');
        }
        this.info('============================');
    }

    /**
     * Logs current terminal settings
     */
    async logTerminalSettings(): Promise<void> {
        const config = vscode.workspace.getConfiguration('terminal.integrated');
        
        this.info('=== Terminal Settings ===');
        
        // Log env settings for all platforms
        const envWindows = config.get('env.windows');
        const envOsx = config.get('env.osx');
        const envLinux = config.get('env.linux');
        
        this.info('env.windows:', envWindows || '(not set)');
        this.info('env.osx:', envOsx || '(not set)');
        this.info('env.linux:', envLinux || '(not set)');
        
        // Log default profiles
        const defaultWindows = config.get('defaultProfile.windows');
        const defaultOsx = config.get('defaultProfile.osx');
        const defaultLinux = config.get('defaultProfile.linux');
        
        this.info('defaultProfile.windows:', defaultWindows || '(not set)');
        this.info('defaultProfile.osx:', defaultOsx || '(not set)');
        this.info('defaultProfile.linux:', defaultLinux || '(not set)');
        
        this.info('=========================');
    }

    /**
     * Disposes of the output channel
     */
    dispose(): void {
        this.outputChannel.dispose();
        CLILogger.instance = null;
    }
}

/**
 * Convenience function to get the CLI logger instance
 */
export function getCLILogger(): CLILogger {
    return CLILogger.getInstance();
}
