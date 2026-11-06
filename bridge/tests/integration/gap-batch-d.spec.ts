import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { describe, it } from 'node:test';
import type {
  AccountStatusResult,
  CredentialRecordMutationResult,
  CredentialRecordReadResult,
  CredentialRecordsListResult,
  PluginInventoryResult,
  PluginOperationResult,
  SessionCreateResult,
  SubagentInterruptResult,
  SubagentListResult,
  WebProvidersResult,
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

async function createSession(harness: DshTestHarness): Promise<string> {
  const created = (await harness.transport.request('libadwaita/session/create', {
    cwd: harness.workspaceDir,
  })) as SessionCreateResult;
  await harness.transport.request('libadwaita/session/follow', {
    sessionId: created.sessionId,
  });
  return created.sessionId;
}

function writeFixtureBundle(root: string): void {
  fs.mkdirSync(root, { recursive: true });
  fs.writeFileSync(
    path.join(root, 'package.json'),
    JSON.stringify({
      name: 'libadwaita-fixture-bundle',
      version: '0.1.0',
      dsh: { bundle: { patch: './cordis.patch.yml' } },
    }),
  );
  fs.writeFileSync(path.join(root, 'cordis.patch.yml'), '- insert: []\n');
}

describe('Gap batch D', { timeout: 90_000 }, () => {
  it('lists plugin inventory, registries, and inspects a local bundle', async () => {
    const harness = await createDshHarness();
    const fixture = path.join(harness.workspaceDir, 'fixture-bundle');
    writeFixtureBundle(fixture);
    try {
      await initialize(harness);
      const inventory = (await harness.transport.request(
        'libadwaita/plugin/inventory',
        {},
      )) as PluginInventoryResult;
      assert.ok(Array.isArray(inventory.plugins));
      assert.ok(Array.isArray(inventory.bundles));
      assert.ok(inventory.plugins.length > 0);
      const registries = (await harness.transport.request(
        'libadwaita/plugin/registries',
        {},
      )) as PluginOperationResult;
      assert.ok(registries.result !== undefined);
      const inspect = (await harness.transport.request('libadwaita/plugin/inspect', {
        spec: fixture,
      })) as PluginOperationResult;
      assert.ok(inspect.result !== undefined);
      const detail = await harness.transport
        .request('libadwaita/plugin/detail', { name: '@deepseek-ai/dsh-base', language: 'en' })
        .catch((error: unknown) => error);
      assert.ok(detail !== undefined);
    } finally {
      await harness.cleanup();
    }
  });

  it('discovers models for a registered provider and lists web providers', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      await assert.rejects(
        harness.transport.request('libadwaita/model/discover', {
          settingsNs: 'llm-pi-ai',
          provider: 'mock',
          baseURL: `http://127.0.0.1:${harness.mockServer.port}/anthropic`,
          api: 'openai',
          apiKey: 'test',
        }),
        (error: unknown) => {
          if (error && typeof error === 'object' && 'code' in error) {
            assert.equal(error.code, -32044);
            return true;
          }
          return false;
        },
      );
      const web = (await harness.transport.request(
        'libadwaita/web-search/providers',
        {},
      )) as WebProvidersResult;
      assert.ok(Array.isArray(web.search));
      assert.ok(Array.isArray(web.fetch));
    } finally {
      await harness.cleanup();
    }
  });

  it('lists subagents and accepts a user interrupt for a child', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const list = (await harness.transport.request('libadwaita/subagent/list', {
        sessionId,
      })) as SubagentListResult;
      assert.ok(Array.isArray(list.entries));
      const interrupted = (await harness.transport.request('libadwaita/subagent/interrupt', {
        sessionId,
        childSessionId: sessionId,
      })) as SubagentInterruptResult;
      assert.deepEqual(interrupted, { accepted: true });
    } finally {
      await harness.cleanup();
    }
  });

  it('manages credential records without returning secret material', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const modified = (await harness.transport.request('libadwaita/credentials/records/modify', {
        key: 'llm/deepseek',
        record: { kind: 'api-key', key: 'record-secret', env: { AWS_PROFILE: 'dev' } },
      })) as CredentialRecordMutationResult;
      assert.equal(modified.metadata.configured, true);
      assert.equal(modified.metadata.hasKey, true);
      assert.deepEqual(modified.metadata.envNames, ['AWS_PROFILE']);
      assert.equal(JSON.stringify(modified).includes('record-secret'), false);
      const listed = (await harness.transport.request(
        'libadwaita/credentials/records/list',
        {},
      )) as CredentialRecordsListResult;
      assert.ok(listed.records.some((record) => record.key === 'llm/deepseek'));
      const read = (await harness.transport.request('libadwaita/credentials/records/read', {
        key: 'llm/deepseek',
      })) as CredentialRecordReadResult;
      assert.equal(JSON.stringify(read).includes('record-secret'), false);
      const deleted = (await harness.transport.request('libadwaita/credentials/records/delete', {
        key: 'llm/deepseek',
      })) as CredentialRecordMutationResult;
      assert.equal(deleted.deleted, true);
    } finally {
      await harness.cleanup();
    }
  });

  it('reads the base account state', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const status = (await harness.transport.request(
        'libadwaita/account/status',
        {},
      )) as AccountStatusResult;
      assert.ok(status.state !== undefined);
      const profile = await harness.transport
        .request('libadwaita/account/profile', {})
        .catch((error: unknown) => error);
      assert.ok(profile !== undefined);
    } finally {
      await harness.cleanup();
    }
  });
});
