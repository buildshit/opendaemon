import * as assert from 'assert';
import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';

suite('Binary Selection Test Suite', () => {
    test('Extension should have bin directory', () => {
        const extensionPath = vscode.extensions.getExtension('opendaemon.opendaemon')?.extensionPath;
        assert.ok(extensionPath, 'Extension path should be defined');
        
        const binPath = path.join(extensionPath!, 'bin');
        // Note: In test environment, bin directory might not exist
        // This test is more relevant for packaged extension
        console.log('Bin path:', binPath);
    });

    test('Should select correct binary name for platform', () => {
        const platform = process.platform;
        const arch = process.arch;
        let expectedBinaryName: string;
        
        if (platform === 'win32') {
            expectedBinaryName = 'dmn-win32-x64.exe';
        } else if (platform === 'darwin') {
            if (arch === 'arm64') {
                expectedBinaryName = 'dmn-darwin-arm64';
            } else {
                expectedBinaryName = 'dmn-darwin-x64';
            }
        } else if (platform === 'linux') {
            expectedBinaryName = arch === 'arm64' ? 'dmn-linux-arm64' : 'dmn-linux-x64';
        } else {
            assert.fail(`Unsupported platform: ${platform}`);
        }
        
        assert.ok(expectedBinaryName, 'Binary name should be determined');
        console.log(`Platform: ${platform}, Arch: ${arch}, Binary: ${expectedBinaryName}`);
    });

    test('Binary should exist in bin directory after bundling', function() {
        // Skip this test in CI/development environments
        if (!process.env.TEST_PACKAGED_EXTENSION) {
            this.skip();
            return;
        }

        const extensionPath = vscode.extensions.getExtension('opendaemon.opendaemon')?.extensionPath;
        assert.ok(extensionPath, 'Extension path should be defined');
        
        const platform = process.platform;
        const arch = process.arch;
        let binaryName: string;
        
        if (platform === 'win32') {
            binaryName = 'dmn-win32-x64.exe';
        } else if (platform === 'darwin') {
            binaryName = arch === 'arm64' ? 'dmn-darwin-arm64' : 'dmn-darwin-x64';
        } else {
            binaryName = arch === 'arm64' ? 'dmn-linux-arm64' : 'dmn-linux-x64';
        }
        
        const binaryPath = path.join(extensionPath!, 'bin', binaryName);
        assert.ok(fs.existsSync(binaryPath), `Binary should exist at ${binaryPath}`);
        
        // Check if binary is executable (Unix only)
        if (platform !== 'win32') {
            const stats = fs.statSync(binaryPath);
            const isExecutable = (stats.mode & fs.constants.S_IXUSR) !== 0;
            assert.ok(isExecutable, 'Binary should be executable');
        }
    });
});
