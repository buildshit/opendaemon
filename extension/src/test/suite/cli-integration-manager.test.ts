/**
 * Integration tests for CLI Integration Manager
 * Tests the full activation flow and component integration
 */

import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import * as os from 'os';
import * as fs from 'fs/promises';
import { CLIIntegrationManager } from '../../cli-integration/cli-integration-manager';

suite('CLI Integration Manager Integration Tests', () => {
  let tempDir: string;
  let mockContext: vscode.ExtensionContext;

  setup(async () => {
    // Create a temporary directory for test files
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'cli-integration-test-'));
    
    // Create mock extension context
    mockContext = createMockExtensionContext(tempDir);
  });

  teardown(async () => {
    // Clean up temporary directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch (error) {
      // Ignore cleanup errors
    }
  });

  test('Should activate successfully with valid binary', async function() {
    this.timeout(10000);
    
    // Create bin directory and binary
    const binDir = path.join(tempDir, 'bin');
    await fs.mkdir(binDir, { recursive: true });
    
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = platform === 'win32' 
      ? `dmn-${platform}-${arch}.exe`
      : `dmn-${platform}-${arch}`;
    const binaryPath = path.join(binDir, binaryName);
    
    // Create a dummy binary
    await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
    
    // Set execute permissions on Unix
    if (platform !== 'win32') {
      await fs.chmod(binaryPath, 0o755);
    }
    
    // Create manager and activate
    const manager = new CLIIntegrationManager(mockContext);
    await manager.activate();
    
    // Activation should succeed without throwing
    assert.ok(true, 'Activation completed successfully');
  });

  test('Should handle missing binary gracefully', async function() {
    this.timeout(10000);
    
    // Don't create the binary - it should be missing
    const manager = new CLIIntegrationManager(mockContext);
    
    // Activation should not throw, but should log error
    await manager.activate();
    
    // Should not be able to create terminal with CLI
    try {
      await manager.createTerminalWithCLI();
      assert.fail('Should have thrown error for missing binary');
    } catch (error) {
      assert.ok(error instanceof Error);
      assert.ok(error.message.includes('not activated'));
    }
  });

  test('Should handle permission issues on Unix', async function() {
    // Skip on Windows
    if (process.platform === 'win32') {
      this.skip();
      return;
    }
    
    this.timeout(10000);
    
    // Create bin directory and binary without execute permissions
    const binDir = path.join(tempDir, 'bin');
    await fs.mkdir(binDir, { recursive: true });
    
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = `dmn-${platform}-${arch}`;
    const binaryPath = path.join(binDir, binaryName);
    
    // Create binary without execute permissions
    await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
    await fs.chmod(binaryPath, 0o644); // No execute permission
    
    // Create manager and activate
    const manager = new CLIIntegrationManager(mockContext);
    await manager.activate();
    
    // Should have attempted to fix permissions
    // Verify permissions were fixed
    const stats = await fs.stat(binaryPath);
    const mode = stats.mode & 0o777;
    assert.ok(mode & 0o100, 'Execute permission should be set');
  });

  test('Should create terminal with PATH injection', async function() {
    this.timeout(10000);
    
    // Create bin directory and binary
    const binDir = path.join(tempDir, 'bin');
    await fs.mkdir(binDir, { recursive: true });
    
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = platform === 'win32' 
      ? `dmn-${platform}-${arch}.exe`
      : `dmn-${platform}-${arch}`;
    const binaryPath = path.join(binDir, binaryName);
    
    // Create a dummy binary
    await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
    
    // Set execute permissions on Unix
    if (platform !== 'win32') {
      await fs.chmod(binaryPath, 0o755);
    }
    
    // Create manager and activate
    const manager = new CLIIntegrationManager(mockContext);
    await manager.activate();
    
    // Create terminal with CLI
    const terminal = await manager.createTerminalWithCLI('Test Terminal');
    
    // Verify terminal was created
    assert.ok(terminal, 'Terminal should be created');
    assert.strictEqual(terminal.name, 'Test Terminal');
    
    // Clean up
    terminal.dispose();
  });

  test('Should show global install instructions', async function() {
    this.timeout(10000);
    
    // Create bin directory and binary
    const binDir = path.join(tempDir, 'bin');
    await fs.mkdir(binDir, { recursive: true });
    
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = platform === 'win32' 
      ? `dmn-${platform}-${arch}.exe`
      : `dmn-${platform}-${arch}`;
    const binaryPath = path.join(binDir, binaryName);
    
    // Create a dummy binary
    await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
    
    // Set execute permissions on Unix
    if (platform !== 'win32') {
      await fs.chmod(binaryPath, 0o755);
    }
    
    // Create manager and activate
    const manager = new CLIIntegrationManager(mockContext);
    await manager.activate();
    
    // Show global install instructions (should not throw)
    await manager.showGlobalInstallInstructions();
    
    assert.ok(true, 'Global install instructions shown successfully');
  });

  test('Should cleanup on deactivation', async function() {
    this.timeout(10000);
    
    // Create bin directory and binary
    const binDir = path.join(tempDir, 'bin');
    await fs.mkdir(binDir, { recursive: true });
    
    const platform = process.platform;
    const arch = process.arch;
    const binaryName = platform === 'win32' 
      ? `dmn-${platform}-${arch}.exe`
      : `dmn-${platform}-${arch}`;
    const binaryPath = path.join(binDir, binaryName);
    
    // Create a dummy binary
    await fs.writeFile(binaryPath, '#!/bin/bash\necho "test"');
    
    // Set execute permissions on Unix
    if (platform !== 'win32') {
      await fs.chmod(binaryPath, 0o755);
    }
    
    // Create manager and activate
    const manager = new CLIIntegrationManager(mockContext);
    await manager.activate();
    
    // Deactivate (should not throw)
    manager.deactivate();
    
    // After deactivation, should not be able to create terminals
    try {
      await manager.createTerminalWithCLI();
      assert.fail('Should have thrown error after deactivation');
    } catch (error) {
      assert.ok(error instanceof Error);
      assert.ok(error.message.includes('not activated'));
    }
  });

  test('Should handle unsupported platform gracefully', async function() {
    this.timeout(10000);
    
    // This test is tricky because we can't actually change the platform
    // We'll just verify that the manager handles errors gracefully
    
    // Create manager with invalid extension path
    const invalidContext = createMockExtensionContext('/invalid/path');
    const manager = new CLIIntegrationManager(invalidContext);
    
    // Activation should not throw, but should handle error
    await manager.activate();
    
    // Should not be able to create terminal
    try {
      await manager.createTerminalWithCLI();
      assert.fail('Should have thrown error');
    } catch (error) {
      assert.ok(error instanceof Error);
    }
  });
});

