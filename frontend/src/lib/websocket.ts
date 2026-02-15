export type TaskEventType = 'task_created' | 'task_updated' | 'task_deleted';

export interface TaskEventMessage<T = unknown> {
    event: TaskEventType;
    data: T;
}

export type TaskEventCallback<T = unknown> = (payload: T, message: TaskEventMessage<T>) => void;

interface WebSocketClientOptions {
    url?: string;
    reconnectBaseDelayMs?: number;
    reconnectMaxDelayMs?: number;
}

export class TaskWebSocketClient {
    private readonly url: string;
    private readonly token: string;
    private readonly reconnectBaseDelayMs: number;
    private readonly reconnectMaxDelayMs: number;

    private socket: WebSocket | null = null;
    private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
    private reconnectAttempts = 0;
    private shouldReconnect = true;

    private subscribers: Record<TaskEventType, Set<TaskEventCallback>> = {
        task_created: new Set(),
        task_updated: new Set(),
        task_deleted: new Set()
    };

    constructor(token: string, options: WebSocketClientOptions = {}) {
        this.token = token;
        this.url = options.url ?? 'ws://localhost:3000/ws';
        this.reconnectBaseDelayMs = options.reconnectBaseDelayMs ?? 1000;
        this.reconnectMaxDelayMs = options.reconnectMaxDelayMs ?? 30000;
    }

    connect(): void {
        if (!this.token) {
            throw new Error('WebSocket auth token is required');
        }

        if (this.socket && (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING)) {
            return;
        }

        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }

        this.shouldReconnect = true;

        const wsUrl = this.buildUrlWithToken();
        this.socket = new WebSocket(wsUrl);
        this.socket.onopen = () => {
            this.reconnectAttempts = 0;
        };

        this.socket.onmessage = (event) => {
            this.handleMessage(event.data);
        };

        this.socket.onclose = () => {
            this.socket = null;
            if (this.shouldReconnect) {
                this.scheduleReconnect();
            }
        };

        this.socket.onerror = () => {
            this.socket?.close();
        };
    }

    disconnect(): void {
        this.shouldReconnect = false;

        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }

        if (this.socket) {
            this.socket.close();
            this.socket = null;
        }
    }

    subscribe<T = unknown>(event: TaskEventType, callback: TaskEventCallback<T>): () => void {
        const typedCallback = callback as TaskEventCallback;
        this.subscribers[event].add(typedCallback);

        return () => {
            this.subscribers[event].delete(typedCallback);
        };
    }

    private buildUrlWithToken(): string {
        const url = new URL(this.url);
        url.searchParams.set('token', this.token);
        return url.toString();
    }

    private scheduleReconnect(): void {
        const delay = Math.min(
            this.reconnectBaseDelayMs * 2 ** this.reconnectAttempts,
            this.reconnectMaxDelayMs
        );
        this.reconnectAttempts += 1;

        this.reconnectTimer = setTimeout(() => {
            this.connect();
        }, delay);
    }

    private handleMessage(rawData: unknown): void {
        if (typeof rawData !== 'string') {
            return;
        }

        let parsed: unknown;
        try {
            parsed = JSON.parse(rawData);
        } catch {
            return;
        }

        if (!this.isTaskEventMessage(parsed)) {
            return;
        }

        const callbacks = this.subscribers[parsed.event];
        callbacks.forEach((callback) => {
            callback(parsed.data, parsed);
        });
    }

    private isTaskEventMessage(value: unknown): value is TaskEventMessage {
        if (!value || typeof value !== 'object') {
            return false;
        }

        const candidate = value as Partial<TaskEventMessage>;
        if (
            candidate.event !== 'task_created' &&
            candidate.event !== 'task_updated' &&
            candidate.event !== 'task_deleted'
        ) {
            return false;
        }

        return 'data' in candidate;
    }
}
