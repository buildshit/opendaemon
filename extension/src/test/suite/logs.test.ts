import * as assert from 'assert';
import { LogManager, LogLine } from '../../logs';
import { RpcClient } from '../../rpc-client';

suite('Log Manager Test Suite', () => {
    let logManager: LogManager;
    let mockRpcClient: RpcClient | null;

    setup(() => {
        mockRpcClient = {
            request: async (method: string, params?: unknown) => {
                if (method === 'GetLogs') {
                    return {
                        logs: [
                            {
                                timestamp: '2024-01-01T00:00:00Z',
                                content: 'Test log line 1',
                                stream: 'stdout'
                            },
                            {
                                timestamp: '2024-01-01T00:00:01Z',
                                content: 'Test log line 2',
                                stream: 'stderr'
                            }
                        ]
                    };
                }
                return {};
            }
        } as unknown as RpcClient;

        logManager = new LogManager(() => mockRpcClient);
    });

    teardown(() => {
        logManager.dispose();
    });

    test('Should create log manager instance', () => {
        assert.ok(logManager);
    });

    test('Should show logs for a service', async () => {
        // Should not throw
        await logManager.showLogs('test-service');
        assert.ok(true);
    });

    test('Should handle RPC client not available', async () => {
        mockRpcClient = null;
        
        // Should not throw
        await logManager.showLogs('test-service');
        assert.ok(true);
    });

    test('Should append log lines', () => {
        const logLine: LogLine = {
            timestamp: '2024-01-01T00:00:00Z',
            content: 'Test log',
            stream: 'stdout'
        };

        // Should not throw
        logManager.appendLogLine('test-service', logLine);
        assert.ok(true);
    });

    test('Should clear logs', () => {
        // Should not throw
        logManager.clear();
        assert.ok(true);
    });
});
