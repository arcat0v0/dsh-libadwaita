import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { describe, it } from 'node:test';
import type {
  CommandExecuteResult,
  FilesListResult,
  FilesReadBytesResult,
  FilesReadResult,
  FilesStatResult,
  GoalGetResult,
  GoalMutationResult,
  GoalUpdatedNotification,
  PermissionCatalogResult,
  PermissionGetResult,
  PermissionSetResult,
  PermissionUpdatedNotification,
  SessionCreateResult,
  SettingsDescribeResult,
  SettingsUpdatedNotification,
  WorkspaceCreateResult,
  WorkspaceListResult,
  WorkspaceRenameResult,
} from '../../src/types.js';
import { createDshHarness, type DshTestHarness } from './harness.js';

async function initialize(harness: DshTestHarness): Promise<void> {
  await harness.transport.request('libadwaita/initialize', {
    clientVersion: '0.1.0',
    protocolVersion: 1,
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

function expectBridgeCode(code: number) {
  return (error: unknown): boolean => {
    if (error && typeof error === 'object' && 'code' in error) {
      assert.equal(error.code, code);
      return true;
    }
    return false;
  };
}

describe('Gap batch B', { timeout: 60_000 }, () => {
  it('persists workspaces with idempotent create, rename, and remove', async () => {
    const harness = await createDshHarness();
    const secondDir = path.join(harness.workspaceDir, 'second');
    fs.mkdirSync(secondDir, { recursive: true });
    try {
      await initialize(harness);
      const first = (await harness.transport.request('libadwaita/workspace/create', {
        path: harness.workspaceDir,
        title: 'Primary',
      })) as WorkspaceCreateResult;
      assert.equal(first.created, true);
      assert.equal(first.workspace.title, 'Primary');
      assert.equal(first.workspace.path, harness.workspaceDir);
      assert.equal(first.workspace.status, 'ok');
      const again = (await harness.transport.request('libadwaita/workspace/create', {
        path: harness.workspaceDir,
      })) as WorkspaceCreateResult;
      assert.equal(again.created, false);
      assert.equal(again.workspace.id, first.workspace.id);
      const second = (await harness.transport.request('libadwaita/workspace/create', {
        path: secondDir,
        title: 'Second',
      })) as WorkspaceCreateResult;
      const renamed = (await harness.transport.request('libadwaita/workspace/rename', {
        workspaceId: second.workspace.id,
        title: 'Renamed',
      })) as WorkspaceRenameResult;
      assert.equal(renamed.workspace.title, 'Renamed');
      await assert.rejects(
        harness.transport.request('libadwaita/workspace/rename', {
          workspaceId: second.workspace.id,
          title: 'Primary',
        }),
        expectBridgeCode(-32022),
      );
      const listed = (await harness.transport.request(
        'libadwaita/workspace/list',
        {},
      )) as WorkspaceListResult;
      assert.ok(listed.workspaces.some((workspace) => workspace.id === first.workspace.id));
      assert.ok(listed.workspaces.some((workspace) => workspace.id === second.workspace.id));
      const removed = await harness.transport.request('libadwaita/workspace/remove', {
        workspaceId: second.workspace.id,
      });
      assert.deepEqual(removed, { deleted: true });
      await assert.rejects(
        harness.transport.request('libadwaita/workspace/remove', {
          workspaceId: second.workspace.id,
        }),
        expectBridgeCode(-32021),
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('describes redacted settings, mutates with revision, and emits updates', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const described = (await harness.transport.request(
        'libadwaita/settings/describe',
        {},
      )) as SettingsDescribeResult;
      assert.ok(described.namespaces.length > 0);
      const target = described.namespaces.find((namespace) => namespace.ns === 'agent-loop');
      assert.ok(target);
      assert.equal(typeof target.revision, 'number');
      const updates: SettingsUpdatedNotification[] = [];
      harness.transport.onNotification((method, params) => {
        if (method === 'settings.updated') {
          updates.push(params as unknown as SettingsUpdatedNotification);
        }
      });
      await harness.transport.request('libadwaita/settings/mutate', {
        namespace: 'agent-loop',
        patch: { maxParallelToolCalls: 11 },
        expectedRevision: target.revision,
      });
      const after = (await harness.transport.request('libadwaita/settings/describe', {
        keys: ['agent-loop'],
      })) as SettingsDescribeResult;
      assert.ok((after.namespaces[0]?.revision ?? 0) > target.revision);
      await assert.rejects(
        harness.transport.request('libadwaita/settings/mutate', {
          namespace: 'agent-loop',
          patch: { maxParallelToolCalls: 12 },
          expectedRevision: target.revision,
        }),
        expectBridgeCode(-32020),
      );
      assert.ok(updates.some((update) => update.namespace === 'agent-loop'));
    } finally {
      await harness.cleanup();
    }
  });

  it('reads the permission catalog, current preset, and sets a new preset', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const catalog = (await harness.transport.request(
        'libadwaita/permission/catalog',
        {},
      )) as PermissionCatalogResult;
      assert.ok(catalog.options.some((option) => option.value === 'workspace-write'));
      const current = (await harness.transport.request('libadwaita/permission/get', {
        sessionId,
      })) as PermissionGetResult;
      assert.equal(typeof current.currentValue, 'string');
      const updates: PermissionUpdatedNotification[] = [];
      harness.transport.onNotification((method, params) => {
        if (method === 'permission.updated') {
          updates.push(params as unknown as PermissionUpdatedNotification);
        }
      });
      const target = catalog.options.find((option) => option.value !== current.currentValue);
      assert.ok(target);
      const set = (await harness.transport.request('libadwaita/permission/set', {
        sessionId,
        preset: target.value,
      })) as PermissionSetResult;
      assert.equal(set.currentValue, target.value);
      assert.ok(
        updates.some(
          (update) => update.sessionId === sessionId && update.currentValue === target.value,
        ),
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('reads and mutates goals with revision checks and notifications', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const empty = (await harness.transport.request('libadwaita/goal/get', {
        sessionId,
      })) as GoalGetResult;
      assert.equal(empty.goal, null);
      const created = (await harness.transport.request('libadwaita/command/execute', {
        sessionId,
        command: 'goal',
        args: 'Ship batch B',
      })) as CommandExecuteResult;
      assert.equal(created.kind, 'success');
      const first = (await harness.transport.request('libadwaita/goal/get', {
        sessionId,
      })) as GoalGetResult;
      assert.ok(first.goal);
      assert.equal(first.goal.objective, 'Ship batch B');
      const updates: GoalUpdatedNotification[] = [];
      harness.transport.onNotification((method, params) => {
        if (method === 'goal.updated') {
          updates.push(params as unknown as GoalUpdatedNotification);
        }
      });
      const edited = (await harness.transport.request('libadwaita/goal/edit', {
        sessionId,
        ref: { id: first.goal.id, revision: first.goal.revision },
        objective: 'Ship batch B safely',
      })) as GoalMutationResult;
      assert.ok(edited.goal);
      assert.equal(edited.goal.objective, 'Ship batch B safely');
      const paused = (await harness.transport.request('libadwaita/goal/pause', {
        sessionId,
        ref: { id: edited.goal.id, revision: edited.goal.revision },
      })) as GoalMutationResult;
      assert.equal(paused.goal?.phase, 'paused');
      const resumed = (await harness.transport.request('libadwaita/goal/resume', {
        sessionId,
        ref: { id: paused.goal?.id, revision: paused.goal?.revision },
      })) as GoalMutationResult;
      assert.equal(resumed.goal?.phase, 'active');
      await assert.rejects(
        harness.transport.request('libadwaita/goal/pause', {
          sessionId,
          ref: { id: first.goal.id, revision: 1 },
        }),
        expectBridgeCode(-32026),
      );
      const cleared = (await harness.transport.request('libadwaita/goal/clear', {
        sessionId,
        ref: { id: resumed.goal?.id, revision: resumed.goal?.revision },
      })) as GoalMutationResult;
      assert.equal(cleared.goal, null);
      assert.ok(updates.length > 0);
    } finally {
      await harness.cleanup();
    }
  });

  it('lists, stats, reads text and bytes, and blocks traversal', async () => {
    const harness = await createDshHarness(undefined, {
      prepareHome: (_homeDir, workspaceDir) => {
        fs.writeFileSync(path.join(workspaceDir, 'hello.txt'), 'hello\n');
        fs.writeFileSync(path.join(workspaceDir, 'binary.bin'), Buffer.from([0, 1, 2, 3]));
        fs.writeFileSync(path.join(workspaceDir, 'large.txt'), 'a'.repeat(2 * 1024 * 1024));
        fs.mkdirSync(path.join(workspaceDir, 'sub'));
        fs.writeFileSync(path.join(workspaceDir, 'sub', 'nested.txt'), 'nested\n');
        fs.symlinkSync('/etc/passwd', path.join(workspaceDir, 'outside-link'));
      },
    });
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const listed = (await harness.transport.request('libadwaita/files/list', {
        sessionId,
      })) as FilesListResult;
      const names = listed.entries.map((entry) => entry.name);
      assert.ok(names.includes('hello.txt'));
      assert.ok(names.includes('sub'));
      assert.ok(names.includes('binary.bin'));
      const stat = (await harness.transport.request('libadwaita/files/stat', {
        sessionId,
        path: 'hello.txt',
      })) as FilesStatResult;
      assert.equal(stat.kind, 'file');
      assert.equal(stat.size, 6);
      const text = (await harness.transport.request('libadwaita/files/read', {
        sessionId,
        path: 'hello.txt',
      })) as FilesReadResult;
      assert.equal(text.kind, 'text');
      if (text.kind === 'text') {
        assert.equal(text.text, 'hello\n');
        assert.equal(text.truncated, false);
      }
      const binary = (await harness.transport.request('libadwaita/files/read', {
        sessionId,
        path: 'binary.bin',
      })) as FilesReadResult;
      assert.equal(binary.kind, 'binary');
      const truncated = (await harness.transport.request('libadwaita/files/read', {
        sessionId,
        path: 'large.txt',
        maxBytes: 1024,
      })) as FilesReadResult;
      assert.equal(truncated.kind, 'text');
      if (truncated.kind === 'text') {
        assert.equal(truncated.truncated, true);
        assert.equal(truncated.text.length, 1024);
      }
      const bytes = (await harness.transport.request('libadwaita/files/readBytes', {
        sessionId,
        path: 'hello.txt',
      })) as FilesReadBytesResult;
      assert.equal(Buffer.from(bytes.data, 'base64').toString('utf8'), 'hello\n');
      await assert.rejects(
        harness.transport.request('libadwaita/files/read', {
          sessionId,
          path: '../outside.txt',
        }),
        expectBridgeCode(-32027),
      );
      await assert.rejects(
        harness.transport.request('libadwaita/files/read', {
          sessionId,
          path: 'outside-link',
        }),
        expectBridgeCode(-32028),
      );
    } finally {
      await harness.cleanup();
    }
  });
});
