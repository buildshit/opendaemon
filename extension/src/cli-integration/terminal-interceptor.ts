import * as vscode from 'vscode';
import * as os from 'os';
import * as fs from 'fs';
import { OpenDaemonTerminalProfileProvider } from './terminal-profile-provider';
import { getCLILogger } from './cli-logger';

/**
 * TerminalInterceptor manages PATH injection for VS Code terminals.
 * 
 * It uses two complementary approaches:
 * 1. **Primary**: Modifies `terminal.integrated.env.*` settings to inject PATH into ALL new terminals
 * 2. **Secondary**: Registers a terminal profile provider as a fallback option
 * 
 * The env.* approach is the official VSCode way to modify terminal environment variables
 * and works automatically for all terminals without requiring users to select a specific profile.
 */
export class TerminalInterceptor {
    private binDir: string;
    private disposable: vscode.Disposable | null = null;
    private profileDisposable: vscode.Disposable | undefined;
    private previousEnvConfig: Record<string, string> | undefined;
    private envConfigKey: string;
    private logger = getCLILogger();

    /**
     * Creates a new TerminalInterceptor instance.
     * @param binDir The directory containing the CLI binary
     */
    constructor(binDir: string) {
        this.binDir = binDir;
        // Determine platform-specific env config key
        this.envConfigKey = process.platform === 'win32' ? 'env.windows' :
                           process.platform === 'darwin' ? 'env.osx' :
                           'env.linux';
        
        this.logger.info('TerminalInterceptor created');
        this.logger.info(`Bin directory: ${binDir}`);
        this.logger.info(`Platform: ${process.platform}`);
        this.logger.info(`Env config key: terminal.integrated.${this.envConfigKey}`);
    }

    /**
     * Starts PATH injection for all terminals using terminal.integrated.env.* settings.
     * Also registers a terminal profile provider as a secondary option.
     */
    async start(): Promise<void> {
        this.logger.info('=== Starting Terminal Interceptor ===');
        this.logger.logSystemInfo();
        this.logger.logWorkspaceInfo();
        
        // Log current terminal settings before changes
        this.logger.info('Current terminal settings (before changes):');
        await this.logger.logTerminalSettings();
        
        // Verify CLI binary exists before setting up PATH injection
        this.logger.info(`Looking for CLI binary in: ${this.binDir}`);
        const binaryPath = this.findBinaryInDir(this.binDir);
        
        if (!binaryPath) {
            this.logger.error('CLI binary not found in bin directory!');
            this.logger.error(`Expected directory: ${this.binDir}`);
            
            // List what's actually in the bin dir (or parent)
            try {
                if (fs.existsSync(this.binDir)) {
                    const files = fs.readdirSync(this.binDir);
                    this.logger.info(`Files in bin directory: ${files.join(', ') || '(empty)'}`);
                } else {
                    this.logger.error(`Bin directory does not exist: ${this.binDir}`);
                    
                    // Check parent directory
                    const parentDir = this.binDir.replace(/[\\\/]bin$/, '');
                    if (fs.existsSync(parentDir)) {
                        const parentFiles = fs.readdirSync(parentDir);
                        this.logger.info(`Files in extension directory: ${parentFiles.join(', ')}`);
                    }
                }
            } catch (err) {
                this.logger.error('Error listing directory contents:', err);
            }
            
            this.logger.warn('Skipping PATH injection - binary not found');
            return;
        }
        
        this.logger.info(`Found CLI binary: ${binaryPath}`);

        try {
            // Primary approach: Inject PATH via terminal.integrated.env.* settings
            // This affects ALL new terminals automatically
            this.logger.info('Injecting PATH via terminal.integrated.env.* settings...');
            await this.injectPathViaSettings();
            
            // Log terminal settings after changes
            this.logger.info('Terminal settings (after changes):');
            await this.logger.logTerminalSettings();
            
            // Secondary approach: Register terminal profile provider as a fallback
            // This creates a selectable "OpenDaemon CLI" profile in the terminal dropdown
            this.logger.info('Registering terminal profile provider...');
            const provider = new OpenDaemonTerminalProfileProvider(this.binDir);
            this.profileDisposable = vscode.window.registerTerminalProfileProvider(
                'opendaemon.terminal',
                provider
            );
            this.logger.info('Terminal profile provider registered successfully');
            
            this.logger.info('=== PATH injection configured successfully ===');
            this.logger.info(`Binary directory added to PATH: ${this.binDir}`);
            this.logger.info('New terminals should now have access to the "dmn" command');
            this.logger.info('NOTE: You must open a NEW terminal for changes to take effect');
            
            // Show the output channel so user can see the logs
            this.logger.show();
            
        } catch (error) {
            const errorMsg = error instanceof Error ? error.message : String(error);
            this.logger.error('Failed to configure PATH injection:', errorMsg);
            if (error instanceof Error && error.stack) {
                this.logger.error('Stack trace:', error.stack);
            }
            // Fall back to manual terminal creation - don't throw
        }
    }

