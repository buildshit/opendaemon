import * as assert from 'assert';
import * as vscode from 'vscode';
import { CommandManager } from '../../commands';
import { RpcClient } from '../../rpc-client';
import { LogManager } from '../../logs';

suite('Commands Test Suite', () => {
    let commandManager: CommandManager;
    let mockRpcClient: RpcClient | null;
    let requestedMethods: Array<{ method: string; params?: unknown }> = [];

    setup(() => {
        requestedMethods = [];

        // Create a mock RPC client
        mockRpcClient = {
            request: async (method: string, params?: unknown) => {
                requestedMethods.push({ method, params });
                return { success: true };
            }
        } as unknown as RpcClient;

        const mockContext = {
            subscriptions: [],
            workspaceState: {} as vscode.Memento,
            globalState: {} as vscode.Memento & { setKeysForSync(keys: readonly string[]): void },
            secrets: {} as vscode.SecretStorage,
            extensionUri: vscode.Uri.file(''),
            extensionPath: '',
            environmentVariableCollection: {} as vscode.GlobalEnvironmentVariableCollection,
            asAbsolutePath: (relativePath: string) => relativePath,
            storageUri: undefined,
            storagePath: undefined,
            globalStorageUri: vscode.Uri.file(''),
            globalStoragePath: '',
            logUri: vscode.Uri.file(''),
            logPath: '',
            extensionMode: vscode.ExtensionMode.Test,
            extension: {} as vscode.Extension<any>,
            languageModelAccessInformation: {} as vscode.LanguageModelAccessInformation
        } as vscode.ExtensionContext;

        // Create a mock LogManager
        const mockLogManager = {
            showLogs: async (serviceName: string, lines?: number) => { },
            appendLogLine: (serviceName: string, log: any) => { },
            clear: () => { },
            dispose: () => { }
        } as LogManager;

        // Create a mock TreeDataProvider
        const mockTreeDataProvider = {
            getAllServices: () => []
        };

        commandManager = new CommandManager(
            mockContext,
            () => mockRpcClient,
            mockLogManager,
            () => mockTreeDataProvider,
            async () => { },
            undefined, // getErrorDisplayManager
            undefined  // getConfigPath
        );
        commandManager.registerCommands();
    });

    test('Should register commands', () => {
        assert.ok(commandManager);
    });

    test('Should execute startAll command', async () => {
        await vscode.commands.executeCommand('opendaemon.startAll');

        assert.strictEqual(requestedMethods.length, 1);
        assert.strictEqual(requestedMethods[0].method, 'StartAll');
    });

    test('Should execute stopAll command', async () => {
        await vscode.commands.executeCommand('opendaemon.stopAll');

        assert.strictEqual(requestedMethods.length, 1);
        assert.strictEqual(requestedMethods[0].method, 'StopAll');
    });

    test('Should handle RPC client not available', async () => {
        // Set RPC client to null
        mockRpcClient = null;

        // Should not throw
        await vscode.commands.executeCommand('opendaemon.startAll');

        // No requests should be made
        assert.strictEqual(requestedMethods.length, 0);
    });
});
