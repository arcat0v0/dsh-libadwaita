import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { Context } from '@deepseek-ai/cordis';
import { credentialKey, credentialRef } from '@deepseek-ai/dsh-credentials';
import { BridgeCredentials } from '../../src/credentials.js';

describe('BridgeCredentials', () => {
  it('resolves injected API key and describes metadata', async () => {
    const ctx = new Context();
    const creds = new BridgeCredentials(ctx);
    const ref = credentialRef('DEEPSEEK_API_KEY');

    const beforeDesc = await creds.describe(ref);
    assert.equal(beforeDesc.configured, false);
    assert.equal(beforeDesc.writable, true);

    const beforeResolved = await creds.resolve(ref);
    assert.equal(beforeResolved, undefined);

    await creds.inject('DEEPSEEK_API_KEY', 'sk-test-secret-1234');

    const afterDesc = await creds.describe(ref);
    assert.equal(afterDesc.configured, true);
    assert.equal(afterDesc.source, 'memory');

    const afterResolved = await creds.resolve(ref);
    assert.ok(afterResolved);
    assert.equal(afterResolved.value, 'sk-test-secret-1234');
    assert.equal(afterResolved.source, 'memory');

    const statusList = creds.status();
    const item = statusList.find((s) => s.provider === 'DEEPSEEK_API_KEY');
    assert.ok(item);
    assert.equal(item.configured, true);
    assert.equal(item.masked, 'sk-...1234');
  });

  it('rotates and removes credentials', async () => {
    const ctx = new Context();
    const creds = new BridgeCredentials(ctx);
    const ref = credentialRef('DEEPSEEK_API_KEY');

    await creds.inject('DEEPSEEK_API_KEY', 'sk-initial');
    await creds.rotate('DEEPSEEK_API_KEY', 'sk-rotated-5678');

    const resolved = await creds.resolve(ref);
    assert.ok(resolved);
    assert.equal(resolved.value, 'sk-rotated-5678');

    await creds.remove('DEEPSEEK_API_KEY');
    const removedResolved = await creds.resolve(ref);
    assert.equal(removedResolved, undefined);
  });

  it('supports record operations for provider grants', async () => {
    const ctx = new Context();
    const creds = new BridgeCredentials(ctx);
    const key = credentialKey('llm', 'deepseek');

    const before = await creds.readRecord(key);
    assert.equal(before, undefined);

    await creds.modifyRecord(key, async () => ({
      kind: 'api-key',
      key: 'test-key',
    }));

    const after = await creds.readRecord(key);
    assert.ok(after);
    assert.equal(after.kind, 'api-key');

    const records = await creds.listRecords();
    assert.equal(records.length, 1);
    assert.equal(records[0]?.key, key);

    await creds.deleteRecord(key);
    const deleted = await creds.readRecord(key);
    assert.equal(deleted, undefined);
  });
});
