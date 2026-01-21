import * as vscode from 'vscode';

/**
 * Error categories from the Rust daemon
 */
export enum ErrorCategory {
    CONFIG = 'CONFIG',
    GRAPH = 'GRAPH',
    PROCESS = 'PROCESS',
    READY = 'READY',
    ORCHESTRATOR = 'ORCHESTRATOR',
    MCP = 'MCP',
    RPC = 'RPC',
    IO = 'IO',
    JSON = 'JSON',
    SERVICE = 'SERVICE'
}

/**
 * Error severity levels
 */
export enum ErrorSeverity {
    Critical = 'critical',
    Warning = 'warning',
    Info = 'info'
}

/**
 * Error information structure
 */
export interface ErrorInfo {
    message: string;
    category: ErrorCategory;
    severity?: ErrorSeverity;
    service?: string;
    exitCode?: number;
    timestamp?: Date;
    details?: string;
}

/**
 * Error action handlers
 */
export interface ErrorActionHandlers {
    openConfig?: () => Promise<void>;
    showLogs?: (service?: string) => void;
    reload?: () => void;
    retry?: () => Promise<void>;
}

/**
 * Manages error display in VS Code
 */
export class ErrorDisplayManager {
    private outputChannel: vscode.OutputChannel;
    private errorHistory: ErrorInfo[] = [];
    private readonly maxHistorySize = 100;

    constructor(private readonly actionHandlers: ErrorActionHandlers) {
        this.outputChannel = vscode.window.createOutputChannel('OpenDaemon Errors');
    }

    /**
     * Display an error with appropriate notification and logging
     */
    async displayError(error: ErrorInfo): Promise<void> {
        // Add timestamp if not provided
        if (!error.timestamp) {
            error.timestamp = new Date();
        }

        // Add to history
        this.addToHistory(error);

        // Determine severity if not provided
        const severity = error.severity || this.determineSeverity(error.category);

        // Log to output panel
        this.logToOutputPanel(error, severity);

        // Show notification with actions
        await this.showNotification(error, severity);
    }

    /**
     * Display a service failure error
     */
    async displayServiceFailure(service: string, errorMessage: string, exitCode?: number): Promise<void> {
        const error: ErrorInfo = {
            message: `Service '${service}' failed${exitCode !== undefined ? ` with exit code ${exitCode}` : ''}: ${errorMessage}`,
            category: ErrorCategory.SERVICE,
            severity: ErrorSeverity.Critical,
            service,
            exitCode
        };

        await this.displayError(error);
    }

    /**
     * Display a configuration error
     */
    async displayConfigError(message: string, details?: string): Promise<void> {
        const error: ErrorInfo = {
            message: `Configuration Error: ${message}`,
            category: ErrorCategory.CONFIG,
            severity: ErrorSeverity.Critical,
            details
        };

        await this.displayError(error);
    }

    /**
     * Display a dependency graph error
     */
    async displayGraphError(message: string): Promise<void> {
        const error: ErrorInfo = {
            message: `Dependency Error: ${message}`,
            category: ErrorCategory.GRAPH,
            severity: ErrorSeverity.Critical
        };

        await this.displayError(error);
    }

    /**
     * Display a process error
     */
    async displayProcessError(service: string, message: string): Promise<void> {
        const error: ErrorInfo = {
            message: `Process Error (${service}): ${message}`,
            category: ErrorCategory.PROCESS,
            severity: ErrorSeverity.Critical,
            service
        };

        await this.displayError(error);
    }

    /**
     * Display a ready check error
     */
    async displayReadyError(service: string, message: string): Promise<void> {
        const error: ErrorInfo = {
            message: `Ready Check Failed (${service}): ${message}`,
            category: ErrorCategory.READY,
            severity: ErrorSeverity.Warning,
            service
        };

        await this.displayError(error);
    }

    /**
     * Show the error output panel
     */
    showOutputPanel(): void {
        this.outputChannel.show();
    }

    /**
     * Clear the error output panel
     */
    clearOutputPanel(): void {
        this.outputChannel.clear();
    }

    /**
     * Get error history
     */
    getErrorHistory(): ErrorInfo[] {
        return [...this.errorHistory];
    }

    /**
     * Clear error history
     */
    clearHistory(): void {
        this.errorHistory = [];
    }

    /**
     * Dispose resources
     */
    dispose(): void {
        this.outputChannel.dispose();
    }

