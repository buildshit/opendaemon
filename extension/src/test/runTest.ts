import * as path from 'path';
import * as fs from 'fs';
import { runTests } from '@vscode/test-electron';

async function main() {
    try {
        const extensionDevelopmentPath = path.resolve(__dirname, '../../');
        const extensionTestsPath = path.resolve(__dirname, './suite/index');
        
        console.log('Extension development path:', extensionDevelopmentPath);
        console.log('Extension tests path:', extensionTestsPath);
        
        // Create a dedicated test workspace directory to avoid VSCode scanning parent directories
        const testWorkspace = path.resolve(__dirname, '../../.test-workspace');
        
        // Create a dedicated user data directory for tests
        const testUserDataDir = path.resolve(__dirname, '../../.test-user-data');
        
        // Create a dedicated extensions directory for tests
        const testExtensionsDir = path.resolve(__dirname, '../../.test-extensions');
        
        console.log('Test workspace:', testWorkspace);
        console.log('Test user data dir:', testUserDataDir);
        console.log('Test extensions dir:', testExtensionsDir);
        
        // Ensure directories exist
        [testWorkspace, testUserDataDir, testExtensionsDir].forEach(dir => {
            if (!fs.existsSync(dir)) {
                fs.mkdirSync(dir, { recursive: true });
            }
        });
        
        // Create a minimal package.json to make it a valid workspace
        const packageJsonPath = path.join(testWorkspace, 'package.json');
        if (!fs.existsSync(packageJsonPath)) {
            fs.writeFileSync(packageJsonPath, JSON.stringify({
                name: 'test-workspace',
                version: '1.0.0',
                private: true
            }, null, 2));
        }

        await runTests({ 
            extensionDevelopmentPath, 
            extensionTestsPath,
            launchArgs: [
                testWorkspace,
                '--disable-extensions',
                `--user-data-dir=${testUserDataDir}`,
                `--extensions-dir=${testExtensionsDir}`
            ]
        });
    } catch (err) {
        console.error('Failed to run tests:', err);
        process.exit(1);
    }
}

main();
