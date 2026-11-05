import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { describe, it } from 'node:test';
import { JsonRpcLineTransport } from '@deepseek-ai/dsh-sdk-protocol';
import { BridgeServer } from '../../src/server.js';

describe('BridgeServer framing and lifecycle', () => {
  it('enforces 0600 socket permissions on Unix socket', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'bridge-sock-'));
    const sockPath = path.join(tmpDir, 'bridge.sock');
    const server = new BridgeServer({
      socketPath: sockPath,
      generationId: 'test-gen',
      dshVersion: '0.2.2-rc.1',
    });

    try {
      await server.start();
      const stats = fs.statSync(sockPath);
      const mode = stats.mode & 0o777;
      assert.equal(mode, 0o600);
    } finally {
      await server.close();
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it('handles handshake version negotiation and rejects mismatch', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'bridge-handshake-'));
    const sockPath = path.join(tmpDir, 'bridge.sock');
    const server = new BridgeServer({
      socketPath: sockPath,
      generationId: 'gen-test-123',
      dshVersion: '0.2.2-rc.1',
    });

    await server.start();

    try {
      const clientSock = net.connect(sockPath);
      const transport = new JsonRpcLineTransport(clientSock, clientSock);
      transport.start();

      const raw = await transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });
      if (raw && typeof raw === 'object' && 'protocolVersion' in raw && 'generationId' in raw) {
        assert.equal(raw.protocolVersion, 1);
        assert.equal(raw.generationId, 'gen-test-123');
      } else {
        assert.fail('invalid initialize response');
      }

      await assert.rejects(
        transport.request('libadwaita/initialize', {
          clientVersion: '0.1.0',
          protocolVersion: 999,
        }),
        (err: unknown) => {
          if (err && typeof err === 'object' && 'code' in err) {
            assert.equal(err.code, -32001);
            return true;
          }
          return false;
        },
      );
      transport.close();
      clientSock.destroy();
    } finally {
      await server.close();
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it('fails closed when client disconnects during pending approval', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'bridge-appr-'));
    const sockPath = path.join(tmpDir, 'bridge.sock');
    const server = new BridgeServer({
      socketPath: sockPath,
      generationId: 'gen-test-appr',
      dshVersion: '0.2.2-rc.1',
    });

    await server.start();

    try {
      const clientSock = net.connect(sockPath);
      const transport = new JsonRpcLineTransport(clientSock, clientSock);
      transport.start();

      await transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      const approvalPromise = server.requestApproval({
        sessionId: 'sess-1',
        toolName: 'bash',
        reason: 'test approval',
      });

      clientSock.destroy();
      transport.close();

      const decision = await approvalPromise;
      assert.equal(decision.outcome, 'rejected');
    } finally {
      await server.close();
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });

  it('fails closed when client disconnects during pending user question', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'bridge-q-'));
    const sockPath = path.join(tmpDir, 'bridge.sock');
    const server = new BridgeServer({
      socketPath: sockPath,
      generationId: 'gen-test-q',
      dshVersion: '0.2.2-rc.1',
    });

    await server.start();

    try {
      const clientSock = net.connect(sockPath);
      const transport = new JsonRpcLineTransport(clientSock, clientSock);
      transport.start();

      await transport.request('libadwaita/initialize', {
        clientVersion: '0.1.0',
        protocolVersion: 1,
      });

      const questionPromise = server.requestUserQuestion({
        requestId: 'q-1',
        sessionId: 'sess-1',
        questions: [{ id: 'q-item-1', question: 'Confirm?' }],
      });

      clientSock.destroy();
      transport.close();

      const answer = await questionPromise;
      assert.deepEqual(answer, { cancelled: true });
    } finally {
      await server.close();
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  });
});