    /**
     * Injects the bin directory into PATH using terminal.integrated.env.* settings.
     * This is the official VSCode way to modify terminal environment variables.
     * 
     * NOTE: We use the FULL PATH value instead of ${env:PATH} variable substitution
     * because VS Code's variable substitution doesn't work reliably on all platforms.
     * 
     * We try BOTH user-level AND workspace-level settings for maximum compatibility,
     * as Cursor and some VS Code configurations may only read from one or the other.
     */
    private async injectPathViaSettings(): Promise<void> {
        this.logger.info('--- injectPathViaSettings() ---');
        
        const config = vscode.workspace.getConfiguration('terminal.integrated');
        this.logger.info(`Getting configuration for: terminal.integrated.${this.envConfigKey}`);
        
        // Get current env settings for this platform (check both workspace and user level)
        const currentEnv = config.get<Record<string, string>>(this.envConfigKey) || {};
        this.logger.info('Current env settings:', currentEnv);
        
        // Store previous env config for restoration
        this.previousEnvConfig = { ...currentEnv };
        this.logger.info('Stored previous env config for restoration');
        
        const pathSeparator = this.getPathSeparator();
        this.logger.info(`PATH separator: "${pathSeparator}"`);
        
        // Get the current system PATH from process.env
        const systemPath = process.env.PATH || process.env.Path || '';
        this.logger.info(`System PATH length: ${systemPath.length} characters`);
        
        // Check if we've already injected our path (avoid duplicates)
        // Check both the settings and the system PATH
        const settingsPath = currentEnv['Path'] || currentEnv['PATH'] || '';
        
        // Check if settings use the OLD broken variable substitution format
        const usesVariableSubstitution = settingsPath.includes('${env:');
        if (usesVariableSubstitution) {
            this.logger.warn('=== FIXING BROKEN SETTINGS ===');
            this.logger.warn('Settings use ${env:} variable substitution which does NOT work in VS Code');
            this.logger.warn('This is why dmn was not found - the PATH was literally "${env:Path}" instead of the actual PATH');
            this.logger.warn('Replacing with full PATH value...');
            // Continue to fix the settings
        } else if (settingsPath.includes(this.binDir)) {
            // Settings have the bin dir and DON'T use variable substitution - check if it's working
            this.logger.info('PATH already contains bin directory with full path');
            this.logger.info('Settings appear to be correct');
            return;
        } else if (systemPath.includes(this.binDir)) {
            this.logger.info('Binary directory is in system PATH - no injection needed');
            return;
        }
        
        // Create new env config with PATH injection
        // IMPORTANT: Use the FULL PATH value, NOT variable substitution
        // Variable substitution like ${env:PATH} doesn't work reliably in VS Code
        const newEnv = { ...currentEnv };
        const newPath = `${this.binDir}${pathSeparator}${systemPath}`;
        
        this.logger.info(`Creating new PATH value...`);
        this.logger.info(`  Bin directory: ${this.binDir}`);
        this.logger.info(`  New PATH length: ${newPath.length} characters`);
        
        // On Windows, set both PATH and Path for maximum compatibility
        if (process.platform === 'win32') {
            newEnv['Path'] = newPath;
            newEnv['PATH'] = newPath;
            this.logger.info('Set both Path and PATH for Windows compatibility');
        } else {
            newEnv['PATH'] = newPath;
        }
        
        // IMPORTANT: Try BOTH user-level AND workspace-level settings
        // Cursor and some VS Code configurations may only read from one scope
        // User-level settings are more reliable as they apply globally
        
        // First, try user-level settings (Global) - more reliable for Cursor
        this.logger.info(`Updating USER settings: terminal.integrated.${this.envConfigKey}`);
        try {
            await config.update(
                this.envConfigKey,
                newEnv,
                vscode.ConfigurationTarget.Global
            );
            this.logger.info('User (global) settings updated successfully');
        } catch (updateError) {
            this.logger.warn('Failed to update user settings (this is OK, will try workspace):', updateError);
        }
        
        // Also update workspace settings as backup
        this.logger.info(`Updating WORKSPACE settings: terminal.integrated.${this.envConfigKey}`);
        try {
            await config.update(
                this.envConfigKey,
                newEnv,
                vscode.ConfigurationTarget.Workspace
            );
            this.logger.info('Workspace settings updated successfully');
        } catch (updateError) {
            this.logger.error('Failed to update workspace settings:', updateError);
            // Don't throw - user settings might have worked
        }
        
        // Verify the update was applied
        const verifyConfig = vscode.workspace.getConfiguration('terminal.integrated');
        const verifyEnv = verifyConfig.get<Record<string, string>>(this.envConfigKey);
        this.logger.info('Verification - settings updated:', verifyEnv ? 'YES' : 'NO');
        
        // Check if our bin dir is now in the setting
        if (verifyEnv) {
            const verifyPath = verifyEnv['Path'] || verifyEnv['PATH'] || '';
            if (verifyPath.startsWith(this.binDir)) {
                this.logger.info('Verification PASSED - bin directory is at start of PATH');
            } else {
                this.logger.warn('Verification WARNING - bin directory may not be correctly set');
            }
        }
        
        this.logger.info(`PATH injection complete via terminal.integrated.${this.envConfigKey}`);
    }

