import type { Context } from '@deepseek-ai/cordis';
import { type BridgeConfig, BridgeService } from './bridge.js';
import { BridgeCredentials } from './credentials.js';

export { BridgeService } from './bridge.js';
export { BridgeCredentials } from './credentials.js';
export { BridgeServer } from './server.js';
export * from './types.js';

export default function apply(ctx: Context, config?: BridgeConfig): void {
  if (!ctx.get('credentials')) {
    ctx.plugin(BridgeCredentials);
  }
  ctx.plugin(BridgeService, config);
}
