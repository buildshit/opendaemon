import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';
import { ServiceTreeItem } from './tree-view';
import { LogManager } from './logs';

export class CommandManager {
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly getRpcClient: () => RpcClient | null,
        private readonly logManager: LogManager
    ) {}

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
                    await rpcClient.request('StartAll');
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
                    await rpcClient.request('StopAll');
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
                    await rpcClient.request('StartService', { service: item.serviceName });
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
                    await rpcClient.request('StopService', { service: item.serviceName });
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
                    await rpcClient.request('RestartService', { service: item.serviceName });
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
}
