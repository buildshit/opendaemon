import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

interface ServiceSuggestion {
    name: string;
    command: string;
    source: string;
}

export class ConfigWizard {
    /**
     * Detect if dmn.json is missing and offer to create it
     */
    static async detectAndOfferCreation(): Promise<string | null> {
        const workspaceFolders = vscode.workspace.workspaceFolders;
        
        if (!workspaceFolders || workspaceFolders.length === 0) {
            return null;
        }

        const rootPath = workspaceFolders[0].uri.fsPath;
        const dmnPath = path.join(rootPath, 'dmn.json');

        // Check if dmn.json already exists
        try {
            await fs.promises.access(dmnPath, fs.constants.F_OK);
            return dmnPath; // Already exists
        } catch {
            // Doesn't exist, offer to create
            const choice = await vscode.window.showInformationMessage(
                'No dmn.json configuration found. Would you like to create one?',
                'Yes',
                'No'
            );

            if (choice === 'Yes') {
                return await this.createConfig(rootPath);
            }
        }

        return null;
    }

    /**
     * Create a new dmn.json configuration
     */
    static async createConfig(rootPath: string): Promise<string | null> {
        try {
            // Scan for services
            const suggestions = await this.scanForServices(rootPath);

            // Generate config
            const config = this.generateConfig(suggestions);

            // Write to file
            const dmnPath = path.join(rootPath, 'dmn.json');
            await fs.promises.writeFile(dmnPath, JSON.stringify(config, null, 2));

            vscode.window.showInformationMessage(
                `Created dmn.json with ${suggestions.length} suggested service(s)`
            );

            // Open the file for editing
            const doc = await vscode.workspace.openTextDocument(dmnPath);
            await vscode.window.showTextDocument(doc);

            return dmnPath;
        } catch (err) {
            vscode.window.showErrorMessage(
                `Failed to create dmn.json: ${err instanceof Error ? err.message : String(err)}`
            );
            return null;
        }
    }

    /**
     * Scan workspace for potential services
     */
    private static async scanForServices(rootPath: string): Promise<ServiceSuggestion[]> {
        const suggestions: ServiceSuggestion[] = [];

        // Check for package.json
        const packageJsonPath = path.join(rootPath, 'package.json');
        try {
            const packageJson = JSON.parse(
                await fs.promises.readFile(packageJsonPath, 'utf-8')
            );

            if (packageJson.scripts) {
                // Look for common dev scripts
                const devScripts = ['dev', 'start', 'serve', 'watch'];
                
                for (const scriptName of devScripts) {
                    if (packageJson.scripts[scriptName]) {
                        suggestions.push({
                            name: scriptName === 'dev' ? 'app' : scriptName,
                            command: `npm run ${scriptName}`,
                            source: 'package.json'
                        });
                        break; // Only add one npm script
                    }
                }
            }
        } catch {
            // package.json not found or invalid
        }

        // Check for docker-compose.yml
        const dockerComposePaths = [
            path.join(rootPath, 'docker-compose.yml'),
            path.join(rootPath, 'docker-compose.yaml')
        ];

        for (const dockerPath of dockerComposePaths) {
            try {
                await fs.promises.access(dockerPath, fs.constants.F_OK);
                suggestions.push({
                    name: 'docker',
                    command: 'docker-compose up',
                    source: 'docker-compose.yml'
                });
                break;
            } catch {
                // File not found
            }
        }

        // If no suggestions, provide a template
        if (suggestions.length === 0) {
            suggestions.push({
                name: 'example',
                command: 'echo "Replace with your command"',
                source: 'template'
            });
        }

        return suggestions;
    }

    /**
     * Generate dmn.json config from suggestions
     */
    private static generateConfig(suggestions: ServiceSuggestion[]): unknown {
        const services: Record<string, unknown> = {};

        for (const suggestion of suggestions) {
            services[suggestion.name] = {
                command: suggestion.command,
                depends_on: [],
                ready_when: null
            };
        }

        return {
            version: '1.0',
            services
        };
    }
}
