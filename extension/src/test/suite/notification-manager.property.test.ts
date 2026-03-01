/**
 * Property-based tests for notification manager
 * Feature: vscode-terminal-cli-integration
 * Property 4: Notification Dismissal Persistence
 */

import * as assert from 'assert';
import * as fc from 'fast-check';
import { NotificationManager } from '../../cli-integration/notification-manager';

suite('Notification Manager Property Tests', () => {
    test('Property 4: Notification Dismissal Persistence - For any user dismissal of the first-time notification, all subsequent extension activations should not display the notification again', async () => {
        // **Validates: Requirements 4.4**
        
        // Arbitrary for generating number of activation attempts
        const activationCountArb = fc.integer({ min: 2, max: 10 });
        
        // Arbitrary for generating bin directory paths
        const binDirArb = fc.stringMatching(/^[a-zA-Z0-9_\-\/\\:]+$/);

        // Property: notification should not be shown after dismissal
        await fc.assert(
            fc.asyncProperty(activationCountArb, binDirArb, async (activationCount, binDir) => {
                // Create a mock ExtensionContext
                const mockContext = createMockContext();
                const notificationManager = new NotificationManager(mockContext);
                
                // Track notification display calls
                let notificationDisplayCount = 0;
                const originalShowInformationMessage = mockContext.showInformationMessage;
                mockContext.showInformationMessage = async (...args: any[]) => {
                    notificationDisplayCount++;
                    return originalShowInformationMessage(...args);
                };
                
                // First activation - show notification and dismiss it
                await notificationManager.showFirstTimeNotification(binDir);
                
                // Verify notification was shown once
                assert.strictEqual(notificationDisplayCount, 1,
                    'Notification should be shown on first activation');
                
                // Reset counter
                notificationDisplayCount = 0;
                
                // Subsequent activations - notification should not be shown
                for (let i = 0; i < activationCount - 1; i++) {
                    await notificationManager.showFirstTimeNotification(binDir);
                }
                
                // Verify notification was not shown again
                assert.strictEqual(notificationDisplayCount, 0,
                    `Notification should not be shown on subsequent ${activationCount - 1} activations`);
                
                // Verify state persistence
                const state = mockContext.globalState.get('opendaemon.cliIntegration.firstTimeNotificationShown');
                assert.strictEqual(state, true,
                    'First-time notification state should be persisted as true');
            }),
            { numRuns: 100 } // Run 100 iterations as specified
        );
    });
});

/**
 * Creates a mock ExtensionContext for testing
 */
function createMockContext(): any {
    const storage = new Map<string, any>();
    
    return {
        globalState: {
            get: <T>(key: string, defaultValue?: T): T => {
                return storage.has(key) ? storage.get(key) : defaultValue as T;
            },
            update: async (key: string, value: any): Promise<void> => {
                storage.set(key, value);
            },
            keys: () => Array.from(storage.keys()),
            setKeysForSync: (keys: readonly string[]) => {}
        },
        showInformationMessage: async (...args: any[]): Promise<any> => {
            // Mock implementation - just return undefined (user dismissed)
            return undefined;
        }
    };
}
