import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { describe, it } from 'node:test';
import type {
  CommandExecuteResult,
  CommandListResult,
  QueueListResult,
  ReferenceListResult,
  SessionCreateResult,
  SessionFollowResult,
  SessionForkResult,
  SessionPromptResult,
  SessionRenameResult,
  SkillListResult,
} from '../../src/types.js';
import { createDshHarness, type DshTestHarness } from './harness.js';

function writeSkill(homeDir: string, name: string): void {
  const dir = path.join(homeDir, 'skills', name);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(
    path.join(dir, 'SKILL.md'),
    `---\nname: ${name}\ndescription: ${name} description\n---\n\nBody.\n`,
  );
}

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

function waitForTurnEnd(harness: DshTestHarness, timeoutMs = 10_000): Promise<void> {
  const { promise, resolve } = Promise.withResolvers<void>();
  const timer = setTimeout(() => resolve(), timeoutMs);
  harness.transport.onNotification((method, params) => {
    if (method === 'session.event') {
      const p = params as { event?: { type?: string } };
      if (p.event?.type === 'turn/end') {
        clearTimeout(timer);
        resolve();
      }
    }
  });
  return promise;
}

describe('Gap batch A', { timeout: 60_000 }, () => {
  it('renames a session and emits session.title', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const titles: string[] = [];
      harness.transport.onNotification((method, params) => {
        if (method === 'session.title') {
          const p = params as { title?: string };
          if (p.title) titles.push(p.title);
        }
      });
      const renamed = (await harness.transport.request('libadwaita/session/rename', {
        sessionId,
        title: 'Renamed Session',
      })) as SessionRenameResult;
      assert.equal(renamed.title, 'Renamed Session');
      assert.equal(renamed.source.kind, 'user');
      const followed = (await harness.transport.request('libadwaita/session/follow', {
        sessionId,
      })) as SessionFollowResult;
      assert.equal(followed.title?.title, 'Renamed Session');
      assert.ok(titles.includes('Renamed Session'));
      await assert.rejects(
        harness.transport.request('libadwaita/session/rename', {
          sessionId: 'missing-session',
          title: 'x',
        }),
        (error: unknown) => {
          if (error && typeof error === 'object' && 'code' in error) {
            assert.equal(error.code, -32002);
            return true;
          }
          return false;
        },
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('forks a completed turn and opens the child session', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      await harness.transport.request('libadwaita/session/rename', {
        sessionId,
        title: 'Fork Source',
      });
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'hello' }],
      });
      await done;
      const followed = (await harness.transport.request('libadwaita/session/follow', {
        sessionId,
      })) as SessionFollowResult;
      const forked = (await harness.transport.request('libadwaita/session/fork', {
        sessionId,
        atSeq: followed.latestSeq,
        increaseTitle: true,
      })) as SessionForkResult;
      assert.ok(forked.sessionId);
      assert.equal(forked.header.isSeeded, true);
      assert.equal(forked.header.cwd, harness.workspaceDir);
      const child = (await harness.transport.request('libadwaita/session/follow', {
        sessionId: forked.sessionId,
      })) as SessionFollowResult;
      assert.equal(child.title?.title, 'Fork Source (1)');
      const childDone = waitForTurnEnd(harness);
      const prompt = (await harness.transport.request('libadwaita/session/prompt', {
        sessionId: forked.sessionId,
        contentBlocks: [{ type: 'text', text: 'continue' }],
      })) as SessionPromptResult;
      assert.ok(prompt.messageId);
      await childDone;
      await assert.rejects(
        harness.transport.request('libadwaita/session/fork', {
          sessionId,
          atSeq: 999_999,
        }),
        (error: unknown) => {
          if (error && typeof error === 'object' && 'code' in error) {
            assert.equal(error.code, -32019);
            return true;
          }
          return false;
        },
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('lists, edits, steers, and removes queued inbox items', async () => {
    let release: (() => void) | undefined;
    const harness = await createDshHarness((_req, res) => {
      res.writeHead(200, {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
        Connection: 'keep-alive',
      });
      const send = (event: string, data: object) =>
        res.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`);
      send('message_start', {
        type: 'message_start',
        message: {
          id: 'msg-queue',
          type: 'message',
          role: 'assistant',
          content: [],
          model: 'deepseek-chat',
          usage: { input_tokens: 10, output_tokens: 1 },
        },
      });
      send('content_block_start', {
        type: 'content_block_start',
        index: 0,
        content_block: { type: 'text', text: '' },
      });
      send('content_block_delta', {
        type: 'content_block_delta',
        index: 0,
        delta: { type: 'text_delta', text: 'working' },
      });
      const { promise, resolve } = Promise.withResolvers<void>();
      release = resolve;
      void promise.then(() => {
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'end_turn' },
          usage: { output_tokens: 5 },
        });
        send('message_stop', { type: 'message_stop' });
        res.end();
      });
    });
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const inboxNotifications: QueueListResult[] = [];
      const started = Promise.withResolvers<void>();
      harness.transport.onNotification((method, params) => {
        if (method === 'session.inbox') {
          const p = params as { items?: QueueListResult['items'] };
          inboxNotifications.push({ items: p.items ?? [] });
        }
        if (method === 'agent.stream') {
          const p = params as { frame?: { type?: string } };
          if (p.frame?.type === 'chunk') started.resolve();
        }
      });
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'slow turn' }],
      });
      await started.promise;
      const queued = (await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        mode: 'queue',
        contentBlocks: [{ type: 'text', text: 'queued original' }],
      })) as SessionPromptResult;
      let list = (await harness.transport.request('libadwaita/session/queue/list', {
        sessionId,
      })) as QueueListResult;
      const item = list.items.find((candidate) => candidate.id === queued.messageId);
      assert.ok(item);
      assert.equal(item.placement, 'queued');
      assert.deepEqual(item.content, [{ type: 'text', text: 'queued original' }]);
      await harness.transport.request('libadwaita/session/queue/edit', {
        sessionId,
        itemId: queued.messageId,
        content: [{ type: 'text', text: 'queued edited' }],
      });
      list = (await harness.transport.request('libadwaita/session/queue/list', {
        sessionId,
      })) as QueueListResult;
      assert.deepEqual(list.items.find((candidate) => candidate.id === queued.messageId)?.content, [
        { type: 'text', text: 'queued edited' },
      ]);
      const extra = (await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        mode: 'queue',
        contentBlocks: [{ type: 'text', text: 'remove me' }],
      })) as SessionPromptResult;
      await harness.transport.request('libadwaita/session/queue/remove', {
        sessionId,
        itemId: extra.messageId,
      });
      list = (await harness.transport.request('libadwaita/session/queue/list', {
        sessionId,
      })) as QueueListResult;
      assert.equal(
        list.items.some((candidate) => candidate.id === extra.messageId),
        false,
      );
      await harness.transport.request('libadwaita/session/queue/steer', {
        sessionId,
        itemId: queued.messageId,
      });
      list = (await harness.transport.request('libadwaita/session/queue/list', {
        sessionId,
      })) as QueueListResult;
      assert.notEqual(
        list.items.find((candidate) => candidate.id === queued.messageId)?.placement,
        'queued',
      );
      assert.ok(inboxNotifications.length > 0);
      release?.();
    } finally {
      release?.();
      await harness.cleanup();
    }
  });

  it('lists commands and toggles plan mode through command execution', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const list = (await harness.transport.request('libadwaita/command/list', {
        sessionId,
      })) as CommandListResult;
      assert.ok(list.commands.length > 0);
      assert.ok(list.commands.some((command) => command.name === 'plan'));
      const on = (await harness.transport.request('libadwaita/command/execute', {
        sessionId,
        command: 'plan',
      })) as CommandExecuteResult;
      assert.equal(on.kind, 'success');
      let followed = (await harness.transport.request('libadwaita/session/follow', {
        sessionId,
      })) as SessionFollowResult;
      assert.equal(followed.projections.planMode, true);
      const off = (await harness.transport.request('libadwaita/command/execute', {
        sessionId,
        command: 'plan',
        args: 'off',
      })) as CommandExecuteResult;
      assert.equal(off.kind, 'success');
      followed = (await harness.transport.request('libadwaita/session/follow', {
        sessionId,
      })) as SessionFollowResult;
      assert.equal(followed.projections.planMode, false);
      await assert.rejects(
        harness.transport.request('libadwaita/command/execute', {
          sessionId,
          command: 'missing-command',
        }),
        (error: unknown) => {
          if (error && typeof error === 'object' && 'code' in error) {
            assert.equal(error.code, -32016);
            return true;
          }
          return false;
        },
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('lists skills and workspace reference candidates', async () => {
    const harness = await createDshHarness(undefined, {
      prepareHome: (homeDir, workspaceDir) => {
        writeSkill(homeDir, 'gap-skill');
        fs.writeFileSync(path.join(workspaceDir, 'hello.txt'), 'hello\n');
      },
    });
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const skills = (await harness.transport.request('libadwaita/skill/list', {
        sessionId,
      })) as SkillListResult;
      assert.ok(skills.skills.some((skill) => skill.name === 'gap-skill'));
      const references = (await harness.transport.request('libadwaita/reference/list', {
        sessionId,
        query: 'hello',
      })) as ReferenceListResult;
      assert.ok(references.candidates.some((candidate) => candidate.path.includes('hello.txt')));
    } finally {
      await harness.cleanup();
    }
  });
});
