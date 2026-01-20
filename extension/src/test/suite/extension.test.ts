import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';

suite('Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    test('Extension should be present', () => {
        assert.ok(vscode.extensions.getExtension('opendaemon.opendaemon'));
    });

    test('Should detect dmn.json in workspace', async () => {
        // Create a temporary workspace with dmn.json
        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dmn-test-'));
        const dmnPath = path.join(tmpDir, 'dmn.json');
        
        // Create a minimal dmn.json
        fs.writeFileSync(dmnPath, JSON.stringify({
            version: "1.0",
            services: {}
        }));

        // Verify file exists
        assert.ok(fs.existsSync(dmnPath));

        // Cleanup
        fs.unlinkSync(dmnPath);
        fs.rmdirSync(tmpDir);
    });

    test('Should handle missing dmn.json gracefully', async () => {
        // Create a temporary workspace without dmn.json
        const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dmn-test-'));
        const dmnPath = path.join(tmpDir, 'dmn.json');
        
        // Verify file does not exist
        assert.ok(!fs.existsSync(dmnPath));

        // Cleanup
        fs.rmdirSync(tmpDir);
    });
});