    /**
     * Determine error severity based on category
     */
    private determineSeverity(category: ErrorCategory): ErrorSeverity {
        switch (category) {
            case ErrorCategory.CONFIG:
            case ErrorCategory.GRAPH:
            case ErrorCategory.PROCESS:
            case ErrorCategory.SERVICE:
                return ErrorSeverity.Critical;
            
            case ErrorCategory.READY:
            case ErrorCategory.ORCHESTRATOR:
                return ErrorSeverity.Warning;
            
            case ErrorCategory.MCP:
            case ErrorCategory.RPC:
            case ErrorCategory.IO:
            case ErrorCategory.JSON:
                return ErrorSeverity.Info;
            
            default:
                return ErrorSeverity.Warning;
        }
    }

    /**
     * Log error to output panel
     */
    private logToOutputPanel(error: ErrorInfo, severity: ErrorSeverity): void {
        const timestamp = error.timestamp?.toISOString() || new Date().toISOString();
        const severityIcon = this.getSeverityIcon(severity);
        
        this.outputChannel.appendLine('');
        this.outputChannel.appendLine(`${severityIcon} [${error.category}] ${timestamp}`);
        this.outputChannel.appendLine(`Message: ${error.message}`);
        
        if (error.service) {
            this.outputChannel.appendLine(`Service: ${error.service}`);
        }
        
        if (error.exitCode !== undefined) {
            this.outputChannel.appendLine(`Exit Code: ${error.exitCode}`);
        }
        
        if (error.details) {
            this.outputChannel.appendLine(`Details: ${error.details}`);
        }
        
        this.outputChannel.appendLine('─'.repeat(80));
    }

    /**
     * Show notification with appropriate actions
     */
    private async showNotification(error: ErrorInfo, severity: ErrorSeverity): Promise<void> {
        const actions = this.getActionsForError(error);
        const message = `OpenDaemon: ${error.message}`;

        let selection: string | undefined;

        switch (severity) {
            case ErrorSeverity.Critical:
                selection = await vscode.window.showErrorMessage(message, ...actions);
                break;
            
            case ErrorSeverity.Warning:
                selection = await vscode.window.showWarningMessage(message, ...actions);
                break;
            
            case ErrorSeverity.Info:
                selection = await vscode.window.showInformationMessage(message, ...actions);
                break;
        }

        // Handle action selection
        if (selection) {
            await this.handleAction(selection, error);
        }
    }

    /**
     * Get appropriate actions for an error
     */
    private getActionsForError(error: ErrorInfo): string[] {
        const actions: string[] = [];

        switch (error.category) {
            case ErrorCategory.CONFIG:
            case ErrorCategory.GRAPH:
                if (this.actionHandlers.openConfig) {
                    actions.push('Open Config');
                }
                actions.push('Show Details');
                break;
            
            case ErrorCategory.PROCESS:
            case ErrorCategory.SERVICE:
                if (error.service && this.actionHandlers.showLogs) {
                    actions.push('Show Logs');
                }
                actions.push('Show Details');
                if (this.actionHandlers.retry) {
                    actions.push('Retry');
                }
                break;
            
            case ErrorCategory.READY:
                if (error.service && this.actionHandlers.showLogs) {
                    actions.push('Show Logs');
                }
                actions.push('Show Details');
                break;
            
            case ErrorCategory.ORCHESTRATOR:
                if (this.actionHandlers.reload) {
                    actions.push('Reload Window');
                }
                actions.push('Show Details');
                break;
            
            default:
                actions.push('Show Details');
        }

        return actions;
    }

    /**
     * Handle action selection
     */
    private async handleAction(action: string, error: ErrorInfo): Promise<void> {
        switch (action) {
            case 'Open Config':
                if (this.actionHandlers.openConfig) {
                    await this.actionHandlers.openConfig();
                }
                break;
            
            case 'Show Logs':
                if (this.actionHandlers.showLogs) {
                    this.actionHandlers.showLogs(error.service);
                }
                break;
            
            case 'Show Details':
                this.showOutputPanel();
                break;
            
            case 'Reload Window':
                if (this.actionHandlers.reload) {
                    this.actionHandlers.reload();
                }
                break;
            
            case 'Retry':
                if (this.actionHandlers.retry) {
                    await this.actionHandlers.retry();
                }
                break;
        }
    }

    /**
     * Add error to history
     */
    private addToHistory(error: ErrorInfo): void {
        this.errorHistory.push(error);
        
        // Trim history if it exceeds max size
        if (this.errorHistory.length > this.maxHistorySize) {
            this.errorHistory = this.errorHistory.slice(-this.maxHistorySize);
        }
    }

    /**
     * Get severity icon for display
     */
    private getSeverityIcon(severity: ErrorSeverity): string {
        switch (severity) {
            case ErrorSeverity.Critical:
                return '❌';
            case ErrorSeverity.Warning:
                return '⚠️';
            case ErrorSeverity.Info:
                return 'ℹ️';
            default:
                return '•';
        }
    }
}
