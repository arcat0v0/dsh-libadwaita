import { type ChildProcess, spawn } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { JsonRpcLineTransport } from '@deepseek-ai/dsh-sdk-protocol';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const bridgeRoot = path.resolve(__dirname, '../../../');
const dshBin = path.join(bridgeRoot, 'node_modules/@deepseek-ai/dsh/lib/bin.js');

export type MockSseResponder = (
  req: http.IncomingMessage,
  res: http.ServerResponse,
) => Promise<void> | void;

export class MockLlmServer {
  server: http.Server;
  port = 0;
  private customResponder: MockSseResponder | undefined;

  constructor() {
    this.server = http.createServer((req, res) => {
      if (req.method === 'POST' && req.url === '/anthropic/v1/messages') {
        if (this.customResponder) {
          void this.customResponder(req, res);
          return;
        }
        this.defaultResponse(res);
        return;
      }
      if (req.method === 'GET' && req.url === '/anthropic/v1/models') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ data: [{ id: 'mock-model', object: 'model' }] }));
        return;
      }
      res.statusCode = 404;
      res.end();
    });
  }

  setResponder(responder: MockSseResponder | undefined): void {
    this.customResponder = responder;
  }

  async start(): Promise<number> {
    const { promise, resolve, reject } = Promise.withResolvers<number>();
    this.server.on('error', reject);
    this.server.listen(0, '127.0.0.1', () => {
      const addr = this.server.address();
      if (addr && typeof addr === 'object') {
        this.port = addr.port;
        resolve(this.port);
      } else {
        reject(new Error('failed to get server address'));
      }
    });
    return promise;
  }

  async close(): Promise<void> {
    const { promise, resolve } = Promise.withResolvers<void>();
    this.server.close(() => resolve());
    return promise;
  }

  private defaultResponse(res: http.ServerResponse): void {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });

    const sendEvent = (event: string, data: object) => {
      res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
    };

    sendEvent('message_start', {
      type: 'message_start',
      message: {
        id: 'msg_mock_1',
        type: 'message',
        role: 'assistant',
        content: [],
        model: 'deepseek-chat',
        usage: { input_tokens: 10, output_tokens: 1 },
      },
    });

    sendEvent('content_block_start', {
      type: 'content_block_start',
      index: 0,
      content_block: { type: 'text', text: '' },
    });

    sendEvent('content_block_delta', {
      type: 'content_block_delta',
      index: 0,
      delta: { type: 'text_delta', text: 'Hello from mock endpoint!' },
    });

    sendEvent('content_block_stop', {
      type: 'content_block_stop',
      index: 0,
    });

    sendEvent('message_delta', {
      type: 'message_delta',
      delta: { stop_reason: 'end_turn' },
      usage: { output_tokens: 5 },
    });

    sendEvent('message_stop', {
      type: 'message_stop',
    });

    res.end();
  }
}

export type HarnessNotificationListener = (method: string, params: Record<string, unknown>) => void;

export type HarnessRequestListener = (
  method: string,
  params: Record<string, unknown>,
) => Promise<unknown>;

export interface DshTestHarness {
  homeDir: string;
  runtimeDir: string;
  workspaceDir: string;
  socketPath: string;
  mockServer: MockLlmServer;
  dshProc: ChildProcess;
  transport: JsonRpcLineTransport;
  socket: net.Socket;
  onNotification: (listener: HarnessNotificationListener) => () => void;
  onRequest: (listener: HarnessRequestListener) => () => void;
  getStdout: () => Buffer;
  getStderr: () => string;
  cleanup: () => Promise<void>;
}

export interface DshHarnessOptions {
  prepareHome?: (homeDir: string, workspaceDir: string) => void;
}

