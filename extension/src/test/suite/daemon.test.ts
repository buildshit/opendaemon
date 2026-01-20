import * as assert from 'assert';
import { DaemonManager } from '../../daemon';
import * as vscode from 'vscode';

suite('Daemon Manager Test Suite', () => {
    let daemonManager: DaemonManager;
    let stdoutData: string[] = [];
    let stderrData: string[] = [];

    setup(() => {
        stdoutData = [];
        stderrData = [];
        
        const mockContext = {
            extensionPath: __dirname
        } as vscode.ExtensionContext;

        daemonManager = new DaemonManager(
            mockContext,
            (data) => stdoutData.push(data),
            (data) => stderrData.push(data)
        );
    });

    teardown(async () => {
        if (daemonManager.isRunning()) {
            await daemonManager.stop();
        }
    });

    test('Should create daemon manager instance', () => {
        assert.ok(daemonManager);
    });

    test('Should not be running initially', () => {
        assert.strictEqual(daemonManager.isRunning(), false);
    });

    test('Should handle write when not running', () => {
        // Should not throw
        daemonManager.write('test');
        assert.ok(true);
    });

    test('Should handle stop when not running', async () => {
        // Should not throw
        await daemonManager.stop();
        assert.ok(true);
    });

    test('Should reset restart attempts', () => {
        daemonManager.resetRestartAttempts();
        assert.ok(true);
    });
});
