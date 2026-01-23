import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';
import { ServiceTreeItem, ServiceInfo } from './tree-view';
import { LogManager } from './logs';
import { ErrorDisplayManager } from './error-display';

export class CommandManager {
    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly getRpcClient: () => RpcClient | null,
        private readonly logManager: LogManager,
        private readonly getTreeDataProvider: () => { getAllServices(): ServiceInfo[] } | null,
        private readonly refreshServices: () => Promise<void>,
        private readonly getErrorDisplayManager?: () => ErrorDisplayManager | null,
        private readonly getConfigPath?: () => string | null
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

        this.context.subscriptions.push(
            vscode.commands.registerCommand(
                'opendaemon.refresh',
                () => this.refresh()
            )
        );
    }

    private async getServiceItem(item?: ServiceTreeItem): Promise<ServiceTreeItem | undefined> {
        if (item) {
            return item;
        }

        const treeDataProvider = this.getTreeDataProvider();
        if (!treeDataProvider) {
            const selection = await vscode.window.showErrorMessage(
                'OpenDaemon tree view not initialized. Please reload the window.',
                'Reload Window'
            );
            if (selection === 'Reload Window') {
                await vscode.commands.executeCommand('workbench.action.reloadWindow');
            }
            return undefined;
        }

        const services = treeDataProvider.getAllServices();
        if (services.length === 0) {
            const configPath = this.getConfigPath?.();
            if (!configPath) {
                const selection = await vscode.window.showErrorMessage(
                    'No dmn.json file found in workspace. Would you like to create one?',
                    'Create dmn.json'
                );
                if (selection === 'Create dmn.json') {
                    await vscode.commands.executeCommand('opendaemon.createConfig');
                }
            } else {
                const selection = await vscode.window.showErrorMessage(
                    'No services found in dmn.json. Please add services to your configuration.',
                    'Open dmn.json'
                );
                if (selection === 'Open dmn.json') {
                    const doc = await vscode.workspace.openTextDocument(configPath);
                    await vscode.window.showTextDocument(doc);
                }
            }
            return undefined;
        }

        const items = services.map(s => ({
            label: s.name,
            description: String(s.status),
            service: s
        }));

        const selected = await vscode.window.showQuickPick(items, {
            placeHolder: 'Select a service'
        });

        if (selected) {
            const s = selected.service;
            return new ServiceTreeItem(s.name, s.status, s.exitCode);
        }

        return undefined;
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
    private async startService(item?: ServiceTreeItem): Promise<void> {
        const targetItem = await this.getServiceItem(item);
        if (!targetItem) {
            return;
        }

        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Starting ${targetItem.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('startService', { service: targetItem.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${targetItem.serviceName} started`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to start ${targetItem.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Stop a specific service
     */
    private async stopService(item?: ServiceTreeItem): Promise<void> {
        const targetItem = await this.getServiceItem(item);
        if (!targetItem) {
            return;
        }

        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Stopping ${targetItem.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('stopService', { service: targetItem.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${targetItem.serviceName} stopped`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to stop ${targetItem.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Restart a specific service
     */
    private async restartService(item?: ServiceTreeItem): Promise<void> {
        const targetItem = await this.getServiceItem(item);
        if (!targetItem) {
            return;
        }

        const rpcClient = this.getRpcClient();
        if (!rpcClient) {
            vscode.window.showErrorMessage('OpenDaemon is not running');
            return;
        }

        try {
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: `Restarting ${targetItem.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('restartService', { service: targetItem.serviceName });
                }
            );

            vscode.window.showInformationMessage(`Service ${targetItem.serviceName} restarted`);
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to restart ${targetItem.serviceName}: ${err instanceof Error ? err.message : String(err)}`
            );
        }
    }

    /**
     * Show logs for a specific service
     */
    private async showLogs(item?: ServiceTreeItem): Promise<void> {
        const targetItem = await this.getServiceItem(item);
        if (!targetItem) {
            return;
        }
        await this.logManager.showLogs(targetItem.serviceName);
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

    /**
     * Refresh services
     */
    private async refresh(): Promise<void> {
        await this.refreshServices();
    }
}