export async function createDshHarness(
  customResponder?: MockSseResponder,
  options: DshHarnessOptions = {},
): Promise<DshTestHarness> {
  const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'dsh-it-'));
  const homeDir = path.join(tmpRoot, 'dsh-home');
  const runtimeDir = path.join(tmpRoot, 'runtime');
  const workspaceDir = path.join(tmpRoot, 'workspace');
  const socketPath = path.join(runtimeDir, 'dsh.sock');

  fs.mkdirSync(homeDir, { recursive: true });
  fs.mkdirSync(runtimeDir, { recursive: true, mode: 0o700 });
  fs.mkdirSync(workspaceDir, { recursive: true });
  options.prepareHome?.(homeDir, workspaceDir);

  const mockServer = new MockLlmServer();
  if (customResponder) {
    mockServer.setResponder(customResponder);
  }
  const mockPort = await mockServer.start();

  const profileDir = path.join(homeDir, 'profiles/libadwaita');
  const nmScopeDir = path.join(profileDir, 'node_modules/@dsh-libadwaita');
  fs.mkdirSync(nmScopeDir, { recursive: true });
  fs.symlinkSync(bridgeRoot, path.join(nmScopeDir, 'bridge'));

  const profilePackageJson = {
    name: 'libadwaita-profile',
    version: '0.1.0',
    private: true,
    dsh: {
      profile: {
        bundles: ['@deepseek-ai/dsh-base', '@dsh-libadwaita/bridge'],
      },
    },
    dependencies: {
      '@deepseek-ai/dsh-base': '0.2.2-rc.1',
      '@dsh-libadwaita/bridge': `file:${bridgeRoot}`,
    },
  };

  fs.writeFileSync(
    path.join(profileDir, 'package.json'),
    JSON.stringify(profilePackageJson, null, 2),
  );

  const cordisPatchYml = `- id: hmr
  name: '@deepseek-ai/dsh-hmr'
  disabled: true
  config:
    root: []
- id: credentials
  name: '@deepseek-ai/dsh-credentials-local'
  disabled: true
- id: llm-deepseek
  name: '@deepseek-ai/dsh-llm-deepseek-api-key'
  config:
    baseURL: http://127.0.0.1:${mockPort}/anthropic
`;
  fs.writeFileSync(path.join(profileDir, 'cordis.patch.yml'), cordisPatchYml);

  const stdoutChunks: Buffer[] = [];
  let stderrText = '';

  const dshProc = spawn(process.execPath, [dshBin, '--profile', 'libadwaita'], {
    env: {
      ...process.env,
      DSH_HOME: homeDir,
      XDG_RUNTIME_DIR: runtimeDir,
      DSH_LIBADWAITA_SOCKET: socketPath,
    },
    stdio: ['pipe', 'pipe', 'pipe'],
  });

  dshProc.stdout.on('data', (chunk) => {
    stdoutChunks.push(Buffer.from(chunk));
  });

  dshProc.stderr.on('data', (chunk) => {
    stderrText += chunk.toString('utf8');
  });

  const deadline = Date.now() + 15_000;
  let socketReady = false;
  while (Date.now() < deadline) {
    if (fs.existsSync(socketPath)) {
      socketReady = true;
      break;
    }
    await new Promise((r) => setTimeout(r, 100));
  }

  if (!socketReady) {
    dshProc.kill('SIGKILL');
    await mockServer.close();
    fs.rmSync(tmpRoot, { recursive: true, force: true });
    throw new Error(`dsh socket not ready within timeout. Stderr: ${stderrText}`);
  }

  let socket: net.Socket | undefined;
  for (let i = 0; i < 20; i++) {
    try {
      const s = net.connect(socketPath);
      const { promise, resolve, reject } = Promise.withResolvers<void>();
      s.once('connect', resolve);
      s.once('error', reject);
      await promise;
      socket = s;
      break;
    } catch {
      await new Promise((r) => setTimeout(r, 100));
    }
  }

  if (!socket) {
    dshProc.kill('SIGKILL');
    await mockServer.close();
    fs.rmSync(tmpRoot, { recursive: true, force: true });
    throw new Error(`failed to connect to dsh socket. Stderr: ${stderrText}`);
  }

  const transport = new JsonRpcLineTransport(socket, socket);
  const notificationListeners = new Set<HarnessNotificationListener>();
  const requestListeners = new Set<
    (method: string, params: Record<string, unknown>) => Promise<unknown>
  >();
  transport.onNotification((method, params) => {
    for (const listener of [...notificationListeners]) listener(method, params);
  });
  transport.onRequest(async (method, params) => {
    for (const listener of [...requestListeners]) {
      const result = await listener(method, params);
      if (result !== undefined) return result;
    }
    return {};
  });
  transport.start();

  const cleanup = async () => {
    transport.close();
    if (!socket.destroyed) {
      socket.destroy();
    }
    if (dshProc.exitCode === null && dshProc.signalCode === null) {
      dshProc.kill('SIGTERM');
      const { promise, resolve } = Promise.withResolvers<void>();
      dshProc.once('exit', () => resolve());
      const timer = setTimeout(() => {
        dshProc.kill('SIGKILL');
        resolve();
      }, 3000);
      await promise;
      clearTimeout(timer);
    }
    await mockServer.close();
    try {
      fs.rmSync(tmpRoot, { recursive: true, force: true });
    } catch {}
  };

  return {
    homeDir,
    runtimeDir,
    workspaceDir,
    socketPath,
    mockServer,
    dshProc,
    transport,
    socket,
    onNotification: (listener) => {
      notificationListeners.add(listener);
      return () => {
        notificationListeners.delete(listener);
      };
    },
    onRequest: (listener) => {
      requestListeners.add(listener);
      return () => {
        requestListeners.delete(listener);
      };
    },
    getStdout: () => Buffer.concat(stdoutChunks),
    getStderr: () => stderrText,
    cleanup,
  };
}
