import * as assert from 'assert';
import { RpcClient } from '../../rpc-client';

suite('RPC Client Test Suite', () => {
    let rpcClient: RpcClient;
    let sentData: string[] = [];

    setup(() => {
        sentData = [];
        rpcClient = new RpcClient((data) => {
            sentData.push(data);
        });
    });

    teardown(() => {
        rpcClient.dispose();
    });

    test('Should create RPC client instance', () => {
        assert.ok(rpcClient);
    });

    test('Should send request with correct format', async () => {
        const requestPromise = rpcClient.request('test_method', { param: 'value' });
        
        // Check that data was sent
        assert.strictEqual(sentData.length, 1);
        
        const request = JSON.parse(sentData[0]);
        assert.strictEqual(request.jsonrpc, '2.0');
        assert.strictEqual(request.method, 'test_method');
        assert.deepStrictEqual(request.params, { param: 'value' });
        assert.ok(typeof request.id === 'number');

        // Simulate response
        rpcClient.handleData(JSON.stringify({
            jsonrpc: '2.0',
            id: request.id,
            result: { success: true }
        }));

        const result = await requestPromise;
        assert.deepStrictEqual(result, { success: true });
    });

    test('Should handle error responses', async () => {
        const requestPromise = rpcClient.request('failing_method');
        
        const request = JSON.parse(sentData[0]);

        // Simulate error response
        rpcClient.handleData(JSON.stringify({
            jsonrpc: '2.0',
            id: request.id,
            error: {
                code: -1,
                message: 'Test error'
            }
        }));

        await assert.rejects(
            requestPromise,
            (err: Error) => {
                assert.strictEqual(err.message, 'Test error');
                return true;
            }
        );
    });

    test('Should handle notifications', (done) => {
        rpcClient.on('notification', (method, params) => {
            assert.strictEqual(method, 'test_event');
            assert.deepStrictEqual(params, { data: 'value' });
            done();
        });

        rpcClient.handleData(JSON.stringify({
            jsonrpc: '2.0',
            method: 'test_event',
            params: { data: 'value' }
        }));
    });

    test('Should handle multiple messages in one data chunk', async () => {
        const promise1 = rpcClient.request('method1');
        const promise2 = rpcClient.request('method2');

        const req1 = JSON.parse(sentData[0]);
        const req2 = JSON.parse(sentData[1]);

        // Send both responses in one chunk
        const responses = [
            { jsonrpc: '2.0', id: req1.id, result: 'result1' },
            { jsonrpc: '2.0', id: req2.id, result: 'result2' }
        ];

        rpcClient.handleData(responses.map(r => JSON.stringify(r)).join('\n'));

        const [result1, result2] = await Promise.all([promise1, promise2]);
        assert.strictEqual(result1, 'result1');
        assert.strictEqual(result2, 'result2');
    });

    test('Should timeout on no response', async () => {
        // Create client with short timeout for testing
        const shortTimeoutClient = new RpcClient((data) => {
            // Don't send response
        });
        
        // Override timeout for testing
        (shortTimeoutClient as any).requestTimeout = 100;

        await assert.rejects(
            shortTimeoutClient.request('timeout_method'),
            (err: Error) => {
                assert.ok(err.message.includes('timeout'));
                return true;
            }
        );

        shortTimeoutClient.dispose();
    });

    test('Should handle invalid JSON gracefully', () => {
        // Should not throw
        rpcClient.handleData('invalid json');
        assert.ok(true);
    });

    test('Should handle invalid message format', () => {
        // Should not throw
        rpcClient.handleData(JSON.stringify({ invalid: 'message' }));
        assert.ok(true);
    });

    test('Should clean up on dispose', async () => {
        const requestPromise = rpcClient.request('test');
        
        rpcClient.dispose();

        await assert.rejects(
            requestPromise,
            (err: Error) => {
                assert.ok(err.message.includes('disposed'));
                return true;
            }
        );
    });
});
