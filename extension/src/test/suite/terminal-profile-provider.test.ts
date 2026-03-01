import * as assert from 'assert';
import * as vscode from 'vscode';
import { OpenDaemonTerminalProfileProvider } from '../../cli-integration/terminal-profile-provider';

suite('Terminal Profile Provider Unit Tests', () => {
    test('Profile creation with valid CLI path', () => {
        const binDir = process.platform === 'win32' ? 'C:\\test\\bin' : '/test/bin';
        const provider = new OpenDaemonTerminalProfileProvider(binDir);
        
        const profile = provider.provideTerminalProfile(
            new vscode.CancellationTokenSource().token
        ) as vscode.TerminalProfile;
        
        assert.ok(profile, 'Profile should be created');
        assert.ok(profile.options, 'Profile should have options');
        
        const options = profile.options as vscode.TerminalOptions;
        assert.strictEqual(options.name, 'OpenDaemon CLI', 'Profile name should be "OpenDaemon CLI"');
        assert.ok(options.iconPath, 'Profile should have an icon');
        assert.ok(options.env, 'Profile should have env');
        
        const path = options.env!.PATH || options.env!.Path;
        assert.ok(path, 'PATH should be set');
        assert.ok(path!.includes(binDir), 'PATH should contain bin directory');
    });

    test('Platform-specific PATH separator - Windows', function() {
        if (process.platform !== 'win32') {
            this.skip();
            return;
        }

        const binDir = 'C:\\test\\bin';
        const originalPath = process.env.PATH;
        process.env.PATH = 'C:\\existing\\path';

        try {
            const provider = new OpenDaemonTerminalProfileProvider(binDir);
            const profile = provider.provideTerminalProfile(
                new vscode.CancellationTokenSource().token
            ) as vscode.TerminalProfile;

            const options = profile.options as vscode.TerminalOptions;
            const path = options.env!.PATH || options.env!.Path;

            assert.ok(path!.includes(';'), 'Windows PATH should use semicolon separator');
            assert.ok(path!.startsWith(binDir), 'Bin directory should be first in PATH');
            
            // Verify both PATH and Path are set on Windows
            assert.ok(options.env!.PATH, 'PATH should be set');
            assert.ok(options.env!.Path, 'Path should also be set for Windows compatibility');
        } finally {
            if (originalPath !== undefined) {
                process.env.PATH = originalPath;
            } else {
                delete process.env.PATH;
            }
        }
    });

    test('Platform-specific PATH separator - Unix', function() {
        if (process.platform === 'win32') {
            this.skip();
            return;
        }

        const binDir = '/test/bin';
        const originalPath = process.env.PATH;
        process.env.PATH = '/existing/path';

        try {
            const provider = new OpenDaemonTerminalProfileProvider(binDir);
            const profile = provider.provideTerminalProfile(
                new vscode.CancellationTokenSource().token
            ) as vscode.TerminalProfile;

            const options = profile.options as vscode.TerminalOptions;
            const path = options.env!.PATH;

            assert.ok(path!.includes(':'), 'Unix PATH should use colon separator');
            assert.ok(path!.startsWith(binDir), 'Bin directory should be first in PATH');
        } finally {
            if (originalPath !== undefined) {
                process.env.PATH = originalPath;
            } else {
                delete process.env.PATH;
            }
        }
    });

    test('Profile name and description', () => {
        const binDir = process.platform === 'win32' ? 'C:\\test\\bin' : '/test/bin';
        const provider = new OpenDaemonTerminalProfileProvider(binDir);
        
        const profile = provider.provideTerminalProfile(
            new vscode.CancellationTokenSource().token
        ) as vscode.TerminalProfile;
        
        const options = profile.options as vscode.TerminalOptions;
        assert.strictEqual(options.name, 'OpenDaemon CLI', 'Profile should have correct name');
        assert.ok(options.iconPath, 'Profile should have an icon');
    });

    test('Empty PATH handling', () => {
        const binDir = process.platform === 'win32' ? 'C:\\test\\bin' : '/test/bin';
        const originalPath = process.env.PATH;
        delete process.env.PATH;
        delete (process.env as any).Path;

        try {
            const provider = new OpenDaemonTerminalProfileProvider(binDir);
            const profile = provider.provideTerminalProfile(
                new vscode.CancellationTokenSource().token
            ) as vscode.TerminalProfile;

            const options = profile.options as vscode.TerminalOptions;
            const path = options.env!.PATH || options.env!.Path;

            assert.strictEqual(path, binDir, 'PATH should be just the bin directory when no existing PATH');
        } finally {
            if (originalPath !== undefined) {
                process.env.PATH = originalPath;
            }
        }
    });

    test('PATH preservation', () => {
        const binDir = process.platform === 'win32' ? 'C:\\test\\bin' : '/test/bin';
        const existingPath = process.platform === 'win32' 
            ? 'C:\\path1;C:\\path2;C:\\path3'
            : '/path1:/path2:/path3';
        
        const originalPath = process.env.PATH;
        process.env.PATH = existingPath;

        try {
            const provider = new OpenDaemonTerminalProfileProvider(binDir);
            const profile = provider.provideTerminalProfile(
                new vscode.CancellationTokenSource().token
            ) as vscode.TerminalProfile;

            const options = profile.options as vscode.TerminalOptions;
            const path = options.env!.PATH || options.env!.Path;

            // Verify all original paths are preserved
            const separator = process.platform === 'win32' ? ';' : ':';
            const originalPaths = existingPath.split(separator);
            
            for (const originalPathEntry of originalPaths) {
                assert.ok(
                    path!.includes(originalPathEntry),
                    `PATH should preserve original entry: ${originalPathEntry}`
                );
            }

            // Verify bin directory is prepended
            assert.ok(path!.startsWith(binDir), 'Bin directory should be first in PATH');
        } finally {
            if (originalPath !== undefined) {
                process.env.PATH = originalPath;
            } else {
                delete process.env.PATH;
            }
        }
    });
});
