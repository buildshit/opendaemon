import * as assert from 'assert';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import { DmnFileWatcher } from '../../file-watcher';

suite('File Watcher Test Suite', () => {
    let tmpDir: string;
    let configPath: string;
    let fileWatcher: DmnFileWatcher;
    let changeCount = 0;
    let deleteCount = 0;

    setup(() => {
        tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'dmn-watcher-test-'));
        configPath = path.join(tmpDir, 'dmn.json');
        changeCount = 0;
        deleteCount = 0;

        fileWatcher = new DmnFileWatcher(
            async () => { changeCount++; },
            async () => { deleteCount++; }
        );
    });

    teardown(() => {
        fileWatcher.stop();
        
        // Clean up temp directory
        try {
            const files = fs.readdirSync(tmpDir);
            for (const file of files) {
                fs.unlinkSync(path.join(tmpDir, file));
            }
            fs.rmdirSync(tmpDir);
        } catch {
            // Ignore cleanup errors
        }
    });

    test('Should create file watcher instance', () => {
        assert.ok(fileWatcher);
    });

    test('Should start watching a file', () => {
        // Create initial config
        fs.writeFileSync(configPath, JSON.stringify({ version: '1.0', services: {} }));
        
        fileWatcher.start(configPath);
        
        assert.strictEqual(fileWatcher.getConfigPath(), configPath);
    });

    test('Should stop watching', () => {
        fs.writeFileSync(configPath, JSON.stringify({ version: '1.0', services: {} }));
        
        fileWatcher.start(configPath);
        fileWatcher.stop();
        
        assert.strictEqual(fileWatcher.getConfigPath(), null);
    });

    test('Should return null config path when not watching', () => {
        assert.strictEqual(fileWatcher.getConfigPath(), null);
    });
});
