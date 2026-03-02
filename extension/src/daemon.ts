import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as vscode from 'vscode';
import { resolveDmnBinaryPath } from './binary-path';

export class DaemonManager {
    private process: ChildProcess | null = null;
    private restartAttempts = 0;
    private readonly maxRestartAttempts = 3;
    private readonly restartDelay = 2000;
    private isShuttingDown = false;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly onStdout: (data: string) => void,
        private readonly onStderr: (data: string) => void
    ) {}

    /**
     * Start the daemon process
     */
    async start(configPath: string): Promise<void> {
        if (this.process) {
            console.log('Daemon already running');
            return;
        }

        const binaryPath = this.getBinaryPath();
        
        console.log(`Starting daemon: ${binaryPath} daemon --config ${configPath}`);
        
        this.process = spawn(binaryPath, ['daemon', '--config', configPath], {
            cwd: path.dirname(configPath),
            stdio: ['pipe', 'pipe', 'pipe']
        });

        this.setupProcessHandlers();
    }

    /**
     * Stop the daemon process
     */
    async stop(): Promise<void> {
        this.isShuttingDown = true;
        
        if (!this.process) {
            return;
        }

        return new Promise((resolve) => {
            if (!this.process) {
                resolve();
                return;
            }

            this.process.once('exit', () => {
                this.process = null;
                resolve();
            });

            // Send SIGTERM
            this.process.kill('SIGTERM');

            // Force kill after 5 seconds
            setTimeout(() => {
                if (this.process) {
                    this.process.kill('SIGKILL');
                }
            }, 5000);
        });
    }

    /**
     * Send data to daemon stdin
     */
    write(data: string): void {
        if (this.process && this.process.stdin) {
            this.process.stdin.write(data);
        }
    }

    /**
     * Check if daemon is running
     */
    isRunning(): boolean {
        return this.process !== null && !this.process.killed;
    }

    /**
     * Get the path to the dmn binary
     */
    private getBinaryPath(): string {
        return resolveDmnBinaryPath(this.context);
    }

    /**
     * Set up process event handlers
     */
    private setupProcessHandlers(): void {
        if (!this.process) {
            return;
        }

        // Handle stdout
        this.process.stdout?.on('data', (data: Buffer) => {
            this.onStdout(data.toString());
        });

        // Handle stderr
        this.process.stderr?.on('data', (data: Buffer) => {
            this.onStderr(data.toString());
        });

        // Handle process exit
        this.process.on('exit', (code, signal) => {
            console.log(`Daemon exited with code ${code}, signal ${signal}`);
            this.process = null;

            if (!this.isShuttingDown) {
                this.handleCrash();
            }
        });

        // Handle process errors
        this.process.on('error', (err) => {
            console.error('Daemon process error:', err);
            vscode.window.showErrorMessage(`OpenDaemon daemon error: ${err.message}`);
        });
    }

    /**
     * Handle daemon crash and attempt restart
     */
    private async handleCrash(): Promise<void> {
        if (this.restartAttempts >= this.maxRestartAttempts) {
            vscode.window.showErrorMessage(
                'OpenDaemon daemon crashed multiple times. Please check the logs and restart manually.'
            );
            return;
        }

        this.restartAttempts++;
        
        vscode.window.showWarningMessage(
            `OpenDaemon daemon crashed. Attempting restart (${this.restartAttempts}/${this.maxRestartAttempts})...`
        );

        // Wait before restarting
        await new Promise(resolve => setTimeout(resolve, this.restartDelay));

        // Note: We would need the config path to restart
        // This should be stored when start() is called
        console.log('Daemon restart not fully implemented - needs config path storage');
    }

    /**
     * Reset restart attempts counter
     */
    resetRestartAttempts(): void {
        this.restartAttempts = 0;
    }
}
