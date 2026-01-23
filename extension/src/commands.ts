import * as vscode from 'vscode';
import { RpcClient } from './rpc-client';
import { ServiceTreeItem, ServiceInfo, ServiceStatus, ServiceTreeDataProvider } from './tree-view';
import { LogManager } from './logs';
import { ErrorDisplayManager, ErrorCategory } from './error-display';
import { TerminalManager } from './terminal-manager';
import { ActivityLogger } from './activity-logger';

export class CommandManager {
    private terminalManager: TerminalManager;
    private activityLogger: ActivityLogger | null;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly getRpcClient: () => RpcClient | null,
        private readonly logManager: LogManager,
        private readonly getTreeDataProvider: () => { getAllServices(): ServiceInfo[] } | null,
        private readonly refreshServices: () => Promise<void>,
        private readonly getErrorDisplayManager?: () => ErrorDisplayManager | null,
        private readonly getConfigPath?: () => string | null,
        activityLogger?: ActivityLogger | null
    ) {
        this.activityLogger = activityLogger || null;
        this.terminalManager = new TerminalManager(this.activityLogger || undefined);
        context.subscriptions.push(this.terminalManager);
    }

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
                'opendaemon.showTerminal',
                (item: ServiceTreeItem) => this.showTerminal(item)
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
            // Log the action
            if (this.activityLogger) {
                this.activityLogger.logServiceAction('all', 'Starting all services');
            }

            // Get all services to create terminals for them
            const treeDataProvider = this.getTreeDataProvider();
            const services = treeDataProvider ? treeDataProvider.getAllServices() : [];

            // Create terminals for all services BEFORE starting
            for (const service of services) {
                this.terminalManager.getOrCreateTerminal(service.name);
            }

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

            // Show terminals for all services
            for (const service of services) {
                this.terminalManager.showTerminal(service.name, true);
            }

            vscode.window.showInformationMessage('All services started');
            
            // Log success
            if (this.activityLogger) {
                this.activityLogger.logServiceAction('all', 'All services started successfully');
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error
            if (this.activityLogger) {
                this.activityLogger.logError('startAll()', errorMessage);
            }
            
            vscode.window.showErrorMessage(
                `Failed to start services: ${errorMessage}`
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
            // Log the action
            if (this.activityLogger) {
                this.activityLogger.logServiceAction('all', 'Stopping all services');
            }

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

            // Close all terminals (terminals will also be closed via notification handlers)
            this.terminalManager.closeAllTerminals();

            vscode.window.showInformationMessage('All services stopped');
            
            // Log success
            if (this.activityLogger) {
                this.activityLogger.logServiceAction('all', 'All services stopped successfully');
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error
            if (this.activityLogger) {
                this.activityLogger.logError('stopAll()', errorMessage);
            }
            
            vscode.window.showErrorMessage(
                `Failed to stop services: ${errorMessage}`
            );
        }
    }

    /**
     * Get all dependencies for a service (recursively)
     * Tries RPC first, falls back to reading config file directly
     */
    private async getDependencyChain(serviceName: string): Promise<string[]> {
        const visited = new Set<string>();
        const result: string[] = [];
        
        // Try to get config for fallback
        let configServices: Record<string, { depends_on?: string[] }> | null = null;
        const configPath = this.getConfigPath?.();
        if (configPath) {
            try {
                const fs = require('fs');
                const configContent = fs.readFileSync(configPath, 'utf-8');
                const config = JSON.parse(configContent);
                configServices = config.services || null;
            } catch {
                // Ignore config read errors
            }
        }

        const visit = async (name: string) => {
            if (visited.has(name)) {
                return;
            }
            visited.add(name);

            let deps: string[] = [];
            
            // Try RPC first
            const rpcClient = this.getRpcClient();
            if (rpcClient) {
                try {
                    const response = await rpcClient.request('getDependencies', { service: name }) as {
                        dependencies?: string[];
                    };
                    deps = response?.dependencies || [];
                } catch {
                    // RPC failed, fall back to config
                    if (configServices && configServices[name]) {
                        deps = configServices[name].depends_on || [];
                        
                        // Log fallback
                        if (this.activityLogger) {
                            this.activityLogger.log(`Using config-based dependencies for ${name}: ${deps.join(', ') || 'none'}`);
                        }
                    }
                }
            } else if (configServices && configServices[name]) {
                // No RPC client, use config directly
                deps = configServices[name].depends_on || [];
            }
            
            // Visit dependencies first (depth-first)
            for (const dep of deps) {
                await visit(dep);
            }
            
            // Add this service after its dependencies
            if (name !== serviceName) { // Don't add the original service
                result.push(name);
            }
        };

        await visit(serviceName);
        return result;
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
            // Log the action
            if (this.activityLogger) {
                this.activityLogger.logServiceAction(targetItem.serviceName, 'Starting service');
            }

            // Get dependency chain and create terminals for all services
            const dependencies = await this.getDependencyChain(targetItem.serviceName);
            
            // Create terminals for dependencies first
            for (const dep of dependencies) {
                this.terminalManager.getOrCreateTerminal(dep);
            }
            
            // Create terminal for the target service
            this.terminalManager.getOrCreateTerminal(targetItem.serviceName);

            // Update tree view to show Starting status immediately
            await vscode.commands.executeCommand('opendaemon.refresh');

            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: dependencies.length > 0 
                        ? `Starting ${targetItem.serviceName} (and ${dependencies.length} dependencies)...`
                        : `Starting ${targetItem.serviceName}...`,
                    cancellable: false
                },
                async () => {
                    await rpcClient.request('startService', { service: targetItem.serviceName });
                }
            );

            // Show terminal after service starts
            this.terminalManager.showTerminal(targetItem.serviceName, true);

            const depMessage = dependencies.length > 0 
                ? ` (with dependencies: ${dependencies.join(', ')})`
                : '';
            // Note: The service is now starting, but may not be ready yet
            // Status will be updated via serviceReady/serviceFailed notifications
            vscode.window.showInformationMessage(`Service ${targetItem.serviceName} is starting${depMessage}`);
            
            // Log success
            if (this.activityLogger) {
                this.activityLogger.logServiceAction(targetItem.serviceName, `Service started successfully${depMessage}`);
            }
            
            // Refresh to get latest status from daemon
            await vscode.commands.executeCommand('opendaemon.refresh');
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error
            if (this.activityLogger) {
                this.activityLogger.logError(`startService(${targetItem.serviceName})`, errorMessage);
            }
            
            // Close the terminal since the service failed to start
            try {
                this.terminalManager.closeTerminal(targetItem.serviceName);
                
                if (this.activityLogger) {
                    this.activityLogger.logTerminalAction(targetItem.serviceName, 'Terminal closed on start failure');
                }
            } catch {
                // Ignore terminal close errors
            }
            
            // Update status to failed in tree view  
            const treeDataProvider = this.getTreeDataProvider() as ServiceTreeDataProvider | null;
            if (treeDataProvider) {
                treeDataProvider.updateServiceStatus(targetItem.serviceName, ServiceStatus.Failed);
            }
            
            vscode.window.showErrorMessage(
                `Failed to start ${targetItem.serviceName}: ${errorMessage}`
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

        // Log the action
        if (this.activityLogger) {
            this.activityLogger.logServiceAction(targetItem.serviceName, 'Stopping service');
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
            
            // Log success
            if (this.activityLogger) {
                this.activityLogger.logServiceAction(targetItem.serviceName, 'Service stopped successfully');
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error
            if (this.activityLogger) {
                this.activityLogger.logError(`stopService(${targetItem.serviceName})`, errorMessage);
            }
            
            vscode.window.showErrorMessage(
                `Failed to stop ${targetItem.serviceName}: ${errorMessage}`
            );
        } finally {
            // Always close the terminal when stopping, regardless of RPC success/failure
            // The notification handler will also try to close it, but this ensures cleanup
            // even if the RPC times out before the notification is sent
            try {
                this.terminalManager.closeTerminal(targetItem.serviceName);
            } catch {
                // Ignore errors closing terminal
            }
            
            // Update tree view status to Stopped
            const treeDataProvider = this.getTreeDataProvider() as ServiceTreeDataProvider | null;
            if (treeDataProvider) {
                treeDataProvider.updateServiceStatus(targetItem.serviceName, ServiceStatus.Stopped);
            }
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
            // Log the action
            if (this.activityLogger) {
                this.activityLogger.logServiceAction(targetItem.serviceName, 'Restarting service');
            }

            // Close and recreate terminal for clean restart
            this.terminalManager.closeTerminal(targetItem.serviceName);
            this.terminalManager.getOrCreateTerminal(targetItem.serviceName);

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

            // Show terminal after restart
            this.terminalManager.showTerminal(targetItem.serviceName, true);

            vscode.window.showInformationMessage(`Service ${targetItem.serviceName} restarted`);
            
            // Log success
            if (this.activityLogger) {
                this.activityLogger.logServiceAction(targetItem.serviceName, 'Service restarted successfully');
            }
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error
            if (this.activityLogger) {
                this.activityLogger.logError(`restartService(${targetItem.serviceName})`, errorMessage);
            }
            
            vscode.window.showErrorMessage(
                `Failed to restart ${targetItem.serviceName}: ${errorMessage}`
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
     * Show terminal for a specific service
     */
    private async showTerminal(item?: ServiceTreeItem): Promise<void> {
        const targetItem = await this.getServiceItem(item);
        if (!targetItem) {
            return;
        }

        // Log the manual terminal command invocation
        if (this.activityLogger) {
            this.activityLogger.logTerminalAction(
                targetItem.serviceName,
                'Manual terminal command invoked'
            );
        }

        try {
            // Show the terminal for this service (terminal remains open even if log fetch fails)
            this.terminalManager.showTerminal(targetItem.serviceName);
        } catch (err) {
            const errorMessage = err instanceof Error ? err.message : String(err);
            
            // Log error to activity channel
            if (this.activityLogger) {
                this.activityLogger.logError(
                    `Showing terminal for ${targetItem.serviceName}`,
                    errorMessage
                );
            }
            
            // Log error to error display manager
            const errorDisplayManager = this.getErrorDisplayManager?.();
            if (errorDisplayManager) {
                await errorDisplayManager.displayError({
                    message: `Failed to show terminal for ${targetItem.serviceName}`,
                    category: ErrorCategory.ORCHESTRATOR,
                    service: targetItem.serviceName,
                    details: errorMessage
                });
            }
            
            // Show user notification
            vscode.window.showErrorMessage(
                `Failed to show terminal for ${targetItem.serviceName}: ${errorMessage}`
            );
            
            return;
        }

        // Fetch and display recent logs in the terminal
        const rpcClient = this.getRpcClient();
        if (rpcClient) {
            try {
                // Log the action
                if (this.activityLogger) {
                    this.activityLogger.logTerminalAction(
                        targetItem.serviceName,
                        'Fetching historical logs',
                        'lines: 100'
                    );
                }

                const response = await rpcClient.request('getLogs', {
                    service: targetItem.serviceName,
                    lines: 100
                }) as { logs?: string[] };

                if (response && response.logs && Array.isArray(response.logs)) {
                    // Clear terminal first
                    this.terminalManager.clearTerminal(targetItem.serviceName);
                    
                    // Write logs to terminal
                    this.terminalManager.writeLines(targetItem.serviceName, response.logs);
                    
                    // Log success
                    if (this.activityLogger) {
                        this.activityLogger.logTerminalAction(
                            targetItem.serviceName,
                            'Historical logs fetched',
                            `${response.logs.length} lines`
                        );
                    }
                }
            } catch (err) {
                const errorMessage = err instanceof Error ? err.message : String(err);
                
                // Log error to activity channel
                if (this.activityLogger) {
                    this.activityLogger.logError(
                        `Fetching historical logs for ${targetItem.serviceName}`,
                        errorMessage
                    );
                }
                
                // Log error to error display manager
                const errorDisplayManager = this.getErrorDisplayManager?.();
                if (errorDisplayManager) {
                    await errorDisplayManager.displayError({
                        message: `Failed to fetch logs for ${targetItem.serviceName}`,
                        category: ErrorCategory.RPC,
                        service: targetItem.serviceName,
                        details: errorMessage
                    });
                }
                
                // Show user notification
                vscode.window.showErrorMessage(
                    `Failed to fetch logs for ${targetItem.serviceName}: ${errorMessage}`
                );
                
                // Terminal remains open for new logs to stream
            }
        }
    }

    /**
     * Get the terminal manager instance
     */
    getTerminalManager(): TerminalManager {
        return this.terminalManager;
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
