import * as vscode from 'vscode';
import * as path from 'path';
import { DaemonManager } from './daemon';
import { RpcClient } from './rpc-client';
import { ServiceTreeDataProvider, ServiceStatus } from './tree-view';
import { CommandManager } from './commands';
import { LogManager, LogLine } from './logs';
import { ConfigWizard } from './wizard';
import { DmnFileWatcher } from './file-watcher';
import { ErrorDisplayManager, ErrorCategory } from './error-display';
import { LogDocumentProvider } from './log-document-provider';
import { ActivityLogger } from './activity-logger';
import { CLIIntegrationManager } from './cli-integration/cli-integration-manager';
import { getCLILogger } from './cli-integration/cli-logger';

let daemonManager: DaemonManager | null = null;
let cliManager: CLIIntegrationManager | null = null;
let rpcClient: RpcClient | null = null;
let treeDataProvider: ServiceTreeDataProvider | null = null;
let commandManager: CommandManager | null = null;
let logManager: LogManager | null = null;
let fileWatcher: DmnFileWatcher | null = null;
let errorDisplayManager: ErrorDisplayManager | null = null;
let extensionContext: vscode.ExtensionContext | null = null;
let logDocumentProvider: LogDocumentProvider | null = null;
let activityLogger: ActivityLogger | null = null;
let statusRefreshInterval: NodeJS.Timeout | null = null;
let statusRefreshInFlight = false;

// Interval for periodic status refresh (as a fallback to real-time notifications)
const STATUS_REFRESH_INTERVAL_MS = 2000;
const STOP_ALL_BEFORE_SHUTDOWN_TIMEOUT_MS = 8000;

