/**
 * Property-based tests for CLI Integration Manager
 * Tests universal properties that should hold across all inputs
 */

import * as assert from 'assert';
import * as fc from 'fast-check';
import * as path from 'path';
import * as os from 'os';
import { TerminalInterceptor } from '../../cli-integration/terminal-interceptor';

suite('CLI Integration Manager Property Tests', () => {
  /**
   * Property 1: PATH Injection Universality
   * Validates: Requirements 1.1, 3.1, 3.2, 3.3
   * 
   * For any terminal created through the extension's terminal creation mechanism,
   * the terminal environment's PATH variable should contain the extension's bin directory.
   */
  test('Property 1: PATH Injection Universality - PATH contains bin directory for all terminals', async function() {
    this.timeout(30000); // Increase timeout for property tests
    
    // Property test: Generate random bin directories and terminal names
    await fc.assert(
      fc.asyncProperty(
        fc.string({ minLength: 1, maxLength: 100 }).filter(s => !s.includes('\0')),
        fc.option(fc.string({ minLength: 1, maxLength: 50 }), { nil: undefined }),
        async (binDir, terminalName) => {
          // Create a terminal interceptor with the generated bin directory
          const interceptor = new TerminalInterceptor(binDir);
          
          // Get the injected environment
          const mockEnv = { ...process.env };
          const injectedEnv = (interceptor as any).injectPath(mockEnv);
          
          // Verify PATH contains bin directory
          const pathVar = injectedEnv.PATH || injectedEnv.Path || '';
          const separator = os.platform() === 'win32' ? ';' : ':';
          const pathEntries = pathVar.split(separator);
          
          // Assert that bin directory is in PATH
          assert.ok(
            pathEntries.includes(binDir),
            `PATH should contain bin directory. PATH: ${pathVar}, binDir: ${binDir}`
          );
          
          // Assert that bin directory is the first entry (prepended)
          assert.strictEqual(
            pathEntries[0],
            binDir,
            `Bin directory should be the first entry in PATH`
          );
        }
      ),
      { numRuns: 100 }
    );
  });
});
