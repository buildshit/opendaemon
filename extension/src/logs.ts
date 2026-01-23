import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';
import { LogDocumentProvider } from './log-document-provider';

export interface LogLine {
    timestamp: string;
    content: string;
    stream: 'stdout' | 'stderr';
}

export class LogManager {
    private currentService: string | null = null;

    constructor(
        private readonly getRpcClient: () => RpcClient | null,
        private readonly logDocumentProvider: LogDocumentProvider
    ) { }

    /**
     * Show logs for a specific service in an editor tab
     */
    async showLogs(serviceName: string, lines?: number): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            this.currentService = serviceName;

            const result = await rpcClient.request('getLogs', {
                service: serviceName,
                lines: lines || 'all'
            }) as { logs: LogLine[] };

            // Format logs and set them in the document provider
            const formattedLines = result.logs.map(log => {
                const prefix = log.stream === 'stderr' ? '[stderr]' : '[stdout]';
                return `${log.timestamp} ${prefix} ${log.content}`;
            });

            this.logDocumentProvider.setLogs(serviceName, formattedLines);

            // Open the log document in an editor tab
            const uri = this.logDocumentProvider.createUri(serviceName);
            const doc = await vscode.workspace.openTextDocument(uri);
            await vscode.window.showTextDocument(doc, { preview: false });
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to get logs for ${serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Append a log line in real-time
     */
    appendLogLine(serviceName: string, log: LogLine): void {
        const prefix = log.stream === 'stderr' ? '[stderr]' : '[stdout]';
        const formattedLine = `${log.timestamp} ${prefix} ${log.content}`;
        this.logDocumentProvider.appendLog(serviceName, formattedLine);
    }

    /**
     * Clear logs for a service
     */
    clear(serviceName?: string): void {
        if (serviceName) {
            this.logDocumentProvider.clearLogs(serviceName);
        }
        this.currentService = null;
    }

    /**
     * Dispose of resources
     */
    dispose(): void {
        // LogDocumentProvider is disposed separately via context.subscriptions
    }
}
