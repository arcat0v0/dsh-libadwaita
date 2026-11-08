import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { RegistryProbe } from '../../src/registry-probe.js';

interface RecordedCall {
  url: string;
  init?: RequestInit;
}

function stubFetch(
  handlers: Map<string, () => Promise<Response>>,
  calls: RecordedCall[],
): typeof fetch {
  return ((url: string | URL, init?: RequestInit) => {
    const href = String(url);
    calls.push({ url: href, init });
    const handler = handlers.get(href);
    if (!handler) {
      return Promise.reject(new Error(`unexpected probe url ${href}`));
    }
    return new Promise<Response>((resolve, reject) => {
      const signal = init?.signal;
      if (signal?.aborted) {
        reject(signal.reason);
        return;
      }
      signal?.addEventListener('abort', () => reject(signal.reason), { once: true });
      handler().then(resolve, reject);
    });
  }) as typeof fetch;
}

const NPM_PING = 'https://registry.npmjs.org/-/ping';
const NPM_ROOT = 'https://registry.npmjs.org/';
const NPM_MIRROR_PING = 'https://registry.npmmirror.com/-/ping';
const NPM_MIRROR_ROOT = 'https://registry.npmmirror.com/';

function okResponse(): Response {
  return new Response('{}', { status: 200 });
}

describe('RegistryProbe', () => {
  it('returns the fastest registry that answers successfully', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => new Promise<Response>(() => {})],
      [NPM_MIRROR_PING, () => Promise.resolve(okResponse())],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls) });
    assert.equal(await probe.fastest(), NPM_MIRROR_ROOT);
    assert.deepEqual(calls.map((call) => call.url).sort(), [NPM_PING, NPM_MIRROR_PING]);
  });

  it('returns null when every probe times out', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => new Promise<Response>(() => {})],
      [NPM_MIRROR_PING, () => new Promise<Response>(() => {})],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls), timeoutMs: 20 });
    assert.equal(await probe.fastest(), null);
    assert.equal(calls.length, 2);
  });

  it('returns null when every probe answers with an HTTP error', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => Promise.resolve(new Response('{}', { status: 503 }))],
      [NPM_MIRROR_PING, () => Promise.resolve(new Response('{}', { status: 503 }))],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls) });
    assert.equal(await probe.fastest(), null);
  });

  it('returns null when every probe fails the request itself', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => Promise.reject(new Error('dns failure'))],
      [NPM_MIRROR_PING, () => Promise.reject(new Error('dns failure'))],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls) });
    assert.equal(await probe.fastest(), null);
  });

  it('prefers npm when it answers first even if the mirror also answers', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => Promise.resolve(okResponse())],
      [NPM_MIRROR_PING, () => Promise.resolve(okResponse())],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls) });
    assert.equal(await probe.fastest(), NPM_ROOT);
  });

  it('serves the cached winner until the TTL expires', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => Promise.resolve(okResponse())],
      [NPM_MIRROR_PING, () => Promise.resolve(okResponse())],
    ]);
    let clockMs = 1_000;
    const probe = new RegistryProbe({
      fetchImpl: stubFetch(handlers, calls),
      cacheTtlMs: 300_000,
      now: () => clockMs,
    });
    assert.equal(await probe.fastest(), NPM_ROOT);
    assert.equal(await probe.fastest(), NPM_ROOT);
    assert.equal(calls.length, 2);
    clockMs += 300_001;
    assert.equal(await probe.fastest(), NPM_ROOT);
    assert.equal(calls.length, 4);
  });

  it('shares one probe between concurrent readers', async () => {
    const calls: RecordedCall[] = [];
    const handlers = new Map<string, () => Promise<Response>>([
      [NPM_PING, () => Promise.resolve(okResponse())],
      [NPM_MIRROR_PING, () => Promise.resolve(okResponse())],
    ]);
    const probe = new RegistryProbe({ fetchImpl: stubFetch(handlers, calls) });
    const [first, second] = await Promise.all([probe.fastest(), probe.fastest()]);
    assert.equal(first, NPM_ROOT);
    assert.equal(second, NPM_ROOT);
    assert.equal(calls.length, 2);
  });
});
