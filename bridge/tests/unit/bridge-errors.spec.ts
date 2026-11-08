import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import type { SessionId } from '@deepseek-ai/dsh-session';
import { SessionPersistenceNotFoundError } from '@deepseek-ai/dsh-session-persistence';
import { isMissingSessionError } from '../../src/bridge.js';

describe('isMissingSessionError', () => {
  it('accepts SessionPersistenceNotFoundError instances', () => {
    const error = new SessionPersistenceNotFoundError('sess-1' as SessionId);
    assert.equal(isMissingSessionError(error), true);
  });

  it('accepts remote errors carrying the documented session/not-found code', () => {
    const error = { isDSHRemoteError: true as const, code: 'session/not-found' };
    assert.equal(isMissingSessionError(error), true);
  });

  it('rejects other remote error codes', () => {
    assert.equal(
      isMissingSessionError({ isDSHRemoteError: true as const, code: 'agent-preset/invalid' }),
      false,
    );
    assert.equal(isMissingSessionError({ code: 'session/not-found' }), false);
  });

  it('rejects unrelated errors', () => {
    assert.equal(isMissingSessionError(new Error('pnpm exited with 1')), false);
    assert.equal(isMissingSessionError(null), false);
    assert.equal(isMissingSessionError('session/not-found'), false);
  });
});
