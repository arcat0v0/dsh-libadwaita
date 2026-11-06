import assert from 'node:assert/strict';
import fs from 'node:fs';
import type { IncomingMessage, ServerResponse } from 'node:http';
import path from 'node:path';
import { describe, it } from 'node:test';
import type {
  AttachmentReadResult,
  AttachmentUploadResult,
  CommandExecuteResult,
  JobFollowResult,
  JobListResult,
  MessageFeedbackDeleteResult,
  MessageFeedbackListResult,
  MessageFeedbackPutResult,
  QuestionReopenResult,
  SessionCreateResult,
  SessionFeedbackRecordResult,
  SessionProjectionsResult,
  SessionSearchResult,
  ToolResultUpdatedNotification,
} from '../../src/types.js';
import { createDshHarness, type DshTestHarness } from './harness.js';

const PNG_1X1 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAACXBIWXMAAAPoAAAD6AG1e1JrAAAADElEQVQImWNgZGIGAAAOAAeCcsnOAAAAAElFTkSuQmCC';

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
  harness.onNotification((method, params) => {
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

function expectBridgeCode(code: number) {
  return (error: unknown): boolean => {
    if (error && typeof error === 'object' && 'code' in error) {
      assert.equal(error.code, code);
      return true;
    }
    return false;
  };
}

function textThenToolResponder(toolName: string, toolArguments: string) {
  let call = 0;
  return (req: IncomingMessage, res: ServerResponse) => {
    call++;
    let body = '';
    req.on('data', (chunk) => {
      body += chunk.toString();
    });
    req.on('end', () => {
      const hasToolResult = body.includes('"tool_result"') || body.includes('"role":"tool"');
      const wantsTool =
        !hasToolResult &&
        (body.includes(`"name":"${toolName}"`) || body.includes(`"name": "${toolName}"`));
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
          id: `msg-${call}`,
          type: 'message',
          role: 'assistant',
          content: [],
          model: 'deepseek-chat',
          usage: { input_tokens: 10, output_tokens: 1 },
        },
      });
      if (wantsTool) {
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'tool_use', id: `call_${call}`, name: toolName, input: {} },
        });
        send('content_block_delta', {
          type: 'content_block_delta',
          index: 0,
          delta: { type: 'input_json_delta', partial_json: toolArguments },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'tool_use' },
          usage: { output_tokens: 5 },
        });
      } else {
        send('content_block_start', {
          type: 'content_block_start',
          index: 0,
          content_block: { type: 'text', text: '' },
        });
        send('content_block_delta', {
          type: 'content_block_delta',
          index: 0,
          delta: { type: 'text_delta', text: 'done' },
        });
        send('content_block_stop', { type: 'content_block_stop', index: 0 });
        send('message_delta', {
          type: 'message_delta',
          delta: { stop_reason: 'end_turn' },
          usage: { output_tokens: 5 },
        });
      }
      send('message_stop', { type: 'message_stop' });
      res.end();
    });
  };
}

