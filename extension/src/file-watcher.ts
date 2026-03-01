import * as vscode from 'vscode';
import * as path from 'path';

export class DmnFileWatcher {
    private watcher: vscode.FileSystemWatcher | null = null;
    private dmnConfigPath: string | null = null;
    private reloadDebounceTimer: NodeJS.Timeout | null = null;
    private isReloading = false;
    private pendingReload = false;
    private readonly reloadDebounceMs = 250;

    constructor(
        private readonly onConfigChanged: () => Promise<void>,
        private readonly onConfigDeleted: () => Promise<void>
    ) {}

    /**
     * Start watching dmn.json file
     */
    start(configPath: string): void {
        // Ensure we do not accumulate multiple watchers across re-initialization.
        this.stop();
        this.dmnConfigPath = configPath;

        // Create file watcher for dmn.json
        const pattern = new vscode.RelativePattern(
            path.dirname(configPath),
            'dmn.json'
        );

        this.watcher = vscode.workspace.createFileSystemWatcher(pattern);

        // Watch for changes
        this.watcher.onDidChange((uri) => {
            console.log('dmn.json changed:', uri.fsPath);
            this.scheduleConfigReload('changed');
        });

        // Watch for deletion
        this.watcher.onDidDelete(async (uri) => {
            console.log('dmn.json deleted:', uri.fsPath);
            if (this.reloadDebounceTimer) {
                clearTimeout(this.reloadDebounceTimer);
                this.reloadDebounceTimer = null;
            }
            this.pendingReload = false;
            vscode.window.showWarningMessage('dmn.json was deleted. Services stopped.');
            await this.onConfigDeleted();
        });

        // Watch for creation (in case it was deleted and recreated)
        this.watcher.onDidCreate((uri) => {
            console.log('dmn.json created:', uri.fsPath);
            this.scheduleConfigReload('created');
        });
    }

    /**
     * Stop watching
     */
    stop(): void {
        if (this.watcher) {
            this.watcher.dispose();
            this.watcher = null;
        }
        if (this.reloadDebounceTimer) {
            clearTimeout(this.reloadDebounceTimer);
            this.reloadDebounceTimer = null;
        }
        this.isReloading = false;
        this.pendingReload = false;
        this.dmnConfigPath = null;
    }

    /**
     * Get the current config path
     */
    getConfigPath(): string | null {
        return this.dmnConfigPath;
    }

    /**
     * Debounce config reloads so a single file save does not trigger
     * overlapping daemon reinitializations.
     */
    private scheduleConfigReload(source: 'changed' | 'created'): void {
        if (this.reloadDebounceTimer) {
            clearTimeout(this.reloadDebounceTimer);
        }

        this.reloadDebounceTimer = setTimeout(() => {
            this.reloadDebounceTimer = null;
            void this.runConfigReload(source);
        }, this.reloadDebounceMs);
    }

    private async runConfigReload(source: 'changed' | 'created'): Promise<void> {
        if (this.isReloading) {
            this.pendingReload = true;
            return;
        }

        this.isReloading = true;
        try {
            if (source === 'created') {
                vscode.window.showInformationMessage('dmn.json created. Loading configuration...');
            } else {
                vscode.window.showInformationMessage('dmn.json changed. Reloading configuration...');
            }
            await this.onConfigChanged();
        } finally {
            this.isReloading = false;
            if (this.pendingReload) {
                this.pendingReload = false;
                // Coalesce overlapping changes into one follow-up reload.
                void this.runConfigReload('changed');
            }
        }
    }
}
