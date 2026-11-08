import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import type {
  SessionArchiveResult,
  SessionCreateResult,
  SessionListResult,
  WorkspaceCreateResult,
  WorkspaceListResult,
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

async function workspaceFor(harness: DshTestHarness) {
  const result = (await harness.transport.request(
    'libadwaita/workspace/list',
    {},
  )) as WorkspaceListResult;
  return result.workspaces.find((workspace) => workspace.path === harness.workspaceDir);
}

describe('Gap batch G: persisted sessions', { timeout: 120_000 }, () => {
  it('lists persisted sessions and restores workspace membership after restart', async () => {
    const first = await createDshHarness(undefined, { preserveRoot: true });
    const rootDir = first.rootDir;
    let sessionId = '';
    let workspaceId = '';
    try {
      await initialize(first);
      const createdWorkspace = (await first.transport.request('libadwaita/workspace/create', {
        path: first.workspaceDir,
      })) as WorkspaceCreateResult;
      workspaceId = createdWorkspace.workspace.id;
      const created = (await first.transport.request('libadwaita/session/create', {
        cwd: first.workspaceDir,
      })) as SessionCreateResult;
      sessionId = created.sessionId;

      const workspace = await workspaceFor(first);
      assert.ok(workspace, 'workspace must be listed after create');
      assert.ok(
        workspace.sessionIds.includes(sessionId),
        'created session must be attached to its workspace',
      );
    } finally {
      await first.cleanup();
    }

    const second = await createDshHarness(undefined, { rootDir });
    try {
      await initialize(second);
      const list = (await second.transport.request('libadwaita/session/list', {
        archived: false,
        limit: 50,
      })) as SessionListResult;
      const persisted = list.sessions.find((session) => session.id === sessionId);
      assert.ok(persisted, 'a restarted dsh must still list the persisted session');
      assert.equal(persisted.cwd, second.workspaceDir);
      assert.equal(persisted.isArchived, false);

      const workspace = await workspaceFor(second);
      assert.ok(workspace, 'workspace must survive restart');
      assert.equal(workspace.id, workspaceId);
      assert.ok(
        workspace.sessionIds.includes(sessionId),
        'workspace membership must survive restart',
      );

      const followed = (await second.transport.request('libadwaita/session/follow', {
        sessionId,
      })) as { header: { id: string } };
      assert.equal(followed.header.id, sessionId);
    } finally {
      await second.cleanup();
    }
  });

  it('keeps archive state durable across restart', async () => {
    const first = await createDshHarness(undefined, { preserveRoot: true });
    const rootDir = first.rootDir;
    let sessionId = '';
    try {
      await initialize(first);
      const created = (await first.transport.request('libadwaita/session/create', {
        cwd: first.workspaceDir,
      })) as SessionCreateResult;
      sessionId = created.sessionId;
      await first.transport.request('libadwaita/session/archive', {
        sessionId,
        archived: true,
      });
      const active = (await first.transport.request('libadwaita/session/list', {
        archived: false,
        limit: 50,
      })) as SessionListResult;
      assert.equal(
        active.sessions.some((session) => session.id === sessionId),
        false,
      );
      const all = (await first.transport.request('libadwaita/session/list', {
        archived: true,
        limit: 50,
      })) as SessionListResult;
      assert.equal(all.sessions.find((session) => session.id === sessionId)?.isArchived, true);
    } finally {
      await first.cleanup();
    }

    const second = await createDshHarness(undefined, { rootDir });
    try {
      await initialize(second);
      const all = (await second.transport.request('libadwaita/session/list', {
        archived: true,
        limit: 50,
      })) as SessionListResult;
      assert.equal(
        all.sessions.find((session) => session.id === sessionId)?.isArchived,
        true,
        'archive state must survive restart',
      );

      const unarchived = (await second.transport.request('libadwaita/session/archive', {
        sessionId,
        archived: false,
      })) as SessionArchiveResult;
      assert.equal(unarchived.archived, false);
      const after = (await second.transport.request('libadwaita/session/list', {
        archived: true,
        limit: 50,
      })) as SessionListResult;
      assert.equal(after.sessions.find((session) => session.id === sessionId)?.isArchived, false);
    } finally {
      await second.cleanup();
    }
  });
});
