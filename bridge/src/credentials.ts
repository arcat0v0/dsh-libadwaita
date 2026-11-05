import type { Context } from '@deepseek-ai/cordis';
import {
  type CredentialInfo,
  type CredentialKey,
  CredentialProvider,
  type CredentialRecord,
  type CredentialRecordEntry,
  type CredentialRecordInfo,
  type CredentialRef,
  credentialRef,
  type ResolvedCredential,
} from '@deepseek-ai/dsh-credentials';
import type { CredentialStatusItem } from './types.js';

export class BridgeCredentials extends CredentialProvider {
  private readonly store = new Map<string, string>();
  private readonly records = new Map<CredentialKey, CredentialRecord>();
  private readonly baseUrls = new Map<string, string>();

  constructor(ctx: Context) {
    super(ctx);
  }

  override resolve(ref: CredentialRef): Promise<ResolvedCredential | undefined> {
    const value = this.store.get(ref);
    if (!value || value.length === 0) {
      return Promise.resolve(undefined);
    }
    return Promise.resolve({
      value,
      source: 'memory',
    });
  }

  override describe(ref: CredentialRef): Promise<CredentialInfo> {
    const value = this.store.get(ref);
    const configured = value !== undefined && value.length > 0;
    return Promise.resolve({
      configured,
      ...(configured ? { source: 'memory' } : {}),
      writable: true,
    });
  }

  override set(ref: CredentialRef, value: string): Promise<void> {
    if (value.length === 0) {
      return Promise.reject(new Error('empty credential value is not allowed; use unset'));
    }
    this.store.set(ref, value);
    this.ctx.emit('credentials/reference-updated', ref);
    return Promise.resolve();
  }

  override unset(ref: CredentialRef): Promise<void> {
    if (this.store.delete(ref)) {
      this.ctx.emit('credentials/reference-updated', ref);
    }
    return Promise.resolve();
  }

  override readRecord(key: CredentialKey): Promise<CredentialRecord | undefined> {
    return Promise.resolve(this.records.get(key));
  }

  override describeRecord(key: CredentialKey): Promise<CredentialRecordInfo> {
    const stored = this.records.get(key);
    return Promise.resolve(
      stored === undefined
        ? { configured: false, writable: true }
        : { configured: true, kind: stored.kind, writable: true },
    );
  }

  override listRecords(): Promise<readonly CredentialRecordEntry[]> {
    return Promise.resolve(
      [...this.records].map(([key, record]) => ({
        key,
        kind: record.kind,
      })),
    );
  }

  override async modifyRecord(
    key: CredentialKey,
    mutate: (current: CredentialRecord | undefined) => Promise<CredentialRecord | undefined>,
  ): Promise<CredentialRecord | undefined> {
    const current = this.records.get(key);
    const next = await mutate(current);
    if (next === undefined) {
      return current;
    }
    this.records.set(key, next);
    this.ctx.emit('credentials/record-updated', key);
    return next;
  }

  override deleteRecord(key: CredentialKey): Promise<void> {
    if (this.records.delete(key)) {
      this.ctx.emit('credentials/record-updated', key);
    }
    return Promise.resolve();
  }

  async inject(provider: string, key: string, baseURL?: string): Promise<void> {
    const ref = credentialRef(provider);
    await this.set(ref, key);
    if (baseURL) {
      this.baseUrls.set(provider, baseURL);
    }
  }

  async rotate(provider: string, key: string): Promise<void> {
    const ref = credentialRef(provider);
    await this.set(ref, key);
  }

  async remove(provider: string): Promise<void> {
    const ref = credentialRef(provider);
    await this.unset(ref);
    this.baseUrls.delete(provider);
  }

  status(): CredentialStatusItem[] {
    const results: CredentialStatusItem[] = [];
    for (const [provider, val] of this.store.entries()) {
      const configured = val.length > 0;
      let masked: string | undefined;
      if (configured) {
        masked = val.length > 8 ? `${val.slice(0, 3)}...${val.slice(-4)}` : '***';
      }
      results.push({
        provider,
        configured,
        ...(masked ? { masked } : {}),
      });
    }
    return results;
  }

  getBaseUrl(provider: string): string | undefined {
    return this.baseUrls.get(provider);
  }
}

export default function apply(ctx: Context): void {
  ctx.plugin(BridgeCredentials);
}