    /**
     * Removes the PATH injection from terminal.integrated.env.* settings.
     */
    private async removePathFromSettings(): Promise<void> {
        if (this.previousEnvConfig === undefined) {
            return;
        }

        try {
            const config = vscode.workspace.getConfiguration('terminal.integrated');
            
            // Restore previous env config
            // If it was empty, set to undefined to remove the setting entirely
            const valueToSet = Object.keys(this.previousEnvConfig).length > 0 
                ? this.previousEnvConfig 
                : undefined;
            
            // Remove from both user and workspace settings
            try {
                await config.update(
                    this.envConfigKey,
                    valueToSet,
                    vscode.ConfigurationTarget.Global
                );
            } catch (e) {
                // Ignore errors from user settings cleanup
            }
            
            await config.update(
                this.envConfigKey,
                valueToSet,
                vscode.ConfigurationTarget.Workspace
            );
            
            console.log('[OpenDaemon] Restored previous terminal environment settings');
        } catch (error) {
            const errorMsg = error instanceof Error ? error.message : String(error);
            console.error('[OpenDaemon] Failed to restore terminal environment:', errorMsg);
            // Log but don't throw during cleanup
        }
    }
    
    /**
     * Runs diagnostic checks and returns information about the CLI setup.
     * This helps users troubleshoot PATH issues.
     */
    async runDiagnostics(): Promise<string[]> {
        const diagnostics: string[] = [];
        
        diagnostics.push('=== OpenDaemon CLI Diagnostics ===');
        diagnostics.push('');
        
        // Check 1: Binary directory exists
        const binDirExists = fs.existsSync(this.binDir);
        diagnostics.push(`1. Binary directory exists: ${binDirExists ? 'YES' : 'NO'}`);
        diagnostics.push(`   Path: ${this.binDir}`);
        
        // Check 2: List files in bin directory
        if (binDirExists) {
            try {
                const files = fs.readdirSync(this.binDir);
                diagnostics.push(`   Files: ${files.join(', ')}`);
                
                // Check for dmn.exe specifically (Windows)
                if (process.platform === 'win32') {
                    const hasDmnExe = files.includes('dmn.exe');
                    const hasDmnCmd = files.includes('dmn.cmd');
                    diagnostics.push(`   dmn.exe present: ${hasDmnExe ? 'YES' : 'NO'}`);
                    diagnostics.push(`   dmn.cmd present: ${hasDmnCmd ? 'YES' : 'NO'}`);
                    
                    if (!hasDmnExe) {
                        diagnostics.push('   WARNING: dmn.exe is missing - this is required for Windows');
                    }
                }
            } catch (e) {
                diagnostics.push(`   Error listing directory: ${e}`);
            }
        }
        
        diagnostics.push('');
        
        // Check 3: Terminal settings
        const config = vscode.workspace.getConfiguration('terminal.integrated');
        const envSettings = config.get<Record<string, string>>(this.envConfigKey) || {};
        diagnostics.push(`2. Terminal env settings (${this.envConfigKey}):`);
        
        const settingsPath = envSettings['Path'] || envSettings['PATH'];
        if (settingsPath) {
            const containsBinDir = settingsPath.includes(this.binDir);
            diagnostics.push(`   PATH contains bin directory: ${containsBinDir ? 'YES' : 'NO'}`);
            
            if (containsBinDir) {
                const startsWithBinDir = settingsPath.startsWith(this.binDir);
                diagnostics.push(`   Bin directory is first in PATH: ${startsWithBinDir ? 'YES' : 'NO'}`);
            }
        } else {
            diagnostics.push('   PATH not set in terminal settings');
        }
        
        diagnostics.push('');
        
        // Check 4: Instructions
        diagnostics.push('3. Troubleshooting steps:');
        diagnostics.push('   a. Close ALL open terminals');
        diagnostics.push('   b. Open a NEW terminal (Ctrl+Shift+`)');
        diagnostics.push('   c. Run: dmn --version');
        diagnostics.push('');
        diagnostics.push('   If still not working, run this in PowerShell:');
        diagnostics.push('   $env:PATH -split ";" | Select-String "opendaemon"');
        diagnostics.push('');
        diagnostics.push('   Or use the full path:');
        diagnostics.push(`   & "${this.binDir}\\dmn.exe" --version`);
        
        return diagnostics;
    }

