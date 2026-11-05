import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';
import { JsonRpcLineTransport } from '@deepseek-ai/dsh-sdk-protocol';
import type {
  CredentialsStatusResult,
  InitializeResult,
  SessionCreateResult,
  SessionFollowResult,
  SessionListResult,
  SessionPageResult,
  SessionPromptResult,
  SessionWorkdirGetResult,
} from '../../src/types.js';
import { createDshHarness } from './harness.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

describe('Real dsh integration tests', { timeout: 60_000 }, () => {
  it('performs handshake and rejects protocol version mismatch', async () => {
    const harness = await createDshHarness();
    try {
      const initRes = (await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      })) as InitializeResult;

      assert.equal(initRes.protocolVersion, 1);
      assert.equal(initRes.dshVersion, '0.2.2-rc.1');
      assert.equal(initRes.profileName, 'libadwaita');
      assert.ok(initRes.generationId);
      assert.equal(initRes.serverInfo.name, 'dsh-libadwaita-bridge');

      await assert.rejects(
        harness.transport.request('libadwaita/initialize', {
          clientVersion: '0.1.0',
          protocolVersion: 2,
        }),
        (err: unknown) => {
          if (err && typeof err === 'object' && 'code' in err) {
            assert.equal((err as { code: number }).code, -32001);
            return true;
          }
          return false;
        },
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('guarantees 0600 socket permissions and 0-byte pure stdout', async () => {
    const harness = await createDshHarness();
    try {
      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      const stats = fs.statSync(harness.socketPath);
      assert.equal(stats.mode & 0o777, 0o600);

      const stdout = harness.getStdout();
      assert.equal(stdout.length, 0);
    } finally {
      await harness.cleanup();
    }
  });

  it('manages sessions and credentials lifecycle', async () => {
    const harness = await createDshHarness();
    try {
      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-deepseek-test-key-1234',
      });

      const credStatus = (await harness.transport.request(
        'libadwaita/credentials/status',
        {},
      )) as CredentialsStatusResult;
      const keyItem = credStatus.items.find((i) => i.provider === 'DEEPSEEK_API_KEY');
      assert.ok(keyItem);
      assert.equal(keyItem.configured, true);
      assert.equal(keyItem.masked, 'sk-...1234');

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;

      assert.ok(created.sessionId);
      assert.equal(created.header.cwd, harness.workspaceDir);

      const followed = (await harness.transport.request('libadwaita/session/follow', {
        sessionId: created.sessionId,
      })) as SessionFollowResult;

      assert.equal(followed.status, 'idle');
      assert.equal(followed.header.cwd, harness.workspaceDir);

      const workdir = (await harness.transport.request('libadwaita/session/workdir/get', {
        sessionId: created.sessionId,
      })) as SessionWorkdirGetResult;
      assert.equal(workdir.cwd, harness.workspaceDir);

      const listRes = (await harness.transport.request(
        'libadwaita/session/list',
        {},
      )) as SessionListResult;
      const listed = listRes.sessions.find((s) => s.id === created.sessionId);
      assert.ok(listed);
      assert.equal(listed.cwd, harness.workspaceDir);
    } finally {
      await harness.cleanup();
    }
  });

  it('submits prompt and receives agent.stream and session.event', async () => {
    const harness = await createDshHarness();
    try {
      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-key-1234',
      });

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;

      await harness.transport.request('libadwaita/session/follow', {
        sessionId: created.sessionId,
      });

      const streamedChunks: string[] = [];
      const sessionEvents: string[] = [];
      const { promise: donePromise, resolve: doneResolve } = Promise.withResolvers<void>();

      harness.transport.onNotification((method, params) => {
        if (method === 'agent.stream') {
          const p = params as {
            frame?: { type?: string; chunk?: { type?: string; text?: string } };
          };
          const frame = p.frame;
          if (
            frame?.type === 'chunk' &&
            frame?.chunk?.type === 'text-delta' &&
            typeof frame.chunk.text === 'string'
          ) {
            streamedChunks.push(frame.chunk.text);
          }
          if (frame?.type === 'end') {
            doneResolve();
          }
        }
        if (method === 'session.event') {
          const p = params as { event?: { type?: string } };
          const ev = p.event;
          if (ev?.type) {
            sessionEvents.push(ev.type);
          }
        }
      });

      const promptRes = (await harness.transport.request('libadwaita/session/prompt', {
        sessionId: created.sessionId,
        contentBlocks: [{ type: 'text', text: 'Hello AI' }],
      })) as SessionPromptResult;

      assert.ok(promptRes.messageId);

      const timeoutTimer = setTimeout(() => {
        doneResolve();
      }, 10_000);

      await donePromise;
      clearTimeout(timeoutTimer);

      assert.ok(streamedChunks.length > 0);
      assert.equal(streamedChunks.join(''), 'Hello from mock endpoint!');
      assert.ok(sessionEvents.includes('user/message'));
      assert.ok(sessionEvents.includes('turn/start'));

      const pageRes = (await harness.transport.request('libadwaita/session/page', {
        sessionId: created.sessionId,
        fromSeq: 0,
        limit: 10,
      })) as SessionPageResult;

      assert.ok(pageRes.events.length > 0);
      assert.equal(pageRes.events[0]?.seq, 0);
    } finally {
      await harness.cleanup();
    }
  });

  it('cancels a running turn and preserves inbox', async () => {
    let mockResolver: (() => void) | undefined;
    const harness = await createDshHarness((_req, res) => {
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      });
      const send = (ev: string, d: object) =>
        res.write(`event: ${ev}\ndata: ${JSON.stringify(d)}\n\n`);
      send('message_start', {
        type: 'message_start',
        message: {
          id: 'msg-slow',
          type: 'message',
          role: 'assistant',
          content: [],
          model: 'deepseek-chat',
          usage: { input_tokens: 10, output_tokens: 1 },
        },
      });
      send('content_block_start', {
        type: 'content_block_start',
        index: 0,
        content_block: { type: 'text', text: '' },
      });
      send('content_block_delta', {
        type: 'content_block_delta',
        index: 0,
        delta: { type: 'text_delta', text: 'Slow message start...' },
      });

      const { promise, resolve } = Promise.withResolvers<void>();
      mockResolver = resolve;
      void promise.then(() => {
        res.end();
      });
    });

    try {
      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-key-1234',
      });

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;

      await harness.transport.request('libadwaita/session/follow', {
        sessionId: created.sessionId,
      });

      const { promise: startedPromise, resolve: startedResolve } = Promise.withResolvers<void>();
      harness.transport.onNotification((method, params) => {
        if (method === 'agent.stream') {
          const p = params as { frame?: { type?: string } };
          if (p.frame?.type === 'chunk') {
            startedResolve();
          }
        }
      });

      await harness.transport.request('libadwaita/session/prompt', {
        sessionId: created.sessionId,
        contentBlocks: [{ type: 'text', text: 'Slow turn test' }],
      });

      await startedPromise;

      const cancelRes = (await harness.transport.request('libadwaita/session/cancel', {
        sessionId: created.sessionId,
      })) as { cancelled: boolean };

      assert.equal(cancelRes.cancelled, true);
      mockResolver?.();
    } finally {
      mockResolver?.();
      await harness.cleanup();
    }
  });

  it('handles tool call with bidirectional approval/request', async () => {
    let callCount = 0;
    const harness = await createDshHarness((_req, res) => {
      callCount++;
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      });
      const send = (ev: string, d: object) =>
        res.write(`event: ${ev}\ndata: ${JSON.stringify(d)}\n\n`);

      if (callCount === 1) {
        send('message_start', {
          type: 'message_start',
          message: {
            id: 'msg-tool-1',
            type: 'message',
            role: 'assistant',
            content: [],
            model: 'deepseek-chat',
            usage: { input_tokens: 10, output_tokens: 1 },
          },
        });
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'tool_use', id: 'call_bash_1', name: 'bash', input: {} },
        });
        send('content_block_delta', {
          type: 'content_block_delta',
          index: 0,
          delta: {
            type: 'input_json_delta',
            partial_json:
              '{"description":"run","command":"echo approval_ok","sandbox_permissions":"danger-full-access","justification":"need root"}',
          },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'tool_use' },
          usage: { output_tokens: 5 },
        });
        send('message_stop', { type: 'message_stop' });
        res.end();
      } else {
        send('message_start', {
          type: 'message_start',
          message: {
            id: 'msg-tool-2',
            type: 'message',
            role: 'assistant',
            content: [],
            model: 'deepseek-chat',
            usage: { input_tokens: 10, output_tokens: 1 },
          },
        });
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'text', text: '' },
        });
        send('content_block_delta', {
          type: 'content_block_delta',
          index: 0,
          delta: { type: 'text_delta', text: 'All done after approval!' },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'end_turn' },
          usage: { output_tokens: 5 },
        });
        send('message_stop', { type: 'message_stop' });
        res.end();
      }
    });

    try {
      let receivedApproval = false;
      harness.transport.onRequest(async (method) => {
        if (method === 'approval/request') {
          receivedApproval = true;
          return { outcome: 'allowed-once' };
        }
        return {};
      });

      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-key-1234',
      });

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;

      await harness.transport.request('libadwaita/session/follow', {
        sessionId: created.sessionId,
      });

      const { promise: donePromise, resolve: doneResolve } = Promise.withResolvers<void>();
      harness.transport.onNotification((method, params) => {
        if (method === 'session.event') {
          const p = params as { event?: { type?: string } };
          if (p.event?.type === 'turn/end') {
            doneResolve();
          }
        }
      });

      await harness.transport.request('libadwaita/session/prompt', {
        sessionId: created.sessionId,
        contentBlocks: [{ type: 'text', text: 'Run bash' }],
      });

      const timer = setTimeout(() => doneResolve(), 10_000);
      await donePromise;
      clearTimeout(timer);

      assert.equal(receivedApproval, true);
    } finally {
      await harness.cleanup();
    }
  });

  it('recovers state on crash with new generationId and re-follow', async () => {
    const harness = await createDshHarness();
    let sessionId = '';
    let gen1 = '';
    try {
      const init1 = (await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      })) as InitializeResult;
      gen1 = init1.generationId;

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-key-1234',
      });

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;
      sessionId = created.sessionId;

      await harness.transport.request('libadwaita/session/follow', {
        sessionId,
      });

      const { promise: donePromise, resolve: doneResolve } = Promise.withResolvers<void>();
      harness.transport.onNotification((method, params) => {
        if (method === 'session.event') {
          const p = params as { event?: { type?: string } };
          if (p.event?.type === 'turn/end') {
            doneResolve();
          }
        }
      });

      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'Pre crash prompt' }],
      });

      const timer = setTimeout(() => doneResolve(), 10_000);
      await donePromise;
      clearTimeout(timer);

      harness.transport.close();
      harness.socket.destroy();
      harness.dshProc.kill('SIGKILL');

      const { promise: exitPromise, resolve: exitResolve } = Promise.withResolvers<void>();
      harness.dshProc.once('exit', () => exitResolve());
      await exitPromise;
      try {
        fs.unlinkSync(harness.socketPath);
      } catch {}

      const dshBinPath = path.resolve(
        __dirname,
        '../../../node_modules/@deepseek-ai/dsh/lib/bin.js',
      );
      const dshProc2 = spawn(process.execPath, [dshBinPath, '--profile', 'libadwaita'], {
        env: {
          ...process.env,
          DSH_HOME: harness.homeDir,
          XDG_RUNTIME_DIR: harness.runtimeDir,
          DSH_LIBADWAITA_SOCKET: harness.socketPath,
        },
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      let clientSock2: net.Socket | undefined;
      for (let i = 0; i < 30; i++) {
        try {
          if (fs.existsSync(harness.socketPath)) {
            const s = net.connect(harness.socketPath);
            const { promise: p, resolve: r, reject: rj } = Promise.withResolvers<void>();
            s.once('connect', r);
            s.once('error', rj);
            await p;
            clientSock2 = s;
            break;
          }
        } catch {}
        await new Promise((r) => setTimeout(r, 100));
      }
      assert.ok(clientSock2);
      const transport2 = new JsonRpcLineTransport(clientSock2, clientSock2);
      transport2.start();

      const init2 = (await transport2.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      })) as InitializeResult;

      assert.notEqual(init2.generationId, gen1);

      const followed = (await transport2.request('libadwaita/session/follow', {
        sessionId,
      })) as SessionFollowResult;

      assert.equal(followed.header.cwd, harness.workspaceDir);
      assert.ok(followed.latestSeq >= 0);

      transport2.close();
      clientSock2.destroy();
      dshProc2.kill('SIGKILL');
    } finally {
      await harness.cleanup();
    }
  });

  it('handles user-question ask with plan-review intent cancellation', async () => {
    let callCount = 0;
    const harness = await createDshHarness((_req, res) => {
      callCount++;
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      });
      const send = (ev: string, d: object) =>
        res.write(`event: ${ev}\ndata: ${JSON.stringify(d)}\n\n`);

      if (callCount === 1) {
        send('message_start', {
          type: 'message_start',
          message: {
            id: 'msg-q-1',
            type: 'message',
            role: 'assistant',
            content: [],
            model: 'deepseek-chat',
            usage: { input_tokens: 10, output_tokens: 1 },
          },
        });
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'tool_use', id: 'call_q_1', name: 'ask_user_question', input: {} },
        });
        send('content_block_delta', {
          type: 'content_block_delta',
          index: 0,
          delta: {
            type: 'input_json_delta',
            partial_json:
              '{"questions":[{"id":"plan-rev","question":"Review plan","detail":"# Plan","intent":{"kind":"plan-review","approve":"Approve"},"options":[{"label":"Approve"},{"label":"Modify"}]}]}',
          },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'tool_use' },
          usage: { output_tokens: 5 },
        });
        send('message_stop', { type: 'message_stop' });
        res.end();
      } else {
        send('message_start', {
          type: 'message_start',
          message: {
            id: 'msg-q-2',
            type: 'message',
            role: 'assistant',
            content: [],
            model: 'deepseek-chat',
            usage: { input_tokens: 10, output_tokens: 1 },
          },
        });
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'text', text: 'Understood, will modify plan.' },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'end_turn' },
          usage: { output_tokens: 5 },
        });
        send('message_stop', { type: 'message_stop' });
        res.end();
      }
    });

    try {
      let receivedQuestion = false;
      harness.transport.onRequest(async (method) => {
        if (method === 'user-question/ask') {
          receivedQuestion = true;
          return { cancelled: true };
        }
        return {};
      });

      await harness.transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      await harness.transport.request('libadwaita/credentials/inject', {
        provider: 'DEEPSEEK_API_KEY',
        key: 'sk-mock-key-1234',
      });

      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;

      await harness.transport.request('libadwaita/session/follow', {
        sessionId: created.sessionId,
      });

      const { promise: donePromise, resolve: doneResolve } = Promise.withResolvers<void>();
      harness.transport.onNotification((method, params) => {
        if (method === 'session.event') {
          const p = params as { event?: { type?: string } };
          if (p.event?.type === 'turn/end') {
            doneResolve();
          }
        }
      });

      await harness.transport.request('libadwaita/session/prompt', {
        sessionId: created.sessionId,
        contentBlocks: [{ type: 'text', text: 'Plan something' }],
      });

      const timer = setTimeout(() => doneResolve(), 10_000);
      await donePromise;
      clearTimeout(timer);

      assert.equal(receivedQuestion, true);
    } finally {
      await harness.cleanup();
    }
  });
});
