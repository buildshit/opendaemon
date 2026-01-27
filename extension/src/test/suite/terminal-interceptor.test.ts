import * as assert from 'assert';
import { TerminalInterceptor } from '../../cli-integration/terminal-interceptor';
import * as os from 'os';

suite('TerminalInterceptor Unit Tests', () => {
    let originalPlatform: string;

    setup(() => {
        originalPlatform = process.platform;
    });

    teardown(() => {
        Object.defineProperty(process, 'platform', {
            value: originalPlatform,
            writable: true,
            configurable: true
        });
    });

    test('Should use semicolon separator on Windows', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });

        const binDir = 'C:\\test\\bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        // Access private method using reflection
        const getPathSeparator = (interceptor as any).getPathSeparator.bind(interceptor);
        const separator = getPathSeparator();
        
        assert.strictEqual(separator, ';', 'Windows should use semicolon as PATH separator');
    });

    test('Should use colon separator on Unix-like systems', () => {
        const unixPlatforms = ['darwin', 'linux'];
        
        for (const platform of unixPlatforms) {
            Object.defineProperty(process, 'platform', {
                value: platform,
                writable: true,
                configurable: true
            });

            const binDir = '/test/bin';
            const interceptor = new TerminalInterceptor(binDir);
            
            const getPathSeparator = (interceptor as any).getPathSeparator.bind(interceptor);
            const separator = getPathSeparator();
            
            assert.strictEqual(separator, ':', `${platform} should use colon as PATH separator`);
        }
    });

    test('Should prepend bin directory to existing PATH on Windows', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });

        const binDir = 'C:\\test\\bin';
        const existingPath = 'C:\\Windows\\System32;C:\\Windows';
        const interceptor = new TerminalInterceptor(binDir);
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath({ PATH: existingPath });
        
        assert.strictEqual(result.PATH, `${binDir};${existingPath}`, 
            'Bin directory should be prepended to PATH');
        assert.strictEqual(result.Path, result.PATH,
            'Windows should also set Path variable for compatibility');
    });

    test('Should prepend bin directory to existing PATH on Unix', () => {
        Object.defineProperty(process, 'platform', {
            value: 'linux',
            writable: true,
            configurable: true
        });

        const binDir = '/test/bin';
        const existingPath = '/usr/local/bin:/usr/bin:/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath({ PATH: existingPath });
        
        assert.strictEqual(result.PATH, `${binDir}:${existingPath}`, 
            'Bin directory should be prepended to PATH');
    });

    test('Should handle empty PATH correctly', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath({ PATH: '' });
        
        assert.strictEqual(result.PATH, binDir, 
            'Empty PATH should be set to just bin directory');
    });

    test('Should handle undefined PATH correctly', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath({});
        
        assert.strictEqual(result.PATH, binDir, 
            'Undefined PATH should be set to just bin directory');
    });

    test('Should handle PATH with lowercase "Path" on Windows', () => {
        Object.defineProperty(process, 'platform', {
            value: 'win32',
            writable: true,
            configurable: true
        });

        const binDir = 'C:\\test\\bin';
        const existingPath = 'C:\\Windows\\System32';
        const interceptor = new TerminalInterceptor(binDir);
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath({ Path: existingPath });
        
        assert.strictEqual(result.PATH, `${binDir};${existingPath}`, 
            'Should handle lowercase Path variable');
    });

    test('Should preserve other environment variables', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        const env = {
            PATH: '/usr/bin',
            HOME: '/home/user',
            USER: 'testuser',
            SHELL: '/bin/bash'
        };
        
        const injectPath = (interceptor as any).injectPath.bind(interceptor);
        const result = injectPath(env);
        
        assert.strictEqual(result.HOME, '/home/user', 'HOME should be preserved');
        assert.strictEqual(result.USER, 'testuser', 'USER should be preserved');
        assert.strictEqual(result.SHELL, '/bin/bash', 'SHELL should be preserved');
    });

    test('Should return bin directory from getBinDir()', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        assert.strictEqual(interceptor.getBinDir(), binDir, 
            'getBinDir should return the bin directory');
    });

    test('Should not throw when start() is called', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        assert.doesNotThrow(() => {
            interceptor.start();
        }, 'start() should not throw');
    });

    test('Should not throw when stop() is called', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        assert.doesNotThrow(() => {
            interceptor.stop();
        }, 'stop() should not throw');
    });

    test('Should handle multiple start/stop cycles', () => {
        const binDir = '/test/bin';
        const interceptor = new TerminalInterceptor(binDir);
        
        assert.doesNotThrow(() => {
            interceptor.start();
            interceptor.stop();
            interceptor.start();
            interceptor.stop();
        }, 'Multiple start/stop cycles should not throw');
    });
});
