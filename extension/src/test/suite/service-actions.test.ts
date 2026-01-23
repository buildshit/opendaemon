import * as assert from 'assert';
import * as vscode from 'vscode';
import { ServiceTreeDataProvider, ServiceStatus } from '../../tree-view';

suite('Service Actions Test Suite', () => {
    let treeDataProvider: ServiceTreeDataProvider;

    setup(() => {
        treeDataProvider = new ServiceTreeDataProvider();
    });

    suite('Status Updates', () => {
        test('Should update status to Starting when service starts', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.NotStarted }
            ]);
            
            treeDataProvider.updateServiceStatus('test-service', ServiceStatus.Starting);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.Starting);
        });

        test('Should update status to Running when service is ready', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.Starting }
            ]);
            
            treeDataProvider.updateServiceStatus('test-service', ServiceStatus.Running);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.Running);
        });

        test('Should update status to Failed when service fails', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.Starting }
            ]);
            
            treeDataProvider.updateServiceStatus('test-service', ServiceStatus.Failed);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.Failed);
        });

        test('Should update status to Stopped when service stops', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.Running }
            ]);
            
            treeDataProvider.updateServiceStatus('test-service', ServiceStatus.Stopped);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.Stopped);
        });

        test('Should preserve exit code on Failed status', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.Running }
            ]);
            
            treeDataProvider.updateServiceStatus('test-service', ServiceStatus.Failed, 1);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.Failed);
            assert.strictEqual(service?.exitCode, 1);
        });
    });

    suite('Service Discovery', () => {
        test('Should load all services from config', () => {
            const services = [
                { name: 'database', status: ServiceStatus.NotStarted },
                { name: 'backend-api', status: ServiceStatus.NotStarted },
                { name: 'frontend', status: ServiceStatus.NotStarted }
            ];
            
            treeDataProvider.updateServices(services);
            
            const allServices = treeDataProvider.getAllServices();
            assert.strictEqual(allServices.length, 3);
            assert.ok(allServices.find(s => s.name === 'database'));
            assert.ok(allServices.find(s => s.name === 'backend-api'));
            assert.ok(allServices.find(s => s.name === 'frontend'));
        });

        test('Should start with NotStarted status', () => {
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.NotStarted }
            ]);
            
            const service = treeDataProvider.getService('test-service');
            assert.strictEqual(service?.status, ServiceStatus.NotStarted);
        });
    });

    suite('Tree Data Provider', () => {
        test('Should return tree items for all services', () => {
            treeDataProvider.updateServices([
                { name: 'service-a', status: ServiceStatus.Running },
                { name: 'service-b', status: ServiceStatus.Stopped }
            ]);
            
            const children = treeDataProvider.getChildren();
            assert.strictEqual(children.length, 2);
        });

        test('Should fire change event on service update', (done) => {
            const disposable = treeDataProvider.onDidChangeTreeData(() => {
                disposable.dispose();
                done();
            });
            
            treeDataProvider.updateServices([
                { name: 'test-service', status: ServiceStatus.NotStarted }
            ]);
        });

        test('Should clear all services', () => {
            treeDataProvider.updateServices([
                { name: 'service-a', status: ServiceStatus.Running },
                { name: 'service-b', status: ServiceStatus.Running }
            ]);
            
            treeDataProvider.clear();
            
            assert.strictEqual(treeDataProvider.getAllServices().length, 0);
        });
    });
});
