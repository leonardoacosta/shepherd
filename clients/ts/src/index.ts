import net from 'node:net';

import type {
  ErrorResponse,
  EventEnvelope,
  EventsSubscribeParams,
  PaneListParams,
  Request,
  SuccessResponse,
  WorkspaceCreateParams,
} from './generated/schema';

export type {
  ErrorResponse,
  EventEnvelope,
  EventsSubscribeParams,
  PaneListParams,
  Request,
  SuccessResponse,
  WorkspaceCreateParams,
};

type SocketTarget = {
  socketPath: string;
};

type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

export type EventSubscription = AsyncIterable<EventEnvelope> & {
  close(): void;
};

export class ShepherdApiError extends Error {
  response: ErrorResponse;

  constructor(response: ErrorResponse) {
    super(response.error.message);
    this.name = 'ShepherdApiError';
    this.response = response;
  }
}

export class ShepherdApiClient {
  readonly socketPath: string;

  constructor(target: SocketTarget) {
    this.socketPath = target.socketPath;
  }

  async request(request: Request): Promise<SuccessResponse> {
    const socket = await connectSocket(this.socketPath);
    try {
      socket.write(`${JSON.stringify(request)}\n`);
      const iterator = readJsonLines(socket);
      const first = await iterator.next();
      if (first.done) {
        throw new Error('empty api response');
      }
      return parseSuccessResponse(first.value);
    } finally {
      socket.destroy();
    }
  }

  async paneList(params: PaneListParams): Promise<SuccessResponse> {
    return this.request({
      id: 'ts-client:pane.list',
      method: 'pane.list',
      params,
    });
  }

  async workspaceCreate(params: WorkspaceCreateParams): Promise<SuccessResponse> {
    return this.request({
      id: 'ts-client:workspace.create',
      method: 'workspace.create',
      params,
    });
  }

  async subscribe(
    subscriptions: EventsSubscribeParams['subscriptions'],
  ): Promise<EventSubscription> {
    const socket = await connectSocket(this.socketPath);
    socket.write(
      `${JSON.stringify({
        id: 'ts-client:events.subscribe',
        method: 'events.subscribe',
        params: { subscriptions },
      })}\n`,
    );

    const iterator = readJsonLines(socket);
    const ack = await iterator.next();
    if (ack.done) {
      socket.destroy();
      throw new Error('empty subscription response');
    }

    const started = parseSuccessResponse(ack.value);
    if (started.result.type !== 'subscription_started') {
      socket.destroy();
      throw new Error(`unexpected subscription response: ${started.result.type}`);
    }

    return {
      close() {
        socket.destroy();
      },
      async *[Symbol.asyncIterator]() {
        try {
          for await (const value of iterator) {
            yield parseEventEnvelope(value);
          }
        } finally {
          socket.destroy();
        }
      },
    };
  }
}

async function connectSocket(socketPath: string): Promise<net.Socket> {
  return await new Promise((resolve, reject) => {
    const socket = net.createConnection(socketPath);
    socket.once('connect', () => resolve(socket));
    socket.once('error', (error) => {
      socket.destroy();
      reject(error);
    });
  });
}

async function* readJsonLines(socket: net.Socket): AsyncGenerator<JsonValue> {
  let buffered = '';

  for await (const chunk of socket) {
    buffered += chunk.toString('utf8');
    while (true) {
      const newlineIndex = buffered.indexOf('\n');
      if (newlineIndex === -1) {
        break;
      }

      const line = buffered.slice(0, newlineIndex).trim();
      buffered = buffered.slice(newlineIndex + 1);
      if (line.length === 0) {
        continue;
      }

      yield JSON.parse(line) as JsonValue;
    }
  }

  const trailing = buffered.trim();
  if (trailing.length > 0) {
    yield JSON.parse(trailing) as JsonValue;
  }
}

function parseSuccessResponse(value: JsonValue): SuccessResponse {
  if (isObject(value) && 'error' in value) {
    throw new ShepherdApiError(value as ErrorResponse);
  }
  return value as SuccessResponse;
}

function parseEventEnvelope(value: JsonValue): EventEnvelope {
  return value as EventEnvelope;
}

function isObject(value: JsonValue): value is { [key: string]: JsonValue } {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
