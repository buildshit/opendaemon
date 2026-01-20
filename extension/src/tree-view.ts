import * as vscode from 'vscode';

export enum ServiceStatus {
    NotStarted = 'NotStarted',
    Starting = 'Starting',
    Running = 'Running',
    Stopped = 'Stopped',
    Failed = 'Failed'
}

export interface ServiceInfo {
    name: string;
    status: ServiceStatus;
    exitCode?: number;
}

export class ServiceTreeItem extends vscode.TreeItem {
    constructor(
        public readonly serviceName: string,
        public readonly status: ServiceStatus,
        public readonly exitCode?: number
    ) {
        super(serviceName, vscode.TreeItemCollapsibleState.None);
        
        this.tooltip = this.getTooltip();
        this.description = this.getDescription();
        this.iconPath = this.getIconForStatus();
        this.contextValue = 'service';
    }

    private getTooltip(): string {
        let tooltip = `${this.serviceName}: ${this.status}`;
        if (this.exitCode !== undefined) {
            tooltip += ` (exit code: ${this.exitCode})`;
        }
        return tooltip;
    }

    private getDescription(): string {
        if (this.status === ServiceStatus.Failed && this.exitCode !== undefined) {
            return `Failed (${this.exitCode})`;
        }
        return this.status;
    }

    private getIconForStatus(): vscode.ThemeIcon {
        switch (this.status) {
            case ServiceStatus.Running:
                return new vscode.ThemeIcon(
                    'pass',
                    new vscode.ThemeColor('testing.iconPassed')
                );
            case ServiceStatus.Starting:
                return new vscode.ThemeIcon('sync~spin');
            case ServiceStatus.Failed:
                return new vscode.ThemeIcon(
                    'error',
                    new vscode.ThemeColor('testing.iconFailed')
                );
            case ServiceStatus.Stopped:
                return new vscode.ThemeIcon('circle-outline');
            case ServiceStatus.NotStarted:
            default:
                return new vscode.ThemeIcon('circle-outline');
        }
    }
}

export class ServiceTreeDataProvider implements vscode.TreeDataProvider<ServiceTreeItem> {
    private _onDidChangeTreeData = new vscode.EventEmitter<ServiceTreeItem | undefined | null>();
    readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

    private services = new Map<string, ServiceInfo>();

    getTreeItem(element: ServiceTreeItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: ServiceTreeItem): ServiceTreeItem[] {
        if (element) {
            // No children for service items
            return [];
        }

        // Return all services as root items
        return Array.from(this.services.values()).map(
            service => new ServiceTreeItem(service.name, service.status, service.exitCode)
        );
    }

    /**
     * Update the list of services
     */
    updateServices(services: ServiceInfo[]): void {
        this.services.clear();
        for (const service of services) {
            this.services.set(service.name, service);
        }
        this.refresh();
    }

    /**
     * Update a single service status
     */
    updateServiceStatus(name: string, status: ServiceStatus, exitCode?: number): void {
        const service = this.services.get(name);
        if (service) {
            service.status = status;
            service.exitCode = exitCode;
            this.refresh();
        }
    }

    /**
     * Add a new service
     */
    addService(name: string, status: ServiceStatus = ServiceStatus.NotStarted): void {
        this.services.set(name, { name, status });
        this.refresh();
    }

    /**
     * Remove a service
     */
    removeService(name: string): void {
        this.services.delete(name);
        this.refresh();
    }

    /**
     * Get a service by name
     */
    getService(name: string): ServiceInfo | undefined {
        return this.services.get(name);
    }

    /**
     * Get all services
     */
    getAllServices(): ServiceInfo[] {
        return Array.from(this.services.values());
    }

    /**
     * Clear all services
     */
    clear(): void {
        this.services.clear();
        this.refresh();
    }

    /**
     * Refresh the tree view
     */
    refresh(): void {
        this._onDidChangeTreeData.fire(undefined);
    }
}
