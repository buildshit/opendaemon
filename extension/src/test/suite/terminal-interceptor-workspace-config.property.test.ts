import * as assert from 'assert';
import * as fc from 'fast-check';
import { TerminalInterceptor } from '../../cli-integration/terminal-interceptor';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

/**
 * Property-based tests for workspace configuration scope
 * Feature: automatic-terminal-profile-registration
 * Property 4: Workspace Configuration Scope
 * Validates: Requirements 2.2
 */
suite('TerminalInterceptor Workspace Configuration Scope Property Tests', () => {
    /**
     * Property 4: Workspace Configuration Scope
     * 
     * For any configuration update related to terminal profile defaults, 
     * the extension should target workspace-level configuration scope, not user-level.
     * 
     * **Validates: Requirements 2.2**
     */
    test('Property 4: Workspace Configuration Scope', async function() {
        this.timeout(30000); // Increase timeout for property tests

        // Create a temporary directory with a mock binary
        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'opendaemon-test-'));
        const binaryName = os.platform() === 'win32' ? 'dmn.exe' : 'dmn';
        const binaryPath = path.join(tempDir, binaryName);
        
        try {
            // Create mock binary file
            fs.writeFileSync(binaryPath, '#!/bin/bash\necho "mock"');
            if (os.platform() !== 'win32') {
                fs.chmodSync(binaryPath, 0o755);
            }

            // Generate random scenarios for configuration updates
            const scenarioArbitrary = fc.record({
                platform: fc.constantFrom('win32', 'darwin', 'linux'),
                profileId: fc.constantFrom('opendaemon.terminal', 'test.profile', 'custom.terminal'),
                previousValue: fc.option(fc.string(), { nil: undefined }),
                binaryPrefix: fc.constantFrom('dmn', 'dmn-')
            });

            await fc.assert(
                fc.asyncProperty(scenarioArbitrary, async (scenario) => {
                    // Create binary with appropriate name for the scenario
                    const scenarioBinaryName = scenario.binaryPrefix === 'dmn' ? 'dmn' : 
                                               (scenario.platform === 'win32' ? 'dmn-test.exe' : 'dmn-test');
                    const scenarioBinaryPath = path.join(tempDir, scenarioBinaryName);
                    
                    // Ensure binary exists for this scenario
                    if (!fs.existsSync(scenarioBinaryPath)) {
                        fs.writeFileSync(scenarioBinaryPath, '#!/bin/bash\necho "mock"');
                        if (scenario.platform !== 'win32') {
                            fs.chmodSync(scenarioBinaryPath, 0o755);
                        }
                    }

                    const interceptor = new TerminalInterceptor(tempDir);

                    // Track configuration updates to verify scope
                    const configUpdates: Array<{
                        key: string;
                        value: any;
                        target: vscode.ConfigurationTarget;
                    }> = [];

                    // Mock vscode.window.registerTerminalProfileProvider
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

                        // Mock workspace configuration with tracking
                        (vscode.workspace as any).getConfiguration = (section?: string) => {
                            return {
                                get: (key: string) => scenario.previousValue,
                                update: async (key: string, value: any, target: vscode.ConfigurationTarget) => {
                                    // Track all configuration updates
                                    configUpdates.push({ key, value, target });
                                }
                            };
                        };

                        // Start the interceptor (triggers registration and config update)
                        await interceptor.start();

                        // Verify that at least one configuration update occurred
                        assert.ok(configUpdates.length > 0, 
                            'At least one configuration update should occur during profile registration');

                        // Verify that ALL configuration updates target workspace scope
                        for (const update of configUpdates) {
                            assert.strictEqual(
                                update.target, 
                                vscode.ConfigurationTarget.Workspace,
                                `Configuration update for "${update.key}" should target Workspace scope, but got ${vscode.ConfigurationTarget[update.target]}`
                            );

                            // Verify it's NOT targeting user or global scope
                            assert.notStrictEqual(
                                update.target,
                                vscode.ConfigurationTarget.Global,
                                `Configuration update for "${update.key}" should NOT target Global scope`
                            );

                            assert.notStrictEqual(
                                update.target,
                                vscode.ConfigurationTarget.WorkspaceFolder,
                                `Configuration update for "${update.key}" should NOT target WorkspaceFolder scope`
                            );
                        }

                        // Verify the configuration key is platform-specific
                        const expectedConfigKey = scenario.platform === 'win32' ? 'defaultProfile.windows' :
                                                  scenario.platform === 'darwin' ? 'defaultProfile.osx' :
                                                  'defaultProfile.linux';

                        const relevantUpdate = configUpdates.find(u => u.key === expectedConfigKey);
                        assert.ok(relevantUpdate, 
                            `Configuration update should include the platform-specific key: ${expectedConfigKey}`);

                        // Test cleanup also uses workspace scope
                        configUpdates.length = 0; // Clear previous updates
                        await interceptor.stop();

                        // If there was a previous value, restoration should also use workspace scope
                        if (scenario.previousValue !== undefined && configUpdates.length > 0) {
                            for (const update of configUpdates) {
                                assert.strictEqual(
                                    update.target,
                                    vscode.ConfigurationTarget.Workspace,
                                    `Configuration restoration for "${update.key}" should also target Workspace scope`
                                );
                            }
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
                        
                        // Clean up scenario binary
                        try {
                            if (fs.existsSync(scenarioBinaryPath)) {
                                fs.unlinkSync(scenarioBinaryPath);
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
     * Property 4: Workspace Configuration Scope - Edge case with no previous value
     * 
     * Verifies that even when there's no previous configuration value,
     * the extension still targets workspace scope.
     */
    test('Property 4: Workspace Configuration Scope - no previous value', async function() {
        this.timeout(30000);

        const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'opendaemon-test-'));
        const binaryName = os.platform() === 'win32' ? 'dmn.exe' : 'dmn';
        const binaryPath = path.join(tempDir, binaryName);
        
        try {
            fs.writeFileSync(binaryPath, '#!/bin/bash\necho "mock"');
            if (os.platform() !== 'win32') {
                fs.chmodSync(binaryPath, 0o755);
            }

            await fc.assert(
                fc.asyncProperty(fc.constant(null), async () => {
                    const interceptor = new TerminalInterceptor(tempDir);
                    const configUpdates: Array<{ target: vscode.ConfigurationTarget }> = [];

                    const originalRegister = vscode.window.registerTerminalProfileProvider;
                    const originalGetConfig = vscode.workspace.getConfiguration;

                    try {
                        (vscode.window as any).registerTerminalProfileProvider = () => ({
                            dispose: () => { /* no-op */ }
                        });

                        (vscode.workspace as any).getConfiguration = () => ({
                            get: () => undefined, // No previous value
                            update: async (key: string, value: any, target: vscode.ConfigurationTarget) => {
                                configUpdates.push({ target });
                            }
                        });

                        await interceptor.start();

                        // Even with no previous value, updates should target workspace scope
                        for (const update of configUpdates) {
                            assert.strictEqual(
                                update.target,
                                vscode.ConfigurationTarget.Workspace,
                                'Configuration updates should target Workspace scope even when no previous value exists'
                            );
                        }

                    } finally {
                        (vscode.window as any).registerTerminalProfileProvider = originalRegister;
                        (vscode.workspace as any).getConfiguration = originalGetConfig;
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
});
