/**
 * Unit tests for binary verifier
 * Tests verification succeeds/fails for various scenarios and permission fixing
 */

import * as assert from 'assert';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import { verifyBinary, fixPermissions } from '../../cli-integration/binary-verifier';

suite('Binary Verifier Test Suite', () => {
    let tempDir: string;
    let testBinaryPath: string;

    setup(async () => {
        // Create a temporary directory for test files
        tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'binary-verifier-test-'));
        testBinaryPath = path.join(tempDir, 'test-binary');
    });

    teardown(async () => {
        // Clean up temporary directory
        try {
            await fs.rm(tempDir, { recursive: true, force: true });
        } catch (error) {
            // Ignore cleanup errors
        }
    });

    test('Should verify binary exists with permissions on Unix', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        // Create a test binary with execute permissions
        await fs.writeFile(testBinaryPath, '#!/bin/bash\necho "test"');
        await fs.chmod(testBinaryPath, 0o755);

        const result = await verifyBinary(testBinaryPath);

        assert.strictEqual(result.exists, true);
        assert.strictEqual(result.hasPermissions, true);
        assert.strictEqual(result.error, undefined);
    });

    test('Should verify binary exists with permissions on Windows', async function() {
        // Only run on Windows
        if (process.platform !== 'win32') {
            this.skip();
            return;
        }

        // Create a test binary
        await fs.writeFile(testBinaryPath + '.exe', 'test content');

        const result = await verifyBinary(testBinaryPath + '.exe');

        assert.strictEqual(result.exists, true);
        assert.strictEqual(result.hasPermissions, true);
        assert.strictEqual(result.error, undefined);
    });

    test('Should fail verification when binary is missing', async () => {
        const nonExistentPath = path.join(tempDir, 'non-existent-binary');

        const result = await verifyBinary(nonExistentPath);

        assert.strictEqual(result.exists, false);
        assert.strictEqual(result.hasPermissions, false);
        assert.ok(result.error);
        assert.ok(result.error.includes('not found'));
    });

    test('Should fail verification when permissions are missing on Unix', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        // Create a test binary without execute permissions
        await fs.writeFile(testBinaryPath, '#!/bin/bash\necho "test"');
        await fs.chmod(testBinaryPath, 0o644); // Read/write only, no execute

        const result = await verifyBinary(testBinaryPath);

        assert.strictEqual(result.exists, true);
        assert.strictEqual(result.hasPermissions, false);
        assert.ok(result.error);
        assert.ok(result.error.includes('permissions'));
    });

    test('Should successfully fix permissions on Unix', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        // Create a test binary without execute permissions
        await fs.writeFile(testBinaryPath, '#!/bin/bash\necho "test"');
        await fs.chmod(testBinaryPath, 0o644);

        // Verify it lacks permissions
        const beforeResult = await verifyBinary(testBinaryPath);
        assert.strictEqual(beforeResult.hasPermissions, false);

        // Fix permissions
        const fixResult = await fixPermissions(testBinaryPath);
        assert.strictEqual(fixResult, true);

        // Verify permissions are now correct
        const afterResult = await verifyBinary(testBinaryPath);
        assert.strictEqual(afterResult.hasPermissions, true);
    });

    test('Should return true for fixPermissions on Windows', async function() {
        // Only run on Windows
        if (process.platform !== 'win32') {
            this.skip();
            return;
        }

        // Create a test binary
        await fs.writeFile(testBinaryPath + '.exe', 'test content');

        // fixPermissions should return true on Windows (no-op)
        const result = await fixPermissions(testBinaryPath + '.exe');
        assert.strictEqual(result, true);
    });

    test('Should fail to fix permissions for non-existent file on Unix', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        const nonExistentPath = path.join(tempDir, 'non-existent-binary');

        const result = await fixPermissions(nonExistentPath);
        assert.strictEqual(result, false);
    });

    test('Should handle read-only directory on Unix', async function() {
        // Skip on Windows (different permission model)
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        // Create a read-only directory
        const readOnlyDir = path.join(tempDir, 'readonly');
        await fs.mkdir(readOnlyDir);
        const readOnlyBinary = path.join(readOnlyDir, 'test-binary');
        await fs.writeFile(readOnlyBinary, '#!/bin/bash\necho "test"');
        
        // Make directory read-only
        await fs.chmod(readOnlyDir, 0o555);

        try {
            // Attempt to fix permissions should fail
            const result = await fixPermissions(readOnlyBinary);
            assert.strictEqual(result, false);
        } finally {
            // Restore permissions for cleanup
            await fs.chmod(readOnlyDir, 0o755);
        }
    });

    test('Should return correct error message for missing binary', async () => {
        const missingPath = path.join(tempDir, 'missing-binary');

        const result = await verifyBinary(missingPath);

        assert.strictEqual(result.exists, false);
        assert.ok(result.error);
        assert.ok(result.error.includes(missingPath));
    });

    test('Should handle absolute paths correctly', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        const absolutePath = path.resolve(testBinaryPath);
        await fs.writeFile(absolutePath, '#!/bin/bash\necho "test"');
        await fs.chmod(absolutePath, 0o755);

        const result = await verifyBinary(absolutePath);

        assert.strictEqual(result.exists, true);
        assert.strictEqual(result.hasPermissions, true);
    });

    test('Should handle relative paths correctly', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        // Create binary in temp dir
        const binaryName = 'relative-test-binary';
        const binaryPath = path.join(tempDir, binaryName);
        await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
        await fs.chmod(binaryPath, 0o755);

        // Change to temp dir and use relative path
        const originalCwd = process.cwd();
        try {
            process.chdir(tempDir);
            const result = await verifyBinary(`./${binaryName}`);

            assert.strictEqual(result.exists, true);
            assert.strictEqual(result.hasPermissions, true);
        } finally {
            process.chdir(originalCwd);
        }
    });

    test('Should verify multiple binaries independently', async function() {
        // Skip on Windows
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        const binary1 = path.join(tempDir, 'binary1');
        const binary2 = path.join(tempDir, 'binary2');
        const binary3 = path.join(tempDir, 'binary3');

        // Create binaries with different states
        await fs.writeFile(binary1, '#!/bin/bash\necho "test"');
        await fs.chmod(binary1, 0o755); // Has permissions

        await fs.writeFile(binary2, '#!/bin/bash\necho "test"');
        await fs.chmod(binary2, 0o644); // No permissions

        // binary3 doesn't exist

        const result1 = await verifyBinary(binary1);
        const result2 = await verifyBinary(binary2);
        const result3 = await verifyBinary(binary3);

        assert.strictEqual(result1.exists, true);
        assert.strictEqual(result1.hasPermissions, true);

        assert.strictEqual(result2.exists, true);
        assert.strictEqual(result2.hasPermissions, false);

        assert.strictEqual(result3.exists, false);
        assert.strictEqual(result3.hasPermissions, false);
    });
});
