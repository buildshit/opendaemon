import * as vscode from 'vscode';
import { ActivityLogger } from './activity-logger';

/**
 * Log line interface for structured log data
 */
export interface LogLine {
    timestamp: string;      // ISO 8601 format
    content: string;        // The actual log content
    stream: 'stdout' | 'stderr';  // Which stream the log came from
}

/**
 * Manages integrated terminals for services
 * Each service gets its own named terminal where logs are displayed in real-time
 */
export class TerminalManager {
    private terminals: Map<string, vscode.Terminal> = new Map();
    private disposables: vscode.Disposable[] = [];
    private activityLogger: ActivityLogger | null;

    constructor(activityLogger?: ActivityLogger) {
        this.activityLogger = activityLogger || null;
        // Listen for terminal closures to clean up our map
        this.disposables.push(
            vscode.window.onDidCloseTerminal((terminal) => {
                // Find and remove the terminal from our map
                for (const [serviceName, term] of this.terminals.entries()) {
                    if (term === terminal) {
                        this.terminals.delete(serviceName);
                        
                        // Log terminal closure
                        if (this.activityLogger) {
                            this.activityLogger.logTerminalAction(serviceName, 'Terminal closed');
                        }
                        break;
                    }
                }
            })
        );
    }

    /**
     * Get or create a terminal for a service
     * @param serviceName Name of the service
     * @returns The terminal instance
     */
    getOrCreateTerminal(serviceName: string): vscode.Terminal {
        let terminal = this.terminals.get(serviceName);

        if (!terminal || terminal.exitStatus !== undefined) {
            try {
                // Terminal doesn't exist or was closed, create a new one
                terminal = vscode.window.createTerminal({
                    name: `dmn: ${serviceName}`,
                    iconPath: new vscode.ThemeIcon('server-process'),
                    isTransient: false, // Keep terminal in list even when not active
                });

                this.terminals.set(serviceName, terminal);
                
                // Log terminal creation
                if (this.activityLogger) {
                    this.activityLogger.logTerminalAction(serviceName, 'Terminal created');
                }
            } catch (err) {
                const errorMsg = err instanceof Error ? err.message : String(err);
                
                // Log to activity channel
                if (this.activityLogger) {
                    this.activityLogger.logError(`Terminal creation for ${serviceName}`, errorMsg);
                }
                
                // Re-throw to let caller handle
                throw new Error(`Failed to create terminal for ${serviceName}: ${errorMsg}`);
            }
        }

        return terminal;
    }

    /**
     * Show the terminal for a service
     * @param serviceName Name of the service
     * @param preserveFocus If true, don't steal focus from current editor
     */
    showTerminal(serviceName: string, preserveFocus: boolean = false): void {
        const terminal = this.getOrCreateTerminal(serviceName);
        terminal.show(preserveFocus);
        
        // Log terminal shown action
        if (this.activityLogger) {
            this.activityLogger.logTerminalAction(
                serviceName,
                'Terminal shown',
                `preserveFocus: ${preserveFocus}`
            );
        }
    }

    /**
     * Write a single log line to a service's terminal with formatting
     * @param serviceName Name of the service
     * @param logLine Log line with timestamp, content, and stream type
     */
    writeLogLine(serviceName: string, logLine: LogLine): void {
        try {
            const terminal = this.getOrCreateTerminal(serviceName);
            
            // Format the log line with timestamp and stream indicator
            const streamPrefix = logLine.stream === 'stderr' ? '[stderr]' : '[stdout]';
            const formattedLine = `${logLine.timestamp} ${streamPrefix} ${logLine.content}`;
            
            // Write to terminal
            terminal.sendText(`echo "${this.escapeForShell(formattedLine)}"`, false);
            
            // Log activity
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(
                    serviceName,
                    'Log line written',
                    `stream: ${logLine.stream}`
                );
            }
        } catch (err) {
            const errorMsg = err instanceof Error ? err.message : String(err);
            
            // Log to activity channel
            if (this.activityLogger) {
                this.activityLogger.logError(
                    `Log streaming for ${serviceName}`,
                    errorMsg
                );
            }
            
            // Don't throw - continue processing other logs
        }
    }

    /**
     * Send text to a service's terminal
     * @param serviceName Name of the service
     * @param text Text to send (will be displayed as output)
     */
    sendText(serviceName: string, text: string): void {
        const terminal = this.getOrCreateTerminal(serviceName);
        // Use echo to display text without executing it
        terminal.sendText(`echo "${this.escapeForShell(text)}"`, false);
    }

    /**
     * Write multiple lines to a service's terminal
     * @param serviceName Name of the service
     * @param lines Array of log lines to display
     */
    writeLines(serviceName: string, lines: string[]): void {
        const terminal = this.getOrCreateTerminal(serviceName);
        
        for (const line of lines) {
            // Display each line
            terminal.sendText(`echo "${this.escapeForShell(line)}"`, false);
        }
    }

    /**
     * Clear a service's terminal
     * @param serviceName Name of the service
     */
    clearTerminal(serviceName: string): void {
        const terminal = this.terminals.get(serviceName);
        if (terminal) {
            // Send clear command (works on both Windows and Unix)
            if (process.platform === 'win32') {
                terminal.sendText('cls', true);
            } else {
                terminal.sendText('clear', true);
            }
        }
    }

    /**
     * Close a service's terminal
     * @param serviceName Name of the service
     */
    closeTerminal(serviceName: string): void {
        const terminal = this.terminals.get(serviceName);
        if (terminal) {
            terminal.dispose();
            this.terminals.delete(serviceName);
            
            // Log terminal closure
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(serviceName, 'Terminal closed manually');
            }
        }
    }

    /**
     * Close all service terminals
     */
    closeAllTerminals(): void {
        const terminalCount = this.terminals.size;
        
        if (terminalCount > 0 && this.activityLogger) {
            this.activityLogger.log(`Closing all terminals (${terminalCount} total)`);
        }
        
        for (const [serviceName, terminal] of this.terminals.entries()) {
            terminal.dispose();
            
            // Log each terminal closure
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(serviceName, 'Terminal closed (cleanup)');
            }
        }
        this.terminals.clear();
    }

    /**
     * Get all active terminal names
     */
    getActiveTerminals(): string[] {
        return Array.from(this.terminals.keys());
    }

    /**
     * Check if a terminal exists for a service
     */
    hasTerminal(serviceName: string): boolean {
        const terminal = this.terminals.get(serviceName);
        return terminal !== undefined && terminal.exitStatus === undefined;
    }

    /**
     * Escape text for safe display in shell
     * @param text Text to escape
     */
    private escapeForShell(text: string): string {
        // Escape double quotes and backslashes for echo command
        return text
            .replace(/\\/g, '\\\\')
            .replace(/"/g, '\\"')
            .replace(/\$/g, '\\$')
            .replace(/`/g, '\\`');
    }

    /**
     * Dispose of all resources
     */
    dispose(): void {
        this.closeAllTerminals();
        this.disposables.forEach(d => d.dispose());
        this.disposables = [];
    }
}
