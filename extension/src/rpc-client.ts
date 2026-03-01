import { EventEmitter } from 'events';
import { ActivityLogger } from './activity-logger';

export interface RpcRequest {
    jsonrpc: '2.0';
    id: number;
    method: string;
    params?: unknown;
}

export interface RpcResponse {
    jsonrpc: '2.0';
    id: number;
    result?: unknown;
    error?: RpcError;
}

export interface RpcError {
    code: number;
    message: string;
    data?: unknown;
}

export interface RpcNotification {
    jsonrpc: '2.0';
    method: string;
    params?: unknown;
}

type PendingRequest = {
    method: string;
    resolve: (result: unknown) => void;
    reject: (error: Error) => void;
    timeout: NodeJS.Timeout;
};

export class RpcClient extends EventEmitter {
    private nextId = 1;
    private pendingRequests = new Map<number, PendingRequest>();
    private readonly requestTimeout = 120000; // 120 seconds
    // Buffer for partial JSON-RPC messages split across stdout chunks
    private incomingBuffer = '';

    constructor(
        private readonly write: (data: string) => void,
        private readonly activityLogger?: ActivityLogger | null
    ) {
        super();
    }

    /**
     * Send an RPC request and wait for response
     */
    async request(method: string, params?: unknown): Promise<unknown> {
        const id = this.nextId++;

        const request: RpcRequest = {
            jsonrpc: '2.0',
            id,
            method,
            params
        };

        // Log the outgoing request
        if (this.activityLogger) {
            this.activityLogger.logRpcAction(
                method,
                'request',
                `id: ${id}, params: ${JSON.stringify(params)}`
            );
        }

        return new Promise((resolve, reject) => {
            // Set up timeout
            const timeout = setTimeout(() => {
                this.pendingRequests.delete(id);
                
                // Log timeout error
                const error = `Request timeout: ${method}`;
                if (this.activityLogger) {
                    this.activityLogger.logError(`RPC request ${method} (id: ${id})`, error);
                }
                
                reject(new Error(error));
            }, this.requestTimeout);

            // Store pending request
            this.pendingRequests.set(id, { method, resolve, reject, timeout });

            // Send request
            this.write(JSON.stringify(request) + '\n');
        });
    }

    /**
     * Handle incoming data from daemon
     */
    handleData(data: string): void {
        // Append new chunk to the carry-over buffer. RPC messages are newline-delimited,
        // but chunks can split messages arbitrarily.
        this.incomingBuffer += data;
        const lines = this.incomingBuffer.split('\n');
        // Keep trailing partial line for the next chunk.
        this.incomingBuffer = lines.pop() || '';

        for (const rawLine of lines) {
            const line = rawLine.trim();
            if (!line) {
                continue;
            }
            try {
                const message = JSON.parse(line);
                this.handleMessage(message);
            } catch (err) {
                console.error('Failed to parse RPC message:', err, 'Data:', line);
            }
        }

        // Backward-compatible fallback: if daemon/client sends one complete JSON
        // object without a trailing newline, process it immediately.
        const trailing = this.incomingBuffer.trim();
        if (trailing) {
            try {
                const message = JSON.parse(trailing);
                this.handleMessage(message);
                this.incomingBuffer = '';
            } catch {
                // Most likely an incomplete chunk; keep buffering.
            }
        }
    }

    /**
     * Handle a parsed RPC message
     */
    private handleMessage(message: unknown): void {
        if (!this.isValidMessage(message)) {
            console.error('Invalid RPC message:', message);
            return;
        }

        // Check if it's a response (has id)
        if (typeof message === 'object' && message !== null && 'id' in message && typeof message.id === 'number') {
            this.handleResponse(message as RpcResponse);
        }
        // Check if it's a notification (no id)
        else if (typeof message === 'object' && message !== null && 'method' in message) {
            this.handleNotification(message as RpcNotification);
        }
    }

    /**
     * Handle RPC response
     */
    private handleResponse(response: RpcResponse): void {
        const pending = this.pendingRequests.get(response.id);

        if (!pending) {
            console.warn('Received response for unknown request:', response.id);
            return;
        }

        // Clear timeout
        clearTimeout(pending.timeout);
        this.pendingRequests.delete(response.id);

        // Log the response
        if (response.error) {
            if (this.activityLogger) {
                this.activityLogger.logRpcAction(
                    pending.method,
                    'response',
                    `id: ${response.id}, error: ${JSON.stringify(response.error)}`
                );
            }
            pending.reject(new Error(response.error.message));
        } else {
            if (this.activityLogger) {
                this.activityLogger.logRpcAction(
                    pending.method,
                    'response',
                    `id: ${response.id}, success`
                );
            }
            pending.resolve(response.result);
        }
    }

    /**
     * Handle RPC notification (event from daemon)
     */
    private handleNotification(notification: RpcNotification): void {
        this.emit('notification', notification.method, notification.params);
    }

    /**
     * Validate message structure
     */
    private isValidMessage(message: unknown): boolean {
        if (typeof message !== 'object' || message === null) {
            return false;
        }

        const msg = message as Record<string, unknown>;

        return msg.jsonrpc === '2.0' &&
            (typeof msg.id === 'number' || typeof msg.method === 'string');
    }

    /**
     * Clean up pending requests
     */
    dispose(): void {
        for (const [id, pending] of this.pendingRequests.entries()) {
            clearTimeout(pending.timeout);
            pending.reject(new Error('RPC client disposed'));
        }
        this.pendingRequests.clear();
        this.incomingBuffer = '';
        this.removeAllListeners();
    }
}