export async function activate(context: vscode.ExtensionContext) {
    console.log('OpenDaemon extension is now active');

    // Store context for use in other functions
    extensionContext = context;

    // Initialize activity logger
    activityLogger = new ActivityLogger();
    context.subscriptions.push(activityLogger);
    activityLogger.log('Extension activated');

    // Initialize error display manager
    errorDisplayManager = new ErrorDisplayManager({
        openConfig: async () => await openDmnConfig(),
        showLogs: (service?: string) => {
            if (service && logManager) {
                logManager.showLogs(service);
            } else {
                errorDisplayManager?.showOutputPanel();
            }
        },
        reload: () => {
            vscode.commands.executeCommand('workbench.action.reloadWindow');
        },
        retry: async () => {
            // Retry by reloading the configuration
            await handleConfigChanged();
        }
    });

    // Initialize tree view
    treeDataProvider = new ServiceTreeDataProvider();
    vscode.window.registerTreeDataProvider('opendaemon.services', treeDataProvider);

    // Initialize log document provider for editor-tab-based log viewing
    logDocumentProvider = new LogDocumentProvider();
    context.subscriptions.push(
        vscode.workspace.registerTextDocumentContentProvider('opendaemon-log', logDocumentProvider)
    );

    // Initialize log manager with document provider
    logManager = new LogManager(() => rpcClient, logDocumentProvider);

    // Initialize command manager
    commandManager = new CommandManager(
        context,
        () => rpcClient,
        logManager,
        () => treeDataProvider,
        async () => await loadServices(),
        () => errorDisplayManager,
        () => fileWatcher?.getConfigPath() ?? null,
        activityLogger
    );
    commandManager.registerCommands();

    // Register CLI integration commands
    context.subscriptions.push(
        vscode.commands.registerCommand('opendaemon.newTerminalWithCLI', async () => {
            if (cliManager) {
                try {
                    const terminal = await cliManager.createTerminalWithCLI();
                    terminal.show();
                } catch (err) {
                    const errorMsg = err instanceof Error ? err.message : String(err);
                    vscode.window.showErrorMessage(`Failed to create terminal: ${errorMsg}`);
                }
            } else {
                vscode.window.showWarningMessage('CLI integration not available');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('opendaemon.showCLIInfo', async () => {
            if (cliManager) {
                try {
                    await cliManager.showCLIInfo();
                } catch (err) {
                    const errorMsg = err instanceof Error ? err.message : String(err);
                    vscode.window.showErrorMessage(`Failed to show CLI info: ${errorMsg}`);
                }
            } else {
                vscode.window.showWarningMessage('CLI integration not available');
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('opendaemon.installCLIGlobally', async () => {
            if (cliManager) {
                try {
                    await cliManager.showGlobalInstallInstructions();
                } catch (err) {
                    const errorMsg = err instanceof Error ? err.message : String(err);
                    vscode.window.showErrorMessage(`Failed to show installation instructions: ${errorMsg}`);
                }
            } else {
                vscode.window.showWarningMessage('CLI integration not available');
            }
        })
    );

    // Register command to show CLI logs
    context.subscriptions.push(
        vscode.commands.registerCommand('opendaemon.showCLILogs', () => {
            const logger = getCLILogger();
            logger.show();
        })
    );

    // Register command to run CLI diagnostics
    context.subscriptions.push(
        vscode.commands.registerCommand('opendaemon.runCLIDiagnostics', async () => {
            if (cliManager) {
                try {
                    await cliManager.runDiagnostics();
                } catch (err) {
                    const errorMsg = err instanceof Error ? err.message : String(err);
                    vscode.window.showErrorMessage(`Failed to run diagnostics: ${errorMsg}`);
                }
            } else {
                vscode.window.showWarningMessage('CLI integration is not active.');
            }
        })
    );

    // Initialize file watcher
    fileWatcher = new DmnFileWatcher(
        async () => await handleConfigChanged(),
        async () => await handleConfigDeleted()
    );

    // Get the CLI logger to output diagnostics to the visible output channel
    const logger = getCLILogger();

    // Initialize CLI integration manager
    try {
        cliManager = new CLIIntegrationManager(context);
        await cliManager.activate();
        logger.info('CLI integration activated successfully');
    } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.error('Failed to activate CLI integration:', errorMsg);
        // Continue with extension activation even if CLI integration fails
    }

    // Check for dmn.json in workspace (or offer to create)
    logger.info('========================================');
    logger.info('    Daemon Initialization Start    ');
    logger.info('========================================');
    logger.info('Looking for dmn.json configuration...');
    let dmnConfigPath: string | null = null;

    try {
        dmnConfigPath = await findDmnConfig();
        logger.info(`findDmnConfig result: ${dmnConfigPath || 'NOT FOUND'}`);
    } catch (err) {
        const errorMsg = err instanceof Error ? err.message : String(err);
        logger.error('Error finding dmn.json:', errorMsg);
    }

    if (!dmnConfigPath) {
        logger.info('dmn.json not found, checking ConfigWizard...');
        try {
            // Offer to create dmn.json
            dmnConfigPath = await ConfigWizard.detectAndOfferCreation();
            logger.info(`ConfigWizard result: ${dmnConfigPath || 'NOT CREATED'}`);
        } catch (err) {
            const errorMsg = err instanceof Error ? err.message : String(err);
            logger.error('Error in ConfigWizard:', errorMsg);
        }
    }

    if (dmnConfigPath) {
        logger.info(`Initializing daemon with config: ${dmnConfigPath}`);
        try {
            await initializeDaemon(dmnConfigPath);
            logger.info('Daemon initialization complete');
            logger.info('========================================');
            logger.info('    Daemon Initialization COMPLETE    ');
            logger.info('========================================');
        } catch (err) {
            const errorMsg = err instanceof Error ? err.message : String(err);
            logger.error('Error initializing daemon:', errorMsg);
            if (err instanceof Error && err.stack) {
                logger.error('Stack trace:', err.stack);
            }
        }
    } else {
        logger.warn('No dmn.json found or created - services panel will be empty');
        logger.info('========================================');
    }
}

export async function deactivate() {
    console.log('OpenDaemon extension is now deactivated');

    // Log deactivation before disposing
    if (activityLogger) {
        activityLogger.log('Extension deactivated');
    }

    // Best-effort service shutdown before daemon teardown so ports are released.
    await requestStopAllBeforeShutdown('extension deactivation');

    // Deactivate CLI integration
    if (cliManager) {
        await cliManager.deactivate();
        cliManager = null;
    }

    // Stop periodic status refresh
    stopPeriodicStatusRefresh();

    if (fileWatcher) {
        fileWatcher.stop();
        fileWatcher = null;
    }

    if (errorDisplayManager) {
        errorDisplayManager.dispose();
        errorDisplayManager = null;
    }

    if (logManager) {
        logManager.dispose();
        logManager = null;
    }

    if (rpcClient) {
        rpcClient.dispose();
        rpcClient = null;
    }

    if (daemonManager) {
        await daemonManager.stop();
        daemonManager = null;
    }

    if (treeDataProvider) {
        treeDataProvider.clear();
        treeDataProvider = null;
    }

    if (activityLogger) {
        activityLogger.dispose();
        activityLogger = null;
    }
}

/**
 * Initialize daemon with config
 */
async function initializeDaemon(dmnConfigPath: string): Promise<void> {
    console.log(`[OpenDaemon] Initializing with config: ${dmnConfigPath}`);

    vscode.window.showInformationMessage(
        `OpenDaemon: Found configuration at ${path.basename(dmnConfigPath)}`
    );

    // Step 1: Start file watcher
    console.log('[OpenDaemon] Step 1: Starting file watcher...');
    if (fileWatcher) {
        fileWatcher.start(dmnConfigPath);
        console.log('[OpenDaemon] File watcher started successfully');
    } else {
        console.warn('[OpenDaemon] File watcher not initialized');
    }

    // Step 2: Initialize daemon manager
    console.log('[OpenDaemon] Step 2: Initializing daemon manager...');
    daemonManager = new DaemonManager(
        extensionContext!,
        (data) => handleDaemonStdout(data),
        (data) => handleDaemonStderr(data)
    );
    console.log('[OpenDaemon] Daemon manager initialized');

    // Step 3: Initialize RPC client
    console.log('[OpenDaemon] Step 3: Initializing RPC client...');
    rpcClient = new RpcClient((data) => {
        if (daemonManager) {
            daemonManager.write(data);
        }
    }, activityLogger);

    // Listen for notifications from daemon
    rpcClient.on('notification', (method, params) => {
        handleDaemonNotification(method, params);
    });
    console.log('[OpenDaemon] RPC client initialized and listening for notifications');

    // Step 4: Load services from config file to populate tree view immediately
    // This is outside try/catch so tree view works even if daemon fails
    console.log('[OpenDaemon] Step 4: Loading services from config...');
    await loadServicesFromConfig(dmnConfigPath);

    // Log tree view state after loading services
    const treeServices = treeDataProvider?.getAllServices() || [];
    console.log(`[OpenDaemon] Tree view now has ${treeServices.length} services: ${treeServices.map(s => s.name).join(', ')}`);
    if (treeServices.length === 0) {
        console.warn('[OpenDaemon] Warning: No services loaded into tree view');
    }

    // Step 5: Start daemon
    console.log('[OpenDaemon] Step 5: Starting daemon...');
    try {
        await daemonManager.start(dmnConfigPath);
        console.log('[OpenDaemon] Daemon started successfully');

        // Step 6: Set up stdin writer and close handler for terminals
        console.log('[OpenDaemon] Step 6: Configuring terminal handlers...');
        if (commandManager && rpcClient) {
            const terminalManager = commandManager.getTerminalManager();

            // Set up stdin writer for forwarding input to daemon
            terminalManager.setStdinWriter(async (serviceName: string, data: string) => {
                if (rpcClient) {
                    try {
                        await rpcClient.request('writeStdin', { service: serviceName, data });
                    } catch (err) {
                        console.error(`Failed to write stdin to ${serviceName}:`, err);
                    }
                }
            });

            // Set up terminal close handler for two-way sync (close terminal -> stop service)
            terminalManager.setTerminalCloseHandler(async (serviceName: string) => {
                if (rpcClient) {
                    try {
                        if (activityLogger) {
                            activityLogger.logServiceAction(serviceName, 'Stopping service (terminal closed by user)');
                        }

                        await rpcClient.request('stopService', { service: serviceName });

                        // Update tree view status to NotStarted (since user closed terminal)
                        if (treeDataProvider) {
                            treeDataProvider.updateServiceStatus(serviceName, ServiceStatus.NotStarted);
                        }

                        if (activityLogger) {
                            activityLogger.logServiceAction(serviceName, 'Service stopped (terminal closed by user)');
                        }
                    } catch (err) {
                        console.error(`Failed to stop service ${serviceName} after terminal close:`, err);
                        if (activityLogger) {
                            activityLogger.logError(
                                `Stopping service ${serviceName}`,
                                err instanceof Error ? err.message : String(err)
                            );
                        }
                    }
                }
            });

            console.log('[OpenDaemon] Terminal handlers configured');
        }

        // Step 7: Load actual service statuses from daemon
        // This synchronizes the tree view with the daemon's actual service states
        console.log('[OpenDaemon] Step 7: Synchronizing service statuses with daemon...');
        console.log('[OpenDaemon] Verifying loadServices() is called after daemon startup...');
        await loadServices();
        console.log('[OpenDaemon] Service status synchronization complete');

        // Log final tree view state to verify synchronization
        const finalServices = treeDataProvider?.getAllServices() || [];
        console.log(`[OpenDaemon] Final tree view state after synchronization: ${finalServices.length} services`);
        if (finalServices.length > 0) {
            finalServices.forEach(s => {
                console.log(`[OpenDaemon]   - ${s.name}: ${s.status}${s.exitCode !== undefined ? ` (exit code: ${s.exitCode})` : ''}`);
            });
        } else {
            console.warn('[OpenDaemon] Warning: Tree view is empty after synchronization');
        }

        console.log('[OpenDaemon] Initialization complete');

        // Step 8: Start periodic status refresh as a fallback
        console.log('[OpenDaemon] Step 8: Starting periodic status refresh...');
        startPeriodicStatusRefresh();
        console.log('[OpenDaemon] Periodic status refresh started');
    } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        console.error('[OpenDaemon] Failed to start daemon:', errorMessage);

        if (errorDisplayManager) {
            await errorDisplayManager.displayError({
                message: `Failed to start OpenDaemon daemon: ${errorMessage}`,
                category: ErrorCategory.ORCHESTRATOR,
                details: err instanceof Error ? err.stack : undefined
            });
        } else {
            vscode.window.showErrorMessage(
                `Failed to start OpenDaemon: ${errorMessage}`
            );
        }
    }
}

/**
 * Handle config file changes
 */
async function handleConfigChanged(): Promise<void> {
    // Stop periodic status refresh
    stopPeriodicStatusRefresh();

    // Best-effort service shutdown before daemon restart on config updates.
    await requestStopAllBeforeShutdown('config change');

    // Stop current daemon
    if (daemonManager) {
        await daemonManager.stop();
        daemonManager = null;
    }

    if (rpcClient) {
        rpcClient.dispose();
        rpcClient = null;
    }

    // Clear tree view
    if (treeDataProvider) {
        treeDataProvider.clear();
    }

    // Restart with new config (this will restart the periodic refresh)
    const configPath = fileWatcher?.getConfigPath();
    if (configPath) {
        await initializeDaemon(configPath);
    }
}

/**
 * Handle config file deletion
 */
async function handleConfigDeleted(): Promise<void> {
    // Stop periodic status refresh
    stopPeriodicStatusRefresh();

    // Best-effort service shutdown before daemon teardown.
    await requestStopAllBeforeShutdown('config deletion');

    // Stop daemon
    if (daemonManager) {
        await daemonManager.stop();
        daemonManager = null;
    }

    if (rpcClient) {
        rpcClient.dispose();
        rpcClient = null;
    }

    // Clear tree view
    if (treeDataProvider) {
        treeDataProvider.clear();
    }

    // Stop file watcher
    if (fileWatcher) {
        fileWatcher.stop();
    }

}

/**
 * Find dmn.json in any workspace folder root
 */
async function findDmnConfig(): Promise<string | null> {
    const logger = getCLILogger();
    const workspaceFolders = vscode.workspace.workspaceFolders;

    logger.info(`findDmnConfig: workspaceFolders count = ${workspaceFolders?.length ?? 0}`);

    if (!workspaceFolders || workspaceFolders.length === 0) {
        logger.warn('findDmnConfig: No workspace folders found');
        return null;
    }

    // Check all workspace folders for dmn.json
    for (const folder of workspaceFolders) {
        const dmnUri = vscode.Uri.joinPath(folder.uri, 'dmn.json');
        logger.info(`findDmnConfig: Checking for ${dmnUri.fsPath}`);

        try {
            await vscode.workspace.fs.stat(dmnUri);
            logger.info(`findDmnConfig: Found dmn.json at ${dmnUri.fsPath}`);
            return dmnUri.fsPath;
        } catch (err) {
            // File not found or error accessing
            // Continue to next folder
        }
    }

    logger.info('findDmnConfig: dmn.json not found in any workspace folder');
    return null;
}

/**
 * Start periodic status refresh
 * This acts as a fallback in case real-time notifications are missed
 */
function startPeriodicStatusRefresh(): void {
    // Stop any existing interval first
    stopPeriodicStatusRefresh();

    statusRefreshInterval = setInterval(async () => {
        await refreshStatusFromDaemon();
    }, STATUS_REFRESH_INTERVAL_MS);

    if (activityLogger) {
        activityLogger.log(`Periodic status refresh started (interval: ${STATUS_REFRESH_INTERVAL_MS}ms)`);
    }
}

/**
 * Stop periodic status refresh
 */
function stopPeriodicStatusRefresh(): void {
    if (statusRefreshInterval) {
        clearInterval(statusRefreshInterval);
        statusRefreshInterval = null;
        statusRefreshInFlight = false;

        if (activityLogger) {
            activityLogger.log('Periodic status refresh stopped');
        }
    }
}

/**
 * Try to stop all services before daemon shutdown/restart.
 * This prevents orphan processes from holding ports after extension lifecycle events.
 */
async function requestStopAllBeforeShutdown(reason: string): Promise<void> {
    const client = rpcClient;
    if (!client) {
        return;
    }

    await new Promise<void>((resolve) => {
        let settled = false;
        const finish = () => {
            if (!settled) {
                settled = true;
                clearTimeout(timeoutHandle);
                resolve();
            }
        };

        const timeoutHandle = setTimeout(() => {
            if (activityLogger) {
                activityLogger.log(
                    `Timed out waiting for stopAll before ${reason} after ${STOP_ALL_BEFORE_SHUTDOWN_TIMEOUT_MS}ms`
                );
            }
            finish();
        }, STOP_ALL_BEFORE_SHUTDOWN_TIMEOUT_MS);

        client.request('stopAll')
            .then(() => {
                if (activityLogger) {
                    activityLogger.logServiceAction('all', `stopAll acknowledged before ${reason}`);
                }
                finish();
            })
            .catch((err) => {
                if (activityLogger) {
                    activityLogger.logError(
                        `stopAll before ${reason}`,
                        err instanceof Error ? err.message : String(err)
                    );
                }
                finish();
            });
    });
}

/**
 * Refresh service status from daemon (silent, for background polling)
 * This is a lightweight version of loadServices that doesn't log errors prominently
 */
async function refreshStatusFromDaemon(): Promise<void> {
    if (!rpcClient || !treeDataProvider) {
        return;
    }

    // Prevent overlapping polling calls when daemon responses are slower than interval.
    if (statusRefreshInFlight) {
        return;
    }

    statusRefreshInFlight = true;
    try {
        const response = await rpcClient.request('getStatus') as { services: Record<string, string> };
        const result = response.services;

        if (!result || Object.keys(result).length === 0) {
            return;
        }

        // Get current services from tree view to detect changes
        const currentServices = treeDataProvider.getAllServices();
        const currentStatusMap = new Map(currentServices.map(s => [s.name, s.status]));

        // Convert status strings to ServiceStatus enum and check for changes
        let hasChanges = false;
        const services = Object.entries(result).map(([name, statusStr]) => {
            const newStatus = parseServiceStatus(statusStr);
            const currentStatus = currentStatusMap.get(name);

            if (currentStatus !== newStatus) {
                hasChanges = true;
                // Log status change detection from polling
                if (activityLogger) {
                    activityLogger.log(`[Status Refresh] ${name}: ${currentStatus} -> ${newStatus}`);
                }
            }

            return { name, status: newStatus };
        });

        // Only update tree view if there are changes
        if (hasChanges) {
            treeDataProvider.updateServices(services);
        }
    } catch (err) {
        // Silently ignore errors during background polling
        // The periodic refresh is a fallback, so we don't want to spam errors
        console.debug('[refreshStatusFromDaemon] Error during periodic refresh:', err);
    } finally {
        statusRefreshInFlight = false;
    }
}

/**
 * Load services from config file directly (without daemon)
 * This populates the tree view immediately so services are visible
 */
async function loadServicesFromConfig(configPath: string): Promise<void> {
    console.log('[loadServicesFromConfig] Starting to load services from config:', configPath);

    if (!treeDataProvider) {
        console.error('[loadServicesFromConfig] Tree data provider not initialized');
        return;
    }

    console.log('[loadServicesFromConfig] Tree data provider is initialized');

    try {
        console.log('[loadServicesFromConfig] Reading config file...');
        // Use workspace.fs instead of fs module for better compatibility
        const configUri = vscode.Uri.file(configPath);
        const configBuffer = await vscode.workspace.fs.readFile(configUri);
        const configContent = new TextDecoder().decode(configBuffer);

        console.log('[loadServicesFromConfig] Config file read successfully, length:', configContent.length);

        console.log('[loadServicesFromConfig] Parsing JSON...');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };
        console.log('[loadServicesFromConfig] JSON parsed successfully');

        if (config.services) {
            const serviceNames = Object.keys(config.services);
            console.log('[loadServicesFromConfig] Found services object with', serviceNames.length, 'services');

            if (serviceNames.length === 0) {
                console.warn('[loadServicesFromConfig] Services object is empty');
                treeDataProvider.updateServices([]);

                if (errorDisplayManager) {
                    await errorDisplayManager.displayError({
                        message: 'No services defined in dmn.json. The "services" object is empty.',
                        category: ErrorCategory.CONFIG,
                        details: `Config file: ${configPath}\n\nPlease add at least one service to your configuration.`
                    });
                }
            } else {
                console.log('[loadServicesFromConfig] Service names:', serviceNames.join(', '));

                const services = serviceNames.map(name => ({
                    name,
                    status: ServiceStatus.NotStarted
                }));

                console.log('[loadServicesFromConfig] Updating tree view with services...');
                treeDataProvider.updateServices(services);
                console.log('[loadServicesFromConfig] Successfully loaded', services.length, 'services from config:', serviceNames.join(', '));
            }
        } else {
            console.warn('[loadServicesFromConfig] No services object found in config');
            treeDataProvider.updateServices([]);

            if (errorDisplayManager) {
                await errorDisplayManager.displayError({
                    message: 'Missing "services" object in dmn.json',
                    category: ErrorCategory.CONFIG,
                    details: `Config file: ${configPath}\n\nThe configuration file must contain a "services" object with at least one service definition.`
                });
            }
        }
    } catch (err) {
        console.error('[loadServicesFromConfig] Failed to load services from config:', err);

        // Build detailed error message based on error type
        let errorMessage = 'Failed to load services from dmn.json';
        let errorDetails = `Config file: ${configPath}\n\n`;

        if (err instanceof SyntaxError) {
            console.error('[loadServicesFromConfig] JSON parsing error:', err.message);
            errorMessage = 'Invalid JSON in dmn.json';
            errorDetails += `JSON parsing error: ${err.message}\n\nPlease check that your configuration file contains valid JSON syntax.`;
        } else if (err instanceof vscode.FileSystemError) {
            console.error('[loadServicesFromConfig] File system error:', err.message);
            errorMessage = 'Error reading configuration file';
            errorDetails += `File system error: ${err.message}`;
        } else if (err instanceof Error) {
            console.error('[loadServicesFromConfig] Error details:', {
                name: err.name,
                message: err.message,
                stack: err.stack
            });
            errorDetails += `Error: ${err.message}`;
        } else {
            console.error('[loadServicesFromConfig] Unknown error type:', String(err));
            errorDetails += `Unknown error: ${String(err)}`;
        }

        // Display error using error display manager
        if (errorDisplayManager) {
            await errorDisplayManager.displayError({
                message: errorMessage,
                category: ErrorCategory.CONFIG,
                details: errorDetails
            });
        }

        // Clear tree view on error
        treeDataProvider.updateServices([]);
    }
}

/**
 * Load services from daemon
 */
async function loadServices(): Promise<void> {
    console.log('[loadServices] Starting to load service statuses from daemon...');

    if (!rpcClient) {
        console.error('[loadServices] RPC client not initialized - cannot load service statuses');
        return;
    }

    if (!treeDataProvider) {
        console.error('[loadServices] Tree data provider not initialized - cannot update service statuses');
        return;
    }

    try {
        console.log('[loadServices] Sending getStatus RPC request to daemon...');
        const response = await rpcClient.request('getStatus') as { services: Record<string, string> };
        console.log('[loadServices] Received response from daemon:', JSON.stringify(response));

        const result = response.services;

        if (!result || Object.keys(result).length === 0) {
            console.warn('[loadServices] Daemon returned no services');
            return;
        }

        // Convert status strings to ServiceStatus enum
        const services = Object.entries(result).map(([name, statusStr]) => ({
            name,
            status: parseServiceStatus(statusStr)
        }));

        console.log('[loadServices] Updating tree view with', services.length, 'service statuses');
        treeDataProvider.updateServices(services);

        // Log successful status updates for each service
        services.forEach(service => {
            console.log(`[loadServices] Successfully updated status for service "${service.name}": ${service.status}`);
        });

        console.log('[loadServices] Service status synchronization complete');
    } catch (err) {
        console.error('[loadServices] Failed to load services from daemon:', err);

        // Provide detailed error information
        let errorMessage = 'Failed to load service status from daemon';
        let errorDetails = '';

        if (err instanceof Error) {
            console.error('[loadServices] Error details:', {
                name: err.name,
                message: err.message,
                stack: err.stack
            });

            // Check for specific RPC error types
            if (err.message.includes('timeout')) {
                errorMessage = 'Timeout while communicating with daemon';
                errorDetails = 'The daemon did not respond to the status request in time. The daemon may be busy or unresponsive.';
            } else if (err.message.includes('connection')) {
                errorMessage = 'Connection error with daemon';
                errorDetails = 'Unable to establish communication with the daemon. The daemon may not be running or may have crashed.';
            } else {
                errorDetails = err.message;
            }
        } else {
            console.error('[loadServices] Unknown error type:', String(err));
            errorDetails = String(err);
        }

        if (errorDisplayManager) {
            await errorDisplayManager.displayError({
                message: errorMessage,
                category: ErrorCategory.RPC,
                details: errorDetails
            });
        }
    }
}

/**
 * Parse service status string to enum
 * Handles both snake_case (from RPC) and PascalCase formats
 * 
 * NOTE: "stopped" is mapped to NotStarted because from the user's perspective,
 * a stopped service should look the same as one that was never started -
 * it's not running and can be started. "Stopped" as a separate state is only
 * meaningful internally. "Failed" is reserved for services that crashed/errored.
 */
function parseServiceStatus(statusStr: string): ServiceStatus {
    // Normalize to lowercase for comparison
    const normalized = statusStr.toLowerCase();

    // Handle failed status with exit code format: "failed (exit code: X)"
    if (normalized.startsWith('failed')) {
        return ServiceStatus.Failed;
    }

    switch (normalized) {
        case 'running':
            return ServiceStatus.Running;
        case 'starting':
            return ServiceStatus.Starting;
        case 'stopped':
            // Map stopped to NotStarted - from user's perspective, a stopped service
            // should appear the same as one that was never started
            return ServiceStatus.NotStarted;
        case 'not_started':
        case 'notstarted':
            return ServiceStatus.NotStarted;
        default:
            return ServiceStatus.NotStarted;
    }
}

/**
 * Handle stdout from daemon process
 */
function handleDaemonStdout(data: string): void {
    console.log('[Daemon stdout]:', data);

    // Pass to RPC client for parsing
    if (rpcClient) {
        rpcClient.handleData(data);
    }
}

/**
 * Handle stderr from daemon process
 */
function handleDaemonStderr(data: string): void {
    console.error('[Daemon stderr]:', data);

    // Display stderr output as errors if they look like error messages
    if (data.toLowerCase().includes('error') || data.toLowerCase().includes('failed')) {
        if (errorDisplayManager) {
            errorDisplayManager.displayError({
                message: data.trim(),
                category: ErrorCategory.ORCHESTRATOR
            });
        }
    }
}

// Throttle log line activity logging to once per second per service
const lastLogTime = new Map<string, number>();

/**
 * Helper function to determine if we should log activity for a log line
 * Throttles to max once per second per service to avoid spam
 */
function shouldLogLine(service: string): boolean {
    const now = Date.now();
    const last = lastLogTime.get(service) || 0;
    if (now - last > 1000) {
        lastLogTime.set(service, now);
        return true;
    }
    return false;
}

/**
 * Handle notifications from daemon
 */
function handleDaemonNotification(method: string, params: unknown): void {
    console.log('[Daemon notification]:', method, params);
    const cliLogger = getCLILogger();

    try {
        // Log all notifications to activity channel
        if (activityLogger) {
            activityLogger.logRpcAction(method, 'notification', JSON.stringify(params));
        }

        // Handle error events
        if (method === 'error') {
            const { message, category } = params as {
                message: string;
                category: string;
            };

            if (errorDisplayManager) {
                errorDisplayManager.displayError({
                    message,
                    category: category as ErrorCategory
                });
            }

            // Log error notification
            if (activityLogger) {
                activityLogger.logError('Daemon notification', message);
            }
            return;
        }

        // Handle service status changes
        if (method === 'ServiceStatusChanged' && treeDataProvider) {
            const { service, status, exit_code } = params as {
                service: string;
                status: string;
                exit_code?: number;
            };

            treeDataProvider.updateServiceStatus(
                service,
                parseServiceStatus(status),
                exit_code
            );

            // Log service status change
            if (activityLogger) {
                const details = `new status: ${status}${exit_code !== undefined ? `, exit code: ${exit_code}` : ''}`;
                activityLogger.logServiceAction(service, 'Status changed', details);
            }
        }

        // Handle service starting
        if (method === 'serviceStarting' && treeDataProvider) {
            const { service } = params as { service: string };
            treeDataProvider.updateServiceStatus(service, ServiceStatus.Starting);

            // Log service starting
            if (activityLogger) {
                activityLogger.logServiceAction(service, 'Starting');
            }
        }

        // Handle service ready
        if (method === 'serviceReady' && treeDataProvider) {
            const { service } = params as { service: string };
            treeDataProvider.updateServiceStatus(service, ServiceStatus.Running);

            // Log service ready
            if (activityLogger) {
                activityLogger.logServiceAction(service, 'Ready');
            }
        }

        // Handle service failed
        if (method === 'serviceFailed' && treeDataProvider) {
            const { service, error } = params as { service: string; error: string };
            treeDataProvider.updateServiceStatus(service, ServiceStatus.Failed);

            // Close the terminal for this service since it failed
            if (commandManager) {
                try {
                    const terminalManager = commandManager.getTerminalManager();
                    terminalManager.closeTerminal(service);

                    if (activityLogger) {
                        activityLogger.logTerminalAction(service, 'Terminal closed on service failure');
                    }
                } catch (err) {
                    if (activityLogger) {
                        activityLogger.logError(
                            `Closing terminal for ${service}`,
                            err instanceof Error ? err.message : String(err)
                        );
                    }
                }
            }

            // Show error notification using error display manager
            if (errorDisplayManager) {
                errorDisplayManager.displayServiceFailure(service, error);
            }

            // Log service failure
            if (activityLogger) {
                activityLogger.logServiceAction(service, 'Failed', error);
            }
        }

        // Handle service stopped
        if (method === 'serviceStopped' && treeDataProvider) {
            const { service } = params as { service: string };
            // Use NotStarted instead of Stopped - from user's perspective, a stopped
            // service should appear the same as one that was never started
            treeDataProvider.updateServiceStatus(service, ServiceStatus.NotStarted);

            // Close the terminal for this service (do this first to ensure cleanup)
            if (commandManager) {
                try {
                    const terminalManager = commandManager.getTerminalManager();
                    terminalManager.closeTerminal(service);

                    if (activityLogger) {
                        activityLogger.logTerminalAction(service, 'Terminal closed on service stop');
                    }
                } catch (err) {
                    if (activityLogger) {
                        activityLogger.logError(
                            `Closing terminal for ${service}`,
                            err instanceof Error ? err.message : String(err)
                        );
                    }
                }
            }

            // Log service stopped
            if (activityLogger) {
                activityLogger.logServiceAction(service, 'Stopped');
            }
        }

        // Handle log lines - route to both terminal and LogManager
        if (method === 'logLine') {
            const { service, timestamp, content, stream } = params as {
                service: string;
                timestamp: number;
                content: string;
                stream: string;
            };

            const logLine: LogLine = {
                timestamp: new Date(timestamp * 1000).toISOString(),
                content,
                stream: stream as 'stdout' | 'stderr'
            };

            // Route to terminal for real-time display
            if (commandManager) {
                try {
                    const terminalManager = commandManager.getTerminalManager();
                    terminalManager.writeLogLine(service, logLine);
                } catch (err) {
                    // Terminal might not exist yet, that's okay
                }
            }

            // Route to LogManager for editor-based viewing
            if (logManager) {
                logManager.appendLogLine(service, logLine);
            }

            // Mirror realtime daemon service output to the OpenDaemon CLI channel.
            cliLogger.logServiceOutput(service, logLine.timestamp, logLine.stream, content);

            // Log activity (throttled to avoid spam)
            if (activityLogger && shouldLogLine(service)) {
                activityLogger.logTerminalAction(
                    service,
                    'Log line received',
                    `stream: ${stream}, length: ${content.length}`
                );
            }
        }
    } catch (err) {
        // Log notification processing errors
        const errorMsg = err instanceof Error ? err.message : String(err);

        if (activityLogger) {
            activityLogger.logError(
                `Processing notification ${method}`,
                errorMsg
            );
        }

        if (errorDisplayManager) {
            errorDisplayManager.displayError({
                message: `Failed to process daemon notification: ${method}`,
                category: ErrorCategory.RPC,
                details: errorMsg
            });
        }

        console.error('[handleDaemonNotification] Error processing notification:', err);
    }
}

/**
 * Open dmn.json config file
 */
async function openDmnConfig(): Promise<void> {
    const configPath = fileWatcher?.getConfigPath();
    if (configPath) {
        const doc = await vscode.workspace.openTextDocument(configPath);
        await vscode.window.showTextDocument(doc);
    }
}

/**
 * Get the RPC client instance (for use by other modules)
 */
export function getRpcClient(): RpcClient | null {
    return rpcClient;
}

/**
 * Get the tree data provider instance (for use by other modules)
 */
export function getTreeDataProvider(): ServiceTreeDataProvider | null {
    return treeDataProvider;
}

/**
 * Get the error display manager instance (for use by other modules)
 */
export function getErrorDisplayManager(): ErrorDisplayManager | null {
    return errorDisplayManager;
}
