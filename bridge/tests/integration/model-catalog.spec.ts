import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import type { ModelCatalogResult } from '../../src/types.js';
import { createDshHarness, type DshTestHarness } from './harness.js';

async function initialize(harness: DshTestHarness): Promise<void> {
  await harness.transport.request('libadwaita/initialize', {
    clientVersion: '0.1.0',
    protocolVersion: 1,
  });
}

describe('model catalog defaults', { timeout: 120_000 }, () => {
  it('reports a default provider and model that exist in the catalog', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const catalog = (await harness.transport.request(
        'libadwaita/model/catalog',
        {},
      )) as ModelCatalogResult;
      assert.ok(catalog.providers.length > 0, 'catalog must list at least one provider');
      const ids = catalog.providers.map((provider) => provider.id);
      assert.ok(
        ids.includes(catalog.defaultProvider),
        `default provider ${catalog.defaultProvider} must be one of ${ids.join(', ')}`,
      );
      const provider = catalog.providers.find(
        (candidate) => candidate.id === catalog.defaultProvider,
      );
      assert.ok(catalog.defaultModel.length > 0, 'default model must be a non-empty id');
      if (provider && provider.models.length > 0) {
        assert.ok(
          provider.models.some((model) => model.id === catalog.defaultModel),
          `default model ${catalog.defaultModel} must exist under ${catalog.defaultProvider}`,
        );
      }
    } finally {
      await harness.cleanup();
    }
  });
});
