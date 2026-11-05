import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { Transform } from 'node:stream';
import { JsonRpcLineTransport } from '@deepseek-ai/dsh-sdk-protocol';
import {
  type ApprovalRequestParams,
  type ApprovalRequestResult,
  BRIDGE_ERROR_CODES,
  BridgeError,
  DSH_PINNED_VERSION,
  type InitializeParams,
  type InitializeResult,
  PROTOCOL_VERSION,
  type SdkUserQuestionRequest,
  type SdkUserQuestionResult,
} from './types.js';

export interface BridgeServerOptions {
  socketPath: string;
  generationId: string;
  dshVersion?: string;
  onMethod?: (method: string, params: Record<string, unknown>) => Promise<unknown>;
}

export class BridgeServer {
  readonly socketPath: string;
  readonly generationId: string;
  readonly dshVersion: string;
  private server: net.Server | undefined;
  private activeTransport: JsonRpcLineTransport | undefined;
  private activeSocket: net.Socket | undefined;
  private onMethodCallback:
    | ((method: string, params: Record<string, unknown>) => Promise<unknown>)
    | undefined;
  private pendingApprovals = new Set<(decision: ApprovalRequestResult) => void>();
  private pendingQuestions = new Set<(result: SdkUserQuestionResult) => void>();
  private pendingErrorsQueue: BridgeError[] = [];

  constructor(options: BridgeServerOptions) {
    this.socketPath = options.socketPath;
    this.generationId = options.generationId;
    this.dshVersion = options.dshVersion ?? DSH_PINNED_VERSION;
    this.onMethodCallback = options.onMethod;
  }

  setMethodHandler(
    handler: (method: string, params: Record<string, unknown>) => Promise<unknown>,
  ): void {
    this.onMethodCallback = handler;
  }

  async start(): Promise<void> {
    const dir = path.dirname(this.socketPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
    }
    if (fs.existsSync(this.socketPath)) {
      try {
        fs.unlinkSync(this.socketPath);
      } catch {}
    }

    const netServer = net.createServer((socket) => {
      this.handleConnection(socket);
    });

    this.server = netServer;

    const { promise, resolve, reject } = Promise.withResolvers<void>();
    netServer.on('error', reject);
    netServer.listen(this.socketPath, () => {
      try {
        fs.chmodSync(this.socketPath, 0o600);
        resolve();
      } catch (err) {
        reject(err);
      }
    });
    try {
      fs.chmodSync(this.socketPath, 0o600);
    } catch {}

    return promise;
  }

  private handleConnection(socket: net.Socket): void {
    if (this.activeSocket && !this.activeSocket.destroyed) {
      this.activeSocket.destroy();
    }

    this.activeSocket = socket;

    const outTransform = new Transform({
      transform: (chunk, _encoding, callback) => {
        const text = chunk.toString('utf8');
        const lines = text.split('\n');
        const transformed: string[] = [];

        for (const line of lines) {
          if (!line.trim()) {
            transformed.push(line);
            continue;
          }
          try {
            const parsed = JSON.parse(line);
            if (
              parsed &&
              typeof parsed === 'object' &&
              parsed.id !== undefined &&
              parsed.error !== undefined
            ) {
              const customErr = this.pendingErrorsQueue.shift();
              if (customErr) {
                parsed.error.code = customErr.code;
                parsed.error.message = customErr.message;
                if (customErr.data !== undefined) {
                  parsed.error.data = customErr.data;
                }
                transformed.push(JSON.stringify(parsed));
                continue;
              }
            }
          } catch {}
          transformed.push(line);
        }

        callback(null, transformed.join('\n'));
      },
    });

    outTransform.pipe(socket);

    const transport = new JsonRpcLineTransport(socket, outTransform);
    this.activeTransport = transport;

    transport.onRequest(async (method, params) => {
      return this.dispatchRequest(method, params);
    });

    transport.start();

    const cleanup = () => {
      if (this.activeTransport === transport) {
        this.activeTransport = undefined;
        this.activeSocket = undefined;
      }
      this.failClosedPending();
    };

    socket.on('close', cleanup);
    socket.on('error', cleanup);
  }

  private async dispatchRequest(method: string, params: Record<string, unknown>): Promise<unknown> {
    if (method === 'libadwaita/initialize') {
      const p = params as unknown as InitializeParams;
      if (p.protocolVersion !== PROTOCOL_VERSION) {
        const err = new BridgeError(
          BRIDGE_ERROR_CODES.PROTOCOL_VERSION_MISMATCH,
          'protocol version mismatch',
          {
            supported: PROTOCOL_VERSION,
            received: p.protocolVersion,
          },
        );
        this.recordNextError(err);
        throw err;
      }
      const result: InitializeResult = {
        protocolVersion: PROTOCOL_VERSION,
        dshVersion: this.dshVersion,
        profileName: 'libadwaita',
        generationId: this.generationId,
        serverInfo: {
          name: 'dsh-libadwaita-bridge',
          version: '0.1.0',
        },
      };
      return result;
    }

    if (this.onMethodCallback) {
      try {
        return await this.onMethodCallback(method, params);
      } catch (err: unknown) {
        if (err instanceof BridgeError) {
          this.recordNextError(err);
          throw err;
        }
        const message = err instanceof Error ? err.message : String(err);
        const bridgeErr = new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, message);
        this.recordNextError(bridgeErr);
        throw bridgeErr;
      }
    }

