/**
 * Notification manager for CLI integration
 * Handles user notifications about CLI availability and instructions
 */

import * as vscode from 'vscode';
import { PlatformInfo } from './platform-detector';

const FIRST_TIME_NOTIFICATION_KEY = 'opendaemon.cliIntegration.firstTimeNotificationShown';

/**
 * Manages user notifications for CLI integration
 */
export class NotificationManager {
  constructor(private context: vscode.ExtensionContext) {}

  /**
   * Shows first-time notification about CLI availability
   * Only displays once per installation
   * @param binDir - Path to the bin directory containing the CLI
   */
  async showFirstTimeNotification(binDir: string): Promise<void> {
    // Check if notification was already shown
    if (this.hasShownFirstTime()) {
      return;
    }

    await this.showCLIInfoNotification(binDir);

    // Mark notification as shown
    this.markFirstTimeShown();
  }

  /**
   * Shows CLI information notification (always displays, regardless of previous state)
   * @param binDir - Path to the bin directory containing the CLI
   */
  async showCLIInfoNotification(binDir: string): Promise<void> {
    const message = "OpenDaemon CLI is now available in VS Code terminals! Type 'dmn --help' to get started.";
    const openTerminal = 'Open Terminal';
    const viewDocs = 'View Documentation';
    const dontShowAgain = "Don't Show Again";

    const selection = await vscode.window.showInformationMessage(
      message,
      openTerminal,
      viewDocs,
      dontShowAgain
    );

    // Handle user selection
    if (selection === openTerminal) {
      // Create a new terminal
      const terminal = vscode.window.createTerminal('OpenDaemon');
      terminal.show();
    } else if (selection === viewDocs) {
      // Open documentation
      const docsUri = vscode.Uri.parse('https://github.com/yourusername/opendaemon#cli-usage');
      vscode.env.openExternal(docsUri);
    }
  }

  /**
   * Shows error notification with troubleshooting information
   * @param error - Error message to display
   */
  async showErrorNotification(error: string): Promise<void> {
    const viewDocs = 'View Documentation';
    const selection = await vscode.window.showErrorMessage(
      `OpenDaemon CLI Error: ${error}`,
      viewDocs
    );

    if (selection === viewDocs) {
      const docsUri = vscode.Uri.parse('https://github.com/yourusername/opendaemon#troubleshooting');
      vscode.env.openExternal(docsUri);
    }
  }

  /**
   * Shows platform-specific instructions for global CLI installation
   * @param platform - Current platform information
   * @param binDir - Path to the bin directory containing the CLI
   */
  async showGlobalInstallInstructions(platform: PlatformInfo, binDir: string): Promise<void> {
    let message: string;
    let instructions: string;

    if (platform.os === 'win32') {
      // Windows instructions
      message = 'To use the OpenDaemon CLI globally in any terminal:';
      instructions = 
        '1. Open System Properties (Win + Pause/Break)\n' +
        '2. Click "Advanced system settings"\n' +
        '3. Click "Environment Variables"\n' +
        '4. Under "User variables", select "Path" and click "Edit"\n' +
        '5. Click "New" and add the following path:\n' +
        `   ${binDir}\n` +
        '6. Click "OK" to save and restart your terminals';
    } else {
      // Unix-like systems (macOS, Linux)
      message = 'To use the OpenDaemon CLI globally in any terminal:';
      instructions =
        'Option 1: Copy to /usr/local/bin (requires sudo):\n' +
        `   sudo cp ${binDir}/dmn-${platform.os}-${platform.arch} /usr/local/bin/dmn\n` +
        `   sudo chmod +x /usr/local/bin/dmn\n\n` +
        'Option 2: Add to your PATH in shell profile (~/.bashrc, ~/.zshrc, etc.):\n' +
        `   export PATH="${binDir}:$PATH"`;
    }

    const copyPath = 'Copy Path';
    const close = 'Close';

    const selection = await vscode.window.showInformationMessage(
      `${message}\n\n${instructions}`,
      { modal: true },
      copyPath,
      close
    );

    if (selection === copyPath) {
      await vscode.env.clipboard.writeText(binDir);
      vscode.window.showInformationMessage('Path copied to clipboard!');
    }
  }

  /**
   * Checks if first-time notification was already shown
   * @returns true if notification was shown before
   */
  private hasShownFirstTime(): boolean {
    return this.context.globalState.get<boolean>(FIRST_TIME_NOTIFICATION_KEY, false);
  }

  /**
   * Marks first-time notification as shown
   */
  private markFirstTimeShown(): void {
    this.context.globalState.update(FIRST_TIME_NOTIFICATION_KEY, true);
  }
}
