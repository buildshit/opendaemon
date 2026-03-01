import * as assert from 'assert';
import { detectPlatform, PlatformInfo } from '../../cli-integration/platform-detector';

suite('Platform Detector Test Suite', () => {
    let originalPlatform: string;
    let originalArch: string;

    setup(() => {
        // Save original values
        originalPlatform = process.platform;
        originalArch = process.arch;
    });

    teardown(() => {
        // Restore original values
        Object.defineProperty(process, 'platform', {
            value: originalPlatform,
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: originalArch,
            writable: true,
            configurable: true
        });
    });

    test('Should detect Windows x64 platform', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'x64',
            writable: true,
            configurable: true
        });

        const result = detectPlatform();
        assert.strictEqual(result.os, 'win32');
        assert.strictEqual(result.arch, 'x64');
    });

    test('Should detect macOS ARM64 platform', () => {
        Object.defineProperty(process, 'platform', {
            value: 'darwin',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'arm64',
            writable: true,
            configurable: true
        });

        const result = detectPlatform();
        assert.strictEqual(result.os, 'darwin');
        assert.strictEqual(result.arch, 'arm64');
    });

    test('Should detect macOS x64 platform', () => {
        Object.defineProperty(process, 'platform', {
            value: 'darwin',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'x64',
            writable: true,
            configurable: true
        });

        const result = detectPlatform();
        assert.strictEqual(result.os, 'darwin');
        assert.strictEqual(result.arch, 'x64');
    });

    test('Should detect Linux x64 platform', () => {
        Object.defineProperty(process, 'platform', {
            value: 'linux',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'x64',
            writable: true,
            configurable: true
        });

        const result = detectPlatform();
        assert.strictEqual(result.os, 'linux');
        assert.strictEqual(result.arch, 'x64');
    });

    test('Should throw error for unsupported operating system', () => {
        Object.defineProperty(process, 'platform', {
            value: 'freebsd',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'x64',
            writable: true,
            configurable: true
        });

        assert.throws(
            () => detectPlatform(),
            /Unsupported operating system: freebsd/
        );
    });

    test('Should throw error for unsupported architecture', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'ia32',
            writable: true,
            configurable: true
        });

        assert.throws(
            () => detectPlatform(),
            /Unsupported architecture: ia32/
        );
    });

    test('Should include supported platforms in error message', () => {
        Object.defineProperty(process, 'platform', {
            value: 'aix',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 'x64',
            writable: true,
            configurable: true
        });

        try {
            detectPlatform();
            assert.fail('Should have thrown an error');
        } catch (error: any) {
            assert.ok(error.message.includes('win32'));
            assert.ok(error.message.includes('darwin'));
            assert.ok(error.message.includes('linux'));
        }
    });

    test('Should include supported architectures in error message', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });
        Object.defineProperty(process, 'arch', {
            value: 's390x',
            writable: true,
            configurable: true
        });

        try {
            detectPlatform();
            assert.fail('Should have thrown an error');
        } catch (error: any) {
            assert.ok(error.message.includes('x64'));
            assert.ok(error.message.includes('arm64'));
        }
    });
});
