import * as vscode from 'vscode';

/**
 * Virtual document provider for service logs
 * This allows logs to be displayed in editor tabs
 */
export class LogDocumentProvider implements vscode.TextDocumentContentProvider {
    private _onDidChange = new vscode.EventEmitter<vscode.Uri>();
    readonly onDidChange = this._onDidChange.event;

    // Store log content per service
    private logContent = new Map<string, string[]>();

    /**
     * Provide the text content for a log document
     */
    provideTextDocumentContent(uri: vscode.Uri): string {
        const serviceName = this.getServiceNameFromUri(uri);
        const lines = this.logContent.get(serviceName) || [];

        if (lines.length === 0) {
            return `=== Logs for ${serviceName} ===\n\nWaiting for logs...`;
        }

        return `=== Logs for ${serviceName} ===\n\n${lines.join('\n')}`;
    }

    /**
     * Append a log line for a service
     */
    appendLog(serviceName: string, line: string): void {
        if (!this.logContent.has(serviceName)) {
            this.logContent.set(serviceName, []);
        }

        this.logContent.get(serviceName)!.push(line);

        // Fire change event to update the document
        const uri = this.createUri(serviceName);
        this._onDidChange.fire(uri);
    }

    /**
     * Set all logs for a service (used when initially loading)
     */
    setLogs(serviceName: string, lines: string[]): void {
        this.logContent.set(serviceName, lines);

        const uri = this.createUri(serviceName);
        this._onDidChange.fire(uri);
    }

    /**
     * Clear logs for a service
     */
    clearLogs(serviceName: string): void {
        this.logContent.delete(serviceName);

        const uri = this.createUri(serviceName);
        this._onDidChange.fire(uri);
    }

    /**
     * Create a URI for a service's log document
     */
    createUri(serviceName: string): vscode.Uri {
        return vscode.Uri.parse(`opendaemon-log:///${serviceName}.log`);
    }

    /**
     * Extract service name from URI
     */
    private getServiceNameFromUri(uri: vscode.Uri): string {
        // URI path is like /service-name.log
        const path = uri.path;
        return path.replace(/^\//, '').replace(/\.log$/, '');
    }

    /**
     * Dispose of resources
     */
    dispose(): void {
        this._onDidChange.dispose();
        this.logContent.clear();
    }
}
