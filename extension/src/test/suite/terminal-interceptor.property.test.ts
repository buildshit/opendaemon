import * as assert from 'assert';
import * as fc from 'fast-check';
import { TerminalInterceptor } from '../../cli-integration/terminal-interceptor';
import * as os from 'os';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

suite('TerminalInterceptor Property Tests', () => {
    /**
     * Property 2: PATH Preservation
     * Validates: Requirements 1.5
     * 
     * For any existing PATH value, after PATH injection, all original PATH entries 
     * should remain present and in their original order, with only the bin directory prepended.
     */
    test('Property 2: PATH Preservation - all original entries remain in order', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        const separator = os.platform() === 'win32' ? ';' : ':';

        // Generate random PATH strings with various entries
        const pathEntryArbitrary = fc.stringMatching(/^[a-zA-Z0-9/_\-.]+$/);
        const pathArbitrary = fc.array(pathEntryArbitrary, { minLength: 0, maxLength: 10 })
            .map(entries => entries.join(separator));

        fc.assert(
            fc.property(pathArbitrary, (originalPath) => {
                // Create environment with the generated PATH
                const env = { PATH: originalPath };
                
                // Use reflection to access private method for testing
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const injectPath = (interceptor as any).injectPath.bind(interceptor);
                const injectedEnv = injectPath(env);
                
                const injectedPath = injectedEnv.PATH || '';
                
                // Verify bin directory is at the start
                assert.ok(injectedPath.startsWith(binDir), 
                    `PATH should start with bin directory. Got: ${injectedPath}`);
                
                if (originalPath) {
                    // Extract the part after bin directory
                    const expectedPrefix = binDir + separator;
                    assert.ok(injectedPath.startsWith(expectedPrefix),
                        `PATH should start with "${expectedPrefix}". Got: ${injectedPath}`);
                    
                    const remainingPath = injectedPath.substring(expectedPrefix.length);
                    
                    // Verify all original entries remain in order
                    assert.strictEqual(remainingPath, originalPath,
                        `Original PATH entries should be preserved. Expected: ${originalPath}, Got: ${remainingPath}`);
                    
                    // Verify each original entry is present
                    const originalEntries = originalPath.split(separator).filter((e: string) => e.length > 0);
                    const injectedEntries = injectedPath.split(separator).filter((e: string) => e.length > 0);
                    
                    // First entry should be bin directory
                    assert.strictEqual(injectedEntries[0], binDir);
                    
                    // Remaining entries should match original entries in order
                    for (let i = 0; i < originalEntries.length; i++) {
                        assert.strictEqual(injectedEntries[i + 1], originalEntries[i],
                            `Entry at position ${i} should be preserved`);
                    }
                } else {
                    // If original PATH was empty, injected PATH should only contain bin directory
                    assert.strictEqual(injectedPath, binDir,
                        `Empty PATH should result in just bin directory. Got: ${injectedPath}`);
                }
            }),
            { numRuns: 100 }
        );
    });

    test('Property 2: PATH Preservation - handles undefined PATH', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);

        fc.assert(
            fc.property(fc.constant(undefined), () => {
                const env = {};
                
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const injectPath = (interceptor as any).injectPath.bind(interceptor);
                const injectedEnv = injectPath(env);
                
                const injectedPath = injectedEnv.PATH || '';
                
                // When PATH is undefined, it should be set to just the bin directory
                assert.strictEqual(injectedPath, binDir,
                    `Undefined PATH should result in just bin directory. Got: ${injectedPath}`);
            }),
            { numRuns: 100 }
        );
    });

    test('Property 2: PATH Preservation - handles empty string PATH', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);

        fc.assert(
            fc.property(fc.constant(''), () => {
                const env = { PATH: '' };
                
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const injectPath = (interceptor as any).injectPath.bind(interceptor);
                const injectedEnv = injectPath(env);
                
                const injectedPath = injectedEnv.PATH || '';
                
                // When PATH is empty string, it should be set to just the bin directory
                assert.strictEqual(injectedPath, binDir,
                    `Empty PATH should result in just bin directory. Got: ${injectedPath}`);
            }),
            { numRuns: 100 }
        );
    });

    /**
     * Feature: automatic-terminal-profile-registration
     * Property 2: Registration Stores Disposable
     * Validates: Requirements 1.2
     * 
     * For any successful terminal profile provider registration, the extension should 
     * store the registration disposable for later cleanup.
     */
    test('Property 2: Registration Stores Disposable', async function() {
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

            // Generate random scenarios for registration
            const scenarioArbitrary = fc.record({
                binDirSuffix: fc.constantFrom('', '/subdir', '/nested/path'),
                profileId: fc.constantFrom('opendaemon.terminal', 'test.profile', 'custom.terminal')
            });

            await fc.assert(
                fc.asyncProperty(scenarioArbitrary, async (scenario) => {
                    // Create interceptor with the temp directory
                    const binDir = scenario.binDirSuffix ? path.join(tempDir, scenario.binDirSuffix) : tempDir;
                    
                    // Ensure the directory exists and has the binary
                    if (scenario.binDirSuffix) {
                        fs.mkdirSync(binDir, { recursive: true });
                        const targetBinary = path.join(binDir, binaryName);
                        fs.copyFileSync(binaryPath, targetBinary);
                        if (os.platform() !== 'win32') {
                            fs.chmodSync(targetBinary, 0o755);
                        }
                    }

                    const interceptor = new TerminalInterceptor(binDir);

                    // Mock vscode.window.registerTerminalProfileProvider
                    const originalRegister = vscode.window.registerTerminalProfileProvider;
                    const originalGetConfig = vscode.workspace.getConfiguration;
                    let disposableCalled = false;
                    const mockDisposable: vscode.Disposable = {
                        dispose: () => {
                            disposableCalled = true;
                        }
                    };

                    try {
                        // Mock the registration to return our mock disposable
                        (vscode.window as any).registerTerminalProfileProvider = () => mockDisposable;

                        // Mock workspace configuration
                        (vscode.workspace as any).getConfiguration = () => ({
                            get: () => undefined,
                            update: async () => { /* no-op */ }
                        });

                        // Start the interceptor (triggers registration)
                        await interceptor.start();

                        // Access the private profileDisposable field to verify it was stored
                        const profileDisposable = (interceptor as any).profileDisposable;

                        // Verify that the disposable was stored
                        assert.ok(profileDisposable !== undefined, 
                            'profileDisposable should be stored after successful registration');
                        assert.strictEqual(profileDisposable, mockDisposable,
                            'Stored disposable should be the one returned by registerTerminalProfileProvider');

                        // Verify that calling stop() disposes the stored disposable
                        await interceptor.stop();
                        assert.ok(disposableCalled, 
                            'Stored disposable should be disposed when stop() is called');

                        // Verify that profileDisposable is cleared after disposal
                        const profileDisposableAfterStop = (interceptor as any).profileDisposable;
                        assert.strictEqual(profileDisposableAfterStop, undefined,
                            'profileDisposable should be cleared after disposal');

                    } finally {
                        // Restore original functions
                        (vscode.window as any).registerTerminalProfileProvider = originalRegister;
                        (vscode.workspace as any).getConfiguration = originalGetConfig;
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
});
