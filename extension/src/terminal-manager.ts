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
 * Function type for sending stdin data to a service
 */
export type StdinWriter = (serviceName: string, data: string) => Promise<void>;

/**
 * Function type for handling terminal close events (to stop the service)
 */
export type TerminalCloseHandler = (serviceName: string) => Promise<void>;

/**
 * Pseudoterminal implementation that displays service logs
 * and can forward stdin to the daemon
 */
class ServicePseudoterminal implements vscode.Pseudoterminal {
    private writeEmitter = new vscode.EventEmitter<string>();
    private closeEmitter = new vscode.EventEmitter<number | void>();
    
    onDidWrite: vscode.Event<string> = this.writeEmitter.event;
    onDidClose: vscode.Event<number | void> = this.closeEmitter.event;
    
    private dimensions: vscode.TerminalDimensions | undefined;
    private isOpen = false;
    private pendingOutput: string[] = [];
    private stdinWriter: StdinWriter | null = null;
    private serviceName: string;
    private activityLogger: ActivityLogger | null;

    constructor(serviceName: string, activityLogger: ActivityLogger | null, stdinWriter?: StdinWriter) {
        this.serviceName = serviceName;
        this.activityLogger = activityLogger;
        this.stdinWriter = stdinWriter || null;
    }

    open(initialDimensions: vscode.TerminalDimensions | undefined): void {
        this.dimensions = initialDimensions;
        this.isOpen = true;
        
        // Write header
        this.writeEmitter.fire(`\x1b[1;36m=== Service: ${this.serviceName} ===\x1b[0m\r\n`);
        this.writeEmitter.fire(`\x1b[90mWaiting for service output...\x1b[0m\r\n\r\n`);
        
        // Write any pending output
        for (const output of this.pendingOutput) {
            this.writeEmitter.fire(output);
        }
        this.pendingOutput = [];
    }

    close(): void {
        this.isOpen = false;
    }

    handleInput(data: string): void {
        // Forward input to the daemon via stdin writer
        if (this.stdinWriter) {
            // Echo the input locally
            this.writeEmitter.fire(data);
            
            // Handle enter key
            if (data === '\r') {
                this.writeEmitter.fire('\n');
            }
            
            // Forward to daemon (with newline for commands)
            this.stdinWriter(this.serviceName, data).catch(err => {
                if (this.activityLogger) {
                    this.activityLogger.logError(
                        `stdin write for ${this.serviceName}`,
                        err instanceof Error ? err.message : String(err)
                    );
                }
            });
        }
    }

    setDimensions(dimensions: vscode.TerminalDimensions): void {
        this.dimensions = dimensions;
    }

    /**
     * Write text to the terminal
     */
    write(text: string): void {
        if (this.isOpen) {
            this.writeEmitter.fire(text);
        } else {
            this.pendingOutput.push(text);
        }
    }

    /**
     * Write a log line with formatting
     */
    writeLogLine(logLine: LogLine): void {
        // Format timestamp to be more readable (just time portion)
        const timestamp = logLine.timestamp.split('T')[1]?.split('.')[0] || logLine.timestamp;
        
        // Color coding using ANSI: stderr is red, stdout has gray timestamp
        const streamPrefix = logLine.stream === 'stderr' ? '\x1b[91m' : '';
        const resetColor = logLine.stream === 'stderr' ? '\x1b[0m' : '';
        
        const formattedLine = `\x1b[90m${timestamp}\x1b[0m ${streamPrefix}${logLine.content}${resetColor}\r\n`;
        
        this.write(formattedLine);
    }

    /**
     * Close the terminal
     */
    terminate(): void {
        this.writeEmitter.fire('\r\n\x1b[1;33m=== Service stopped ===\x1b[0m\r\n');
        this.closeEmitter.fire(0);
    }

    /**
     * Clear the terminal
     */
    clear(): void {
        // Send ANSI clear screen and cursor home
        this.write('\x1b[2J\x1b[H');
    }
}

/**
 * Manages integrated terminals for services using pseudoterminals
 * that display log output from the daemon
 */
export class TerminalManager implements vscode.Disposable {
    private terminals: Map<string, vscode.Terminal> = new Map();
    private pseudoterminals: Map<string, ServicePseudoterminal> = new Map();
    private disposables: vscode.Disposable[] = [];
    private activityLogger: ActivityLogger | null;
    private stdinWriter: StdinWriter | null = null;
    private terminalCloseHandler: TerminalCloseHandler | null = null;
    // Track terminal objects that we're closing programmatically to avoid triggering stop
    private closingProgrammatically: Set<vscode.Terminal> = new Set();

