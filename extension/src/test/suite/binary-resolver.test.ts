/**
 * Unit tests for binary resolver
 * Tests correct binary name for each platform, full path construction, and bin directory extraction
 */

import * as assert from 'assert';
import * as path from 'path';
import { resolveBinary } from '../../cli-integration/binary-resolver';
import { PlatformInfo } from '../../cli-integration/platform-detector';

suite('Binary Resolver Test Suite', () => {
    const testExtensionPath = '/test/extension/path';
    const testExtensionPathWin = 'C:\\test\\extension\\path';

    test('Should return correct binary name for Windows x64', () => {
        const platform: PlatformInfo = { os: 'win32', arch: 'x64' };
        const result = resolveBinary(testExtensionPathWin, platform);
        
        assert.strictEqual(result.name, 'dmn-win32-x64.exe');
    });

    test('Should reuse Windows x64 binary for Windows arm64', () => {
        const platform: PlatformInfo = { os: 'win32', arch: 'arm64' };
        const result = resolveBinary(testExtensionPathWin, platform);

        assert.strictEqual(result.name, 'dmn-win32-x64.exe');
    });

    test('Should return correct binary name for macOS ARM64', () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'arm64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        assert.strictEqual(result.name, 'dmn-darwin-arm64');
    });

    test('Should return correct binary name for macOS x64', () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        assert.strictEqual(result.name, 'dmn-darwin-x64');
    });

    test('Should return correct binary name for Linux x64', () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        assert.strictEqual(result.name, 'dmn-linux-x64');
    });

    test('Should return correct binary name for Linux arm64', () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'arm64' };
        const result = resolveBinary(testExtensionPath, platform);

        assert.strictEqual(result.name, 'dmn-linux-arm64');
    });

    test('Should construct full path correctly on Unix-like systems', () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        const expectedBinDir = path.join(testExtensionPath, 'bin');
        const expectedFullPath = path.join(expectedBinDir, 'dmn-linux-x64');
        
        assert.strictEqual(result.fullPath, expectedFullPath);
    });

    test('Should construct full path correctly on Windows', () => {
        const platform: PlatformInfo = { os: 'win32', arch: 'x64' };
        const result = resolveBinary(testExtensionPathWin, platform);
        
        const expectedBinDir = path.join(testExtensionPathWin, 'bin');
        const expectedFullPath = path.join(expectedBinDir, 'dmn-win32-x64.exe');
        
        assert.strictEqual(result.fullPath, expectedFullPath);
    });

    test('Should extract bin directory correctly', () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'arm64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        const expectedBinDir = path.join(testExtensionPath, 'bin');
        assert.strictEqual(result.binDir, expectedBinDir);
    });

    test('Should handle various extension paths correctly', () => {
        const testPaths = [
            '/usr/local/vscode/extensions/opendaemon',
            '/home/user/.vscode/extensions/opendaemon',
            'C:\\Users\\User\\AppData\\Local\\Programs\\VSCode\\extensions\\opendaemon'
        ];
        
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        
        testPaths.forEach(testPath => {
            const result = resolveBinary(testPath, platform);
            
            // Verify binDir is constructed correctly
            assert.strictEqual(result.binDir, path.join(testPath, 'bin'));
            
            // Verify fullPath contains binDir
            assert.ok(result.fullPath.includes(result.binDir));
            
            // Verify fullPath ends with binary name
            assert.ok(result.fullPath.endsWith(result.name));
        });
    });

    test('Should return BinaryInfo with all required fields', () => {
        const platform: PlatformInfo = { os: 'darwin', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        // Verify all fields are present
        assert.ok(result.name);
        assert.ok(result.fullPath);
        assert.ok(result.binDir);
        
        // Verify types
        assert.strictEqual(typeof result.name, 'string');
        assert.strictEqual(typeof result.fullPath, 'string');
        assert.strictEqual(typeof result.binDir, 'string');
    });

    test('Should only add .exe extension for Windows', () => {
        const platforms: PlatformInfo[] = [
            { os: 'win32', arch: 'x64' },
            { os: 'win32', arch: 'arm64' },
            { os: 'darwin', arch: 'x64' },
            { os: 'darwin', arch: 'arm64' },
            { os: 'linux', arch: 'x64' },
            { os: 'linux', arch: 'arm64' }
        ];
        
        platforms.forEach(platform => {
            const result = resolveBinary(testExtensionPath, platform);
            
            if (platform.os === 'win32') {
                assert.ok(result.name.endsWith('.exe'), 
                    `Windows binary should end with .exe: ${result.name}`);
            } else {
                assert.ok(!result.name.endsWith('.exe'), 
                    `Non-Windows binary should not end with .exe: ${result.name}`);
            }
        });
    });

    test('Should construct path using path.join for cross-platform compatibility', () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        // Verify the path uses the correct separator for the current platform
        const expectedPath = path.join(testExtensionPath, 'bin', result.name);
        assert.strictEqual(result.fullPath, expectedPath);
    });

    test('Should handle extension path with trailing slash', () => {
        const pathWithSlash = testExtensionPath + '/';
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const result = resolveBinary(pathWithSlash, platform);
        
        // path.join should normalize the path
        assert.ok(result.fullPath.includes('bin'));
        assert.ok(result.fullPath.endsWith(result.name));
    });

    test('Should handle extension path without trailing slash', () => {
        const platform: PlatformInfo = { os: 'linux', arch: 'x64' };
        const result = resolveBinary(testExtensionPath, platform);
        
        assert.ok(result.fullPath.includes('bin'));
        assert.ok(result.fullPath.endsWith(result.name));
    });
});
