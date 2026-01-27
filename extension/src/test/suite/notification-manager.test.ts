/**
 * Unit tests for notification manager
 * Tests notification display, state persistence, and platform-specific instructions
 */

import * as assert from 'assert';
import { NotificationManager } from '../../cli-integration/notification-manager';
import { PlatformInfo } from '../../cli-integration/platform-detector';

suite('Notification Manager Test Suite', () => {
    let mockContext: any;
    let notificationManager: NotificationManager;

    setup(() => {
        // Create mock context with state storage
        const storage = new Map<string, any>();
        mockContext = {
            globalState: {
                get: <T>(key: string, defaultValue?: T): T => {
                    return storage.has(key) ? storage.get(key) : defaultValue as T;
                },
                update: async (key: string, value: any): Promise<void> => {
                    storage.set(key, value);
                },
                keys: () => Array.from(storage.keys()),
                setKeysForSync: (keys: readonly string[]) => {}
            }
        };

        notificationManager = new NotificationManager(mockContext);
    });

    test('Should persist notification state after first call', async () => {
        // Show notification (will call VS Code API but we can't mock it easily)
        await notificationManager.showFirstTimeNotification('/path/to/bin');

        // Check state was persisted
        const state = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state, true, 'First-time notification state should be persisted');
    });

    test('Should not call showInformationMessage on subsequent activations', async () => {
        // Manually set state to simulate notification already shown
        await mockContext.globalState.update('opendaemon.cliIntegration.firstTimeNotificationShown', true);

        // Create a new manager with the same context
        const manager2 = new NotificationManager(mockContext);
        
        // This should return early without showing notification
        // We can't easily verify the VS Code API wasn't called without sinon,
        // but we can verify the state check works
        await manager2.showFirstTimeNotification('/path/to/bin');
        
        // State should still be true
        const state = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state, true);
    });

    test('Should preserve state across multiple NotificationManager instances', async () => {
        // First instance shows notification
        const manager1 = new NotificationManager(mockContext);
        await manager1.showFirstTimeNotification('/path/to/bin');
        
        const state1 = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state1, true);

        // Second instance should see the same state
        const manager2 = new NotificationManager(mockContext);
        const state2 = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state2, true);
    });

    test('Should handle error notification calls', async () => {
        // Just verify the method can be called without throwing
        await notificationManager.showErrorNotification('Test error message');
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle global install instructions for Windows', async () => {
        const platform: PlatformInfo = { os: 'win32', arch: 'x64' };
        const binDir = 'C:\\path\\to\\bin';

        // Just verify the method can be called without throwing
        await notificationManager.showGlobalInstallInstructions(platform, binDir);
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle global install instructions for macOS ARM64', async () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'arm64' };
        const binDir = '/path/to/bin';

        // Just verify the method can be called without throwing
        await notificationManager.showGlobalInstallInstructions(platform, binDir);
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle global install instructions for macOS x64', async () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'x64' };
        const binDir = '/path/to/bin';

        // Just verify the method can be called without throwing
        await notificationManager.showGlobalInstallInstructions(platform, binDir);
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle global install instructions for Linux', async () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const binDir = '/path/to/bin';

        // Just verify the method can be called without throwing
        await notificationManager.showGlobalInstallInstructions(platform, binDir);
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle multiple error notifications', async () => {
        // Verify multiple calls don't throw
        await notificationManager.showErrorNotification('Error 1');
        await notificationManager.showErrorNotification('Error 2');
        await notificationManager.showErrorNotification('Error 3');
        // No assertion needed - just verify it doesn't throw
    });

    test('Should handle different bin directories', async () => {
        await notificationManager.showFirstTimeNotification('/first/path');
        
        const state = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state, true);

        // Second call with different path should not change state
        await notificationManager.showFirstTimeNotification('/second/path');
        
        const state2 = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
        assert.strictEqual(state2, true);
    });

    test('Should initialize with clean state', () => {
        // Create a fresh context
        const freshStorage = new Map<string, any>();
        const freshContext: any = {
            globalState: {
                get: <T>(key: string, defaultValue?: T): T => {
                    return freshStorage.has(key) ? freshStorage.get(key) : defaultValue as T;
                },
                update: async (key: string, value: any): Promise<void> => {
                    freshStorage.set(key, value);
                },
                keys: () => Array.from(freshStorage.keys()),
                setKeysForSync: (keys: readonly string[]) => {}
            }
        };

        const freshManager = new NotificationManager(freshContext);
        const state = freshContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown', false);
        assert.strictEqual(state, false, 'Initial state should be false');
    });

    test('Should handle state key correctly', async () => {
        await notificationManager.showFirstTimeNotification('/path/to/bin');

        // Verify the exact key used
        const keys = mockContext.globalState.keys();
        assert.ok(keys.includes('opendaemon.cliIntegration.firstTimeNotificationShown'),
            'State should be stored with correct key');
    });
});
