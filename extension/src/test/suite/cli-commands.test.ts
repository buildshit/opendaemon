/**
 * Tests for CLI integration command registration
 */

import * as assert from 'assert';
import * as vscode from 'vscode';

suite('CLI Commands Test Suite', () => {
    test('Should register opendaemon.newTerminalWithCLI command', async () => {
        // Get all registered commands
        const commands = await vscode.commands.getCommands(true);
        
        // Check if our command is registered
        assert.ok(
            commands.includes('opendaemon.newTerminalWithCLI'),
            'opendaemon.newTerminalWithCLI command should be registered'
        );
    });

    test('Should register opendaemon.showCLIInfo command', async () => {
        // Get all registered commands
        const commands = await vscode.commands.getCommands(true);
        
        // Check if our command is registered
        assert.ok(
            commands.includes('opendaemon.showCLIInfo'),
            'opendaemon.showCLIInfo command should be registered'
        );
    });

    test('Should register opendaemon.installCLIGlobally command', async () => {
        // Get all registered commands
        const commands = await vscode.commands.getCommands(true);
        
        // Check if our command is registered
        assert.ok(
            commands.includes('opendaemon.installCLIGlobally'),
            'opendaemon.installCLIGlobally command should be registered'
        );
    });
});