/**
 * Creates a mock ExtensionContext for testing
 */
function createMockExtensionContext(extensionPath: string): vscode.ExtensionContext {
  return {
    extensionPath: extensionPath,
    globalState: {
      get: (key: string, defaultValue?: any) => defaultValue,
      update: async (key: string, value: any) => {},
      keys: () => [],
      setKeysForSync: (keys: string[]) => {}
    },
    workspaceState: {
      get: (key: string, defaultValue?: any) => defaultValue,
      update: async (key: string, value: any) => {},
      keys: () => []
    },
    subscriptions: [],
    extensionUri: vscode.Uri.file(extensionPath),
    extensionMode: vscode.ExtensionMode.Test,
    storagePath: undefined,
    globalStoragePath: path.join(os.tmpdir(), 'vscode-test-storage'),
    logPath: path.join(os.tmpdir(), 'vscode-test-logs'),
    asAbsolutePath: (relativePath: string) => path.join(extensionPath, relativePath),
    storageUri: undefined,
    globalStorageUri: vscode.Uri.file(path.join(os.tmpdir(), 'vscode-test-storage')),
    logUri: vscode.Uri.file(path.join(os.tmpdir(), 'vscode-test-logs')),
    environmentVariableCollection: {} as any,
    secrets: {} as any,
    extension: {} as any,
    languageModelAccessInformation: {} as any
  } as vscode.ExtensionContext;
}
