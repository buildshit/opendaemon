import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';

export interface LogLine {
    timestamp: string;
    content: string;
    stream: 'stdout' | 'stderr';
}

export class LogManager {
    private outputChannel: vscode.OutputChannel;
    private currentService: string | null = null;

    constructor(
        private readonly getRpcClient: () => RpcClient | null
    ) {
        this.outputChannel = vscode.window.createOutputChannel('OpenDaemon');
    }

    /**
     * Show logs for a specific service
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

            this.displayLogs(serviceName, result.logs);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to get logs for ${serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Display logs in the output channel
     */
    private displayLogs(serviceName: string, logs: LogLine[]): void {
        this.outputChannel.clear();
        this.outputChannel.appendLine(`=== Logs for ${serviceName} ===`);
        this.outputChannel.appendLine('');

        for (const log of logs) {
            const prefix = log.stream === 'stderr' ? '[stderr]' : '[stdout]';
            this.outputChannel.appendLine(`${log.timestamp} ${prefix} ${log.content}`);
        }

        this.outputChannel.show();
    }

    /**
     * Append a log line in real-time
     */
    appendLogLine(serviceName: string, log: LogLine): void {
        // Only append if we're currently viewing this service's logs
        if (this.currentService === serviceName) {
            const prefix = log.stream === 'stderr' ? '[stderr]' : '[stdout]';
            this.outputChannel.appendLine(`${log.timestamp} ${prefix} ${log.content}`);
        }
    }

    /**
     * Clear the output channel
     */
    clear(): void {
        this.outputChannel.clear();
        this.currentService = null;
    }

    /**
     * Dispose of resources
     */
    dispose(): void {
        this.outputChannel.dispose();
    }
}