    const notFound = new BridgeError(-32601, `method not found: ${method}`);
    this.recordNextError(notFound);
    throw notFound;
  }

  private recordNextError(err: BridgeError): void {
    this.pendingErrorsQueue.push(err);
  }

  private failClosedPending(): void {
    for (const resolve of this.pendingApprovals) {
      resolve({ outcome: 'rejected' });
    }
    this.pendingApprovals.clear();

    for (const resolve of this.pendingQuestions) {
      resolve({ cancelled: true });
    }
    this.pendingQuestions.clear();
  }

  requestApproval(
    params: ApprovalRequestParams,
    timeoutMs = 300_000,
    signal?: AbortSignal,
  ): Promise<ApprovalRequestResult> {
    const transport = this.activeTransport;
    if (!transport) {
      return Promise.resolve({ outcome: 'rejected' });
    }

    const { promise, resolve } = Promise.withResolvers<ApprovalRequestResult>();
    let timer: NodeJS.Timeout | undefined;
    let settled = false;

    const finish = (result: ApprovalRequestResult) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      this.pendingApprovals.delete(finish);
      resolve(result);
    };

    this.pendingApprovals.add(finish);

    if (timeoutMs > 0) {
      timer = setTimeout(() => {
        finish({ outcome: 'rejected' });
      }, timeoutMs);
    }

    if (signal) {
      if (signal.aborted) {
        finish({ outcome: 'cancelled' });
        return promise;
      }
      signal.addEventListener('abort', () => finish({ outcome: 'cancelled' }), { once: true });
    }

    transport
      .request('approval/request', params, signal)
      .then((res: unknown) => {
        if (res && typeof res === 'object' && 'outcome' in res) {
          const out = res.outcome;
          if (out === 'allowed-once' || out === 'rejected' || out === 'cancelled') {
            finish({ outcome: out });
            return;
          }
        }
        finish({ outcome: 'rejected' });
      })
      .catch(() => {
        finish({ outcome: 'rejected' });
      });

    return promise;
  }

  requestUserQuestion(
    params: SdkUserQuestionRequest,
    timeoutMs = 300_000,
    signal?: AbortSignal,
  ): Promise<SdkUserQuestionResult> {
    const transport = this.activeTransport;
    if (!transport) {
      return Promise.resolve({ cancelled: true });
    }

    const { promise, resolve } = Promise.withResolvers<SdkUserQuestionResult>();
    let timer: NodeJS.Timeout | undefined;
    let settled = false;

    const finish = (result: SdkUserQuestionResult) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      this.pendingQuestions.delete(finish);
      resolve(result);
    };

    this.pendingQuestions.add(finish);

    if (timeoutMs > 0) {
      timer = setTimeout(() => {
        finish({ cancelled: true });
      }, timeoutMs);
    }

    if (signal) {
      if (signal.aborted) {
        finish({ cancelled: true });
        return promise;
      }
      signal.addEventListener('abort', () => finish({ cancelled: true }), { once: true });
    }

    transport
      .request('user-question/ask', params, signal)
      .then((res: unknown) => {
        if (res && typeof res === 'object') {
          if ('cancelled' in res && res.cancelled === true) {
            finish({ cancelled: true });
            return;
          }
          if ('answers' in res && Array.isArray(res.answers)) {
            finish(res as SdkUserQuestionResult);
            return;
          }
        }
        finish({ cancelled: true });
      })
      .catch(() => {
        finish({ cancelled: true });
      });

    return promise;
  }

  notify(method: string, params?: object): void {
    if (this.activeTransport) {
      try {
        this.activeTransport.notify(method, params);
      } catch {}
    }
  }

  async close(): Promise<void> {
    this.failClosedPending();
    if (this.activeSocket && !this.activeSocket.destroyed) {
      this.activeSocket.destroy();
    }
    this.activeTransport = undefined;
    this.activeSocket = undefined;

    if (this.server) {
      const { promise, resolve } = Promise.withResolvers<void>();
      this.server.close(() => resolve());
      await promise;
      this.server = undefined;
    }

    if (fs.existsSync(this.socketPath)) {
      try {
        fs.unlinkSync(this.socketPath);
      } catch {}
    }
  }
}
