import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import { describe, it } from 'node:test';
import type { SessionCreateResult } from '../../src/types.js';
import { createDshHarness, type DshTestHarness } from './harness.js';

async function initialize(harness: DshTestHarness): Promise<void> {
  await harness.transport.request('libadwaita/initialize', {
    clientVersion: '0.1.0',
    protocolVersion: 1,
  });
  await harness.transport.request('libadwaita/credentials/inject', {
    provider: 'DEEPSEEK_API_KEY',
    key: 'sk-mock-key-1234',
  });
}

describe('Gap batch F', { timeout: 120_000 }, () => {
  it('reuses an existing session when session/create names its id', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const first = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
      })) as SessionCreateResult;
      assert.ok(first.sessionId);
      const reused = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
        sessionId: first.sessionId,
      })) as SessionCreateResult;
      assert.equal(reused.sessionId, first.sessionId);
      assert.equal(reused.header.id, first.sessionId);
    } finally {
      await harness.cleanup();
    }
  });

  it('creates a new session for a genuinely unknown named id', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = `sess-${crypto.randomUUID()}`;
      const created = (await harness.transport.request('libadwaita/session/create', {
        cwd: harness.workspaceDir,
        sessionId,
      })) as SessionCreateResult;
      assert.equal(created.sessionId, sessionId);
      assert.equal(created.header.id, sessionId);
    } finally {
      await harness.cleanup();
    }
  });
});
