import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { type Context, Service } from '@deepseek-ai/cordis';
import { buildForkSeed } from '@deepseek-ai/dsh-session/fork';
import type { BridgeCredentials } from './credentials.js';
import { BridgeServer } from './server.js';
import {
  type AssistantStreamFrame,
  BRIDGE_ERROR_CODES,
  BridgeError,
  type CommandDescriptor,
  type CommandExecuteParams,
  type CommandExecuteResult,
  type CommandListParams,
  type CommandListResult,
  DSH_PINNED_VERSION,
  type FileEntry,
  type FilesListParams,
  type FilesListResult,
  type FilesReadBytesParams,
  type FilesReadBytesResult,
  type FilesReadParams,
  type FilesReadResult,
  type FilesStatParams,
  type FilesStatResult,
  type GoalEditParams,
  type GoalGetParams,
  type GoalGetResult,
  type GoalMutationParams,
  type GoalMutationResult,
  type GoalUpdatedNotification,
  type GoalView,
  type ModelCatalogResult,
  type PermissionCatalogResult,
  type PermissionGetParams,
  type PermissionGetResult,
  type PermissionSetParams,
  type PermissionSetResult,
  type PermissionUpdatedNotification,
  type PluginListResult,
  type PluginToggleResult,
  type QueueEditParams,
  type QueueItem,
  type QueueListParams,
  type QueueListResult,
  type QueueMutationResult,
  type QueueRemoveParams,
  type QueueSteerParams,
  type ReferenceListParams,
  type ReferenceListResult,
  type SdkPromptContentBlock,
  type SdkUserQuestionRequest,
  type SessionArchiveParams,
  type SessionArchiveResult,
  type SessionAssistantBaseline,
  type SessionCancelParams,
  type SessionCancelResult,
  type SessionCreateParams,
  type SessionCreateResult,
  type SessionEvent,
  type SessionFollowParams,
  type SessionFollowResult,
  type SessionForkParams,
  type SessionForkResult,
  type SessionHeader,
  type SessionListItem,
  type SessionListParams,
  type SessionListResult,
  type SessionModelGetParams,
  type SessionModelGetResult,
  type SessionModelSetParams,
  type SessionPageParams,
  type SessionPageResult,
  type SessionProjectionsSnapshot,
  type SessionPromptParams,
  type SessionPromptResult,
  type SessionRenameParams,
  type SessionRenameResult,
  type SessionTitleSnapshot,
  type SessionWorkdirGetParams,
  type SessionWorkdirGetResult,
  type SettingsDescribeParams,
  type SettingsDescribeResult,
  type SettingsDescriptor,
  type SettingsGetParams,
  type SettingsGetResult,
  type SettingsMutateParams,
  type SettingsMutateResult,
  type SettingsUpdatedNotification,
  type SkillListParams,
  type SkillListResult,
  type StreamChunk,
  type TodoItem,
  type WorkspaceChangesParams,
  type WorkspaceChangesResult,
  type WorkspaceCreateParams,
  type WorkspaceCreateResult,
  type WorkspaceListResult,
  type WorkspaceRemoveParams,
  type WorkspaceRemoveResult,
  type WorkspaceRenameParams,
  type WorkspaceRenameResult,
  type WorkspaceView,
} from './types.js';

function latestCompletedPrefixBoundary(events: readonly SessionEvent[]): number | undefined {
  const lastTurnEnd = events.findLast((event) => event.type === 'turn/end');
  if (!lastTurnEnd) return undefined;
  let boundary = lastTurnEnd.seq;
  for (const next of events.slice(boundary + 1)) {
    if (
      next.type === 'turn/start' ||
      (next.type === 'user/message' && next.surfaceOp === 'append') ||
      next.type === 'agent/inbox/spliced'
    ) {
      break;
    }
    boundary = next.seq;
  }
  return boundary;
}

function incrementedTitle(title: string): string {
  const ascii = /^(.*?)\((\d+)\)$/u.exec(title);
  if (ascii) return `${ascii[1]}(${Number(ascii[2]) + 1})`;
  const fullWidth = /^(.*?)（(\d+)）$/u.exec(title);
  if (fullWidth) return `${fullWidth[1]}（${Number(fullWidth[2]) + 1}）`;
  return `${title} (1)`;
}

function increasedForkTitle(title: string, taken: ReadonlySet<string>): string {
  let candidate = incrementedTitle(title);
  while (taken.has(candidate)) candidate = incrementedTitle(candidate);
  return candidate;
}

interface PendingMessage {
  id: string;
  source?: { kind?: string };
  content?: unknown[];
  [key: string]: unknown;
}

interface SessionLike {
  id: string;
  header: SessionHeader;
  snapshotEvents: () => SessionEvent[];
}

interface AgentLike {
  id: string;
  status: 'idle' | 'running';
  session: SessionLike;
  model?: {
    current: () => { provider?: string; model?: string; reasoningEffort?: string } | undefined;
    select: (options: object) => void;
  };
  inbox: {
    nextTurn: readonly PendingMessage[];
    nextStep: readonly PendingMessage[];
    replace: (messageId: string, message: object) => boolean;
    remove: (messageId: string) => boolean;
    clear: () => void;
  };
  steer: (message: object) => void;
  followup: (message: object) => void;
  cancel: (cause: object, options?: object) => void;
}

interface WorkspaceRecordLike {
  id: string;
  path: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  sessionIds: readonly string[];
  setTitle: (title: string) => Promise<void>;
  status: () => Promise<'ok' | 'missing-dir'>;
}

interface GoalServiceLike {
  get: (agent: AgentLike) => GoalView | undefined;
  edit: (agent: AgentLike, ref: object, request: object) => GoalView;
  pause: (agent: AgentLike, ref: object) => GoalView;
  resume: (agent: AgentLike, ref: object) => GoalView;
  clear: (agent: AgentLike, ref: object) => object;
}

export interface BridgeConfig {
  socketPath?: string;
  generationId?: string;
}

interface ActiveTurnState {
  attemptId: string;
  turn: number;
  step: number;
  revision: number;
  chunks: StreamChunk[];
}

export class BridgeService extends Service {
  static inject = ['agents', 'sessions'];

  readonly generationId: string;
  readonly server: BridgeServer;
  private readonly followedSessions = new Set<string>();
  private readonly activeTurnBaselines = new Map<string, ActiveTurnState>();
  private readonly archivedSessions = new Set<string>();

  constructor(ctx: Context, config: BridgeConfig = {}) {
    super(ctx, 'bridge');
    this.generationId = config.generationId ?? `gen-${crypto.randomUUID()}`;
    const socketPath =
      config.socketPath ??
      process.env.DSH_LIBADWAITA_SOCKET ??
      `${process.env.XDG_RUNTIME_DIR ?? '/tmp'}/dsh-libadwaita/bridge.sock`;

    this.server = new BridgeServer({
      socketPath,
      generationId: this.generationId,
      dshVersion: DSH_PINNED_VERSION,
      onMethod: (method, params) => this.dispatchMethod(method, params),
    });
  }

