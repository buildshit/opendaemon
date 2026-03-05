import { spawn, ChildProcess, execFile } from 'child_process';
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
        
        const daemonProcess = this.process;
        if (!daemonProcess) {
            return;
        }

        return new Promise((resolve) => {
            let termTimer: NodeJS.Timeout | undefined;
            let killTimer: NodeJS.Timeout | undefined;
            let settled = false;

            const finalize = () => {
                if (settled) {
                    return;
                }
                settled = true;
                if (termTimer) {
                    clearTimeout(termTimer);
                }
                if (killTimer) {
                    clearTimeout(killTimer);
                }
                this.process = null;
                resolve();
            };

            daemonProcess.once('exit', finalize);

            // Prefer a graceful EOF shutdown so daemon code can stop all services.
            // If it doesn't exit in time, escalate to hard kills.
            try {
                daemonProcess.stdin?.end();
            } catch {
                // Ignore stdin closure errors; fallback timers below handle shutdown.
            }

            termTimer = setTimeout(() => {
                if (this.process === daemonProcess && daemonProcess.exitCode === null) {
                    try {
                        daemonProcess.kill('SIGTERM');
                    } catch {
                        // Ignore signal errors; SIGKILL fallback remains.
                    }
                }
            }, 3000);

            killTimer = setTimeout(() => {
                if (this.process === daemonProcess && daemonProcess.exitCode === null) {
                    if (process.platform === 'win32' && daemonProcess.pid) {
                        execFile(
                            'taskkill',
                            ['/PID', String(daemonProcess.pid), '/T', '/F'],
                            () => {
                                // Ignore taskkill errors; process may have already exited.
                            }
                        );
                    } else {
                        try {
                            daemonProcess.kill('SIGKILL');
                        } catch {
                            // Process may already be gone.
                        }
                    }
                }
            }, 8000);
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
