import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';
import { ServiceTreeItem } from './tree-view';
import { LogManager } from './logs';
import { ErrorDisplayManager } from './error-display';

export class CommandManager {
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly getRpcClient: () => RpcClient | null,
        private readonly logManager: LogManager,
        private readonly getErrorDisplayManager?: () => ErrorDisplayManager | null
    ) { }

    /**
     * Register all commands
     */
    registerCommands(): void {
        this.context.subscriptions.push(
            vscode.commands.registerCommand('opendaemon.startAll', () => this.startAll())
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand('opendaemon.stopAll', () => this.stopAll())
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.startService',
                (item: ServiceTreeItem) => this.startService(item)
            )
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.stopService',
                (item: ServiceTreeItem) => this.stopService(item)
            )
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.restartService',
                (item: ServiceTreeItem) => this.restartService(item)
            )
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.showLogs',
                (item: ServiceTreeItem) => this.showLogs(item)
            )
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.showErrors',
                () => this.showErrors()
            )
        );

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.clearErrors',
                () => this.clearErrors()
            )
        );
    }

    /**
     * Start all services
     */
    private async startAll(): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Starting all services...',
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('startAll');
                }
            );

            vscode.window.showInformationMessage('All services started');
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to start services: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Stop all services
     */
    private async stopAll(): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Stopping all services...',
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('stopAll');
                }
            );

            vscode.window.showInformationMessage('All services stopped');
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to stop services: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Start a specific service
     */
    private async startService(item: ServiceTreeItem): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Starting ${item.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('startService', { service: item.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${item.serviceName} started`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to start ${item.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Stop a specific service
     */
    private async stopService(item: ServiceTreeItem): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Stopping ${item.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('stopService', { service: item.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${item.serviceName} stopped`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to stop ${item.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Restart a specific service
     */
    private async restartService(item: ServiceTreeItem): Promise<void> {
        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Restarting ${item.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('restartService', { service: item.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${item.serviceName} restarted`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to restart ${item.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Show logs for a specific service
     */
    private async showLogs(item: ServiceTreeItem): Promise<void> {
        await this.logManager.showLogs(item.serviceName);
    }

    /**
     * Show error history
     */
    private showErrors(): void {
        const errorDisplayManager = this.getErrorDisplayManager?.();
        if (!errorDisplayManager) {
            vscode.window.showInformationMessage('No errors to display');
            return;
        }

        errorDisplayManager.showOutputPanel();
    }

    /**
     * Clear error history
     */
    private clearErrors(): void {
        const errorDisplayManager = this.getErrorDisplayManager?.();
        if (!errorDisplayManager) {
            return;
        }

        errorDisplayManager.clearHistory();
        errorDisplayManager.clearOutputPanel();
        vscode.window.showInformationMessage('Error history cleared');
    }
}