    constructor(activityLogger?: ActivityLogger) {
        this.activityLogger = activityLogger || null;
        
        // Listen for terminal closures to clean up our map and optionally stop service
        this.disposables.push(
            vscode.window.onDidCloseTerminal((terminal) => {
                // Determine close origin first so we can clean stale tracking even
                // if the terminal has already been removed from our service maps.
                const wasProgrammatic = this.closingProgrammatically.delete(terminal);

                // Find the service name for this terminal
                let foundServiceName: string | null = null;
                for (const [serviceName, term] of this.terminals.entries()) {
                    if (term === terminal) {
                        foundServiceName = serviceName;
                        break;
                    }
                }
                
                // If not found in our map, it might have already been cleaned up by closeTerminal
                if (!foundServiceName) {
                    return;
                }
                
                const serviceName = foundServiceName;
                
                // Clean up maps
                this.terminals.delete(serviceName);
                this.pseudoterminals.delete(serviceName);
                
                // Check if this was a user-initiated close (not programmatic)
                if (!wasProgrammatic) {
                    // Log terminal closure by user
                    if (this.activityLogger) {
                        this.activityLogger.logTerminalAction(serviceName, 'Terminal closed by user - stopping service');
                    }
                    
                    // Stop the service when user closes the terminal (two-way sync)
                    if (this.terminalCloseHandler) {
                        this.terminalCloseHandler(serviceName).catch(err => {
                            if (this.activityLogger) {
                                this.activityLogger.logError(
                                    `Stopping service ${serviceName} after terminal close`,
                                    err instanceof Error ? err.message : String(err)
                                );
                            }
                        });
                    }
                }
                // Note: For programmatic closes, we already logged in closeTerminal()
            })
        );
    }

    /**
     * Set the stdin writer function for forwarding input to daemon
     */
    setStdinWriter(writer: StdinWriter): void {
        this.stdinWriter = writer;
    }

    /**
     * Set the terminal close handler for stopping services when terminals are closed by user
     */
    setTerminalCloseHandler(handler: TerminalCloseHandler): void {
        this.terminalCloseHandler = handler;
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
                // Create a pseudoterminal that can display logs
                const pty = new ServicePseudoterminal(
                    serviceName,
                    this.activityLogger,
                    this.stdinWriter || undefined
                );
                
                // Create terminal with pseudoterminal
                terminal = vscode.window.createTerminal({
                    name: `dmn: ${serviceName}`,
                    iconPath: new vscode.ThemeIcon('server-process'),
                    pty: pty,
                    isTransient: false,
                });

                this.terminals.set(serviceName, terminal);
                this.pseudoterminals.set(serviceName, pty);
                
                // Log terminal creation
                if (this.activityLogger) {
                    this.activityLogger.logTerminalAction(serviceName, 'Terminal created (pseudoterminal)');
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
            // Ensure terminal exists
            this.getOrCreateTerminal(serviceName);
            
            // Get the pseudoterminal and write to it
            const pty = this.pseudoterminals.get(serviceName);
            if (pty) {
                pty.writeLogLine(logLine);
            }
            
            // Log activity (throttled elsewhere)
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(
                    serviceName,
                    'Log line received',
                    `stream: ${logLine.stream}, length: ${logLine.content.length}`
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
     * Write raw text to a service's terminal
     * @param serviceName Name of the service
     * @param text Text to write
     */
    writeText(serviceName: string, text: string): void {
        const pty = this.pseudoterminals.get(serviceName);
        if (pty) {
            pty.write(text);
        }
    }

    /**
     * Write multiple lines to a service's terminal
     * @param serviceName Name of the service
     * @param lines Array of log lines to display
     */
    writeLines(serviceName: string, lines: string[]): void {
        const pty = this.pseudoterminals.get(serviceName);
        if (pty) {
            for (const line of lines) {
                pty.write(line + '\r\n');
            }
        }
    }

    /**
     * Clear a service's terminal
     * @param serviceName Name of the service
     */
    clearTerminal(serviceName: string): void {
        const pty = this.pseudoterminals.get(serviceName);
        if (pty) {
            pty.clear();
        }
    }

    /**
     * Close a service's terminal
     * @param serviceName Name of the service
     */
    closeTerminal(serviceName: string): void {
        const pty = this.pseudoterminals.get(serviceName);
        const terminal = this.terminals.get(serviceName);
        
        if (pty) {
            pty.terminate();
        }
        
        if (terminal) {
            // Mark as closing programmatically so onDidCloseTerminal doesn't trigger stop
            this.closingProgrammatically.add(terminal);
            terminal.dispose();
            
            // Explicitly clean up maps (onDidCloseTerminal will also do this, but we do it
            // here immediately for safety in case dispose doesn't trigger the event sync)
            this.terminals.delete(serviceName);
            this.pseudoterminals.delete(serviceName);
            
            // Log terminal closure
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(serviceName, 'Terminal closed programmatically');
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
            const pty = this.pseudoterminals.get(serviceName);
            if (pty) {
                pty.terminate();
            }
            // Mark as closing programmatically so onDidCloseTerminal doesn't trigger stop
            this.closingProgrammatically.add(terminal);
            terminal.dispose();
            
            // Log each terminal closure
            if (this.activityLogger) {
                this.activityLogger.logTerminalAction(serviceName, 'Terminal closed (cleanup)');
            }
        }
        
        // Clear maps explicitly (onDidCloseTerminal will also try but we do it here for safety)
        this.terminals.clear();
        this.pseudoterminals.clear();
        // We already cleared service maps, so stale programmatic tracking is no longer useful.
        this.closingProgrammatically.clear();
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
     * Dispose of all resources
     */
    dispose(): void {
        this.closeAllTerminals();
        this.disposables.forEach(d => d.dispose());
        this.disposables = [];
    }
}
