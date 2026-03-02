/**
 * Property-based tests for binary resolver
 * Feature: vscode-terminal-cli-integration
 * Property 3: Path Construction Pattern
 */

import * as assert from 'assert';
import * as path from 'path';
import * as fc from 'fast-check';
import { resolveBinary } from '../../cli-integration/binary-resolver';

suite('Binary Resolver Property Tests', () => {
    test('Property 3: Path Construction Pattern - For any valid extension path and platform-specific binary name, the Binary_Resolver should construct a full path following the pattern {extensionPath}/bin/{binaryName}', () => {
        // **Validates: Requirements 2.6**
        
        // Arbitrary for generating valid extension paths
        const extensionPathArb = fc.array(
            fc.stringMatching(/^[a-zA-Z0-9_-]+$/),
            { minLength: 1, maxLength: 5 }
        ).map(parts => {
            // Create a valid path structure
            if (process.platform === 'win32') {
                return 'C:\\' + parts.join('\\');
            } else {
                return '/' + parts.join('/');
            }
        });

        // Arbitrary for generating platform info
        const platformArb = fc.record({
            os: fc.constantFrom('win32' as const, 'darwin' as const, 'linux' as const),
            arch: fc.constantFrom('x64' as const, 'arm64' as const)
        });

        // Property: constructed path should match pattern {extensionPath}/bin/{binaryName}
        fc.assert(
            fc.property(extensionPathArb, platformArb, (extensionPath, platform) => {
                const result = resolveBinary(extensionPath, platform);
                
                // Verify the pattern: fullPath should be extensionPath/bin/binaryName
                const expectedBinDir = path.join(extensionPath, 'bin');
                const expectedFullPath = path.join(expectedBinDir, result.name);
                
                // Assert the constructed path matches the expected pattern
                assert.strictEqual(result.binDir, expectedBinDir, 
                    `binDir should be ${expectedBinDir} but got ${result.binDir}`);
                assert.strictEqual(result.fullPath, expectedFullPath,
                    `fullPath should be ${expectedFullPath} but got ${result.fullPath}`);
                
                // Verify fullPath contains binDir
                assert.ok(result.fullPath.startsWith(result.binDir),
                    `fullPath ${result.fullPath} should start with binDir ${result.binDir}`);
                
                // Verify fullPath ends with binary name
                assert.ok(result.fullPath.endsWith(result.name),
                    `fullPath ${result.fullPath} should end with binary name ${result.name}`);
                
                // Verify binary name follows the expected platform mapping
                const expectedName = platform.os === 'win32'
                    ? 'dmn-win32-x64.exe'
                    : platform.os === 'darwin'
                        ? (platform.arch === 'arm64' ? 'dmn-darwin-arm64' : 'dmn-darwin-x64')
                        : (platform.arch === 'arm64' ? 'dmn-linux-arm64' : 'dmn-linux-x64');
                assert.strictEqual(
                    result.name,
                    expectedName,
                    `Binary name should be ${expectedName} but got ${result.name}`
                );
                
                // Verify .exe extension for Windows
                if (platform.os === 'win32') {
                    assert.ok(result.name.endsWith('.exe'),
                        `Windows binary ${result.name} should end with .exe`);
                } else {
                    assert.ok(!result.name.endsWith('.exe'),
                        `Non-Windows binary ${result.name} should not end with .exe`);
                }
            }),
            { numRuns: 100 } // Run 100 iterations as specified
        );
    });
});
