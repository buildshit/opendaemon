import * as vscode from 'vscode';

/**
 * ActivityLogger manages extension activity logging to a dedicated output channel.
 * This provides a comprehensive audit trail of all extension operations for debugging
 * and troubleshooting purposes.
 */
export class ActivityLogger {
    private outputChannel: vscode.OutputChannel;

    constructor() {
        this.outputChannel = vscode.window.createOutputChannel('OpenDaemon Activity');
    }

    /**
     * Log a general activity message with timestamp
     * @param message The message to log
     */
    log(message: string): void {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] ${message}`);
    }

    /**
     * Log a service-related action
     * @param service The service name
     * @param action The action being performed (e.g., "Starting service", "Stopped")
     * @param details Optional additional details about the action
     */
    logServiceAction(service: string, action: string, details?: string): void {
        const detailsStr = details ? ` - ${details}` : '';
        this.log(`Service [${service}]: ${action}${detailsStr}`);
    }

    /**
     * Log a terminal-related action
     * @param service The service name associated with the terminal
     * @param action The action being performed (e.g., "Terminal created", "Terminal closed")
     * @param details Optional additional details about the action
     */
    logTerminalAction(service: string, action: string, details?: string): void {
        const detailsStr = details ? ` - ${details}` : '';
        this.log(`Terminal [${service}]: ${action}${detailsStr}`);
    }

    /**
     * Log an RPC-related action
     * @param method The RPC method name
     * @param direction The direction of the RPC call ('request', 'response', or 'notification')
     * @param details Optional additional details about the RPC action
     */
    logRpcAction(method: string, direction: 'request' | 'response' | 'notification', details?: string): void {
        const detailsStr = details ? ` - ${details}` : '';
        this.log(`RPC [${method}] ${direction}${detailsStr}`);
    }

    /**
     * Log an error with context information
     * @param context The context where the error occurred (e.g., "startService(database)")
     * @param error The error object or error message
     */
    logError(context: string, error: Error | string): void {
        const errorMsg = error instanceof Error ? error.message : error;
        this.log(`ERROR in ${context}: ${errorMsg}`);
    }

    /**
     * Show the activity log output channel to the user
     */
    show(): void {
        this.outputChannel.show();
    }

    /**
     * Dispose of the output channel and clean up resources
     */
    dispose(): void {
        this.outputChannel.dispose();
    }
}
