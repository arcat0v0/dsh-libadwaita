import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import type {
  PluginInventoryResult,
  PluginOperationResult,
  PluginToggleResult,
} from '../../src/types.js';
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

async function firstBundleName(harness: DshTestHarness): Promise<string> {
  const inventory = (await harness.transport.request(
    'libadwaita/plugin/inventory',
    {},
  )) as PluginInventoryResult;
  const name = (inventory.bundles as Array<{ name?: string }>).find(
    (bundle) => typeof bundle.name === 'string',
  )?.name;
  assert.ok(name, 'inventory lists at least one named bundle');
  return name;
}

async function firstPluginEntryId(harness: DshTestHarness): Promise<string> {
  const inventory = (await harness.transport.request(
    'libadwaita/plugin/inventory',
    {},
  )) as PluginInventoryResult;
  const entryId = (inventory.plugins as Array<{ entryId?: string }>)[0]?.entryId;
  assert.ok(entryId, 'inventory lists at least one plugin entry');
  return entryId;
}

describe('Gap batch E', { timeout: 120_000 }, () => {
  it('toggles a bundle and returns the real change result', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const name = await firstBundleName(harness);
      const enabled = (await harness.transport.request('libadwaita/plugin/bundle/enable', {
        name,
      })) as PluginToggleResult;
      assert.equal(enabled.target, name);
      assert.equal(enabled.stage, 'enable');
      assert.equal(enabled.enabled, true);
      assert.ok(
        ['applied', 'restart-required', 'overridden', 'cancelled', 'failed'].includes(
          enabled.application,
        ),
      );
      assert.equal(typeof enabled.changed, 'boolean');
    } finally {
      await harness.cleanup();
    }
  });

  it('reports row enable failures instead of swallowing them', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const failed = (await harness.transport.request('libadwaita/plugin/enable', {
        id: 'no-such-plugin-entry',
      })) as PluginToggleResult;
      assert.equal(failed.application, 'failed');
      assert.ok(failed.error);
      const disabled = (await harness.transport.request('libadwaita/plugin/disable', {
        id: 'no-such-plugin-entry',
      })) as PluginToggleResult;
      assert.equal(disabled.application, 'failed');
      assert.ok(disabled.error);
    } finally {
      await harness.cleanup();
    }
  });

  it('forwards plugin.changed and plugin.update-state notifications', async () => {
    const harness = await createDshHarness();
    try {
      const changed: unknown[] = [];
      const updateStates: unknown[] = [];
      harness.onNotification((method, params) => {
        if (method === 'plugin.changed') changed.push(params);
        if (method === 'plugin.update-state') updateStates.push(params);
      });
      await initialize(harness);
      const entryId = await firstPluginEntryId(harness);
      await harness.transport.request('libadwaita/plugin/enable', { id: entryId });
      assert.ok(changed.some((payload) => (payload as { reason?: string }).reason === 'plugin'));
      const name = await firstBundleName(harness);
      const updated = (await harness.transport.request('libadwaita/plugin/update', {
        name,
        version: '0.0.1-no-such-newer-version',
        requestId: 'req-update-1',
      })) as PluginOperationResult;
      assert.ok(updated.result !== undefined);
      assert.ok(
        updateStates.some((payload) => {
          const progress = payload as { requestId?: string; step?: string };
          return progress.requestId === 'req-update-1' && progress.step === 'checking';
        }),
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('reads bundle detail with npm file sources', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const detail = (await harness.transport.request('libadwaita/plugin/detail', {
        name: '@deepseek-ai/dsh-base',
        language: 'en',
        npmFileSources: ['jsdelivr'],
      })) as PluginOperationResult;
      assert.ok(detail.result !== undefined);
      assert.equal((detail.result as { status?: string }).status, 'found');
    } finally {
      await harness.cleanup();
    }
  });

  it('lists and configures plugin market indexes', async () => {
    const harness = await createDshHarness();
    try {
      const marketChanged: unknown[] = [];
      harness.onNotification((method, params) => {
        if (method === 'plugin.market-changed') marketChanged.push(params);
      });
      await initialize(harness);
      const initial = (await harness.transport.request(
        'libadwaita/plugin-market/list-indexes',
        {},
      )) as PluginOperationResult;
      assert.deepEqual(initial.result, []);
      const url = 'https://127.0.0.1:9/market-index.json';
      const set = (await harness.transport.request('libadwaita/plugin-market/set-indexes', {
        urls: [url],
      })) as PluginOperationResult;
      assert.equal(set.result, null);
      const listed = (await harness.transport.request(
        'libadwaita/plugin-market/list-indexes',
        {},
      )) as PluginOperationResult;
      assert.ok((listed.result as Array<{ url: string }>).some((index) => index.url === url));
      assert.ok(marketChanged.length > 0);
    } finally {
      await harness.cleanup();
    }
  });

  it('preserves schedule input error codes in bridge error data', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      await assert.rejects(
        harness.transport.request('libadwaita/schedule/create', {
          title: 'bad target',
          prompt: 'do work',
          timing: { kind: 'after', after_seconds: 3600 },
          target: { kind: 'session', sessionId: 'session-does-not-exist' },
        }),
        (error: unknown) => {
          if (!error || typeof error !== 'object') return false;
          const e = error as { code?: number; data?: { code?: string } };
          assert.equal(e.code, -32045);
          assert.equal(e.data?.code, 'invalid_rule');
          return true;
        },
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('creates a new_once schedule, runs it, and reports limits and cleanup', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const created = (await harness.transport.request('libadwaita/schedule/create', {
        title: 'gap batch e schedule',
        prompt: 'say hi',
        timing: { kind: 'after', after_seconds: 3600 },
        target: { kind: 'new_once' },
      })) as {
        result: { id: string; target?: { kind?: string; sessionId?: string } };
      };
      assert.ok(created.result.id);
      assert.equal(created.result.target?.kind, 'new_once');
      assert.ok(created.result.target?.sessionId);
      const run = (await harness.transport.request('libadwaita/schedule/run', {
        id: created.result.id,
      })) as {
        result: { id: string; lastDelivery?: { messageId: string; targetSessionId: string } };
      };
      assert.ok(run.result.lastDelivery?.messageId);
      assert.equal(run.result.lastDelivery?.targetSessionId, created.result.target?.sessionId);
      const sessions = (await harness.transport.request('libadwaita/session/list', {})) as {
        sessions: Array<{ id: string }>;
      };
      assert.ok(
        sessions.sessions.some((session) => session.id === created.result.target?.sessionId),
      );
      const history = (await harness.transport.request('libadwaita/schedule/history', {
        id: created.result.id,
        limit: 10,
      })) as { result: { records: unknown[] } };
      assert.ok(history.result.records.length > 0);
      const limits = (await harness.transport.request(
        'libadwaita/schedule/delivery-limits',
        {},
      )) as { result: { newSessionIntervalSeconds: number } };
      assert.equal(typeof limits.result.newSessionIntervalSeconds, 'number');
      const inactive = (await harness.transport.request(
        'libadwaita/schedule/delete-inactive',
        {},
      )) as { result: { deleted: number } };
      assert.equal(typeof inactive.result.deleted, 'number');
    } finally {
      await harness.cleanup();
    }
  });
});
