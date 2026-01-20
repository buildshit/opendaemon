import * as vscode from 'vscode';
import * as path from 'path';

export class DmnFileWatcher {
    private watcher: vscode.FileSystemWatcher | null = null;
    private dmnConfigPath: string | null = null;

    constructor(
        private readonly onConfigChanged: () => Promise<void>,
        private readonly onConfigDeleted: () => Promise<void>
    ) {}

    /**
     * Start watching dmn.json file
     */
    start(configPath: string): void {
        this.dmnConfigPath = configPath;

        // Create file watcher for dmn.json
        const pattern = new vscode.RelativePattern(
            path.dirname(configPath),
            'dmn.json'
        );

        this.watcher = vscode.workspace.createFileSystemWatcher(pattern);

        // Watch for changes
        this.watcher.onDidChange(async (uri) => {
            console.log('dmn.json changed:', uri.fsPath);
            vscode.window.showInformationMessage('dmn.json changed. Reloading configuration...');
            await this.onConfigChanged();
        });

        // Watch for deletion
        this.watcher.onDidDelete(async (uri) => {
            console.log('dmn.json deleted:', uri.fsPath);
            vscode.window.showWarningMessage('dmn.json was deleted. Services stopped.');
            await this.onConfigDeleted();
        });

        // Watch for creation (in case it was deleted and recreated)
        this.watcher.onDidCreate(async (uri) => {
            console.log('dmn.json created:', uri.fsPath);
            vscode.window.showInformationMessage('dmn.json created. Loading configuration...');
            await this.onConfigChanged();
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
        this.dmnConfigPath = null;
    }

    /**
     * Get the current config path
     */
    getConfigPath(): string | null {
        return this.dmnConfigPath;
    }
}
