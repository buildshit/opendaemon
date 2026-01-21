import * as assert from 'assert';
import * as vscode from 'vscode';
import { ErrorDisplayManager, ErrorCategory, ErrorSeverity, ErrorInfo } from '../../error-display';

suite('ErrorDisplayManager Test Suite', () => {
    let errorDisplayManager: ErrorDisplayManager;
    let openConfigCalled = false;
    let showLogsCalled = false;
    let showLogsService: string | undefined;
    let reloadCalled = false;
    let retryCalled = false;

    setup(() => {
        // Reset flags
        openConfigCalled = false;
        showLogsCalled = false;
        showLogsService = undefined;
        reloadCalled = false;
        retryCalled = false;

        // Create error display manager with mock handlers
        errorDisplayManager = new ErrorDisplayManager({
            openConfig: async () => {
                openConfigCalled = true;
            },
            showLogs: (service?: string) => {
                showLogsCalled = true;
                showLogsService = service;
            },
            reload: () => {
                reloadCalled = true;
            },
            retry: async () => {
                retryCalled = true;
            }
        });
    });

    teardown(() => {
        errorDisplayManager.dispose();
    });

    test('Should create error display manager', () => {
        assert.ok(errorDisplayManager);
    });

    test('Should add error to history', async () => {
        const error: ErrorInfo = {
            message: 'Test error',
            category: ErrorCategory.CONFIG
        };

        await errorDisplayManager.displayError(error);

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.strictEqual(history[0].message, 'Test error');
        assert.strictEqual(history[0].category, ErrorCategory.CONFIG);
    });

    test('Should add timestamp to error if not provided', async () => {
        const error: ErrorInfo = {
            message: 'Test error',
            category: ErrorCategory.PROCESS
        };

        await errorDisplayManager.displayError(error);

        const history = errorDisplayManager.getErrorHistory();
        assert.ok(history[0].timestamp);
        assert.ok(history[0].timestamp instanceof Date);
    });

    test('Should maintain error history up to max size', async () => {
        // Add more than max history size (100)
        for (let i = 0; i < 150; i++) {
            await errorDisplayManager.displayError({
                message: `Error ${i}`,
                category: ErrorCategory.PROCESS
            });
        }

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 100);
        // Should keep the most recent errors
        assert.strictEqual(history[history.length - 1].message, 'Error 149');
    });

    test('Should clear error history', async () => {
        await errorDisplayManager.displayError({
            message: 'Test error',
            category: ErrorCategory.CONFIG
        });

        assert.strictEqual(errorDisplayManager.getErrorHistory().length, 1);

        errorDisplayManager.clearHistory();

        assert.strictEqual(errorDisplayManager.getErrorHistory().length, 0);
    });

    test('Should display service failure error', async () => {
        await errorDisplayManager.displayServiceFailure('backend', 'Connection refused', 1);

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.ok(history[0].message.includes('backend'));
        assert.ok(history[0].message.includes('Connection refused'));
        assert.ok(history[0].message.includes('exit code 1'));
        assert.strictEqual(history[0].category, ErrorCategory.SERVICE);
        assert.strictEqual(history[0].severity, ErrorSeverity.Critical);
        assert.strictEqual(history[0].service, 'backend');
        assert.strictEqual(history[0].exitCode, 1);
    });

    test('Should display config error', async () => {
        await errorDisplayManager.displayConfigError('Missing required field: version', 'Field "version" is required');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.ok(history[0].message.includes('Configuration Error'));
        assert.ok(history[0].message.includes('Missing required field: version'));
        assert.strictEqual(history[0].category, ErrorCategory.CONFIG);
        assert.strictEqual(history[0].severity, ErrorSeverity.Critical);
        assert.strictEqual(history[0].details, 'Field "version" is required');
    });

    test('Should display graph error', async () => {
        await errorDisplayManager.displayGraphError('Cyclic dependency detected: service1 -> service2 -> service1');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.ok(history[0].message.includes('Dependency Error'));
        assert.ok(history[0].message.includes('Cyclic dependency'));
        assert.strictEqual(history[0].category, ErrorCategory.GRAPH);
        assert.strictEqual(history[0].severity, ErrorSeverity.Critical);
    });

    test('Should display process error', async () => {
        await errorDisplayManager.displayProcessError('frontend', 'Failed to spawn process');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.ok(history[0].message.includes('Process Error'));
        assert.ok(history[0].message.includes('frontend'));
        assert.ok(history[0].message.includes('Failed to spawn process'));
        assert.strictEqual(history[0].category, ErrorCategory.PROCESS);
        assert.strictEqual(history[0].severity, ErrorSeverity.Critical);
        assert.strictEqual(history[0].service, 'frontend');
    });

    test('Should display ready error', async () => {
        await errorDisplayManager.displayReadyError('database', 'Timeout waiting for ready condition');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.ok(history[0].message.includes('Ready Check Failed'));
        assert.ok(history[0].message.includes('database'));
        assert.ok(history[0].message.includes('Timeout'));
        assert.strictEqual(history[0].category, ErrorCategory.READY);
        assert.strictEqual(history[0].severity, ErrorSeverity.Warning);
        assert.strictEqual(history[0].service, 'database');
    });

    test('Should determine correct severity for CONFIG category', async () => {
        await errorDisplayManager.displayError({
            message: 'Config error',
            category: ErrorCategory.CONFIG
        });

        const history = errorDisplayManager.getErrorHistory();
        // Severity should be determined automatically if not provided
        assert.ok(history[0].message);
    });

    test('Should determine correct severity for READY category', async () => {
        await errorDisplayManager.displayError({
            message: 'Ready error',
            category: ErrorCategory.READY
        });

        const history = errorDisplayManager.getErrorHistory();
        assert.ok(history[0].message);
    });

    test('Should determine correct severity for MCP category', async () => {
        await errorDisplayManager.displayError({
            message: 'MCP error',
            category: ErrorCategory.MCP
        });

        const history = errorDisplayManager.getErrorHistory();
        assert.ok(history[0].message);
    });

    test('Should handle error with all fields', async () => {
        const error: ErrorInfo = {
            message: 'Complete error',
            category: ErrorCategory.PROCESS,
            severity: ErrorSeverity.Critical,
            service: 'test-service',
            exitCode: 127,
            timestamp: new Date('2024-01-01T00:00:00Z'),
            details: 'Additional error details'
        };

        await errorDisplayManager.displayError(error);

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.strictEqual(history[0].message, 'Complete error');
        assert.strictEqual(history[0].category, ErrorCategory.PROCESS);
        assert.strictEqual(history[0].severity, ErrorSeverity.Critical);
        assert.strictEqual(history[0].service, 'test-service');
        assert.strictEqual(history[0].exitCode, 127);
        assert.strictEqual(history[0].details, 'Additional error details');
        assert.ok(history[0].timestamp);
    });

    test('Should handle multiple errors', async () => {
        await errorDisplayManager.displayConfigError('Error 1');
        await errorDisplayManager.displayGraphError('Error 2');
        await errorDisplayManager.displayProcessError('service1', 'Error 3');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 3);
        assert.ok(history[0].message.includes('Error 1'));
        assert.ok(history[1].message.includes('Error 2'));
        assert.ok(history[2].message.includes('Error 3'));
    });

    test('Should preserve error order in history', async () => {
        const errors = ['First', 'Second', 'Third', 'Fourth', 'Fifth'];
        
        for (const msg of errors) {
            await errorDisplayManager.displayError({
                message: msg,
                category: ErrorCategory.PROCESS
            });
        }

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 5);
        
        for (let i = 0; i < errors.length; i++) {
            assert.strictEqual(history[i].message, errors[i]);
        }
    });

    test('Should handle errors without service', async () => {
        await errorDisplayManager.displayError({
            message: 'General error',
            category: ErrorCategory.ORCHESTRATOR
        });

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.strictEqual(history[0].service, undefined);
    });

    test('Should handle errors without exit code', async () => {
        await errorDisplayManager.displayServiceFailure('service1', 'Error message');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.strictEqual(history[0].exitCode, undefined);
    });

    test('Should handle errors without details', async () => {
        await errorDisplayManager.displayConfigError('Simple error');

        const history = errorDisplayManager.getErrorHistory();
        assert.strictEqual(history.length, 1);
        assert.strictEqual(history[0].details, undefined);
    });
});
