/**
 * CLI Integration Manager
 * Main coordinator for CLI integration lifecycle
 */

import * as vscode from 'vscode';
import { detectPlatform, PlatformInfo } from './platform-detector';
import { resolveBinary, BinaryInfo } from './binary-resolver';
import { verifyBinary, fixPermissions } from './binary-verifier';
import { TerminalInterceptor } from './terminal-interceptor';
import { NotificationManager } from './notification-manager';
import { getCLILogger } from './cli-logger';

/**
 * Manages the CLI integration lifecycle
 */
export class CLIIntegrationManager {
  private context: vscode.ExtensionContext;
  private interceptor: TerminalInterceptor | null = null;
  private notificationManager: NotificationManager;
  private binaryInfo: BinaryInfo | null = null;
  private platform: PlatformInfo | null = null;
  private logger = getCLILogger();

  constructor(context: vscode.ExtensionContext) {
    this.context = context;
    this.notificationManager = new NotificationManager(context);
    this.logger.info('CLIIntegrationManager created');
    this.logger.info(`Extension path: ${context.extensionPath}`);
  }

  /**
   * Activates the CLI integration
   * Performs platform detection, binary resolution, verification, and setup
   */
  async activate(): Promise<void> {
    this.logger.info('========================================');
    this.logger.info('    CLI Integration Activation Start    ');
    this.logger.info('========================================');
    
    try {
      // 1. Detect platform
      this.logger.info('Step 1: Detecting platform...');
      this.platform = detectPlatform();
      this.logger.info(`Platform detected: OS=${this.platform.os}, Arch=${this.platform.arch}`);
      
      // 2. Resolve binary path
      this.logger.info('Step 2: Resolving binary path...');
      this.binaryInfo = resolveBinary(this.context.extensionPath, this.platform);
      this.logger.info(`Binary info:`);
      this.logger.info(`  Name: ${this.binaryInfo.name}`);
      this.logger.info(`  Full path: ${this.binaryInfo.fullPath}`);
      this.logger.info(`  Bin directory: ${this.binaryInfo.binDir}`);
      
      // 3. Verify binary exists and has permissions
      this.logger.info('Step 3: Verifying binary...');
      const verificationResult = await verifyBinary(this.binaryInfo.fullPath);
      this.logger.info(`Verification result:`, verificationResult);
      
      // 4. Handle verification failures
      if (!verificationResult.exists) {
        const errorMsg = `CLI binary not found at: ${this.binaryInfo.fullPath}`;
        this.logger.error(errorMsg);
        this.logger.error('Make sure to run the bundle-extension script to copy binaries');
        await this.notificationManager.showErrorNotification(errorMsg);
        this.logger.show(); // Show output channel so user can see the error
        return; // Return early, disable terminal integration
      }
      
      this.logger.info('Binary exists: YES');
      
      if (!verificationResult.hasPermissions) {
        this.logger.warn('Binary lacks execute permissions, attempting to fix...');
        
        // Attempt to fix permissions
        const fixed = await fixPermissions(this.binaryInfo.fullPath);
        
        if (!fixed) {
          const errorMsg = `Failed to set execute permissions. Please run: chmod +x ${this.binaryInfo.fullPath}`;
          this.logger.error(errorMsg);
          await this.notificationManager.showErrorNotification(errorMsg);
          this.logger.show();
          return; // Return early, disable terminal integration
        }
        
        this.logger.info('Execute permissions set successfully');
      } else {
        this.logger.info('Binary has execute permissions: YES');
      }
      
      // 5. Initialize TerminalInterceptor with bin directory
      this.logger.info('Step 4: Initializing TerminalInterceptor...');
      this.interceptor = new TerminalInterceptor(this.binaryInfo.binDir);
      
      // 6. Start intercepting terminals (register profile)
      this.logger.info('Step 5: Starting terminal interceptor...');
      await this.interceptor.start();
      
      // 7. Show first-time notification if applicable
      this.logger.info('Step 6: Showing first-time notification (if applicable)...');
      await this.notificationManager.showFirstTimeNotification(this.binaryInfo.binDir);
      
      this.logger.info('========================================');
      this.logger.info('  CLI Integration Activation COMPLETE   ');
      this.logger.info('========================================');
      this.logger.info(`Binary: ${this.binaryInfo.fullPath}`);
      this.logger.info(`Bin directory: ${this.binaryInfo.binDir}`);
      this.logger.info('');
      this.logger.info('To use the CLI:');
      this.logger.info('1. Open a NEW terminal (existing terminals won\'t have the updated PATH)');
      this.logger.info('2. Run: dmn --version');
      this.logger.info('');
      
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      this.logger.error('========================================');
      this.logger.error('   CLI Integration Activation FAILED    ');
      this.logger.error('========================================');
      this.logger.error(`Error: ${errorMsg}`);
      if (error instanceof Error && error.stack) {
        this.logger.error('Stack trace:', error.stack);
      }
      await this.notificationManager.showErrorNotification(`Activation failed: ${errorMsg}`);
      this.logger.show(); // Show output channel so user can see the error
    }
  }

  /**
   * Creates a terminal with CLI available in PATH
   * @param name - Optional name for the terminal
   * @returns The created terminal instance
   */
  async createTerminalWithCLI(name?: string): Promise<vscode.Terminal> {
    if (!this.interceptor) {
      throw new Error('CLI integration not activated. Cannot create terminal with CLI.');
    }
    
    return this.interceptor.createTerminalWithCLI(name);
  }

  /**
   * Shows global installation instructions for the current platform
   */
  async showGlobalInstallInstructions(): Promise<void> {
    if (!this.platform || !this.binaryInfo) {
      await this.notificationManager.showErrorNotification(
        'CLI integration not properly initialized. Cannot show installation instructions.'
      );
      return;
    }
    
    await this.notificationManager.showGlobalInstallInstructions(
      this.platform,
      this.binaryInfo.binDir
    );
  }

  /**
   * Shows CLI information notification
   */
  async showCLIInfo(): Promise<void> {
    if (!this.binaryInfo) {
      await this.notificationManager.showErrorNotification(
        'CLI integration not properly initialized. Cannot show CLI info.'
      );
      return;
    }
    
    await this.notificationManager.showCLIInfoNotification(this.binaryInfo.binDir);
  }

  /**
   * Runs diagnostic checks and shows results
   */
  async runDiagnostics(): Promise<void> {
    if (!this.interceptor) {
      this.logger.error('CLI integration not activated. Cannot run diagnostics.');
      this.logger.show();
      return;
    }
    
    const diagnostics = await this.interceptor.runDiagnostics();
    
    // Log all diagnostics
    for (const line of diagnostics) {
      this.logger.info(line);
    }
    
    // Show output channel
    this.logger.show();
    
    // Also show a notification
    vscode.window.showInformationMessage(
      'CLI diagnostics complete. Check the "OpenDaemon CLI" output channel for details.',
      'Show Output'
    ).then(selection => {
      if (selection === 'Show Output') {
        this.logger.show();
      }
    });
  }

  /**
   * Deactivates the CLI integration and cleans up resources
   */
  async deactivate(): Promise<void> {
    if (this.interceptor) {
      await this.interceptor.stop();
      this.interceptor = null;
    }
    
    console.log(`[OpenDaemon CLI] Integration deactivated`);
  }
}
