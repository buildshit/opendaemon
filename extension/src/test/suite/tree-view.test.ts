import * as assert from 'assert';
import { ServiceTreeDataProvider, ServiceStatus, ServiceInfo } from '../../tree-view';

suite('Service Tree View Test Suite', () => {
    let treeProvider: ServiceTreeDataProvider;

    setup(() => {
        treeProvider = new ServiceTreeDataProvider();
    });

    test('Should create tree provider instance', () => {
        assert.ok(treeProvider);
    });

    test('Should return empty children initially', () => {
        const children = treeProvider.getChildren();
        assert.strictEqual(children.length, 0);
    });

    test('Should add services', () => {
        treeProvider.addService('service1', ServiceStatus.NotStarted);
        treeProvider.addService('service2', ServiceStatus.Running);

        const children = treeProvider.getChildren();
        assert.strictEqual(children.length, 2);
        assert.strictEqual(children[0].serviceName, 'service1');
        assert.strictEqual(children[1].serviceName, 'service2');
    });

    test('Should update service status', () => {
        treeProvider.addService('service1', ServiceStatus.NotStarted);
        treeProvider.updateServiceStatus('service1', ServiceStatus.Running);

        const service = treeProvider.getService('service1');
        assert.ok(service);
        assert.strictEqual(service.status, ServiceStatus.Running);
    });

    test('Should update services in bulk', () => {
        const services: ServiceInfo[] = [
            { name: 'service1', status: ServiceStatus.Running },
            { name: 'service2', status: ServiceStatus.Starting },
            { name: 'service3', status: ServiceStatus.Failed, exitCode: 1 }
        ];

        treeProvider.updateServices(services);

        const children = treeProvider.getChildren();
        assert.strictEqual(children.length, 3);
    });

    test('Should remove service', () => {
        treeProvider.addService('service1');
        treeProvider.addService('service2');
        
        treeProvider.removeService('service1');

        const children = treeProvider.getChildren();
        assert.strictEqual(children.length, 1);
        assert.strictEqual(children[0].serviceName, 'service2');
    });

    test('Should clear all services', () => {
        treeProvider.addService('service1');
        treeProvider.addService('service2');
        
        treeProvider.clear();

        const children = treeProvider.getChildren();
        assert.strictEqual(children.length, 0);
    });

    test('Should get all services', () => {
        treeProvider.addService('service1', ServiceStatus.Running);
        treeProvider.addService('service2', ServiceStatus.Stopped);

        const services = treeProvider.getAllServices();
        assert.strictEqual(services.length, 2);
    });

    test('Should handle status with exit code', () => {
        treeProvider.addService('service1');
        treeProvider.updateServiceStatus('service1', ServiceStatus.Failed, 127);

        const service = treeProvider.getService('service1');
        assert.ok(service);
        assert.strictEqual(service.status, ServiceStatus.Failed);
        assert.strictEqual(service.exitCode, 127);
    });

    test('Should return no children for service items', () => {
        treeProvider.addService('service1');
        const children = treeProvider.getChildren();
        const serviceItem = children[0];

        const serviceChildren = treeProvider.getChildren(serviceItem);
        assert.strictEqual(serviceChildren.length, 0);
    });

    test('Should handle updating non-existent service gracefully', () => {
        // Should not throw
        treeProvider.updateServiceStatus('nonexistent', ServiceStatus.Running);
        assert.ok(true);
    });
});