    /**
     * Finds the CLI binary in the bin directory
     * @param binDir The bin directory to search
     * @returns The full path to the binary if found, undefined otherwise
     */
    private findBinaryInDir(binDir: string): string | undefined {
        try {
            const files = fs.readdirSync(binDir);
            const binaryFile = files.find(file => file.startsWith('dmn-') || file === 'dmn');
            return binaryFile ? `${binDir}/${binaryFile}` : undefined;
        } catch (error) {
            return undefined;
        }
    }

    /**
     * Stops PATH injection and cleans up resources.
     */
    async stop(): Promise<void> {
        // Dispose profile registration
        if (this.profileDisposable) {
            try {
                this.profileDisposable.dispose();
                this.profileDisposable = undefined;
                console.log('[OpenDaemon] Terminal profile provider disposed');
            } catch (error) {
                const errorMsg = error instanceof Error ? error.message : String(error);
                console.error('[OpenDaemon] Failed to dispose profile registration:', errorMsg);
                this.profileDisposable = undefined;
            }
        }

        // Remove PATH injection from settings
        await this.removePathFromSettings();

        // Dispose other resources
        if (this.disposable) {
            try {
                this.disposable.dispose();
                this.disposable = null;
            } catch (error) {
                const errorMsg = error instanceof Error ? error.message : String(error);
                console.error('[OpenDaemon] Failed to dispose resources:', errorMsg);
                this.disposable = null;
            }
        }
    }

    /**
     * Injects the bin directory into the PATH environment variable.
     * @param existingEnv The existing environment variables
     * @returns A new environment object with the injected PATH
     */
    private injectPath(existingEnv: { [key: string]: string | undefined }): { [key: string]: string | undefined } {
        const newEnv = { ...existingEnv };
        const separator = this.getPathSeparator();
        const existingPath = existingEnv.PATH || existingEnv.Path || '';

        if (existingPath) {
            newEnv.PATH = `${this.binDir}${separator}${existingPath}`;
        } else {
            newEnv.PATH = this.binDir;
        }

        // On Windows, also set Path for compatibility
        if (os.platform() === 'win32') {
            newEnv.Path = newEnv.PATH;
        }

        return newEnv;
    }

    /**
     * Creates a terminal with the CLI binary directory injected into PATH.
     * @param name Optional name for the terminal
     * @param cwd Optional working directory for the terminal
     * @returns The created terminal instance
     */
    createTerminalWithCLI(name?: string, cwd?: string): vscode.Terminal {
        const env = this.injectPath(process.env);
        
        return vscode.window.createTerminal({
            name: name || 'OpenDaemon Terminal',
            cwd: cwd,
            env: env as { [key: string]: string }
        });
    }

    /**
     * Gets the PATH separator for the current platform.
     * @returns ';' for Windows, ':' for Unix-like systems
     */
    private getPathSeparator(): string {
        return os.platform() === 'win32' ? ';' : ':';
    }

    /**
     * Gets the bin directory path.
     * @returns The bin directory path
     */
    getBinDir(): string {
        return this.binDir;
    }
}
