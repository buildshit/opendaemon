import * as assert from 'assert';
import * as vscode from 'vscode';
import { CommandManager } from '../../commands';
import { RpcClient } from '../../rpc-client';

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
            subscriptions: []
        } as vscode.ExtensionContext;

        commandManager = new CommandManager(mockContext, () => mockRpcClient);
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
