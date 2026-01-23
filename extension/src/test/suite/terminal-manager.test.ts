import * as assert from 'assert';
import * as vscode from 'vscode';
import { TerminalManager } from '../../terminal-manager';

suite('Terminal Manager Test Suite', () => {
    let terminalManager: TerminalManager;

    setup(() => {
        terminalManager = new TerminalManager();
    });

    teardown(() => {
        terminalManager.dispose();
    });

    test('Should create a terminal for a service', () => {
        const terminal = terminalManager.getOrCreateTerminal('test-service');
        
        assert.ok(terminal, 'Terminal should be created');
        assert.strictEqual(terminal.name, 'dmn: test-service', 'Terminal name should match');
    });

    test('Should reuse existing terminal for same service', () => {
        const terminal1 = terminalManager.getOrCreateTerminal('test-service');
        const terminal2 = terminalManager.getOrCreateTerminal('test-service');
        
        assert.strictEqual(terminal1, terminal2, 'Should return same terminal instance');
    });

    test('Should create different terminals for different services', () => {
        const terminal1 = terminalManager.getOrCreateTerminal('service-a');
        const terminal2 = terminalManager.getOrCreateTerminal('service-b');
        
        assert.notStrictEqual(terminal1, terminal2, 'Should return different terminals');
        assert.strictEqual(terminal1.name, 'dmn: service-a');
        assert.strictEqual(terminal2.name, 'dmn: service-b');
    });

    test('Should track active terminals', () => {
        terminalManager.getOrCreateTerminal('service-a');
        terminalManager.getOrCreateTerminal('service-b');
        
        const activeTerminals = terminalManager.getActiveTerminals();
        
        assert.strictEqual(activeTerminals.length, 2);
        assert.ok(activeTerminals.includes('service-a'));
        assert.ok(activeTerminals.includes('service-b'));
    });

    test('Should check if terminal exists for service', () => {
        assert.strictEqual(terminalManager.hasTerminal('test-service'), false);
        
        terminalManager.getOrCreateTerminal('test-service');
        
        assert.strictEqual(terminalManager.hasTerminal('test-service'), true);
    });

    test('Should close terminal for a service', () => {
        terminalManager.getOrCreateTerminal('test-service');
        assert.strictEqual(terminalManager.hasTerminal('test-service'), true);
        
        terminalManager.closeTerminal('test-service');
        
        assert.strictEqual(terminalManager.hasTerminal('test-service'), false);
    });

    test('Should close all terminals', () => {
        terminalManager.getOrCreateTerminal('service-a');
        terminalManager.getOrCreateTerminal('service-b');
        terminalManager.getOrCreateTerminal('service-c');
        
        assert.strictEqual(terminalManager.getActiveTerminals().length, 3);
        
        terminalManager.closeAllTerminals();
        
        assert.strictEqual(terminalManager.getActiveTerminals().length, 0);
    });

    test('Should not throw when closing non-existent terminal', () => {
        assert.doesNotThrow(() => {
            terminalManager.closeTerminal('non-existent');
        });
    });

    test('Should dispose cleanly', () => {
        terminalManager.getOrCreateTerminal('service-a');
        terminalManager.getOrCreateTerminal('service-b');
        
        assert.doesNotThrow(() => {
            terminalManager.dispose();
        });
        
        assert.strictEqual(terminalManager.getActiveTerminals().length, 0);
    });
});
