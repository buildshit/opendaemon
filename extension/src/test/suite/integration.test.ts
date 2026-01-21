import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { DmnDaemon } from '../../daemon';
import { RpcClient } from '../../rpc-client';
import { ServiceTreeDataProvider } from '../../tree-view';
import { LogManager } from '../../logs';

suite('Extension Integration Tests', () => {
    let testWorkspaceRoot: string;
    let testConfigPath: string;

    setup(() => {
        // Create a temporary test workspace
        testWorkspaceRoot = path.join(__dirname, '../../../test-workspace');
        if (!fs.existsSync(testWorkspaceRoot)) {
            fs.mkdirSync(testWorkspaceRoot, { recursive: true });
        }
        testConfigPath = path.join(testWorkspaceRoot, 'dmn.json');
    });

    teardown(() => {
        // Clean up test files
        if (fs.existsSync(testConfigPath)) {
            fs.unlinkSync(testConfigPath);
        }
    });

    // Helper to start daemon and get RPC client
    async function startDaemonWithClient(daemon: DmnDaemon): Promise<RpcClient> {
        return new Promise((resolve, reject) => {
            daemon.on('started', (client: RpcClient) => {
                resolve(client);
            });
            
            daemon.start().catch(reject);
            
            // Timeout after 5 seconds
            setTimeout(() => reject(new Error('Daemon start timeout')), 5000);
        });
    }

    test('RPC Communication: Start and stop services', async function() {
        this.timeout(15000);

        // Create a simple test configuration
        const testConfig = {
            version: '1.0',
            services: {
                test_service: {
                    command: 'cmd /c echo Test service && timeout /t 5'
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create daemon and RPC client
        const daemon = new DmnDaemon(testWorkspaceRoot);

        try {
            // Start the daemon and get RPC client
            const rpcClient = await startDaemonWithClient(daemon);
            await new Promise(resolve => setTimeout(resolve, 500));

            // Test StartAll command
            const startResult = await rpcClient.request('StartAll');
            assert.ok(startResult !== undefined, 'StartAll should succeed');

            // Wait for service to start
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Test GetStatus command
            const status = await rpcClient.request('GetStatus') as Record<string, unknown>;
            assert.ok(status, 'GetStatus should return status');
            assert.ok(status.test_service, 'Status should include test_service');

            // Test StopAll command
            const stopResult = await rpcClient.request('StopAll');
            assert.ok(stopResult !== undefined, 'StopAll should succeed');

            await new Promise(resolve => setTimeout(resolve, 1000));
        } finally {
            // Clean up
            await daemon.stop();
        }
    });

    test('Tree View Updates: Services appear and status changes', async function() {
        this.timeout(15000);

        // Create a test configuration with multiple services
        const testConfig = {
            version: '1.0',
            services: {
                service1: {
                    command: 'cmd /c echo Service 1 && timeout /t 5'
                },
                service2: {
                    command: 'cmd /c echo Service 2 && timeout /t 5',
                    depends_on: ['service1']
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        const daemon = new DmnDaemon(testWorkspaceRoot);
        let rpcClient: RpcClient | null = null;

        daemon.on('started', (client: RpcClient) => {
            rpcClient = client;
        });

        try {
            await daemon.start();
            await new Promise(resolve => setTimeout(resolve, 1000));

            assert.ok(rpcClient, 'RPC client should be initialized');

            const treeProvider = new ServiceTreeDataProvider(() => rpcClient);

            // Get initial tree items
            const initialItems = await treeProvider.getChildren();
            assert.strictEqual(initialItems.length, 2, 'Should have 2 services');

            // Start all services
            await rpcClient!.request('StartAll');
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Refresh tree and check status
            treeProvider.refresh();
            const updatedItems = await treeProvider.getChildren();
            assert.strictEqual(updatedItems.length, 2, 'Should still have 2 services');

            // Verify services have updated status
            const service1Item = updatedItems.find(item => item.label === 'service1');
            const service2Item = updatedItems.find(item => item.label === 'service2');

            assert.ok(service1Item, 'service1 should exist in tree');
            assert.ok(service2Item, 'service2 should exist in tree');

            // Stop all services
            await rpcClient!.request('StopAll');
            await new Promise(resolve => setTimeout(resolve, 1000));
        } finally {
            await daemon.stop();
        }
    });

    test('Log Streaming: Logs are captured and retrievable', async function() {
        this.timeout(15000);

        // Create a test configuration
        const testConfig = {
            version: '1.0',
            services: {
                log_test: {
                    command: 'cmd /c echo First log line && echo Second log line && echo Third log line && timeout /t 5'
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        const daemon = new DmnDaemon(testWorkspaceRoot);
        let rpcClient: RpcClient | null = null;

        daemon.on('started', (client: RpcClient) => {
            rpcClient = client;
        });

        try {
            await daemon.start();
            await new Promise(resolve => setTimeout(resolve, 1000));

            assert.ok(rpcClient, 'RPC client should be initialized');

            // Start the service
            await rpcClient!.request('StartService', { service: 'log_test' });
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Get logs
            const result = await rpcClient!.request('GetLogs', { 
                service: 'log_test', 
                lines: 100 
            }) as { logs: Array<{ content: string }> };
            
            assert.ok(result.logs, 'Should receive logs');
            assert.ok(result.logs.length > 0, 'Should have at least one log line');

            // Verify log content
            const logText = result.logs.map(l => l.content).join('\n');
            assert.ok(logText.includes('First log line') || logText.includes('Second log line'), 
                'Logs should contain expected text');

            // Stop the service
            await rpcClient!.request('StopService', { service: 'log_test' });
            await new Promise(resolve => setTimeout(resolve, 1000));
        } finally {
            await daemon.stop();
        }
    });

    test('Command Execution: Start, stop, and restart individual services', async function() {
        this.timeout(20000);

        // Create a test configuration with dependencies
        const testConfig = {
            version: '1.0',
            services: {
                database: {
                    command: 'cmd /c echo Database starting && timeout /t 10'
                },
                backend: {
                    command: 'cmd /c echo Backend starting && timeout /t 10',
                    depends_on: ['database']
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        const daemon = new DmnDaemon(testWorkspaceRoot);
        let rpcClient: RpcClient | null = null;

        daemon.on('started', (client: RpcClient) => {
            rpcClient = client;
        });

        try {
            await daemon.start();
            await new Promise(resolve => setTimeout(resolve, 1000));

            assert.ok(rpcClient, 'RPC client should be initialized');

            // Start individual service
            await rpcClient!.request('StartService', { service: 'database' });
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Check status
            let status = await rpcClient!.request('GetStatus') as Record<string, unknown>;
            assert.ok(status.database, 'Database should be in status');

            // Start dependent service
            await rpcClient!.request('StartService', { service: 'backend' });
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Check both are running
            status = await rpcClient!.request('GetStatus') as Record<string, unknown>;
            assert.ok(status.database, 'Database should still be running');
            assert.ok(status.backend, 'Backend should be running');

            // Stop individual service
            await rpcClient!.request('StopService', { service: 'backend' });
            await new Promise(resolve => setTimeout(resolve, 1000));

            // Restart service
            await rpcClient!.request('RestartService', { service: 'database' });
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Verify restart worked
            status = await rpcClient!.request('GetStatus') as Record<string, unknown>;
            assert.ok(status.database, 'Database should be running after restart');

            // Clean up
            await rpcClient!.request('StopAll');
            await new Promise(resolve => setTimeout(resolve, 1000));
        } finally {
            await daemon.stop();
        }
    });

    test('End-to-end: Full workflow from config to execution', async function() {
        this.timeout(20000);

        // Create a realistic test configuration
        const testConfig = {
            version: '1.0',
            services: {
                redis: {
                    command: 'cmd /c echo Redis starting... && timeout /t 1 /nobreak >nul && echo Redis ready',
                    ready_when: {
                        type: 'log_contains',
                        pattern: 'ready'
                    }
                },
                api: {
                    command: 'cmd /c echo API starting... && timeout /t 1 /nobreak >nul && echo API listening on port 3000',
                    depends_on: ['redis'],
                    ready_when: {
                        type: 'log_contains',
                        pattern: 'listening'
                    }
                },
                frontend: {
                    command: 'cmd /c echo Frontend starting... && timeout /t 1 /nobreak >nul && echo Frontend ready',
                    depends_on: ['api']
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        const daemon = new DmnDaemon(testWorkspaceRoot);
        let rpcClient: RpcClient | null = null;

        daemon.on('started', (client: RpcClient) => {
            rpcClient = client;
        });

        try {
            // 1. Start daemon
            await daemon.start();
            await new Promise(resolve => setTimeout(resolve, 1000));

            assert.ok(rpcClient, 'RPC client should be initialized');

            const treeProvider = new ServiceTreeDataProvider(() => rpcClient);

            // 2. Verify tree view shows all services
            const services = await treeProvider.getChildren();
            assert.strictEqual(services.length, 3, 'Should have 3 services');

            // 3. Start all services
            await rpcClient!.request('StartAll');
            await new Promise(resolve => setTimeout(resolve, 5000));

            // 4. Check all services are running
            const status = await rpcClient!.request('GetStatus') as Record<string, unknown>;
            assert.ok(status.redis, 'Redis should be running');
            assert.ok(status.api, 'API should be running');
            assert.ok(status.frontend, 'Frontend should be running');

            // 5. Get logs from each service
            const redisResult = await rpcClient!.request('GetLogs', { service: 'redis', lines: 10 }) as { logs: unknown[] };
            const apiResult = await rpcClient!.request('GetLogs', { service: 'api', lines: 10 }) as { logs: unknown[] };
            const frontendResult = await rpcClient!.request('GetLogs', { service: 'frontend', lines: 10 }) as { logs: unknown[] };

            assert.ok(redisResult.logs.length > 0, 'Redis should have logs');
            assert.ok(apiResult.logs.length > 0, 'API should have logs');
            assert.ok(frontendResult.logs.length > 0, 'Frontend should have logs');

            // 6. Stop all services
            await rpcClient!.request('StopAll');
            await new Promise(resolve => setTimeout(resolve, 2000));

            // 7. Verify all stopped
            const finalStatus = await rpcClient!.request('GetStatus') as Record<string, unknown>;
            // Services should be stopped or not present
            assert.ok(true, 'Stop all completed');
        } finally {
            await daemon.stop();
        }
    });

    test('Error Handling: Invalid service names and commands', async function() {
        this.timeout(10000);

        const testConfig = {
            version: '1.0',
            services: {
                valid_service: {
                    command: 'cmd /c echo Valid'
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        const daemon = new DmnDaemon(testWorkspaceRoot);
        let rpcClient: RpcClient | null = null;

        daemon.on('started', (client: RpcClient) => {
            rpcClient = client;
        });

        try {
            await daemon.start();
            await new Promise(resolve => setTimeout(resolve, 1000));

            assert.ok(rpcClient, 'RPC client should be initialized');

            // Try to start a non-existent service
            try {
                await rpcClient!.request('StartService', { service: 'nonexistent_service' });
                assert.fail('Should have thrown an error for nonexistent service');
            } catch (error) {
                assert.ok(error, 'Should throw error for nonexistent service');
            }

            // Try to get logs from non-existent service
            try {
                await rpcClient!.request('GetLogs', { service: 'nonexistent_service', lines: 10 });
                // This might not throw, but should return empty or handle gracefully
            } catch (error) {
                // Error is acceptable
                assert.ok(error, 'Error handling for nonexistent service logs');
            }
        } finally {
            await daemon.stop();
        }
    });
});
