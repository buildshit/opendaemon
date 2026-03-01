import * as assert from 'assert';
import * as fc from 'fast-check';
import { TerminalInterceptor } from '../../cli-integration/terminal-interceptor';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

/**
 * Property-based tests for configuration preservation
 * Feature: automatic-terminal-profile-registration
 * Property 5: Configuration Preservation
 * Validates: Requirements 2.4
 */
suite('TerminalInterceptor Configuration Preservation Property Tests', () => {
    /**
     * Property 5: Configuration Preservation
     * 
     * For any existing workspace terminal profile configuration, after profile registration,
     * the previous configuration value should be stored for restoration.
     * 
     * **Validates: Requirements 2.4**
     */
    test('Property 5: Configuration Preservation', async function() {
        this.timeout(30000); // Increase timeout for property tests

        // Create a temporary directory with a mock binary
        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'opendaemon-test-'));
        
        try {
            // Generate random scenarios with various existing configuration values
            const configValueArbitrary = fc.oneof(
                fc.constant(undefined), // No previous value
                fc.constant(null), // Null value
                fc.string(), // Random string value
                fc.constantFrom(
                    'bash',
                    'zsh',
                    'powershell',
                    'cmd',
                    'fish',
                    'custom.profile',
                    'Git Bash',
                    'WSL'
                )
            );

            const scenarioArbitrary = fc.record({
                platform: fc.constantFrom('win32', 'darwin', 'linux'),
                previousValue: configValueArbitrary,
                profileId: fc.constantFrom('opendaemon.terminal', 'test.profile')
            });

            await fc.assert(
                fc.asyncProperty(scenarioArbitrary, async (scenario) => {
                    // Create binary that matches the findBinaryInDir pattern (starts with 'dmn-' or is 'dmn')
                    const binaryExt = scenario.platform === 'win32' ? '.exe' : '';
                    const binaryName = 'dmn-test' + binaryExt; // Always use 'dmn-test' to match the pattern
                    const binaryPath = path.join(tempDir, binaryName);
                    
                    // Ensure binary exists for this scenario
                    fs.writeFileSync(binaryPath, '#!/bin/bash\necho "mock"');
                    if (scenario.platform !== 'win32') {
                        fs.chmodSync(binaryPath, 0o755);
                    }

                    const interceptor = new TerminalInterceptor(tempDir);

                    // Track the stored previous value
                    let configGetCalled = false;
                    let configSetCalled = false;

                    const originalRegister = vscode.window.registerTerminalProfileProvider;
                    const originalGetConfig = vscode.workspace.getConfiguration;
                    const originalPlatform = process.platform;

                    const mockDisposable: vscode.Disposable = {
                        dispose: () => { /* no-op */ }
                    };

                    try {
                        // Mock the platform
                        Object.defineProperty(process, 'platform', {
                            value: scenario.platform,
                            writable: true,
                            configurable: true
                        });

                        // Mock the registration
                        (vscode.window as any).registerTerminalProfileProvider = () => mockDisposable;

                        // Mock workspace configuration to track get/set operations
                        (vscode.workspace as any).getConfiguration = (_section?: string) => {
                            return {
                                get: (_key: string) => {
                                    configGetCalled = true;
                                    // Return the scenario's previous value
                                    return scenario.previousValue;
                                },
                                update: async (_key: string, _value: unknown, _target: vscode.ConfigurationTarget) => {
                                    configSetCalled = true;
                                    // The first update should be setting the new profile
                                    // We need to verify that the previous value was read before this
                                }
                            };
                        };

                        // Start the interceptor (triggers registration and config update)
                        await interceptor.start();

                        // Verify that configuration was read (to get previous value)
                        assert.ok(configGetCalled, 
                            'Configuration should be read to retrieve the previous value');

                        // Verify that configuration was updated (to set new profile)
                        assert.ok(configSetCalled,
                            'Configuration should be updated to set the new profile');

                        // Access the private field to verify the previous value was stored
                        // This is a bit of a hack, but necessary to verify internal state
                        const previousValue = (interceptor as any).previousDefaultProfile;

                        // Verify that the previous value was stored correctly
                        assert.strictEqual(
                            previousValue,
                            scenario.previousValue,
                            `Previous configuration value should be stored. Expected: ${scenario.previousValue}, Got: ${previousValue}`
                        );

                        // If there was a previous value, verify it's preserved for restoration
                        if (scenario.previousValue !== undefined) {
                            assert.strictEqual(
                                previousValue,
                                scenario.previousValue,
                                'Non-undefined previous values should be preserved exactly'
                            );
                        }

                    } finally {
                        // Restore original functions and platform
                        (vscode.window as any).registerTerminalProfileProvider = originalRegister;
                        (vscode.workspace as any).getConfiguration = originalGetConfig;
                        Object.defineProperty(process, 'platform', {
                            value: originalPlatform,
                            writable: true,
                            configurable: true
                        });
                        
                        // Clean up binary
                        try {
                            if (fs.existsSync(binaryPath)) {
                                fs.unlinkSync(binaryPath);
                            }
                        } catch (error) {
                            // Ignore cleanup errors
                        }
                    }
                }),
                { numRuns: 100 }
            );
        } finally {
            // Cleanup temp directory
            try {
                fs.rmSync(tempDir, { recursive: true, force: true });
            } catch (error) {
                // Ignore cleanup errors
            }
        }
    });

    /**
     * Property 5: Configuration Preservation - Restoration verification
     * 
     * Verifies that the stored previous configuration value is actually used
     * during restoration when stop() is called.
     */
    test('Property 5: Configuration Preservation - restoration uses stored value', async function() {
        this.timeout(30000);

        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'opendaemon-test-'));
        
        try {
            // Generate scenarios with non-undefined previous values
            const configValueArbitrary = fc.oneof(
                fc.string(),
                fc.constantFrom('bash', 'zsh', 'powershell', 'cmd', 'custom.profile')
            );

            const scenarioArbitrary = fc.record({
                platform: fc.constantFrom('win32', 'darwin', 'linux'),
                previousValue: configValueArbitrary
            });

            await fc.assert(
                fc.asyncProperty(scenarioArbitrary, async (scenario) => {
                    // Create binary that matches the findBinaryInDir pattern
                    const binaryExt = scenario.platform === 'win32' ? '.exe' : '';
                    const binaryName = 'dmn-test' + binaryExt;
                    const binaryPath = path.join(tempDir, binaryName);
                    
                    // Ensure binary exists for this scenario
                    fs.writeFileSync(binaryPath, '#!/bin/bash\necho "mock"');
                    if (scenario.platform !== 'win32') {
                        fs.chmodSync(binaryPath, 0o755);
                    }

                    const interceptor = new TerminalInterceptor(tempDir);

                    let restoredValue: unknown = null;
                    let restorationCalled = false;

                    const originalRegister = vscode.window.registerTerminalProfileProvider;
                    const originalGetConfig = vscode.workspace.getConfiguration;
                    const originalPlatform = process.platform;

                    try {
                        // Mock the platform
                        Object.defineProperty(process, 'platform', {
                            value: scenario.platform,
                            writable: true,
                            configurable: true
                        });

                        // Mock the registration
                        (vscode.window as any).registerTerminalProfileProvider = () => ({
                            dispose: () => { /* no-op */ }
                        });

                        // Mock workspace configuration
                        (vscode.workspace as any).getConfiguration = (_section?: string) => {
                            return {
                                get: (_key: string) => scenario.previousValue,
                                update: async (_key: string, value: unknown, _target: vscode.ConfigurationTarget) => {
                                    // Track restoration calls (second update is restoration)
                                    if (value === scenario.previousValue) {
                                        restorationCalled = true;
                                        restoredValue = value;
                                    }
                                }
                            };
                        };

                        // Start and then stop the interceptor
                        await interceptor.start();
                        await interceptor.stop();

                        // Verify that restoration was called with the stored previous value
                        assert.ok(restorationCalled,
                            'Configuration restoration should be called during stop()');

                        assert.strictEqual(
                            restoredValue,
                            scenario.previousValue,
                            `Restored value should match the stored previous value. Expected: ${scenario.previousValue}, Got: ${restoredValue}`
                        );

                    } finally {
                        // Restore original functions and platform
                        (vscode.window as any).registerTerminalProfileProvider = originalRegister;
                        (vscode.workspace as any).getConfiguration = originalGetConfig;
                        Object.defineProperty(process, 'platform', {
                            value: originalPlatform,
                            writable: true,
                            configurable: true
                        });
                        
                        // Clean up binary
                        try {
                            if (fs.existsSync(binaryPath)) {
                                fs.unlinkSync(binaryPath);
                            }
                        } catch (error) {
                            // Ignore cleanup errors
                        }
                    }
                }),
                { numRuns: 100 }
            );
        } finally {
            try {
                fs.rmSync(tempDir, { recursive: true, force: true });
            } catch (error) {
                // Ignore cleanup errors
            }
        }
    });

    /**
     * Property 5: Configuration Preservation - No restoration when undefined
     * 
     * Verifies that when there's no previous configuration (undefined),
     * restoration is skipped appropriately.
     */
    test('Property 5: Configuration Preservation - no restoration when undefined', async function() {
        this.timeout(30000);

        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'opendaemon-test-'));
        
        try {
            await fc.assert(
                fc.asyncProperty(
                    fc.record({
                        platform: fc.constantFrom('win32', 'darwin', 'linux')
                    }),
                    async (scenario) => {
                        // Create binary that matches the findBinaryInDir pattern
                        const binaryExt = scenario.platform === 'win32' ? '.exe' : '';
                        const binaryName = 'dmn-test' + binaryExt;
                        const binaryPath = path.join(tempDir, binaryName);
                        
                        // Ensure binary exists for this scenario
                        fs.writeFileSync(binaryPath, '#!/bin/bash\necho "mock"');
                        if (scenario.platform !== 'win32') {
                            fs.chmodSync(binaryPath, 0o755);
                        }

                        const interceptor = new TerminalInterceptor(tempDir);

                        let updateCallCount = 0;

                        const originalRegister = vscode.window.registerTerminalProfileProvider;
                        const originalGetConfig = vscode.workspace.getConfiguration;
                        const originalPlatform = process.platform;

                        try {
                            // Mock the platform
                            Object.defineProperty(process, 'platform', {
                                value: scenario.platform,
                                writable: true,
                                configurable: true
                            });

                            // Mock the registration
                            (vscode.window as any).registerTerminalProfileProvider = () => ({
                                dispose: () => { /* no-op */ }
                            });

                            // Mock workspace configuration with no previous value
                            (vscode.workspace as any).getConfiguration = (_section?: string) => {
                                return {
                                    get: (_key: string) => undefined, // No previous value
                                    update: async (_key: string, _value: unknown, _target: vscode.ConfigurationTarget) => {
                                        updateCallCount++;
                                    }
                                };
                            };

                            // Start and then stop the interceptor
                            await interceptor.start();
                            const updateCountAfterStart = updateCallCount;
                            
                            await interceptor.stop();
                            const updateCountAfterStop = updateCallCount;

                            // Verify that at least one update occurred during start (setting the profile)
                            assert.ok(updateCountAfterStart > 0,
                                'At least one configuration update should occur during start()');

                            // Verify that no additional updates occurred during stop (no restoration)
                            assert.strictEqual(
                                updateCountAfterStop,
                                updateCountAfterStart,
                                'No additional configuration updates should occur during stop() when previous value is undefined'
                            );

                        } finally {
                            // Restore original functions and platform
                            (vscode.window as any).registerTerminalProfileProvider = originalRegister;
                            (vscode.workspace as any).getConfiguration = originalGetConfig;
                            Object.defineProperty(process, 'platform', {
                                value: originalPlatform,
                                writable: true,
                                configurable: true
                            });
                            
                            // Clean up binary
                            try {
                                if (fs.existsSync(binaryPath)) {
                                    fs.unlinkSync(binaryPath);
                                }
                            } catch (error) {
                                // Ignore cleanup errors
                            }
                        }
                    }
                ),
                { numRuns: 100 }
            );
        } finally {
            try {
                fs.rmSync(tempDir, { recursive: true, force: true });
            } catch (error) {
                // Ignore cleanup errors
            }
        }
    });
});
