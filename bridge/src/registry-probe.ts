const PROBE_ENDPOINTS = [
  'https://registry.npmjs.org/-/ping',
  'https://registry.npmmirror.com/-/ping',
];

export interface RegistryProbeOptions {
  fetchImpl?: typeof fetch;
  timeoutMs?: number;
  cacheTtlMs?: number;
  now?: () => number;
}

export class RegistryProbe {
  private readonly fetchImpl: typeof fetch;
  private readonly timeoutMs: number;
  private readonly cacheTtlMs: number;
  private readonly now: () => number;
  private pending: Promise<string | null> | undefined;
  private cached: { registry: string | null; expiresAt: number } | undefined;

  constructor(options: RegistryProbeOptions = {}) {
    this.fetchImpl = options.fetchImpl ?? fetch;
    this.timeoutMs = options.timeoutMs ?? 1500;
    this.cacheTtlMs = options.cacheTtlMs ?? 300_000;
    this.now = options.now ?? Date.now;
  }

  async fastest(): Promise<string | null> {
    if (this.cached !== undefined && this.cached.expiresAt > this.now()) {
      return this.cached.registry;
    }
    this.pending ??= this.probe().finally(() => {
      this.pending = undefined;
    });
    return this.pending;
  }

  private async probe(): Promise<string | null> {
    const finished = new AbortController();
    const signal = AbortSignal.any([finished.signal, AbortSignal.timeout(this.timeoutMs)]);
    const requests = PROBE_ENDPOINTS.map(async (endpoint) => ({
      registry: new URL('/', endpoint).href,
      response: await this.fetchImpl(endpoint, { signal, redirect: 'error' }),
    }));
    const successful = requests.map(async (request) => {
      const { registry, response } = await request;
      if (!response.ok) throw new Error(`Registry ping returned HTTP ${response.status}`);
      return registry;
    });
    let registry: string | null;
    try {
      registry = await Promise.any(successful);
    } catch {
      registry = null;
    } finally {
      finished.abort();
      const responses = await Promise.allSettled(requests);
      await Promise.allSettled(
        responses.map(async (result) => {
          if (result.status === 'fulfilled') await result.value.response.body?.cancel();
        }),
      );
    }
    this.cached = { registry, expiresAt: this.now() + this.cacheTtlMs };
    return registry;
  }
}
