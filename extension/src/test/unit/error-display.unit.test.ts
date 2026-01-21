/**
 * Unit tests for ErrorDisplayManager
 * These tests don't require VS Code API and can run in Node.js
 */

import * as assert from 'assert';
import { ErrorCategory, ErrorSeverity, ErrorInfo } from '../../error-display';

suite('ErrorDisplayManager Unit Tests', () => {
    test('ErrorCategory enum should have all expected values', () => {
        assert.strictEqual(ErrorCategory.CONFIG, 'CONFIG');
        assert.strictEqual(ErrorCategory.GRAPH, 'GRAPH');
        assert.strictEqual(ErrorCategory.PROCESS, 'PROCESS');
        assert.strictEqual(ErrorCategory.READY, 'READY');
        assert.strictEqual(ErrorCategory.ORCHESTRATOR, 'ORCHESTRATOR');
        assert.strictEqual(ErrorCategory.MCP, 'MCP');
        assert.strictEqual(ErrorCategory.RPC, 'RPC');
        assert.strictEqual(ErrorCategory.IO, 'IO');
        assert.strictEqual(ErrorCategory.JSON, 'JSON');
        assert.strictEqual(ErrorCategory.SERVICE, 'SERVICE');
    });

    test('ErrorSeverity enum should have all expected values', () => {
        assert.strictEqual(ErrorSeverity.Critical, 'critical');
        assert.strictEqual(ErrorSeverity.Warning, 'warning');
        assert.strictEqual(ErrorSeverity.Info, 'info');
    });

    test('ErrorInfo interface should accept all fields', () => {
        const error: ErrorInfo = {
            message: 'Test error',
            category: ErrorCategory.CONFIG,
            severity: ErrorSeverity.Critical,
            service: 'test-service',
            exitCode: 1,
            timestamp: new Date(),
            details: 'Additional details'
        };

        assert.strictEqual(error.message, 'Test error');
        assert.strictEqual(error.category, ErrorCategory.CONFIG);
        assert.strictEqual(error.severity, ErrorSeverity.Critical);
        assert.strictEqual(error.service, 'test-service');
        assert.strictEqual(error.exitCode, 1);
        assert.ok(error.timestamp instanceof Date);
        assert.strictEqual(error.details, 'Additional details');
    });

    test('ErrorInfo interface should accept minimal fields', () => {
        const error: ErrorInfo = {
            message: 'Minimal error',
            category: ErrorCategory.PROCESS
        };

        assert.strictEqual(error.message, 'Minimal error');
        assert.strictEqual(error.category, ErrorCategory.PROCESS);
        assert.strictEqual(error.severity, undefined);
        assert.strictEqual(error.service, undefined);
        assert.strictEqual(error.exitCode, undefined);
        assert.strictEqual(error.timestamp, undefined);
        assert.strictEqual(error.details, undefined);
    });

    test('Error categories should match Rust error types', () => {
        // Verify that our TypeScript error categories match the Rust implementation
        const expectedCategories = [
            'CONFIG',
            'GRAPH',
            'PROCESS',
            'READY',
            'ORCHESTRATOR',
            'MCP',
            'RPC',
            'IO',
            'JSON',
            'SERVICE'
        ];

        const actualCategories = Object.values(ErrorCategory);
        
        for (const expected of expectedCategories) {
            assert.ok(
                actualCategories.includes(expected as ErrorCategory),
                `Missing category: ${expected}`
            );
        }
    });

    test('Error severity levels should be distinct', () => {
        const severities = Object.values(ErrorSeverity);
        const uniqueSeverities = new Set(severities);
        
        assert.strictEqual(
            severities.length,
            uniqueSeverities.size,
            'Severity levels should be unique'
        );
    });
});