  protected async [Service.init](): Promise<void> {
    await this.server.start();

    this.ctx.on('agent/assistant-stream', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as { agent?: { session?: { id?: string } }; frame?: AssistantStreamFrame };
      const sessionId = p.agent?.session?.id;
      const frame = p.frame;
      if (!sessionId || !frame) return;

      if (frame.type === 'start') {
        this.activeTurnBaselines.set(sessionId, {
          attemptId: frame.attemptId,
          turn: Number(frame.turn ?? 0),
          step: Number(frame.step ?? 0),
          revision: frame.revision,
          chunks: [],
        });
      } else if (frame.type === 'chunk') {
        const baseline = this.activeTurnBaselines.get(sessionId);
        if (baseline && baseline.attemptId === frame.attemptId) {
          baseline.revision = frame.revision;
          if (frame.chunk) {
            baseline.chunks.push(frame.chunk as StreamChunk);
          }
        }
      } else if (frame.type === 'end') {
        this.activeTurnBaselines.delete(sessionId);
      }

      if (this.followedSessions.has(sessionId)) {
        this.server.notify('agent.stream', {
          sessionId,
          frame,
        });
      }
    });

    this.ctx.on('session/event', (session: unknown, event: unknown) => {
      const s = session as { id?: string };
      const ev = event as SessionEvent;
      if (!s?.id || !ev) return;

      if (this.followedSessions.has(s.id)) {
        this.server.notify('session.event', {
          sessionId: s.id,
          event: ev,
        });

        if (ev.type === 'session/title' && ev.data && typeof ev.data === 'object') {
          const titleData = ev.data as { title: string; source: unknown };
          this.server.notify('session.title', {
            sessionId: s.id,
            title: titleData.title,
            source: titleData.source,
          });
        }

        if (ev.type === 'agent/inbox/spliced') {
          const agents = this.ctx.get('agents') as unknown as {
            get: (id: string) => AgentLike | undefined;
          };
          const agent = agents?.get(s.id);
          if (agent) {
            this.server.notify('session.inbox', {
              sessionId: s.id,
              items: this.buildQueueItems(agent),
            });
          }
        }

        if (ev.type === 'goal/change') {
          const agents = this.ctx.get('agents') as unknown as {
            get: (id: string) => AgentLike | undefined;
          };
          const goals = this.ctx.get('goals') as unknown as {
            get: (agent: AgentLike) => GoalView | undefined;
          };
          const agent = agents?.get(s.id);
          if (agent) {
            const notification: GoalUpdatedNotification = {
              sessionId: s.id,
              goal: goals?.get(agent) ?? null,
            };
            this.server.notify('goal.updated', notification);
          }
        }

        if (
          ev.type === 'permission/preset' ||
          ev.type === 'sandbox/mode' ||
          ev.type === 'approval/policy'
        ) {
          const sessions = this.ctx.get('sessions') as unknown as {
            get: (id: string) => SessionLike | undefined;
          };
          const permissions = this.ctx.get('permissionPresets') as unknown as {
            current: (session: SessionLike) => string;
          };
          const current = sessions?.get(s.id);
          if (current && permissions?.current) {
            const notification: PermissionUpdatedNotification = {
              sessionId: s.id,
              currentValue: permissions.current(current),
            };
            this.server.notify('permission.updated', notification);
          }
        }
      }
    });

    this.ctx.on('settings/document-updated', (namespace: string, revision: number) => {
      const notification: SettingsUpdatedNotification = { namespace, revision };
      this.server.notify('settings.updated', notification);
    });

