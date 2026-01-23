import * as assert from 'assert';
import * as path from 'path';
import * as fs from 'fs';
import { ServiceTreeDataProvider, ServiceStatus } from '../../tree-view';

suite('Service Discovery Tests', () => {
    let testWorkspaceRoot: string;
    let testConfigPath: string;

    setup(() => {
        // Create a temporary test workspace
        testWorkspaceRoot = path.join(__dirname, '../../../test-workspace');
        if (!fs.existsSync(testWorkspaceRoot)) {
            fs.mkdirSync(testWorkspaceRoot, { recursive: true });
        }
        testConfigPath = path.join(testWorkspaceRoot, 'dmn.json');
    });

    teardown(() => {
        // Clean up test files
        if (fs.existsSync(testConfigPath)) {
            fs.unlinkSync(testConfigPath);
        }
    });

    test('Services are loaded from valid dmn.json config', async () => {
        // Create a test configuration with multiple services
        const testConfig = {
            version: '1.0',
            services: {
                database: {
                    command: 'cmd /c echo Database starting && timeout /t 5'
                },
                backend: {
                    command: 'cmd /c echo Backend starting && timeout /t 5',
                    depends_on: ['database']
                },
                frontend: {
                    command: 'cmd /c echo Frontend starting && timeout /t 5',
                    depends_on: ['backend']
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create tree data provider (simulating extension activation)
        const treeDataProvider = new ServiceTreeDataProvider();

        // Simulate loadServicesFromConfig function
        const configContent = await fs.promises.readFile(testConfigPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        assert.ok(config.services, 'Config should have services object');

        const serviceNames = Object.keys(config.services);
        const services = serviceNames.map(name => ({
            name,
            status: ServiceStatus.NotStarted
        }));

        // Load services into tree view BEFORE daemon starts
        treeDataProvider.updateServices(services);

        // Verify tree view is populated
        const treeItems = treeDataProvider.getChildren();
        assert.strictEqual(treeItems.length, 3, 'Tree view should have 3 services');

        // Verify all services are present
        const serviceNamesInTree = treeItems.map(item => item.serviceName).sort();
        assert.deepStrictEqual(
            serviceNamesInTree,
            ['backend', 'database', 'frontend'],
            'All services should be in tree view'
        );
    });

    test('Tree view is populated before daemon starts', async () => {
        // Create a simple test configuration
        const testConfig = {
            version: '1.0',
            services: {
                service1: {
                    command: 'cmd /c echo Service 1'
                },
                service2: {
                    command: 'cmd /c echo Service 2'
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create tree data provider
        const treeDataProvider = new ServiceTreeDataProvider();

        // Load services from config (this happens BEFORE daemon starts)
        const configContent = await fs.promises.readFile(testConfigPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        const serviceNames = Object.keys(config.services!);
        const services = serviceNames.map(name => ({
            name,
            status: ServiceStatus.NotStarted
        }));

        treeDataProvider.updateServices(services);

        // At this point, daemon has NOT started yet
        // But tree view should already be populated

        const treeItems = treeDataProvider.getChildren();
        assert.strictEqual(treeItems.length, 2, 'Tree view should be populated before daemon starts');
        assert.ok(treeItems.find(item => item.serviceName === 'service1'), 'service1 should be in tree');
        assert.ok(treeItems.find(item => item.serviceName === 'service2'), 'service2 should be in tree');
    });

    test('Services have NotStarted status initially', async () => {
        // Create a test configuration
        const testConfig = {
            version: '1.0',
            services: {
                database: {
                    command: 'cmd /c echo Database'
                },
                api: {
                    command: 'cmd /c echo API'
                },
                frontend: {
                    command: 'cmd /c echo Frontend'
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create tree data provider
        const treeDataProvider = new ServiceTreeDataProvider();

        // Load services from config
        const configContent = await fs.promises.readFile(testConfigPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        const serviceNames = Object.keys(config.services!);
        const services = serviceNames.map(name => ({
            name,
            status: ServiceStatus.NotStarted
        }));

        treeDataProvider.updateServices(services);

        // Verify all services have NotStarted status
        const treeItems = treeDataProvider.getChildren();
        
        for (const item of treeItems) {
            assert.strictEqual(
                item.status,
                ServiceStatus.NotStarted,
                `Service ${item.serviceName} should have NotStarted status`
            );
            assert.strictEqual(
                item.description,
                'NotStarted',
                `Service ${item.serviceName} should display NotStarted`
            );
            assert.strictEqual(
                item.exitCode,
                undefined,
                `Service ${item.serviceName} should not have an exit code initially`
            );
        }

        // Verify services can be retrieved individually with correct status
        const dbService = treeDataProvider.getService('database');
        assert.ok(dbService, 'Database service should be retrievable');
        assert.strictEqual(dbService.status, ServiceStatus.NotStarted);

        const apiService = treeDataProvider.getService('api');
        assert.ok(apiService, 'API service should be retrievable');
        assert.strictEqual(apiService.status, ServiceStatus.NotStarted);

        const frontendService = treeDataProvider.getService('frontend');
        assert.ok(frontendService, 'Frontend service should be retrievable');
        assert.strictEqual(frontendService.status, ServiceStatus.NotStarted);

        // Verify getAllServices returns all services with NotStarted status
        const allServices = treeDataProvider.getAllServices();
        assert.strictEqual(allServices.length, 3, 'getAllServices should return 3 services');
        
        for (const service of allServices) {
            assert.strictEqual(
                service.status,
                ServiceStatus.NotStarted,
                `Service ${service.name} should have NotStarted status in getAllServices`
            );
        }
    });

    test('Empty services object is handled correctly', async () => {
        // Create a config with empty services
        const testConfig = {
            version: '1.0',
            services: {}
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create tree data provider
        const treeDataProvider = new ServiceTreeDataProvider();

        // Load services from config
        const configContent = await fs.promises.readFile(testConfigPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        const serviceNames = Object.keys(config.services!);
        const services = serviceNames.map(name => ({
            name,
            status: ServiceStatus.NotStarted
        }));

        treeDataProvider.updateServices(services);

        // Verify tree view is empty
        const treeItems = treeDataProvider.getChildren();
        assert.strictEqual(treeItems.length, 0, 'Tree view should be empty when no services defined');
        
        const allServices = treeDataProvider.getAllServices();
        assert.strictEqual(allServices.length, 0, 'getAllServices should return empty array');
    });

    test('Config with complex service definitions is parsed correctly', async () => {
        // Create a config with various service configurations
        const testConfig = {
            version: '1.0',
            services: {
                redis: {
                    command: 'redis-server',
                    ready_when: {
                        type: 'log_contains',
                        pattern: 'Ready to accept connections',
                        timeout_seconds: 30
                    }
                },
                postgres: {
                    command: 'postgres',
                    env_file: '.env',
                    ready_when: {
                        type: 'log_contains',
                        pattern: 'database system is ready'
                    }
                },
                api: {
                    command: 'npm start',
                    depends_on: ['redis', 'postgres'],
                    ready_when: {
                        type: 'url_responds',
                        url: 'http://localhost:3000/health',
                        timeout_seconds: 60
                    }
                }
            }
        };

        fs.writeFileSync(testConfigPath, JSON.stringify(testConfig, null, 2));

        // Create tree data provider
        const treeDataProvider = new ServiceTreeDataProvider();

        // Load services from config
        const configContent = await fs.promises.readFile(testConfigPath, 'utf-8');
        const config = JSON.parse(configContent) as { services?: Record<string, unknown> };

        const serviceNames = Object.keys(config.services!);
        const services = serviceNames.map(name => ({
            name,
            status: ServiceStatus.NotStarted
        }));

        treeDataProvider.updateServices(services);

        // Verify all services are loaded regardless of complexity
        const treeItems = treeDataProvider.getChildren();
        assert.strictEqual(treeItems.length, 3, 'All services should be loaded');

        const serviceNamesInTree = treeItems.map(item => item.serviceName).sort();
        assert.deepStrictEqual(
            serviceNamesInTree,
            ['api', 'postgres', 'redis'],
            'All services with complex configs should be in tree view'
        );

        // All should have NotStarted status
        for (const item of treeItems) {
            assert.strictEqual(item.status, ServiceStatus.NotStarted);
        }
    });
});
