import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { DaemonManager } from './daemon';
import { RpcClient } from './rpc-client';
import { ServiceTreeDataProvider, ServiceStatus } from './tree-view';
import { CommandManager } from './commands';
import { LogManager, LogLine } from './logs';
import { ConfigWizard } from './wizard';
import { DmnFileWatcher } from './file-watcher';

let daemonManager: DaemonManager | null = null;
let rpcClient: RpcClient | null = null;
let treeDataProvider: ServiceTreeDataProvider | null = null;
let commandManager: CommandManager | null = null;
let logManager: LogManager | null = null;
let fileWatcher: DmnFileWatcher | null = null;

export async function activate(context: vscode.ExtensionContext) {
    console.log('OpenDaemon extension is now active');
    
    // Initialize tree view
    treeDataProvider = new ServiceTreeDataProvider();
    vscode.window.registerTreeDataProvider('opendaemon.services', treeDataProvider);
    
    // Initialize log manager
    logManager = new LogManager(() => rpcClient);
    
    // Initialize command manager
    commandManager = new CommandManager(context, () => rpcClient, logManager);
    commandManager.registerCommands();
    
    // Initialize file watcher
    fileWatcher = new DmnFileWatcher(
        async () => await handleConfigChanged(),
        async () => await handleConfigDeleted()
    );
    
    // Check for dmn.json in workspace (or offer to create)
    let dmnConfigPath = await findDmnConfig();
    
    if (!dmnConfigPath) {
        // Offer to create dmn.json
        dmnConfigPath = await ConfigWizard.detectAndOfferCreation();
    }
    
    if (dmnConfigPath) {
        await initializeDaemon(dmnConfigPath);
    }
}

export async function deactivate() {
    console.log('OpenDaemon extension is now deactivated');
    
    if (fileWatcher) {
        fileWatcher.stop();
        fileWatcher = null;
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
}

/**
 * Initialize daemon with config
 */
async function initializeDaemon(dmnConfigPath: string): Promise<void> {
    vscode.window.showInformationMessage(
        `OpenDaemon: Found configuration at ${path.basename(dmnConfigPath)}`
    );
    
    // Start file watcher
    if (fileWatcher) {
        fileWatcher.start(dmnConfigPath);
    }
    
    // Initialize daemon manager
    daemonManager = new DaemonManager(
        vscode.extensions.getExtension('opendaemon.opendaemon')!.extensionContext,
        (data) => handleDaemonStdout(data),
        (data) => handleDaemonStderr(data)
    );
    
    // Initialize RPC client
    rpcClient = new RpcClient((data) => {
        if (daemonManager) {
            daemonManager.write(data);
        }
    });
    
    // Listen for notifications from daemon
    rpcClient.on('notification', (method, params) => {
        handleDaemonNotification(method, params);
    });
    
    // Start daemon
    try {
        await daemonManager.start(dmnConfigPath);
        
        // Load initial service list
        await loadServices();
    } catch (err) {
        vscode.window.showErrorMessage(
            `Failed to start OpenDaemon: ${err instanceof Error ? err.message : String(err)}`
        );
    }
}

/**
 * Handle config file changes
 */
async function handleConfigChanged(): Promise<void> {
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
    
    // Restart with new config
    const configPath = fileWatcher?.getConfigPath();
    if (configPath) {
        await initializeDaemon(configPath);
    }
}

/**
 * Handle config file deletion
 */
async function handleConfigDeleted(): Promise<void> {
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
 * Find dmn.json in the workspace root
 */
async function findDmnConfig(): Promise<string | null> {
    const workspaceFolders = vscode.workspace.workspaceFolders;
    
    if (!workspaceFolders || workspaceFolders.length === 0) {
        return null;
    }
    
    // Check first workspace folder for dmn.json
    const rootPath = workspaceFolders[0].uri.fsPath;
    const dmnPath = path.join(rootPath, 'dmn.json');
    
    try {
        await fs.promises.access(dmnPath, fs.constants.F_OK);
        return dmnPath;
    } catch {
        return null;
    }
}

/**
 * Load services from daemon
 */
async function loadServices(): Promise<void> {
    if (!rpcClient || !treeDataProvider) {
        return;
    }

    try {
        const result = await rpcClient.request('GetStatus') as Record<string, string>;
        
        // Convert status strings to ServiceStatus enum
        const services = Object.entries(result).map(([name, statusStr]) => ({
            name,
            status: parseServiceStatus(statusStr)
        }));

        treeDataProvider.updateServices(services);
    } catch (err) {
        console.error('Failed to load services:', err);
    }
}

/**
 * Parse service status string to enum
 */
function parseServiceStatus(statusStr: string): ServiceStatus {
    // Handle Rust enum format like "Failed { exit_code: 1 }"
    if (statusStr.startsWith('Failed')) {
        return ServiceStatus.Failed;
    }
    
    switch (statusStr) {
        case 'Running':
            return ServiceStatus.Running;
        case 'Starting':
            return ServiceStatus.Starting;
        case 'Stopped':
            return ServiceStatus.Stopped;
        case 'NotStarted':
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
}

/**
 * Handle notifications from daemon
 */
function handleDaemonNotification(method: string, params: unknown): void {
    console.log('[Daemon notification]:', method, params);
    
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
    }
    
    // Handle log lines
    if (method === 'LogLine' && logManager) {
        const { service, line } = params as {
            service: string;
            line: LogLine;
        };
        
        logManager.appendLogLine(service, line);
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