    this.ctx.on('goal/activation-changed', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as {
        sessionId?: string;
        goal?: { id?: string; revision?: number; activation?: string };
      };
      if (!p.sessionId) return;
      const agents = this.ctx.get('agents') as unknown as {
        get: (id: string) => AgentLike | undefined;
      };
      const goals = this.ctx.get('goals') as unknown as {
        get: (agent: AgentLike) => GoalView | undefined;
      };
      const agent = agents?.get(p.sessionId);
      if (agent) {
        const notification: GoalUpdatedNotification = {
          sessionId: p.sessionId,
          goal: goals?.get(agent) ?? null,
        };
        this.server.notify('goal.updated', notification);
      }
    });

    this.ctx.on('agent/status', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as { agent?: { session?: { id?: string } }; status?: 'idle' | 'running' };
      const sessionId = p.agent?.session?.id;
      const status = p.status;
      if (!sessionId || !status) return;

      if (this.followedSessions.has(sessionId)) {
        this.server.notify('session.status', {
          sessionId,
          status,
        });
      }
    });

    this.ctx.on('approval/request', async (req: unknown) => {
      if (!req || typeof req !== 'object') {
        return 'rejected';
      }
      const r = req as {
        agent?: { session?: { id?: string } };
        toolName?: string;
        callId?: string;
        reason?: string;
        signal?: AbortSignal;
      };
      const sessionId = r.agent?.session?.id ?? '';
      const decision = await this.server.requestApproval(
        {
          sessionId,
          toolName: r.toolName ?? 'unknown',
          callId: r.callId,
          reason: r.reason,
        },
        300_000,
        r.signal,
      );
      return decision.outcome;
    });

    this.ctx.on('user-questions/request', async (req: unknown) => {
      if (!req || typeof req !== 'object') {
        throw new Error('user-questions request is invalid');
      }
      const r = req as {
        agent?: { session?: { id?: string } };
        questions?: unknown[];
        signal?: AbortSignal;
      };
      const sessionId = r.agent?.session?.id ?? '';
      const requestId = `q-${crypto.randomUUID()}`;

      const res = await this.server.requestUserQuestion(
        {
          requestId,
          sessionId,
          questions: (r.questions ?? []) as SdkUserQuestionRequest['questions'],
        },
        300_000,
        r.signal,
      );

      if ('answers' in res) {
        return { answers: res.answers };
      }

      const cancelErr = new Error('plan review was cancelled');
      (cancelErr as unknown as { code: string }).code = 'ASK_CANCELLED';
      (cancelErr as unknown as { name: string }).name = 'UserQuestionError';
      throw cancelErr;
    });

    this.ctx.effect(() => {
      return () => {
        void this.server.close();
      };
    }, 'bridge.server.lifecycle');
  }

  private async dispatchMethod(method: string, params: Record<string, unknown>): Promise<unknown> {
    switch (method) {
      case 'libadwaita/shutdown':
        return this.handleShutdown();
      case 'libadwaita/session/create':
        return this.handleSessionCreate(params as unknown as SessionCreateParams);
      case 'libadwaita/session/list':
        return this.handleSessionList(params as unknown as SessionListParams);
      case 'libadwaita/session/follow':
        return this.handleSessionFollow(params as unknown as SessionFollowParams);
      case 'libadwaita/session/page':
        return this.handleSessionPage(params as unknown as SessionPageParams);
      case 'libadwaita/session/prompt':
        return this.handleSessionPrompt(params as unknown as SessionPromptParams);
      case 'libadwaita/session/cancel':
        return this.handleSessionCancel(params as unknown as SessionCancelParams);
      case 'libadwaita/session/release':
        return this.handleSessionRelease(params);
      case 'libadwaita/session/archive':
        return this.handleSessionArchive(params as unknown as SessionArchiveParams);
      case 'libadwaita/session/workdir/get':
        return this.handleSessionWorkdirGet(params as unknown as SessionWorkdirGetParams);
      case 'libadwaita/session/model/get':
        return this.handleSessionModelGet(params as unknown as SessionModelGetParams);
      case 'libadwaita/session/model/set':
        return this.handleSessionModelSet(params as unknown as SessionModelSetParams);
      case 'libadwaita/session/rename':
        return this.handleSessionRename(params as unknown as SessionRenameParams);
      case 'libadwaita/session/fork':
        return this.handleSessionFork(params as unknown as SessionForkParams);
      case 'libadwaita/session/queue/list':
        return this.handleQueueList(params as unknown as QueueListParams);
      case 'libadwaita/session/queue/edit':
        return this.handleQueueEdit(params as unknown as QueueEditParams);
      case 'libadwaita/session/queue/remove':
        return this.handleQueueRemove(params as unknown as QueueRemoveParams);
      case 'libadwaita/session/queue/steer':
        return this.handleQueueSteer(params as unknown as QueueSteerParams);
      case 'libadwaita/command/list':
        return this.handleCommandList(params as unknown as CommandListParams);
      case 'libadwaita/command/execute':
        return this.handleCommandExecute(params as unknown as CommandExecuteParams);
      case 'libadwaita/skill/list':
        return this.handleSkillList(params as unknown as SkillListParams);
      case 'libadwaita/reference/list':
        return this.handleReferenceList(params as unknown as ReferenceListParams);
      case 'libadwaita/model/catalog':
        return this.handleModelCatalog();
      case 'libadwaita/settings/get':
        return this.handleSettingsGet(params as unknown as SettingsGetParams);
      case 'libadwaita/settings/describe':
        return this.handleSettingsDescribe(params as unknown as SettingsDescribeParams);
      case 'libadwaita/settings/mutate':
        return this.handleSettingsMutate(params as unknown as SettingsMutateParams);
      case 'libadwaita/workspace/list':
        return this.handleWorkspaceList();
      case 'libadwaita/workspace/create':
        return this.handleWorkspaceCreate(params as unknown as WorkspaceCreateParams);
      case 'libadwaita/workspace/rename':
        return this.handleWorkspaceRename(params as unknown as WorkspaceRenameParams);
      case 'libadwaita/workspace/remove':
        return this.handleWorkspaceRemove(params as unknown as WorkspaceRemoveParams);
      case 'libadwaita/permission/catalog':
        return this.handlePermissionCatalog();
      case 'libadwaita/permission/get':
        return this.handlePermissionGet(params as unknown as PermissionGetParams);
      case 'libadwaita/permission/set':
        return this.handlePermissionSet(params as unknown as PermissionSetParams);
      case 'libadwaita/goal/get':
        return this.handleGoalGet(params as unknown as GoalGetParams);
      case 'libadwaita/goal/edit':
        return this.handleGoalEdit(params as unknown as GoalEditParams);
      case 'libadwaita/goal/pause':
        return this.handleGoalMutation('pause', params as unknown as GoalMutationParams);
      case 'libadwaita/goal/resume':
        return this.handleGoalMutation('resume', params as unknown as GoalMutationParams);
      case 'libadwaita/goal/clear':
        return this.handleGoalMutation('clear', params as unknown as GoalMutationParams);
      case 'libadwaita/files/list':
        return this.handleFilesList(params as unknown as FilesListParams);
      case 'libadwaita/files/stat':
        return this.handleFilesStat(params as unknown as FilesStatParams);
      case 'libadwaita/files/read':
        return this.handleFilesRead(params as unknown as FilesReadParams);
      case 'libadwaita/files/readBytes':
        return this.handleFilesReadBytes(params as unknown as FilesReadBytesParams);
      case 'libadwaita/plugin/list':
        return this.handlePluginList();
      case 'libadwaita/plugin/enable':
        return this.handlePluginEnable(params as unknown as { id: string });
      case 'libadwaita/plugin/disable':
        return this.handlePluginDisable(params as unknown as { id: string });
      case 'libadwaita/credentials/status':
        return this.handleCredentialsStatus();
      case 'libadwaita/credentials/inject':
        return this.handleCredentialsInject(
          params as unknown as { provider: string; key: string; baseURL?: string },
        );
      case 'libadwaita/credentials/rotate':
        return this.handleCredentialsRotate(params as unknown as { provider: string; key: string });
      case 'libadwaita/credentials/remove':
        return this.handleCredentialsRemove(params as unknown as { provider: string });
      case 'libadwaita/workspace/changes':
        return this.handleWorkspaceChanges(params as unknown as WorkspaceChangesParams);
      default:
        throw new BridgeError(-32601, `method not found: ${method}`);
    }
  }

  private async handleShutdown(): Promise<{ ok: boolean }> {
    queueMicrotask(() => {
      void this.server.close().then(() => {
        process.exit(0);
      });
    });
    return { ok: true };
  }

  private async handleSessionCreate(params: SessionCreateParams): Promise<SessionCreateResult> {
    const agents = this.ctx.get('agents') as unknown as {
      create: (
        options: object,
      ) => Promise<{ agent: { id: string; session: { header: SessionHeader } } }>;
    };
    if (!agents) {
      throw new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, 'agents service not available');
    }

    const sessionId = params.sessionId ?? `sess-${crypto.randomUUID()}`;
    const handle = await agents.create({
      sessionId,
      meta: {
        cwd: params.cwd,
        ...(params.agentPreset ? { agentPreset: params.agentPreset } : {}),
      },
    });

    const agent = handle.agent;
    const provider = params.provider ?? 'deepseek-official';
    const model = params.model ?? 'deepseek-chat';
    const modelService = (agent as unknown as { model?: { select: (opt: object) => void } }).model;
    modelService?.select({ provider, model });
    return {
      sessionId: agent.id,
      header: agent.session.header,
    };
  }

  private async handleSessionList(params: SessionListParams): Promise<SessionListResult> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      list: () => Array<{
        id: string;
        header: SessionHeader;
        snapshotEvents: () => SessionEvent[];
      }>;
    };

    if (!sessionsService) {
      return { sessions: [] };
    }

    const includeArchived = params.archived ?? false;
    const limit = params.limit ?? 100;
    const all = sessionsService.list();
    const items: SessionListItem[] = [];

    for (const session of all) {
      const isArchived = this.archivedSessions.has(session.id);
      if (isArchived && !includeArchived) continue;

      const events = session.snapshotEvents();
      const lastEvent = events.at(-1);
      const titleEvent = events.findLast((e) => e.type === 'session/title');
      const title =
        titleEvent?.data && typeof titleEvent.data === 'object' && 'title' in titleEvent.data
          ? String((titleEvent.data as { title: unknown }).title)
          : session.header.cwd
            ? `Session in ${session.header.cwd}`
            : 'New Session';

      const turnStarts = events.filter((e) => e.type === 'turn/start').length;

      items.push({
        id: session.id,
        createdAt: session.header.createdAt,
        updatedAt: lastEvent ? lastEvent.time : session.header.createdAt,
        cwd: session.header.cwd ?? '',
        title,
        turnCount: turnStarts,
        isArchived,
      });

      if (items.length >= limit) break;
    }

    items.sort((a, b) => b.updatedAt - a.updatedAt);
    return { sessions: items };
  }

  private async handleSessionFollow(params: SessionFollowParams): Promise<SessionFollowResult> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      get: (
        id: string,
      ) => { header: SessionHeader; snapshotEvents: () => SessionEvent[] } | undefined;
    };
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) => { status: 'idle' | 'running' } | undefined;
      resume?: (opt: object) => Promise<{
        agent: { session: { header: SessionHeader; snapshotEvents: () => SessionEvent[] } };
      }>;
    };

    let session = sessionsService?.get(params.sessionId);
    if (!session && agentsService?.resume) {
      try {
        const handle = await agentsService.resume({ resumeSessionId: params.sessionId });
        session = handle.agent.session;
      } catch {}
    }
    if (!session) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    this.followedSessions.add(params.sessionId);

    const agent = agentsService?.get(params.sessionId);
    const status = agent?.status ?? 'idle';
    const events = session.snapshotEvents();
    const latestSeq = events.length > 0 ? events[events.length - 1]!.seq : -1;

    const titleEvent = events.findLast((e) => e.type === 'session/title');
    let title: SessionTitleSnapshot | null = null;
    if (titleEvent?.data && typeof titleEvent.data === 'object' && 'title' in titleEvent.data) {
      const td = titleEvent.data as { title: string; source: unknown };
      title = {
        title: td.title,
        source: td.source as SessionTitleSnapshot['source'],
      };
    }

    let planMode = false;
    const planEvent = events.findLast((e) => e.type === 'plan/mode');
    if (planEvent?.data && typeof planEvent.data === 'object' && 'active' in planEvent.data) {
      planMode = Boolean((planEvent.data as { active: unknown }).active);
    }

    let todos: TodoItem[] | null = null;
    const todoEvent = events.findLast((e) => e.type === 'todo/write');
    if (todoEvent?.data && typeof todoEvent.data === 'object' && 'todos' in todoEvent.data) {
      todos = (todoEvent.data as { todos: TodoItem[] }).todos;
    }

    const projections: SessionProjectionsSnapshot = {
      planMode,
      todos,
    };

    let assistantBaseline: SessionAssistantBaseline | null = null;
    if (status === 'running') {
      const active = this.activeTurnBaselines.get(params.sessionId);
      if (active) {
        assistantBaseline = {
          attemptId: active.attemptId,
          turn: active.turn,
          step: active.step,
          revision: active.revision,
          chunks: [...active.chunks],
        };
      }
    }

    return {
      header: session.header,
      latestSeq,
      status,
      title,
      projections,
      assistantBaseline,
    };
  }

  private async handleSessionPage(params: SessionPageParams): Promise<SessionPageResult> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      get: (id: string) => { snapshotEvents: () => SessionEvent[] } | undefined;
    };
    const session = sessionsService?.get(params.sessionId);
    if (!session) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    const events = session.snapshotEvents();
    const limit = Math.min(params.limit ?? 50, 200);
    const toSeq = params.toSeq ?? Number.MAX_SAFE_INTEGER;

    const filtered = events.filter((e) => e.seq >= params.fromSeq && e.seq <= toSeq);
    const slice = filtered.slice(0, limit);
    const hasMore = filtered.length > limit;

    return {
      events: slice,
      hasMore,
    };
  }

  private async handleSessionPrompt(params: SessionPromptParams): Promise<SessionPromptResult> {
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) =>
        | {
            status: string;
            followup: (msg: object) => void;
            steer: (msg: object) => void;
          }
        | undefined;
    };
    const agent = agentsService?.get(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    const messageId = `msg-${crypto.randomUUID()}`;
    const blocks: SdkPromptContentBlock[] = params.contentBlocks.map((b) => {
      if (b.type === 'text' && 'text' in b) {
        return { type: 'text', text: String(b.text) };
      }
      return b;
    });

    const userMessage = {
      id: messageId,
      role: 'user',
      content: blocks,
      source: { kind: 'user' },
    };

    if (params.mode === 'steer') {
      agent.steer(userMessage);
    } else {
      agent.followup(userMessage);
    }

    return { messageId };
  }

  private async handleSessionCancel(params: SessionCancelParams): Promise<SessionCancelResult> {
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) =>
        | {
            status: string;
            cancel: (cause: object, options?: object) => void;
          }
        | undefined;
    };
    const agent = agentsService?.get(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    if (agent.status === 'running') {
      agent.cancel({ kind: 'user' }, { keepInbox: true });
      return { cancelled: true };
    }

    return { cancelled: false };
  }

  private async handleSessionRelease(params: Record<string, unknown>): Promise<{ ok: boolean }> {
    const sessionId = String(params.sessionId ?? '');
    this.followedSessions.delete(sessionId);
    this.activeTurnBaselines.delete(sessionId);
    return { ok: true };
  }

  private async handleSessionArchive(params: SessionArchiveParams): Promise<SessionArchiveResult> {
    const sessionId = params.sessionId;
    if (params.archived) {
      this.archivedSessions.add(sessionId);
      const agentsService = this.ctx.get('agents') as unknown as {
        get: (id: string) =>
          | {
              status: string;
              cancel: (cause: object, options?: object) => void;
              inbox?: { clear: () => void };
            }
          | undefined;
      };
      const agent = agentsService?.get(sessionId);
      if (agent) {
        if (agent.status === 'running') {
          agent.cancel({ kind: 'user' });
        }
        agent.inbox?.clear();
      }
    } else {
      this.archivedSessions.delete(sessionId);
    }
    return {
      sessionId,
      archived: params.archived,
    };
  }

  private async resolveSessionEntry(
    sessionId: string,
  ): Promise<{ session: SessionLike; agent?: AgentLike }> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      get: (id: string) => SessionLike | undefined;
    };
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) => AgentLike | undefined;
      resume?: (options: object) => Promise<{ agent: AgentLike }>;
    };
    let session = sessionsService?.get(sessionId);
    let agent = agentsService?.get(sessionId);
    if (!session && agentsService?.resume) {
      try {
        const handle = await agentsService.resume({ resumeSessionId: sessionId });
        session = handle.agent.session;
        agent = handle.agent;
      } catch {}
    }
    if (!session) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${sessionId}`,
      );
    }
    return { session, agent };
  }

  private buildQueueItems(agent: AgentLike): QueueItem[] {
    const items: QueueItem[] = [];
    for (const message of agent.inbox.nextTurn) {
      items.push({
        id: message.id,
        placement: 'queued',
        content: Array.isArray(message.content) ? message.content : [],
      });
    }
    for (const message of agent.inbox.nextStep) {
      items.push({
        id: message.id,
        placement: message.source?.kind === 'user' ? 'steering' : 'context',
        content: Array.isArray(message.content) ? message.content : [],
      });
    }
    return items;
  }

  private async handleSessionRename(params: SessionRenameParams): Promise<SessionRenameResult> {
    if (!params.sessionId || typeof params.title !== 'string') {
      throw new BridgeError(-32602, 'session/rename requires sessionId and title');
    }
    const { session } = await this.resolveSessionEntry(params.sessionId);
    const titleService = this.ctx.get('sessionTitle') as unknown as {
      rename: (session: SessionLike, title: string) => SessionTitleSnapshot;
    };
    if (!titleService?.rename) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'session title service not available',
      );
    }
    try {
      const snapshot = titleService.rename(session, params.title);
      return {
        sessionId: params.sessionId,
        title: snapshot.title,
        source: snapshot.source,
      };
    } catch (error) {
      if (error instanceof Error && error.name === 'SessionTitleInvalidError') {
        throw new BridgeError(BRIDGE_ERROR_CODES.TITLE_INVALID, error.message);
      }
      throw error;
    }
  }

  private async handleSessionFork(params: SessionForkParams): Promise<SessionForkResult> {
    const { session } = await this.resolveSessionEntry(params.sessionId);
    const events = session.snapshotEvents();
    const atSeq = params.atSeq;
    if (atSeq !== undefined && (!Number.isSafeInteger(atSeq) || atSeq < 0)) {
      throw new BridgeError(-32602, 'atSeq must be a non-negative safe integer');
    }
    const boundary = atSeq ?? latestCompletedPrefixBoundary(events);
    if (boundary === undefined || boundary < 0 || events[boundary]?.seq !== boundary) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FORK_UNAVAILABLE,
        atSeq === undefined
          ? `session "${params.sessionId}" has no completed turn to fork from`
          : `event ${String(atSeq)} does not exist in session "${params.sessionId}"`,
        { sessionId: params.sessionId },
      );
    }
    const seed = buildForkSeed(events as never, boundary as never);
    const childId = `sess-${crypto.randomUUID()}`;
    const agentsService = this.ctx.get('agents') as unknown as {
      create: (options: object) => Promise<{ agent: AgentLike }>;
    };
    const defaults = this.ctx.get('agentDefaultModel') as unknown as {
      currentSelection?: () => { provider?: string; model?: string; reasoningEffort?: string };
    };
    const selection = defaults?.currentSelection?.() ?? {
      provider: 'deepseek-official',
      model: 'deepseek-chat',
    };
    let handle: { agent: AgentLike };
    try {
      handle = await agentsService.create({
        sessionId: childId,
        seed,
        inheritedEventCount: boundary + 1,
        meta: {
          ...(session.header.cwd === undefined ? {} : { cwd: session.header.cwd }),
          isSeeded: true,
        },
        agentOptions: selection,
      });
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FORK_UNAVAILABLE,
        error instanceof Error ? error.message : String(error),
        { sessionId: params.sessionId },
      );
    }
    if (params.increaseTitle) {
      const titleService = this.ctx.get('sessionTitle') as unknown as {
        get: (session: SessionLike) => SessionTitleSnapshot | undefined;
        rename: (session: SessionLike, title: string) => SessionTitleSnapshot;
      };
      const sourceTitle = titleService?.get?.(session)?.title;
      if (sourceTitle && titleService?.rename) {
        const sessionsService = this.ctx.get('sessions') as unknown as {
          list: () => SessionLike[];
        };
        const taken = new Set<string>();
        for (const candidate of sessionsService?.list() ?? []) {
          const title = titleService.get?.(candidate)?.title;
          if (title) taken.add(title);
        }
        try {
          titleService.rename(handle.agent.session, increasedForkTitle(sourceTitle, taken));
        } catch {}
      }
    }
    return {
      sessionId: childId,
      header: handle.agent.session.header,
    };
  }

  private async handleQueueList(params: QueueListParams): Promise<QueueListResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    return { items: this.buildQueueItems(agent) };
  }

  private async handleQueueEdit(params: QueueEditParams): Promise<QueueMutationResult> {
    if (!Array.isArray(params.content) || params.content.some((block) => block.type !== 'text')) {
      throw new BridgeError(-32602, 'queue edits accept text content only');
    }
    const text = params.content.map((block) => block.text).join('');
    if (text.trim().length === 0) {
      throw new BridgeError(-32602, 'queue edit content must include non-whitespace text');
    }
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    const message = [...agent.inbox.nextTurn, ...agent.inbox.nextStep].find(
      (candidate) => candidate.id === params.itemId,
    );
    if (!message) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.QUEUE_ITEM_NOT_FOUND,
        'queued item is no longer pending',
        { itemId: params.itemId },
      );
    }
    const replaced = agent.inbox.replace(params.itemId, {
      ...message,
      content: params.content,
    });
    if (!replaced) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.QUEUE_ITEM_NOT_FOUND,
        'queued item is no longer pending',
        { itemId: params.itemId },
      );
    }
    return { accepted: true };
  }

  private async handleQueueRemove(params: QueueRemoveParams): Promise<QueueMutationResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    if (!agent.inbox.remove(params.itemId)) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.QUEUE_ITEM_NOT_FOUND,
        'queued item is no longer pending',
        { itemId: params.itemId },
      );
    }
    return { accepted: true };
  }

  private async handleQueueSteer(params: QueueSteerParams): Promise<QueueMutationResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    const message = agent.inbox.nextTurn.find((candidate) => candidate.id === params.itemId);
    if (!message || agent.status !== 'running') {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.QUEUE_STEER_UNAVAILABLE,
        'current turn no longer accepts steering',
        { itemId: params.itemId },
      );
    }
    agent.inbox.remove(params.itemId);
    agent.steer(message);
    return { accepted: true };
  }

  private async handleCommandList(params: CommandListParams): Promise<CommandListResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    const commands = this.ctx.get('commands') as unknown as {
      list: (agent: AgentLike) => readonly CommandDescriptor[];
    };
    if (!commands?.list) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.COMMANDS_UNAVAILABLE,
        'commands service not available',
      );
    }
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    return {
      commands: commands.list(agent).map((command) => ({
        name: command.name,
        description: command.description,
        ...(command.input
          ? {
              input: {
                hint: command.input.hint,
                ...(command.input.attachments === undefined
                  ? {}
                  : { attachments: command.input.attachments }),
              },
            }
          : {}),
      })),
    };
  }

  private async handleCommandExecute(params: CommandExecuteParams): Promise<CommandExecuteResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    const commands = this.ctx.get('commands') as unknown as {
      execute: (
        agent: AgentLike,
        line: string,
        attachments: readonly object[],
        signal: AbortSignal,
      ) => Promise<
        | {
            commandId: string;
            result: { kind: 'success' | 'error'; text?: string; sourceEventSeq?: number };
          }
        | undefined
      >;
    };
    if (!commands?.execute) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.COMMANDS_UNAVAILABLE,
        'commands service not available',
      );
    }
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    const command = params.command.startsWith('/') ? params.command.slice(1) : params.command;
    const line = params.args ? `/${command} ${params.args}` : `/${command}`;
    const execution = await commands.execute(agent, line, [], new AbortController().signal);
    if (!execution) {
      throw new BridgeError(BRIDGE_ERROR_CODES.COMMAND_NOT_FOUND, `unknown command: ${line}`);
    }
    return {
      commandId: execution.commandId,
      kind: execution.result.kind,
      ...(execution.result.text === undefined ? {} : { text: execution.result.text }),
      ...(execution.result.sourceEventSeq === undefined
        ? {}
        : { sourceEventSeq: execution.result.sourceEventSeq }),
    };
  }

  private async handleSkillList(params: SkillListParams): Promise<SkillListResult> {
    const skills = this.ctx.get('skills') as unknown as {
      snapshot: (options: object) => Promise<{
        skills: Array<{
          name: string;
          description: string;
          whenToUse?: string;
          path?: string;
          source: string;
          provider: string;
          invocation: { modelInvocable: boolean; userInvocable: boolean };
        }>;
        complete: boolean;
      }>;
    };
    if (!skills?.snapshot) {
      throw new BridgeError(BRIDGE_ERROR_CODES.SKILLS_UNAVAILABLE, 'skills service not available');
    }
    let cwd = params.cwd;
    if (cwd === undefined && params.sessionId) {
      const { session } = await this.resolveSessionEntry(params.sessionId);
      cwd = session.header.cwd;
    }
    const snapshot = await skills.snapshot(cwd === undefined ? {} : { cwd });
    return {
      skills: snapshot.skills.map((skill) => ({
        name: skill.name,
        description: skill.description,
        ...(skill.whenToUse === undefined ? {} : { whenToUse: skill.whenToUse }),
        ...(skill.path === undefined ? {} : { path: skill.path }),
        source: skill.source,
        provider: skill.provider,
        invocation: skill.invocation,
      })),
      complete: snapshot.complete,
    };
  }

  private async handleReferenceList(params: ReferenceListParams): Promise<ReferenceListResult> {
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    const references = this.ctx.get('fileReferences') as unknown as {
      list: (
        agent: AgentLike,
        query: string,
        signal: AbortSignal,
      ) => Promise<Array<{ path: string; kind: 'file' | 'directory' }>>;
    };
    if (!references?.list) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.REFERENCES_UNAVAILABLE,
        'file reference service not available',
      );
    }
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    const candidates = await references.list(agent, params.query, new AbortController().signal);
    return {
      candidates: candidates.map((candidate) => ({
        path: candidate.path,
        kind: candidate.kind,
      })),
    };
  }

  private async handleSessionWorkdirGet(
    params: SessionWorkdirGetParams,
  ): Promise<SessionWorkdirGetResult> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      get: (id: string) => { header: SessionHeader } | undefined;
    };
    const session = sessionsService?.get(params.sessionId);
    if (!session) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    const workingDirectoryService = this.ctx.get('workingDirectory') as unknown as {
      get: (sess: object) => string;
    };
    const cwd = workingDirectoryService?.get(session) ?? session.header.cwd ?? process.cwd();
    return { cwd };
  }

  private async handleSessionModelGet(
    params: SessionModelGetParams,
  ): Promise<SessionModelGetResult> {
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) =>
        | {
            model?: {
              current: () =>
                | { provider?: string; model?: string; reasoningEffort?: string }
                | undefined;
            };
          }
        | undefined;
    };
    const agent = agentsService?.get(params.sessionId);
    const curr = agent?.model?.current();
    return {
      provider: curr?.provider ?? 'deepseek',
      model: curr?.model ?? 'deepseek-chat',
      ...(curr?.reasoningEffort ? { reasoningEffort: curr.reasoningEffort } : {}),
    };
  }

  private async handleSessionModelSet(params: SessionModelSetParams): Promise<{ ok: boolean }> {
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) =>
        | {
            model?: {
              select: (opt: object) => void;
            };
          }
        | undefined;
    };
    const agent = agentsService?.get(params.sessionId);
    if (!agent?.model) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }

    agent.model.select({
      provider: params.provider,
      model: params.model,
      ...(params.reasoningEffort ? { reasoningEffort: params.reasoningEffort } : {}),
    });
    return { ok: true };
  }

  private async handleModelCatalog(): Promise<ModelCatalogResult> {
    const llm = this.ctx.get('llm') as unknown as {
      listProviders?: () => Array<{ id: string; name: string }>;
      listModels?: (p: string) => Promise<Array<{ id: string; name?: string }>>;
    };
    const creds = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;

    const providersList = llm?.listProviders?.() ?? [
      { id: 'deepseek', name: 'DeepSeek' },
      { id: 'deepseek-official', name: 'DeepSeek Official' },
    ];

    const results = [];
    for (const p of providersList) {
      const models = (await llm?.listModels?.(p.id)) ?? [
        { id: 'deepseek-chat', name: 'DeepSeek Chat' },
      ];
      const configured =
        creds?.status().some((s) => s.provider.toLowerCase().includes(p.id) && s.configured) ??
        false;
      results.push({
        id: p.id,
        name: p.name,
        configured,
        models: models.map((m) => ({
          id: m.id,
          displayName: m.name ?? m.id,
          supportsReasoning: m.id.includes('reasoner') || m.id.includes('r1'),
        })),
      });
    }

    return {
      defaultProvider: 'deepseek',
      defaultModel: 'deepseek-chat',
      providers: results,
    };
  }

  private settingsDescriptors(params: SettingsDescribeParams): SettingsDescriptor[] {
    const settings = this.ctx.get('settings') as unknown as {
      describe?: (options: object) => SettingsDescriptor[];
    };
    if (!settings?.describe) return [];
    const keys = params.keys ? new Set(params.keys) : undefined;
    return settings
      .describe({ redactSecrets: true })
      .filter((descriptor) => !keys || keys.has(descriptor.ns));
  }

  private async handleSettingsGet(params: SettingsGetParams): Promise<SettingsGetResult> {
    const dict: Record<string, unknown> = {};
    for (const descriptor of this.settingsDescriptors(params)) {
      dict[descriptor.ns] = descriptor.value;
    }
    return { settings: dict };
  }

  private async handleSettingsDescribe(
    params: SettingsDescribeParams,
  ): Promise<SettingsDescribeResult> {
    return { namespaces: this.settingsDescriptors(params) };
  }

  private async handleSettingsMutate(params: SettingsMutateParams): Promise<SettingsMutateResult> {
    const settings = this.ctx.get('settings') as unknown as {
      update?: (ns: string, patch: object, expectedRevision?: number) => Promise<void>;
    };
    if (settings?.update) {
      try {
        if (params.namespace) {
          await settings.update(params.namespace, params.patch, params.expectedRevision);
        } else {
          for (const [ns, value] of Object.entries(params.patch)) {
            if (value && typeof value === 'object') {
              await settings.update(ns, value as object, params.expectedRevision);
            }
          }
        }
      } catch (error) {
        if (error instanceof Error && 'code' in error && error.code === 'SETTINGS_CONFLICT') {
          const conflict = error as Error & {
            expected?: number;
            actual?: number;
          };
          throw new BridgeError(BRIDGE_ERROR_CODES.SETTINGS_CONFLICT, error.message, {
            expected: conflict.expected,
            actual: conflict.actual,
            namespace: params.namespace,
          });
        }
        throw error;
      }
    }
    return this.handleSettingsGet({});
  }

  private async waitForService<T>(name: string, timeoutMs = 15_000): Promise<T | undefined> {
    const deadline = Date.now() + timeoutMs;
    for (;;) {
      const service = this.ctx.get(name) as T | undefined;
      if (service) return service;
      if (Date.now() > deadline) return undefined;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
  }

  private async workspaceRegistry(): Promise<
    | {
        list: () => WorkspaceRecordLike[];
        get: (id: string) => WorkspaceRecordLike | undefined;
        create: (path: string, title?: string) => Promise<WorkspaceRecordLike>;
        resolveByPath: (path: string) => Promise<WorkspaceRecordLike | undefined>;
        delete: (id: string) => Promise<boolean>;
      }
    | undefined
  > {
    return this.waitForService('workspaceRegistry');
  }

  private async workspaceView(workspace: WorkspaceRecordLike): Promise<WorkspaceView> {
    let status: 'ok' | 'missing-dir' | undefined;
    try {
      status = await workspace.status();
    } catch {}
    return {
      id: workspace.id,
      path: workspace.path,
      title: workspace.title,
      createdAt: workspace.createdAt,
      updatedAt: workspace.updatedAt,
      sessionIds: [...workspace.sessionIds],
      ...(status === undefined ? {} : { status }),
    };
  }

  private async handleWorkspaceList(): Promise<WorkspaceListResult> {
    const registry = await this.workspaceRegistry();
    if (!registry) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_UNAVAILABLE,
        'workspace registry is not available',
      );
    }
    return {
      workspaces: await Promise.all(
        registry.list().map((workspace) => this.workspaceView(workspace)),
      ),
    };
  }

  private async handleWorkspaceCreate(
    params: WorkspaceCreateParams,
  ): Promise<WorkspaceCreateResult> {
    const registry = await this.workspaceRegistry();
    if (!registry) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_UNAVAILABLE,
        'workspace registry is not available',
      );
    }
    if (!params.path) {
      throw new BridgeError(-32602, 'workspace/create requires path');
    }
    const existing = await registry.resolveByPath(params.path);
    if (existing) {
      return { workspace: await this.workspaceView(existing), created: false };
    }
    const workspace = await registry.create(params.path, params.title);
    return { workspace: await this.workspaceView(workspace), created: true };
  }

  private async handleWorkspaceRename(
    params: WorkspaceRenameParams,
  ): Promise<WorkspaceRenameResult> {
    const registry = await this.workspaceRegistry();
    if (!registry) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_UNAVAILABLE,
        'workspace registry is not available',
      );
    }
    const workspace = registry.get(params.workspaceId);
    if (!workspace) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_NOT_FOUND,
        `workspace not found: ${params.workspaceId}`,
        { workspaceId: params.workspaceId },
      );
    }
    if (
      params.title !== workspace.title &&
      registry
        .list()
        .some((candidate) => candidate.id !== workspace.id && candidate.title === params.title)
    ) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_NAME_CONFLICT,
        `workspace title already in use: ${params.title}`,
      );
    }
    await workspace.setTitle(params.title);
    return { workspace: await this.workspaceView(workspace) };
  }

  private async handleWorkspaceRemove(
    params: WorkspaceRemoveParams,
  ): Promise<WorkspaceRemoveResult> {
    const registry = await this.workspaceRegistry();
    if (!registry) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_UNAVAILABLE,
        'workspace registry is not available',
      );
    }
    const deleted = await registry.delete(params.workspaceId);
    if (!deleted) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKSPACE_NOT_FOUND,
        `workspace not found: ${params.workspaceId}`,
        { workspaceId: params.workspaceId },
      );
    }
    return { deleted: true };
  }

  private permissionService():
    | {
        catalog: () => PermissionCatalogResult;
        current: (session: SessionLike) => string;
        set: (session: SessionLike, preset: string) => void;
      }
    | undefined {
    return this.ctx.get('permissionPresets') as
      | {
          catalog: () => PermissionCatalogResult;
          current: (session: SessionLike) => string;
          set: (session: SessionLike, preset: string) => void;
        }
      | undefined;
  }

  private async handlePermissionCatalog(): Promise<PermissionCatalogResult> {
    const permissions = this.permissionService();
    if (!permissions) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PERMISSIONS_UNAVAILABLE,
        'permission preset service is not available',
      );
    }
    return permissions.catalog();
  }

  private async handlePermissionGet(params: PermissionGetParams): Promise<PermissionGetResult> {
    const permissions = this.permissionService();
    if (!permissions) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PERMISSIONS_UNAVAILABLE,
        'permission preset service is not available',
      );
    }
    const { session } = await this.resolveSessionEntry(params.sessionId);
    return {
      currentValue: permissions.current(session),
      catalog: permissions.catalog(),
    };
  }

  private async handlePermissionSet(params: PermissionSetParams): Promise<PermissionSetResult> {
    const permissions = this.permissionService();
    if (!permissions) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PERMISSIONS_UNAVAILABLE,
        'permission preset service is not available',
      );
    }
    const { session } = await this.resolveSessionEntry(params.sessionId);
    permissions.set(session, params.preset);
    return { currentValue: permissions.current(session) };
  }

  private goalView(goal: GoalView | undefined): GoalView | null {
    if (!goal) return null;
    return {
      id: goal.id,
      revision: goal.revision,
      objective: goal.objective,
      phase: goal.phase,
      ...(goal.blockedReason === undefined ? {} : { blockedReason: goal.blockedReason }),
      maxGoalRounds: goal.maxGoalRounds,
      roundsStarted: goal.roundsStarted,
      createdAt: goal.createdAt,
      updatedAt: goal.updatedAt,
      activation: goal.activation,
    };
  }

  private async requireGoalAgent(
    sessionId: string,
  ): Promise<{ agent: AgentLike; goals: GoalServiceLike }> {
    const goals = this.ctx.get('goals') as GoalServiceLike | undefined;
    if (!goals) {
      throw new BridgeError(BRIDGE_ERROR_CODES.GOAL_UNAVAILABLE, 'goal service is not available');
    }
    const { agent } = await this.resolveSessionEntry(sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${sessionId}`,
      );
    }
    return { agent, goals };
  }

  private async handleGoalGet(params: GoalGetParams): Promise<GoalGetResult> {
    const { agent, goals } = await this.requireGoalAgent(params.sessionId);
    return { goal: this.goalView(goals.get(agent)) };
  }

  private async handleGoalEdit(params: GoalEditParams): Promise<GoalMutationResult> {
    const { agent, goals } = await this.requireGoalAgent(params.sessionId);
    try {
      const goal = goals.edit(agent, params.ref, {
        ...(params.objective === undefined ? {} : { objective: params.objective }),
        ...(params.maxGoalRounds === undefined ? {} : { maxGoalRounds: params.maxGoalRounds }),
      });
      return { goal: this.goalView(goal) };
    } catch (error) {
      throw this.goalError(error);
    }
  }

  private async handleGoalMutation(
    action: 'pause' | 'resume' | 'clear',
    params: GoalMutationParams,
  ): Promise<GoalMutationResult> {
    const { agent, goals } = await this.requireGoalAgent(params.sessionId);
    try {
      if (action === 'clear') {
        goals.clear(agent, params.ref);
        return { goal: null };
      }
      const goal = goals[action](agent, params.ref);
      return { goal: this.goalView(goal) };
    } catch (error) {
      throw this.goalError(error);
    }
  }

  private goalError(error: unknown): BridgeError {
    if (error instanceof Error && 'code' in error) {
      return new BridgeError(BRIDGE_ERROR_CODES.GOAL_INVALID, error.message, {
        code: error.code,
      });
    }
    return new BridgeError(
      BRIDGE_ERROR_CODES.GOAL_INVALID,
      error instanceof Error ? error.message : String(error),
    );
  }

  private async resolveFileScope(
    sessionId: string,
    requestedPath: string,
  ): Promise<{ root: string; target: string; display: string }> {
    const { session } = await this.resolveSessionEntry(sessionId);
    const workingDirectory = this.ctx.get('workingDirectory') as unknown as {
      get: (session: SessionLike) => string;
    };
    const root = workingDirectory?.get(session) ?? session.header.cwd;
    if (!root) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKDIR_UNAVAILABLE,
        'session has no working directory',
      );
    }
    let rootReal: string;
    try {
      rootReal = await fs.realpath(root);
    } catch {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.WORKDIR_UNAVAILABLE,
        `working directory is unavailable: ${root}`,
      );
    }
    let target: string;
    try {
      target = await fs.realpath(path.resolve(rootReal, requestedPath));
    } catch (error) {
      if (error instanceof Error && 'code' in error && error.code === 'ENOENT') {
        throw new BridgeError(
          BRIDGE_ERROR_CODES.FILE_NOT_FOUND,
          `file not found: ${requestedPath}`,
        );
      }
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_ACCESS_DENIED,
        `cannot access: ${requestedPath}`,
      );
    }
    if (target !== rootReal && !target.startsWith(`${rootReal}${path.sep}`)) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_ACCESS_DENIED,
        `path escapes session working directory: ${requestedPath}`,
      );
    }
    return {
      root: rootReal,
      target,
      display: path.relative(rootReal, target) || '.',
    };
  }

  private fileKind(entry: {
    isFile(): boolean;
    isDirectory(): boolean;
    isSymbolicLink(): boolean;
  }): FileEntry['kind'] {
    if (entry.isSymbolicLink()) return 'symlink';
    if (entry.isDirectory()) return 'directory';
    if (entry.isFile()) return 'file';
    return 'other';
  }

  private async handleFilesList(params: FilesListParams): Promise<FilesListResult> {
    const scope = await this.resolveFileScope(params.sessionId, params.path ?? '.');
    let stat: Awaited<ReturnType<typeof fs.stat>>;
    try {
      stat = await fs.stat(scope.target);
    } catch {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_NOT_FOUND,
        `file not found: ${params.path ?? '.'}`,
      );
    }
    if (!stat.isDirectory()) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_ACCESS_DENIED,
        `not a directory: ${params.path ?? '.'}`,
      );
    }
    const entries: FileEntry[] = [];
    for (const entry of await fs.readdir(scope.target, { withFileTypes: true })) {
      if (entries.length >= 2_000) break;
      const entryPath = path.join(scope.target, entry.name);
      let size = 0;
      try {
        size = (await fs.lstat(entryPath)).size;
      } catch {}
      entries.push({
        name: entry.name,
        path: path.relative(scope.root, entryPath).split(path.sep).join('/'),
        kind: this.fileKind(entry),
        size,
      });
    }
    entries.sort((left, right) => left.name.localeCompare(right.name));
    return { path: scope.display, entries };
  }

  private async handleFilesStat(params: FilesStatParams): Promise<FilesStatResult> {
    const scope = await this.resolveFileScope(params.sessionId, params.path);
    let stat: Awaited<ReturnType<typeof fs.lstat>>;
    try {
      stat = await fs.lstat(scope.target);
    } catch {
      throw new BridgeError(BRIDGE_ERROR_CODES.FILE_NOT_FOUND, `file not found: ${params.path}`);
    }
    return {
      path: scope.display,
      kind: this.fileKind(stat),
      size: stat.size,
      mtime: stat.mtimeMs,
    };
  }

  private async handleFilesRead(params: FilesReadParams): Promise<FilesReadResult> {
    const scope = await this.resolveFileScope(params.sessionId, params.path);
    let stat: Awaited<ReturnType<typeof fs.stat>>;
    try {
      stat = await fs.stat(scope.target);
    } catch {
      throw new BridgeError(BRIDGE_ERROR_CODES.FILE_NOT_FOUND, `file not found: ${params.path}`);
    }
    if (!stat.isFile()) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_ACCESS_DENIED,
        `not a regular file: ${params.path}`,
      );
    }
    const requested = params.maxBytes ?? 1_048_576;
    const maxBytes = Math.min(Math.max(requested, 1), 8_388_608);
    const truncated = stat.size > maxBytes;
    const handle = await fs.open(scope.target, 'r');
    let buffer: Buffer;
    try {
      buffer = Buffer.alloc(Math.min(stat.size, maxBytes));
      await handle.read(buffer, 0, buffer.length, 0);
    } finally {
      await handle.close();
    }
    const text = buffer.toString('utf8');
    if (buffer.includes(0) || text.includes('\uFFFD')) {
      return { kind: 'binary', path: scope.display, size: stat.size };
    }
    return {
      kind: 'text',
      path: scope.display,
      text,
      size: stat.size,
      truncated,
    };
  }

  private async handleFilesReadBytes(params: FilesReadBytesParams): Promise<FilesReadBytesResult> {
    const scope = await this.resolveFileScope(params.sessionId, params.path);
    let stat: Awaited<ReturnType<typeof fs.stat>>;
    try {
      stat = await fs.stat(scope.target);
    } catch {
      throw new BridgeError(BRIDGE_ERROR_CODES.FILE_NOT_FOUND, `file not found: ${params.path}`);
    }
    if (!stat.isFile()) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_ACCESS_DENIED,
        `not a regular file: ${params.path}`,
      );
    }
    const requested = params.maxBytes ?? 16_777_216;
    const maxBytes = Math.min(Math.max(requested, 1), 16_777_216);
    if (stat.size > maxBytes) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FILE_TOO_LARGE,
        `file exceeds ${maxBytes} bytes: ${params.path}`,
        { size: stat.size, maxBytes },
      );
    }
    const buffer = await fs.readFile(scope.target);
    return {
      path: scope.display,
      data: buffer.toString('base64'),
      size: buffer.length,
    };
  }

  private async handlePluginList(): Promise<PluginListResult> {
    const pm = this.ctx.get('pluginManager') as unknown as {
      listPlugins?: () => Promise<Array<{ entryId: string; moduleName: string; enabled: boolean }>>;
    };

    if (!pm?.listPlugins) {
      return { plugins: [] };
    }

    const list = await pm.listPlugins();
    return {
      plugins: list.map((item) => ({
        id: item.entryId,
        name: item.moduleName,
        enabled: item.enabled,
        restartRequired: false,
      })),
    };
  }

  private async handlePluginEnable(params: { id: string }): Promise<PluginToggleResult> {
    const pm = this.ctx.get('pluginManager') as unknown as {
      setPluginEnabled?: (id: string, enabled: boolean) => Promise<unknown>;
    };

    if (pm?.setPluginEnabled) {
      try {
        await pm.setPluginEnabled(params.id, true);
      } catch {}
    }

    return {
      id: params.id,
      enabled: true,
      status: 'restart-required',
      restartRequired: true,
    };
  }

  private async handlePluginDisable(params: { id: string }): Promise<PluginToggleResult> {
    const pm = this.ctx.get('pluginManager') as unknown as {
      setPluginEnabled?: (id: string, enabled: boolean) => Promise<unknown>;
    };

    if (pm?.setPluginEnabled) {
      try {
        await pm.setPluginEnabled(params.id, false);
      } catch {}
    }

    return {
      id: params.id,
      enabled: false,
      status: 'restart-required',
      restartRequired: true,
    };
  }

  private async handleCredentialsStatus(): Promise<{ items: unknown[] }> {
    const creds = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    return { items: creds?.status() ?? [] };
  }

  private async handleCredentialsInject(params: {
    provider: string;
    key: string;
    baseURL?: string;
  }): Promise<{ provider: string; configured: boolean }> {
    const creds = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (creds?.inject) {
      await creds.inject(params.provider, params.key, params.baseURL);
    }
    return { provider: params.provider, configured: true };
  }

  private async handleCredentialsRotate(params: {
    provider: string;
    key: string;
  }): Promise<{ provider: string; configured: boolean }> {
    const creds = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (creds?.rotate) {
      await creds.rotate(params.provider, params.key);
    }
    return { provider: params.provider, configured: true };
  }

  private async handleCredentialsRemove(params: {
    provider: string;
  }): Promise<{ provider: string; configured: boolean }> {
    const creds = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (creds?.remove) {
      await creds.remove(params.provider);
    }
    return { provider: params.provider, configured: false };
  }

  private async handleWorkspaceChanges(
    params: WorkspaceChangesParams,
  ): Promise<WorkspaceChangesResult> {
    const changes = this.ctx.get('workspaceChanges') as unknown as {
      summary?: (sessId: string, seq: number) => unknown;
      diff?: (sessId: string, seq: number, idx: number, sig?: AbortSignal) => Promise<unknown>;
    };

    if (!changes) {
      return { summary: null, diff: null };
    }

    const summary = changes.summary?.(params.sessionId, params.turn ?? 0) ?? null;
    let diff: unknown = null;
    if (params.fileIndex !== undefined && changes.diff) {
      diff = (await changes.diff(params.sessionId, params.turn ?? 0, params.fileIndex)) ?? null;
    }

    return { summary, diff };
  }
}

export default function apply(ctx: Context, config?: BridgeConfig): void {
  ctx.plugin(BridgeService, config);
}