describe('Gap batch C', { timeout: 60_000 }, () => {
  it('reads available projections and emits session.projection updates', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const projections: Array<{ kind: string; revision: number }> = [];
      harness.onNotification((method, params) => {
        if (method === 'session.projection') {
          projections.push(params as unknown as { kind: string; revision: number });
        }
      });
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'hello' }],
      });
      await done;
      const baseline = (await harness.transport.request('libadwaita/session/projections', {
        sessionId,
      })) as SessionProjectionsResult;
      assert.ok(baseline.asOfSeq >= 0);
      assert.ok(baseline.tokenUsage !== undefined);
      assert.ok(projections.some((entry) => entry.kind === 'tokenUsage'));
    } finally {
      await harness.cleanup();
    }
  });

  it('watches live tool results derived from tool/result events', async () => {
    const harness = await createDshHarness(
      textThenToolResponder('read', '{"file_path":"hello.txt"}'),
      {
        prepareHome: (_homeDir, workspaceDir) => {
          fs.writeFileSync(path.join(workspaceDir, 'hello.txt'), 'hello\n');
        },
      },
    );
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const updates: ToolResultUpdatedNotification[] = [];
      harness.onNotification((method, params) => {
        if (method === 'tool-result.updated') {
          updates.push(params as unknown as ToolResultUpdatedNotification);
        }
      });
      const subscribed = await harness.transport.request('libadwaita/tool-result/watch', {
        sessionId,
        subscribe: true,
      });
      assert.deepEqual(subscribed, { sessionId, subscribed: true });
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'read the file' }],
      });
      await done;
      assert.ok(updates.some((update) => update.sessionId === sessionId));
      assert.ok(updates.some((update) => update.data !== undefined));
      const unsubscribed = await harness.transport.request('libadwaita/tool-result/watch', {
        sessionId,
        subscribe: false,
      });
      assert.deepEqual(unsubscribed, { sessionId, subscribed: false });
    } finally {
      await harness.cleanup();
    }
  });

  it('records message feedback with CAS and session feedback', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'hello' }],
      });
      await done;
      const page = (await harness.transport.request('libadwaita/session/page', {
        sessionId,
        fromSeq: 0,
        limit: 200,
      })) as { events: Array<{ type: string; data: unknown }> };
      const assistant = page.events.find((event) => event.type === 'assistant/message');
      const messageId =
        assistant?.data && typeof assistant.data === 'object'
          ? String((assistant.data as { message?: { id?: string } }).message?.id ?? '')
          : '';
      assert.ok(messageId);
      const empty = (await harness.transport.request('libadwaita/message-feedback/list', {
        sessionId,
      })) as MessageFeedbackListResult;
      assert.deepEqual(empty.items, []);
      const put = (await harness.transport.request('libadwaita/message-feedback/put', {
        sessionId,
        messageId,
        rating: 'positive',
        note: 'nice',
        ifVersion: null,
      })) as MessageFeedbackPutResult;
      assert.equal(put.item.rating, 'positive');
      assert.equal(put.item.note, 'nice');
      const listed = (await harness.transport.request('libadwaita/message-feedback/list', {
        sessionId,
      })) as MessageFeedbackListResult;
      assert.equal(listed.items.length, 1);
      await assert.rejects(
        harness.transport.request('libadwaita/message-feedback/put', {
          sessionId,
          messageId,
          rating: 'negative',
          ifVersion: null,
        }),
        expectBridgeCode(-32031),
      );
      const deleted = (await harness.transport.request('libadwaita/message-feedback/delete', {
        sessionId,
        messageId,
        ifVersion: put.item.version,
      })) as MessageFeedbackDeleteResult;
      assert.deepEqual(deleted, { absent: true });
      const recorded = (await harness.transport.request('libadwaita/session-feedback/record', {
        sessionId,
        text: 'overall good',
      })) as SessionFeedbackRecordResult;
      assert.deepEqual(recorded, { recorded: true });
    } finally {
      await harness.cleanup();
    }
  });

  it('reopens a pending user question while the answerer is waiting', async () => {
    const harness = await createDshHarness(
      textThenToolResponder(
        'ask_user_question',
        '{"questions":[{"id":"q1","question":"Continue?","options":[{"label":"Yes"},{"label":"No"}]}]}',
      ),
    );
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      let reopened: QuestionReopenResult | undefined;
      harness.onRequest(async (method, _params) => {
        if (method === 'user-question/ask') {
          reopened = (await harness.transport.request('libadwaita/question/reopen', {
            sessionId,
          })) as QuestionReopenResult;
          return { cancelled: true };
        }
        return {};
      });
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'ask me' }],
      });
      await done;
      assert.ok(reopened);
      assert.equal(reopened?.request.sessionId, sessionId);
      assert.ok((reopened?.request.questions.length ?? 0) > 0);
      await assert.rejects(
        harness.transport.request('libadwaita/question/reopen', { sessionId }),
        expectBridgeCode(-32030),
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('lists, follows, and kills background jobs', async () => {
    const harness = await createDshHarness(
      textThenToolResponder(
        'bash',
        '{"description":"sleep","command":"sleep 3; echo hi","run_in_background":true}',
      ),
    );
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [{ type: 'text', text: 'run echo' }],
      });
      await done;
      const listed = (await harness.transport.request('libadwaita/job/list', {
        sessionId,
      })) as JobListResult;
      assert.ok(listed.jobs.length > 0);
      const job = listed.jobs[0];
      assert.ok(job?.id.startsWith('bash-'));
      const follow = (await harness.transport.request('libadwaita/job/follow', {
        sessionId,
        jobId: job.id,
      })) as JobFollowResult;
      assert.equal(follow.job.id, job.id);
      assert.ok(Array.isArray(follow.chunks));
      const kill = await harness.transport.request('libadwaita/job/kill', {
        sessionId,
        jobId: job.id,
      });
      assert.ok(
        (kill as { status?: string }).status === 'requested' ||
          (kill as { status?: string }).status === 'already-finished',
      );
      await assert.rejects(
        harness.transport.request('libadwaita/job/follow', {
          sessionId,
          jobId: 'bash-missing',
        }),
        expectBridgeCode(-32033),
      );
    } finally {
      await harness.cleanup();
    }
  });

  it('searches session metadata without full-text provider', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const first = await createSession(harness);
      const second = await createSession(harness);
      await harness.transport.request('libadwaita/session/rename', {
        sessionId: second,
        title: 'Alpha Search Target',
      });
      const found = (await harness.transport.request('libadwaita/session/search', {
        query: 'alpha search',
      })) as SessionSearchResult;
      assert.ok(found.items.some((item) => item.id === second));
      assert.equal(found.fullText, false);
      const cwdMatch = (await harness.transport.request('libadwaita/session/search', {
        query: harness.workspaceDir,
      })) as SessionSearchResult;
      assert.ok(cwdMatch.items.length >= 2);
      const missing = (await harness.transport.request('libadwaita/session/search', {
        query: 'zzz-no-match',
      })) as SessionSearchResult;
      assert.equal(missing.items.length, 0);
      assert.equal(first.length > 0, true);
    } finally {
      await harness.cleanup();
    }
  });

  it('stages attachments with limits and accepts an inline image prompt', async () => {
    const harness = await createDshHarness();
    try {
      await initialize(harness);
      const sessionId = await createSession(harness);
      const upload = (await harness.transport.request('libadwaita/attachment/upload', {
        sessionId,
        name: 'notes.txt',
        data: Buffer.from('attachment body\n').toString('base64'),
        mimeType: 'text/plain',
      })) as AttachmentUploadResult;
      assert.equal(upload.size, 16);
      const text = (await harness.transport.request('libadwaita/attachment/read', {
        sessionId,
        attachmentId: upload.attachmentId,
        encoding: 'text',
      })) as AttachmentReadResult;
      assert.equal(text.data, 'attachment body\n');
      const bytes = (await harness.transport.request('libadwaita/attachment/read', {
        sessionId,
        attachmentId: upload.attachmentId,
      })) as AttachmentReadResult;
      assert.equal(Buffer.from(bytes.data, 'base64').toString('utf8'), 'attachment body\n');
      await assert.rejects(
        harness.transport.request('libadwaita/attachment/upload', {
          sessionId,
          name: 'huge.bin',
          data: Buffer.alloc(17 * 1024 * 1024).toString('base64'),
        }),
        expectBridgeCode(-32036),
      );
      await assert.rejects(
        harness.transport.request('libadwaita/attachment/read', {
          sessionId,
          attachmentId: '../../../etc/passwd',
        }),
        expectBridgeCode(-32037),
      );
      await assert.rejects(
        harness.transport.request('libadwaita/attachment/read', {
          sessionId,
          attachmentId: '00000000-0000-0000-0000-000000000000',
        }),
        expectBridgeCode(-32035),
      );
      const done = waitForTurnEnd(harness);
      await harness.transport.request('libadwaita/session/prompt', {
        sessionId,
        contentBlocks: [
          { type: 'text', text: 'see image' },
          { type: 'image', data: PNG_1X1, mimeType: 'image/png', name: 'pixel.png' },
        ],
      });
      await done;
      assert.ok(true);
      const plan = (await harness.transport.request('libadwaita/command/execute', {
        sessionId,
        command: 'goal',
        args: 'attachment smoke goal',
      })) as CommandExecuteResult;
      assert.equal(plan.kind, 'success');
    } finally {
      await harness.cleanup();
    }
  });
});
