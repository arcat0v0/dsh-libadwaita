import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { type Context, Service } from '@deepseek-ai/cordis';
import type { SessionId } from '@deepseek-ai/dsh-session';
import { buildForkSeed } from '@deepseek-ai/dsh-session/fork';
import { SessionPersistenceNotFoundError } from '@deepseek-ai/dsh-session-persistence';
import type { BridgeCredentials } from './credentials.js';
import { RegistryProbe } from './registry-probe.js';
import { BridgeServer } from './server.js';
import {
  type AccountBalanceResult,
  type AccountProfileResult,
  type AccountSignOutResult,
  type AccountStatusResult,
  type AssistantStreamFrame,
  type AttachmentReadParams,
  type AttachmentReadResult,
  type AttachmentUploadParams,
  type AttachmentUploadResult,
  BRIDGE_ERROR_CODES,
  BridgeError,
  type CommandDescriptor,
  type CommandExecuteParams,
  type CommandExecuteResult,
  type CommandListParams,
  type CommandListResult,
  type CredentialRecordDeleteParams,
  type CredentialRecordItem,
  type CredentialRecordModifyParams,
  type CredentialRecordMutationResult,
  type CredentialRecordReadParams,
  type CredentialRecordReadResult,
  type CredentialRecordsListResult,
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
  type JobChunkWire,
  type JobFollowParams,
  type JobFollowResult,
  type JobKillParams,
  type JobKillResult,
  type JobListParams,
  type JobListResult,
  type JobOutputNotification,
  type JobWireView,
  type MessageFeedbackDeleteParams,
  type MessageFeedbackDeleteResult,
  type MessageFeedbackItem,
  type MessageFeedbackListParams,
  type MessageFeedbackListResult,
  type MessageFeedbackPutParams,
  type MessageFeedbackPutResult,
  type ModelCatalogResult,
  type ModelDiscoverParams,
  type ModelDiscoverResult,
  type PermissionCatalogResult,
  type PermissionGetParams,
  type PermissionGetResult,
  type PermissionSetParams,
  type PermissionSetResult,
  type PermissionUpdatedNotification,
  type PluginBundleToggleParams,
  type PluginChangedNotification,
  type PluginDetailParams,
  type PluginInspectParams,
  type PluginInstallLogNotification,
  type PluginInstallParams,
  type PluginInstallStateNotification,
  type PluginInventoryResult,
  type PluginListResult,
  type PluginMarketChangedNotification,
  type PluginMarketReadEntryDetailParams,
  type PluginMarketSearchParams,
  type PluginMarketSetIndexesParams,
  type PluginOperationResult,
  type PluginRegistryProbeResult,
  type PluginRemoveParams,
  type PluginRequestParams,
  type PluginToggleParams,
  type PluginToggleResult,
  type PluginUpdateParams,
  type PluginUpdateStateNotification,
  type PluginVersionsParams,
  type QuestionReopenParams,
  type QuestionReopenResult,
  type QueueEditParams,
  type QueueItem,
  type QueueListParams,
  type QueueListResult,
  type QueueMutationResult,
  type QueueRemoveParams,
  type QueueSteerParams,
  type ReferenceListParams,
  type ReferenceListResult,
  type ScheduleChangedNotification,
  type ScheduleCreateParams,
  type ScheduleEditParams,
  type ScheduleHistoryParams,
  type ScheduleOperationResult,
  type ScheduleRunParams,
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
  type SessionFeedbackRecordParams,
  type SessionFeedbackRecordResult,
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
  type SessionProjectionNotification,
  type SessionProjectionsParams,
  type SessionProjectionsResult,
  type SessionProjectionsSnapshot,
  type SessionPromptParams,
  type SessionPromptResult,
  type SessionRenameParams,
  type SessionRenameResult,
  type SessionSearchItem,
  type SessionSearchParams,
  type SessionSearchResult,
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
  type SubagentInterruptParams,
  type SubagentInterruptResult,
  type SubagentListParams,
  type SubagentListResult,
  type SubagentPromptParams,
  type SubagentPromptResult,
  type TeamGetParams,
  type TeamGetResult,
  type TeamSetParams,
  type TeamSetResult,
  type TerminalCloseParams,
  type TerminalCreateParams,
  type TerminalCreateResult,
  type TerminalInputParams,
  type TerminalListParams,
  type TerminalListResult,
  type TerminalOutputParams,
  type TerminalOutputResult,
  type TodoItem,
  type ToolResultUpdatedNotification,
  type ToolResultWatchParams,
  type ToolResultWatchResult,
  type WebProvidersResult,
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

export function isMissingSessionError(error: unknown): boolean {
  if (error instanceof SessionPersistenceNotFoundError) return true;
  if (!error || typeof error !== 'object') return false;
  const candidate = error as { isDSHRemoteError?: unknown; code?: unknown };
  return candidate.isDSHRemoteError === true && candidate.code === 'session/not-found';
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
  attachSession: (sessionId: string) => Promise<void>;
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
  private readonly watchedToolResults = new Set<string>();
  private readonly jobTimers = new Map<string, NodeJS.Timeout>();
  private readonly pendingQuestionsBySession = new Map<string, SdkUserQuestionRequest>();
  private readonly projectionRevisions = new Map<string, number>();
  private readonly projectionSignatures = new Map<string, string>();
  private readonly registryProbe = new RegistryProbe();

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
    this.ctx.reflect.provide('sessionController', {
      create: (request: { sessionId?: string; cwd?: string; agentPreset?: string }) =>
        this.createSessionAgent(request).then((agent) => ({ agent })),
      resolveAgent: async (sessionId: string) => {
        try {
          return { agent: await this.resolveScheduleAgent(sessionId) };
        } catch (error) {
          return { error: error instanceof Error ? error : new Error(String(error)) };
        }
      },
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
      const s = session as SessionLike;
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

        if (ev.type === 'tool/result' && this.watchedToolResults.has(s.id)) {
          const data =
            ev.data && typeof ev.data === 'object'
              ? (ev.data as { callId?: string; name?: string })
              : {};
          const notification: ToolResultUpdatedNotification = {
            sessionId: s.id,
            seq: ev.seq,
            ...(data.callId === undefined ? {} : { callId: data.callId }),
            ...(data.name === undefined ? {} : { name: data.name }),
            data: ev.data,
          };
          this.server.notify('tool-result.updated', notification);
        }
        if (
          ev.type === 'assistant/message' ||
          ev.type === 'step/end' ||
          ev.type === 'turn/end' ||
          ev.type === 'compaction/end'
        ) {
          const projections = this.ctx.get('sessionProjections') as unknown as {
            snapshot?: (
              session: SessionLike,
              keys: string[],
            ) => { values: Record<string, unknown> };
          };
          const keys = ['tokenUsage', 'contextPressure', 'contextBreakdown'];
          const snapshot = projections?.snapshot?.(s, keys);
          for (const kind of keys) {
            const value = snapshot?.values[kind];
            if (value === undefined) continue;
            const cacheKey = `${s.id}:${kind}`;
            const signature = JSON.stringify(value);
            if (this.projectionSignatures.get(cacheKey) === signature) continue;
            this.projectionSignatures.set(cacheKey, signature);
            const revision = Math.max(this.projectionRevisions.get(cacheKey) ?? 0, ev.seq);
            this.projectionRevisions.set(cacheKey, revision);
            const notification: SessionProjectionNotification = {
              sessionId: s.id,
              kind,
              value,
              revision,
            };
            this.server.notify('session.projection', notification);
          }
        }
      }
    });

    this.ctx.on('plugin-manager/install-state', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as { requestId?: string; phase?: string; attempt?: unknown };
      if (!p.requestId || !p.phase) return;
      const notification: PluginInstallStateNotification = {
        requestId: p.requestId,
        phase: p.phase,
        ...(p.attempt === undefined ? {} : { attempt: p.attempt }),
      };
      this.server.notify('plugin.install-state', notification);
    });

    this.ctx.on('plugin-manager/install-log', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as {
        requestId?: string;
        jobId?: string;
        argv?: string[];
        cwd?: string;
        stream?: 'stdout' | 'stderr';
        text?: string;
        exitCode?: number | null;
      };
      if (!p.jobId || !p.stream || p.text === undefined) return;
      const notification: PluginInstallLogNotification = {
        ...(p.requestId === undefined ? {} : { requestId: p.requestId }),
        jobId: p.jobId,
        argv: p.argv ?? [],
        cwd: p.cwd ?? '',
        stream: p.stream,
        text: p.text,
        ...(p.exitCode === undefined ? {} : { exitCode: p.exitCode }),
      };
      this.server.notify('plugin.install-log', notification);
    });

    this.ctx.on('plugin-manager/changed', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as { reason?: string };
      if (!p.reason) return;
      const notification: PluginChangedNotification = { reason: p.reason };
      this.server.notify('plugin.changed', notification);
    });

    this.ctx.on('plugin-manager/update-state', (payload: unknown) => {
      if (!payload || typeof payload !== 'object') return;
      const p = payload as { requestId?: string; step?: string };
      if (!p.requestId || !p.step) return;
      const notification: PluginUpdateStateNotification = {
        requestId: p.requestId,
        step: p.step,
      };
      this.server.notify('plugin.update-state', notification);
    });

    this.ctx.on('plugin-market/changed', () => {
      const notification: PluginMarketChangedNotification = { changed: true };
      this.server.notify('plugin.market-changed', notification);
    });

    this.ctx.on(
      'schedule/resolve-agent',
      async (sessionId: string, next: () => Promise<unknown>) => {
        try {
          return await this.resolveScheduleAgent(sessionId);
        } catch {
          return next();
        }
      },
    );

    this.ctx.on('schedule/changed', () => {
      const notification: ScheduleChangedNotification = { changed: true };
      this.server.notify('schedule.changed', notification);
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
        displayReason?: Record<string, string>;
        signal?: AbortSignal;
      };
      const sessionId = r.agent?.session?.id ?? '';
      const decision = await this.server.requestApproval(
        {
          sessionId,
          toolName: r.toolName ?? 'unknown',
          ...(r.callId === undefined ? {} : { callId: r.callId }),
          ...(r.reason === undefined ? {} : { reason: r.reason }),
          ...(r.displayReason === undefined ? {} : { displayReason: r.displayReason }),
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
        wait?: { callId?: string; timed?: boolean };
      };
      const sessionId = r.agent?.session?.id ?? '';
      const requestId = `q-${crypto.randomUUID()}`;
      const payload: SdkUserQuestionRequest = {
        requestId,
        sessionId,
        questions: (r.questions ?? []) as SdkUserQuestionRequest['questions'],
        ...(r.wait?.callId === undefined ? {} : { callId: r.wait.callId }),
        ...(r.wait?.timed === undefined ? {} : { timed: r.wait.timed }),
      };
      this.pendingQuestionsBySession.set(sessionId, payload);

      const res = await this.server.requestUserQuestion(payload, 300_000, r.signal).finally(() => {
        if (this.pendingQuestionsBySession.get(sessionId) === payload) {
          this.pendingQuestionsBySession.delete(sessionId);
        }
      });

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
        for (const timer of this.jobTimers.values()) clearInterval(timer);
        this.jobTimers.clear();
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
      case 'libadwaita/session/projections':
        return this.handleSessionProjections(params as unknown as SessionProjectionsParams);
      case 'libadwaita/tool-result/watch':
        return this.handleToolResultWatch(params as unknown as ToolResultWatchParams);
      case 'libadwaita/message-feedback/list':
        return this.handleMessageFeedbackList(params as unknown as MessageFeedbackListParams);
      case 'libadwaita/message-feedback/put':
        return this.handleMessageFeedbackPut(params as unknown as MessageFeedbackPutParams);
      case 'libadwaita/message-feedback/delete':
        return this.handleMessageFeedbackDelete(params as unknown as MessageFeedbackDeleteParams);
      case 'libadwaita/session-feedback/record':
        return this.handleSessionFeedbackRecord(params as unknown as SessionFeedbackRecordParams);
      case 'libadwaita/question/reopen':
        return this.handleQuestionReopen(params as unknown as QuestionReopenParams);
      case 'libadwaita/job/list':
        return this.handleJobList(params as unknown as JobListParams);
      case 'libadwaita/job/follow':
        return this.handleJobFollow(params as unknown as JobFollowParams);
      case 'libadwaita/job/kill':
        return this.handleJobKill(params as unknown as JobKillParams);
      case 'libadwaita/session/search':
        return this.handleSessionSearch(params as unknown as SessionSearchParams);
      case 'libadwaita/attachment/upload':
        return this.handleAttachmentUpload(params as unknown as AttachmentUploadParams);
      case 'libadwaita/attachment/read':
        return this.handleAttachmentRead(params as unknown as AttachmentReadParams);
      case 'libadwaita/plugin/list':
        return this.handlePluginList();
      case 'libadwaita/plugin/enable':
        return this.handlePluginEnable(params as unknown as PluginToggleParams);
      case 'libadwaita/plugin/disable':
        return this.handlePluginDisable(params as unknown as PluginToggleParams);
      case 'libadwaita/plugin/bundle/enable':
        return this.handlePluginBundleEnable(params as unknown as PluginBundleToggleParams);
      case 'libadwaita/plugin/bundle/disable':
        return this.handlePluginBundleDisable(params as unknown as PluginBundleToggleParams);
      case 'libadwaita/plugin/registry-probe':
        return this.handlePluginRegistryProbe();
      case 'libadwaita/plugin-market/list-indexes':
        return this.handlePluginMarketListIndexes();
      case 'libadwaita/plugin-market/set-indexes':
        return this.handlePluginMarketSetIndexes(params as unknown as PluginMarketSetIndexesParams);
      case 'libadwaita/plugin-market/search':
        return this.handlePluginMarketSearch(params as unknown as PluginMarketSearchParams);
      case 'libadwaita/plugin-market/read-entry-detail':
        return this.handlePluginMarketReadEntryDetail(
          params as unknown as PluginMarketReadEntryDetailParams,
        );
      case 'libadwaita/plugin/detail':
        return this.handlePluginDetail(params as unknown as PluginDetailParams);
      case 'libadwaita/plugin/inspect':
        return this.handlePluginInspect(params as unknown as PluginInspectParams);
      case 'libadwaita/plugin/install':
        return this.handlePluginInstall(params as unknown as PluginInstallParams);
      case 'libadwaita/plugin/cancel-install':
        return this.handlePluginCancelInstall(params as unknown as PluginRequestParams);
      case 'libadwaita/plugin/wait-install':
        return this.handlePluginWaitInstall(params as unknown as PluginRequestParams);
      case 'libadwaita/plugin/remove':
        return this.handlePluginRemove(params as unknown as PluginRemoveParams);
      case 'libadwaita/plugin/versions':
        return this.handlePluginVersions(params as unknown as PluginVersionsParams);
      case 'libadwaita/plugin/update':
        return this.handlePluginUpdate(params as unknown as PluginUpdateParams);
      case 'libadwaita/plugin/registries':
        return this.handlePluginRegistries();
      case 'libadwaita/plugin/inventory':
        return this.handlePluginInventory();
      case 'libadwaita/plugin/retry':
        return this.handlePluginInventory();
      case 'libadwaita/model/discover':
        return this.handleModelDiscover(params as unknown as ModelDiscoverParams);
      case 'libadwaita/web-search/providers':
        return this.handleWebProviders();
      case 'libadwaita/subagent/list':
        return this.handleSubagentList(params as unknown as SubagentListParams);
      case 'libadwaita/subagent/prompt':
        return this.handleSubagentPrompt(params as unknown as SubagentPromptParams);
      case 'libadwaita/subagent/interrupt':
        return this.handleSubagentInterrupt(params as unknown as SubagentInterruptParams);
      case 'libadwaita/agent-team/get':
        return this.handleTeamGet(params as unknown as TeamGetParams);
      case 'libadwaita/agent-team/set-enabled':
        return this.handleTeamSet(params as unknown as TeamSetParams);
      case 'libadwaita/credentials/records/list':
        return this.handleCredentialRecordsList();
      case 'libadwaita/credentials/records/read':
        return this.handleCredentialRecordRead(params as unknown as CredentialRecordReadParams);
      case 'libadwaita/credentials/records/modify':
        return this.handleCredentialRecordModify(params as unknown as CredentialRecordModifyParams);
      case 'libadwaita/credentials/records/delete':
        return this.handleCredentialRecordDelete(params as unknown as CredentialRecordDeleteParams);
      case 'libadwaita/account/status':
        return this.handleAccountStatus();
      case 'libadwaita/account/profile':
        return this.handleAccountProfile();
      case 'libadwaita/account/balance':
        return this.handleAccountBalance();
      case 'libadwaita/account/sign-out':
        return this.handleAccountSignOut();
      case 'libadwaita/schedule/catalog':
        return this.handleScheduleCatalog();
      case 'libadwaita/schedule/create':
        return this.handleScheduleCreate(params as unknown as ScheduleCreateParams);
      case 'libadwaita/schedule/edit':
        return this.handleScheduleEdit(params as unknown as ScheduleEditParams);
      case 'libadwaita/schedule/run':
        return this.handleScheduleRun(params as unknown as ScheduleRunParams);
      case 'libadwaita/schedule/history':
        return this.handleScheduleHistory(params as unknown as ScheduleHistoryParams);
      case 'libadwaita/schedule/delete':
        return this.handleScheduleDelete(params as unknown as ScheduleEditParams);
      case 'libadwaita/schedule/delete-inactive':
        return this.handleScheduleDeleteInactive();
      case 'libadwaita/schedule/delivery-limits':
        return this.handleScheduleDeliveryLimits();
      case 'libadwaita/terminal/create':
        return this.handleTerminalCreate(params as unknown as TerminalCreateParams);
      case 'libadwaita/terminal/list':
        return this.handleTerminalList(params as unknown as TerminalListParams);
      case 'libadwaita/terminal/input':
        return this.handleTerminalInput(params as unknown as TerminalInputParams);
      case 'libadwaita/terminal/output':
        return this.handleTerminalOutput(params as unknown as TerminalOutputParams);
      case 'libadwaita/terminal/close':
        return this.handleTerminalClose(params as unknown as TerminalCloseParams);
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

  private async createSessionAgent(options: {
    sessionId?: string;
    cwd?: string;
    agentPreset?: string;
    provider?: string;
    model?: string;
  }): Promise<AgentLike> {
    const agents = this.ctx.get('agents') as unknown as {
      create: (options: object) => Promise<{ agent: AgentLike }>;
    };
    if (!agents) {
      throw new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, 'agents service not available');
    }
    const handle = await agents.create({
      sessionId: options.sessionId ?? `sess-${crypto.randomUUID()}`,
      meta: {
        ...(options.cwd === undefined ? {} : { cwd: options.cwd }),
        ...(options.agentPreset === undefined ? {} : { agentPreset: options.agentPreset }),
      },
    });
    const agent = handle.agent;
    this.applySessionModel(agent, {
      sessionId: options.sessionId,
      cwd: options.cwd ?? '',
      provider: options.provider,
      model: options.model,
      agentPreset: options.agentPreset,
    });
    return agent;
  }

  private async handleSessionCreate(params: SessionCreateParams): Promise<SessionCreateResult> {
    if (params.sessionId !== undefined) {
      const reused = await this.reuseNamedSession(params.sessionId, params);
      if (reused) {
        await this.attachSessionToWorkspace(reused.header.cwd ?? params.cwd, reused.sessionId);
        return reused;
      }
    }
    const agent = await this.createSessionAgent(params);
    await this.attachSessionToWorkspace(agent.session.header.cwd ?? params.cwd, agent.id);
    return {
      sessionId: agent.id,
      header: agent.session.header,
    };
  }

  private async reuseNamedSession(
    sessionId: string,
    params: SessionCreateParams,
  ): Promise<SessionCreateResult | undefined> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      get: (id: string) => SessionLike | undefined;
    };
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) => AgentLike | undefined;
      resume?: (options: object) => Promise<{ agent: AgentLike }>;
    };
    const session = sessionsService?.get(sessionId);
    if (session) {
      const live = agentsService?.get(sessionId);
      if (live) this.applySessionModel(live, params);
      return { sessionId, header: session.header };
    }
    if (!agentsService?.resume) return undefined;
    try {
      const handle = await agentsService.resume({ resumeSessionId: sessionId });
      this.applySessionModel(handle.agent, params);
      return { sessionId, header: handle.agent.session.header };
    } catch (error) {
      if (isMissingSessionError(error)) return undefined;
      throw error;
    }
  }

  private applySessionModel(agent: AgentLike, params: SessionCreateParams): void {
    const provider = params.provider ?? 'deepseek-official';
    const model = params.model ?? 'deepseek-chat';
    agent.model?.select({ provider, model });
  }

  private async handleSessionList(params: SessionListParams): Promise<SessionListResult> {
    const sessionsService = this.ctx.get('sessions') as unknown as {
      list: () => SessionLike[];
    };
    if (!sessionsService) {
      return { sessions: [] };
    }

    const includeArchived = params.archived ?? false;
    const limit = params.limit ?? 100;
    const liveSessions = sessionsService.list();
    const liveById = new Map(liveSessions.map((session) => [session.id, session]));

    const query = this.ctx.get('sessionQuery') as unknown as {
      listSessions: (
        signal?: AbortSignal,
      ) => Promise<Array<{ header: SessionHeader; live?: boolean }>>;
    };
    let headers: SessionHeader[];
    if (query?.listSessions) {
      try {
        headers = (await query.listSessions()).map((record) => record.header);
      } catch (error) {
        throw new BridgeError(
          BRIDGE_ERROR_CODES.INTERNAL_ERROR,
          `session listing failed: ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    } else {
      headers = liveSessions.map((session) => session.header);
    }

    const archivedIds = await this.archivedSessionIds();

    const projectionCache = this.ctx.get('sessionProjectionCache') as unknown as
      | {
          cachedSnapshot?: (
            header: SessionHeader,
            keys: string[],
          ) => { values: Record<string, unknown> } | undefined;
        }
      | undefined;

    const items: SessionListItem[] = [];
    for (const header of headers) {
      const isArchived = archivedIds.has(header.id);
      if (isArchived && !includeArchived) continue;

      const live = liveById.get(header.id);
      let title = header.cwd ? `Session in ${header.cwd}` : 'New Session';
      let updatedAt = header.createdAt;
      let turnCount = 0;

      if (live) {
        const events = live.snapshotEvents();
        const lastEvent = events.at(-1);
        const titleEvent = events.findLast((e) => e.type === 'session/title');
        if (titleEvent?.data && typeof titleEvent.data === 'object' && 'title' in titleEvent.data) {
          title = String((titleEvent.data as { title: unknown }).title);
        }
        if (lastEvent) updatedAt = lastEvent.time;
        turnCount = events.filter((e) => e.type === 'turn/start').length;
      } else {
        const cached = projectionCache?.cachedSnapshot?.(header, ['title', 'turnBoundary']);
        const cachedTitle = cached?.values.title;
        if (typeof cachedTitle === 'string' && cachedTitle.length > 0) {
          title = cachedTitle;
        }
        const boundary = cached?.values.turnBoundary as { lastTurn?: unknown } | undefined;
        if (typeof boundary?.lastTurn === 'number') turnCount = boundary.lastTurn;
      }

      items.push({
        id: header.id,
        createdAt: header.createdAt,
        updatedAt,
        cwd: header.cwd ?? '',
        title,
        turnCount,
        isArchived,
      });
    }

    items.sort((a, b) => b.updatedAt - a.updatedAt);
    return { sessions: items.slice(0, limit) };
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
    const encodedImages = params.contentBlocks.filter(
      (block): block is { type: 'image'; data: string; mimeType: string; name?: string } =>
        block.type === 'image' && 'data' in block && typeof block.data === 'string',
    );
    const attachments = this.ctx.get('attachments') as unknown as {
      saveImages?: (
        inputs: Array<{ data: Uint8Array; mediaType: string; name?: string }>,
      ) => Promise<Array<{ id: string }>>;
    };
    let imageRefs: Array<{ id: string }> = [];
    if (encodedImages.length > 0) {
      if (!attachments?.saveImages) {
        throw new BridgeError(
          BRIDGE_ERROR_CODES.ATTACHMENT_INVALID,
          'image attachment store is not available',
        );
      }
      try {
        imageRefs = await attachments.saveImages(
          encodedImages.map((image) => ({
            data: Buffer.from(image.data, 'base64'),
            mediaType: image.mimeType,
            ...(image.name === undefined ? {} : { name: image.name }),
          })),
        );
      } catch (error) {
        throw new BridgeError(
          BRIDGE_ERROR_CODES.ATTACHMENT_INVALID,
          error instanceof Error ? error.message : String(error),
        );
      }
    }
    let imageIndex = 0;
    const blocks: SdkPromptContentBlock[] = params.contentBlocks.map((block) => {
      if (block.type === 'text' && 'text' in block) {
        return { type: 'text', text: String(block.text) };
      }
      if (block.type === 'image' && 'data' in block && typeof block.data === 'string') {
        const ref = imageRefs[imageIndex++];
        return ref ? { type: 'image', attachment: ref } : block;
      }
      return block;
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
    const registry = await this.workspaceRegistry();
    if (params.archived) {
      if (registry) {
        try {
          await registry.archiveSession(sessionId, { stopActivity: true });
        } catch (error) {
          throw new BridgeError(
            BRIDGE_ERROR_CODES.INTERNAL_ERROR,
            `failed to archive session "${sessionId}": ${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        }
      }
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
      if (registry) {
        try {
          await registry.unarchiveSession(sessionId);
        } catch (error) {
          throw new BridgeError(
            BRIDGE_ERROR_CODES.INTERNAL_ERROR,
            `failed to unarchive session "${sessionId}": ${
              error instanceof Error ? error.message : String(error)
            }`,
          );
        }
      }
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

  private async resolveScheduleAgent(sessionId: string): Promise<AgentLike> {
    const agentsService = this.ctx.get('agents') as unknown as {
      get: (id: string) => AgentLike | undefined;
      resume?: (options: object) => Promise<{ agent: AgentLike }>;
    };
    const live = agentsService?.get(sessionId);
    if (live) return live;
    if (!agentsService?.resume) {
      throw new SessionPersistenceNotFoundError(sessionId as SessionId);
    }
    try {
      const handle = await agentsService.resume({ resumeSessionId: sessionId });
      return handle.agent;
    } catch (error) {
      if (!isMissingSessionError(error)) throw error;
      throw new SessionPersistenceNotFoundError(sessionId as SessionId);
    }
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
    await this.attachSessionToWorkspace(session.header.cwd, childId);
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
        archiveSession: (sessionId: string, options?: object) => Promise<void>;
        unarchiveSession: (sessionId: string) => Promise<void>;
        readonly archivedSessionIds: readonly string[];
      }
    | undefined
  > {
    return this.waitForService('workspaceRegistry');
  }

  private async archivedSessionIds(): Promise<Set<string>> {
    const registry = await this.workspaceRegistry();
    const ids = new Set<string>(registry?.archivedSessionIds ?? []);
    for (const id of this.archivedSessions) ids.add(id);
    return ids;
  }

  private async attachSessionToWorkspace(
    cwd: string | undefined,
    sessionId: string,
  ): Promise<void> {
    if (!cwd) return;
    const registry = await this.workspaceRegistry();
    if (!registry) return;
    const workspace = await registry.resolveByPath(cwd);
    if (!workspace) return;
    await workspace.attachSession(sessionId);
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

  private async handleSessionProjections(
    params: SessionProjectionsParams,
  ): Promise<SessionProjectionsResult> {
    const { session, agent } = await this.resolveSessionEntry(params.sessionId);
    const projections = this.ctx.get('sessionProjections') as unknown as {
      snapshot: (
        session: SessionLike,
        keys: string[],
      ) => { asOfSeq: number; values: Record<string, unknown> };
    };
    if (!projections?.snapshot) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'session projection service is not available',
      );
    }
    const snapshot = projections.snapshot(session, [
      'tokenUsage',
      'contextPressure',
      'contextBreakdown',
    ]);
    const goals = this.ctx.get('goals') as unknown as {
      get: (agent: AgentLike) => GoalView | undefined;
    };
    return {
      asOfSeq: snapshot.asOfSeq,
      ...(snapshot.values.tokenUsage === undefined
        ? {}
        : { tokenUsage: snapshot.values.tokenUsage }),
      ...(snapshot.values.contextPressure === undefined
        ? {}
        : { contextPressure: snapshot.values.contextPressure }),
      ...(snapshot.values.contextBreakdown === undefined
        ? {}
        : { contextBreakdown: snapshot.values.contextBreakdown }),
      ...(agent === undefined ? {} : { goal: this.goalView(goals?.get(agent)) }),
    };
  }

  private async handleToolResultWatch(
    params: ToolResultWatchParams,
  ): Promise<ToolResultWatchResult> {
    await this.resolveSessionEntry(params.sessionId);
    if (params.subscribe) {
      this.watchedToolResults.add(params.sessionId);
    } else {
      this.watchedToolResults.delete(params.sessionId);
    }
    return {
      sessionId: params.sessionId,
      subscribed: this.watchedToolResults.has(params.sessionId),
    };
  }

  private mapFeedbackFailure(error: {
    code?: string;
    sessionId?: string;
    messageId?: string;
    item?: MessageFeedbackItem | null;
  }): never {
    if (error.code === 'session-not-found') {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${String(error.sessionId)}`,
      );
    }
    if (error.code === 'target-not-found') {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.FEEDBACK_TARGET_NOT_FOUND,
        `message not found: ${String(error.messageId)}`,
      );
    }
    if (error.code === 'version-conflict') {
      throw new BridgeError(BRIDGE_ERROR_CODES.FEEDBACK_CONFLICT, 'feedback version conflict', {
        item: error.item,
      });
    }
    throw new BridgeError(-32602, `feedback rejected: ${String(error.code)}`);
  }

  private mapFeedbackItem(item: MessageFeedbackItem): MessageFeedbackItem {
    return {
      messageId: item.messageId,
      rating: item.rating,
      ...(item.note === undefined ? {} : { note: item.note }),
      ...(item.category === undefined ? {} : { category: item.category }),
      version: item.version,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
    };
  }

  private async handleMessageFeedbackList(
    params: MessageFeedbackListParams,
  ): Promise<MessageFeedbackListResult> {
    const feedback = this.ctx.get('messageFeedback') as unknown as {
      list: (request: object) => Promise<
        | { ok: true; value: { items: MessageFeedbackItem[] } }
        | {
            ok: false;
            error: {
              code?: string;
              sessionId?: string;
              messageId?: string;
              item?: MessageFeedbackItem | null;
            };
          }
      >;
    };
    if (!feedback?.list) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'message feedback service is not available',
      );
    }
    const result = await feedback.list({ sessionId: params.sessionId });
    if (!result.ok) this.mapFeedbackFailure(result.error);
    return { items: result.value.items.map((item) => this.mapFeedbackItem(item)) };
  }

  private async handleMessageFeedbackPut(
    params: MessageFeedbackPutParams,
  ): Promise<MessageFeedbackPutResult> {
    const feedback = this.ctx.get('messageFeedback') as unknown as {
      put: (request: object) => Promise<
        | { ok: true; value: MessageFeedbackItem }
        | {
            ok: false;
            error: {
              code?: string;
              sessionId?: string;
              messageId?: string;
              item?: MessageFeedbackItem | null;
            };
          }
      >;
    };
    if (!feedback?.put) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'message feedback service is not available',
      );
    }
    const result = await feedback.put({
      sessionId: params.sessionId,
      messageId: params.messageId,
      rating: params.rating,
      ...(params.note === undefined ? {} : { note: params.note }),
      ...(params.category === undefined ? {} : { category: params.category }),
      ifVersion: params.ifVersion,
    });
    if (!result.ok) this.mapFeedbackFailure(result.error);
    return { item: this.mapFeedbackItem(result.value) };
  }

  private async handleMessageFeedbackDelete(
    params: MessageFeedbackDeleteParams,
  ): Promise<MessageFeedbackDeleteResult> {
    const feedback = this.ctx.get('messageFeedback') as unknown as {
      delete: (request: object) => Promise<
        | { ok: true; value: { absent: true } }
        | {
            ok: false;
            error: {
              code?: string;
              sessionId?: string;
              messageId?: string;
              item?: MessageFeedbackItem | null;
            };
          }
      >;
    };
    if (!feedback?.delete) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'message feedback service is not available',
      );
    }
    const result = await feedback.delete({
      sessionId: params.sessionId,
      messageId: params.messageId,
      ifVersion: params.ifVersion,
    });
    if (!result.ok) this.mapFeedbackFailure(result.error);
    return { absent: true };
  }

  private async handleSessionFeedbackRecord(
    params: SessionFeedbackRecordParams,
  ): Promise<SessionFeedbackRecordResult> {
    const feedback = this.ctx.get('sessionFeedback') as unknown as {
      record: (
        request: object,
      ) => Promise<
        | { ok: true; value: { recorded: true } }
        | { ok: false; error: { code?: string; sessionId?: string } }
      >;
    };
    if (!feedback?.record) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.INTERNAL_ERROR,
        'session feedback service is not available',
      );
    }
    const result = await feedback.record({
      sessionId: params.sessionId,
      ...(params.text === undefined ? {} : { text: params.text }),
      ...(params.category === undefined ? {} : { category: params.category }),
    });
    if (!result.ok) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    return { recorded: true };
  }

  private async handleQuestionReopen(params: QuestionReopenParams): Promise<QuestionReopenResult> {
    const request = this.pendingQuestionsBySession.get(params.sessionId);
    if (!request) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.QUESTION_NOT_PENDING,
        'no pending user question for this session',
      );
    }
    return { request };
  }

  private mapJob(job: {
    id: string;
    kind: string;
    label: string;
    owner?: string;
    status: JobWireView['status'];
    progress?: string;
    detail?: string;
    startedAt: number;
    finishedAt?: number;
    output: { total: number; earliest: number; spillPaths?: readonly string[] };
  }): JobWireView {
    return {
      id: job.id,
      kind: job.kind,
      label: job.label,
      ...(job.owner === undefined ? {} : { owner: job.owner }),
      status: job.status,
      ...(job.progress === undefined ? {} : { progress: job.progress }),
      ...(job.detail === undefined ? {} : { detail: job.detail }),
      startedAt: job.startedAt,
      ...(job.finishedAt === undefined ? {} : { finishedAt: job.finishedAt }),
      output: {
        total: job.output.total,
        earliest: job.output.earliest,
        ...(job.output.spillPaths === undefined ? {} : { spillPaths: [...job.output.spillPaths] }),
      },
    };
  }

  private mapJobChunk(chunk: JobChunkWire): JobChunkWire {
    return {
      at: chunk.at,
      text: chunk.text,
      ...(chunk.channel === undefined ? {} : { channel: chunk.channel }),
      ...(chunk.gapBefore === undefined ? {} : { gapBefore: chunk.gapBefore }),
    };
  }

  private jobService():
    | {
        list: (sessionId: string) => Array<object>;
        get: (jobId: string, sessionId: string) => object;
        readAt: (
          jobId: string,
          from: number,
          sessionId: string,
        ) => { chunks: JobChunkWire[]; next: number; lossy: boolean };
        kill: (jobId: string, sessionId: string, reason?: string) => JobKillResult['status'];
      }
    | undefined {
    return this.ctx.get('jobs') as
      | {
          list: (sessionId: string) => Array<object>;
          get: (jobId: string, sessionId: string) => object;
          readAt: (
            jobId: string,
            from: number,
            sessionId: string,
          ) => { chunks: JobChunkWire[]; next: number; lossy: boolean };
          kill: (jobId: string, sessionId: string, reason?: string) => JobKillResult['status'];
        }
      | undefined;
  }

  private jobError(error: unknown): BridgeError {
    if (error instanceof Error) {
      return new BridgeError(BRIDGE_ERROR_CODES.JOB_NOT_FOUND, error.message);
    }
    return new BridgeError(BRIDGE_ERROR_CODES.JOB_NOT_FOUND, String(error));
  }

  private async handleJobList(params: JobListParams): Promise<JobListResult> {
    const jobs = this.jobService();
    if (!jobs) {
      throw new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, 'jobs service is not available');
    }
    return {
      jobs: jobs
        .list(params.sessionId)
        .map((job) => this.mapJob(job as Parameters<BridgeService['mapJob']>[0])),
    };
  }

  private async handleJobFollow(params: JobFollowParams): Promise<JobFollowResult> {
    const jobs = this.jobService();
    if (!jobs) {
      throw new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, 'jobs service is not available');
    }
    try {
      const job = jobs.get(params.jobId, params.sessionId) as Parameters<
        BridgeService['mapJob']
      >[0];
      const read = jobs.readAt(params.jobId, params.from ?? 0, params.sessionId);
      const result: JobFollowResult = {
        job: this.mapJob(job),
        chunks: read.chunks.map((chunk) => this.mapJobChunk(chunk)),
        next: read.next,
        lossy: read.lossy,
      };
      const key = `${params.sessionId}:${params.jobId}`;
      clearInterval(this.jobTimers.get(key));
      if (job.status === 'running' || job.status === 'stopping') {
        let cursor = read.next;
        const timer = setInterval(() => {
          try {
            const next = jobs.readAt(params.jobId, cursor, params.sessionId);
            cursor = next.next;
            const current = jobs.get(params.jobId, params.sessionId) as Parameters<
              BridgeService['mapJob']
            >[0];
            const notification: JobOutputNotification = {
              sessionId: params.sessionId,
              jobId: params.jobId,
              job: this.mapJob(current),
              chunks: next.chunks.map((chunk) => this.mapJobChunk(chunk)),
              next: next.next,
              lossy: next.lossy,
            };
            this.server.notify('job.output', notification);
            if (current.status !== 'running' && current.status !== 'stopping') {
              clearInterval(timer);
              this.jobTimers.delete(key);
            }
          } catch {
            clearInterval(timer);
            this.jobTimers.delete(key);
          }
        }, 250);
        this.jobTimers.set(key, timer);
      }
      return result;
    } catch (error) {
      throw this.jobError(error);
    }
  }

  private async handleJobKill(params: JobKillParams): Promise<JobKillResult> {
    const jobs = this.jobService();
    if (!jobs) {
      throw new BridgeError(BRIDGE_ERROR_CODES.INTERNAL_ERROR, 'jobs service is not available');
    }
    try {
      return { status: jobs.kill(params.jobId, params.sessionId, params.reason) };
    } catch (error) {
      throw this.jobError(error);
    }
  }

  private async handleSessionSearch(params: SessionSearchParams): Promise<SessionSearchResult> {
    const sessions = this.ctx.get('sessions') as unknown as {
      list: () => SessionLike[];
    };
    const limit = Math.min(Math.max(params.limit ?? 50, 1), 200);
    const query = params.query.trim().toLowerCase();
    const includeArchived = params.archived ?? false;
    const archivedIds = await this.archivedSessionIds();
    const items: SessionSearchItem[] = [];
    for (const session of sessions?.list() ?? []) {
      const archived = archivedIds.has(session.id);
      if (archived && !includeArchived) continue;
      const events = session.snapshotEvents();
      const titleEvent = events.findLast((event) => event.type === 'session/title');
      const title =
        titleEvent?.data && typeof titleEvent.data === 'object' && 'title' in titleEvent.data
          ? String(titleEvent.data.title)
          : '';
      const cwd = session.header.cwd ?? '';
      if (
        query.length > 0 &&
        !title.toLowerCase().includes(query) &&
        !cwd.toLowerCase().includes(query)
      ) {
        continue;
      }
      const last = events.at(-1);
      items.push({
        id: session.id,
        title,
        cwd,
        updatedAt: last?.time ?? session.header.createdAt,
      });
    }
    items.sort((left, right) => right.updatedAt - left.updatedAt);
    return {
      items: items.slice(0, limit),
      hasMore: items.length > limit,
      fullText: false,
    };
  }

  private attachmentsRoot(sessionId: string): string {
    const sessionDir = path.basename(sessionId);
    if (sessionDir !== sessionId) {
      throw new BridgeError(BRIDGE_ERROR_CODES.ATTACHMENT_INVALID, 'invalid session id');
    }
    return path.join(
      process.env.DSH_HOME ?? process.cwd(),
      'attachments',
      'libadwaita',
      sessionDir,
    );
  }

  private async handleAttachmentUpload(
    params: AttachmentUploadParams,
  ): Promise<AttachmentUploadResult> {
    await this.resolveSessionEntry(params.sessionId);
    if (!params.name.trim() || typeof params.data !== 'string') {
      throw new BridgeError(BRIDGE_ERROR_CODES.ATTACHMENT_INVALID, 'name and data are required');
    }
    if (!/^[A-Za-z0-9+/]*={0,2}$/.test(params.data) || params.data.length % 4 !== 0) {
      throw new BridgeError(BRIDGE_ERROR_CODES.ATTACHMENT_INVALID, 'data must be valid base64');
    }
    const bytes = Buffer.from(params.data, 'base64');
    if (bytes.length > 16_777_216) {
      throw new BridgeError(BRIDGE_ERROR_CODES.ATTACHMENT_TOO_LARGE, 'attachment exceeds 16 MiB');
    }
    const safeName = path.basename(params.name).replace(/[^A-Za-z0-9._-]/g, '_');
    const attachmentId = crypto.randomUUID();
    const dir = this.attachmentsRoot(params.sessionId);
    await fs.mkdir(dir, { recursive: true, mode: 0o700 });
    await fs.writeFile(path.join(dir, `${attachmentId}__${safeName}`), bytes, { mode: 0o600 });
    return {
      attachmentId,
      name: safeName,
      size: bytes.length,
      mimeType: params.mimeType ?? 'application/octet-stream',
    };
  }

  private async handleAttachmentRead(params: AttachmentReadParams): Promise<AttachmentReadResult> {
    await this.resolveSessionEntry(params.sessionId);
    if (!/^[0-9a-f-]{36}$/i.test(params.attachmentId)) {
      throw new BridgeError(BRIDGE_ERROR_CODES.ATTACHMENT_INVALID, 'invalid attachment id');
    }
    const dir = this.attachmentsRoot(params.sessionId);
    let entries: string[];
    try {
      entries = await fs.readdir(dir);
    } catch {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ATTACHMENT_NOT_FOUND,
        `attachment not found: ${params.attachmentId}`,
      );
    }
    const match = entries.find((entry) => entry.startsWith(`${params.attachmentId}__`));
    if (!match) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ATTACHMENT_NOT_FOUND,
        `attachment not found: ${params.attachmentId}`,
      );
    }
    const bytes = await fs.readFile(path.join(dir, match));
    const encoding = params.encoding ?? 'base64';
    if (encoding === 'text') {
      const text = bytes.toString('utf8');
      if (bytes.includes(0) || text.includes('\uFFFD')) {
        throw new BridgeError(
          BRIDGE_ERROR_CODES.ATTACHMENT_INVALID,
          'attachment is not valid UTF-8 text',
        );
      }
      return {
        attachmentId: params.attachmentId,
        name: match.slice(match.indexOf('__') + 2),
        size: bytes.length,
        mimeType: 'text/plain',
        encoding: 'text',
        data: text,
      };
    }
    return {
      attachmentId: params.attachmentId,
      name: match.slice(match.indexOf('__') + 2),
      size: bytes.length,
      mimeType: 'application/octet-stream',
      encoding: 'base64',
      data: bytes.toString('base64'),
    };
  }

  private pluginManager():
    | {
        readBundleDetail?: (name: string, language: string, options?: object) => Promise<unknown>;
        inspect?: (spec: string, options?: object, signal?: AbortSignal) => Promise<unknown>;
        installBundle?: (spec: string, options?: object) => Promise<unknown>;
        cancelInstall?: (requestId: string) => Promise<unknown>;
        waitForInstall?: (requestId: string) => Promise<unknown>;
        removeBundle?: (name: string) => Promise<unknown>;
        listVersions?: (name: string, options?: object, signal?: AbortSignal) => Promise<unknown>;
        updateBundle?: (name: string, version: string, options?: object) => Promise<unknown>;
        registries?: () => Promise<unknown>;
        setPluginEnabled?: (id: string, enabled: boolean) => Promise<unknown>;
        setBundleEnabled?: (name: string, enabled: boolean) => Promise<unknown>;
        listPlugins?: () => Promise<unknown[]>;
        listBundles?: () => Promise<unknown[]>;
      }
    | undefined {
    return this.ctx.get('pluginManager') as
      | {
          readBundleDetail?: (name: string, language: string, options?: object) => Promise<unknown>;
          inspect?: (spec: string, options?: object, signal?: AbortSignal) => Promise<unknown>;
          installBundle?: (spec: string, options?: object) => Promise<unknown>;
          cancelInstall?: (requestId: string) => Promise<unknown>;
          waitForInstall?: (requestId: string) => Promise<unknown>;
          removeBundle?: (name: string) => Promise<unknown>;
          listVersions?: (name: string, options?: object, signal?: AbortSignal) => Promise<unknown>;
          updateBundle?: (name: string, version: string, options?: object) => Promise<unknown>;
          registries?: () => Promise<unknown>;
          setPluginEnabled?: (id: string, enabled: boolean) => Promise<unknown>;
          setBundleEnabled?: (name: string, enabled: boolean) => Promise<unknown>;
          listPlugins?: () => Promise<unknown[]>;
          listBundles?: () => Promise<unknown[]>;
        }
      | undefined;
  }

  private async pluginOperation<T>(operation: () => Promise<T>): Promise<PluginOperationResult> {
    try {
      return { result: await operation() };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async pluginChange(operation: () => Promise<unknown>): Promise<PluginToggleResult> {
    try {
      return (await operation()) as PluginToggleResult;
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handlePluginDetail(params: PluginDetailParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.readBundleDetail) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() =>
      manager.readBundleDetail!(params.name, params.language ?? 'en', {
        ...(params.npmFileSources === undefined ? {} : { npmFileSources: params.npmFileSources }),
      }),
    );
  }

  private async handlePluginInspect(params: PluginInspectParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.inspect) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() =>
      manager.inspect!(
        params.spec,
        params.registry === undefined ? undefined : { registry: params.registry },
        new AbortController().signal,
      ),
    );
  }

  private async handlePluginInstall(params: PluginInstallParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.installBundle) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() =>
      manager.installBundle!(params.spec, {
        ...(params.enabled === undefined ? {} : { enabled: params.enabled }),
        ...(params.saveExact === undefined ? {} : { saveExact: params.saveExact }),
        ...(params.requestId === undefined ? {} : { requestId: params.requestId }),
        ...(params.approvedBuilds === undefined ? {} : { approvedBuilds: params.approvedBuilds }),
        ...(params.registry === undefined ? {} : { registry: params.registry }),
      }),
    );
  }

  private async handlePluginCancelInstall(
    params: PluginRequestParams,
  ): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.cancelInstall) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() => manager.cancelInstall!(params.requestId));
  }

  private async handlePluginWaitInstall(
    params: PluginRequestParams,
  ): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.waitForInstall) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() => manager.waitForInstall!(params.requestId));
  }

  private async handlePluginRemove(params: PluginRemoveParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.removeBundle) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() => manager.removeBundle!(params.name));
  }

  private async handlePluginVersions(params: PluginVersionsParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.listVersions) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() =>
      manager.listVersions!(
        params.name,
        params.registry === undefined ? undefined : { registry: params.registry },
        new AbortController().signal,
      ),
    );
  }

  private async handlePluginUpdate(params: PluginUpdateParams): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.updateBundle) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() =>
      manager.updateBundle!(params.name, params.version, {
        ...(params.requestId === undefined ? {} : { requestId: params.requestId }),
        ...(params.registry === undefined ? {} : { registry: params.registry }),
      }),
    );
  }

  private async handlePluginRegistries(): Promise<PluginOperationResult> {
    const manager = this.pluginManager();
    if (!manager?.registries) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginOperation(() => manager.registries!());
  }

  private async handlePluginInventory(): Promise<PluginInventoryResult> {
    const manager = this.pluginManager();
    if (!manager?.listPlugins || !manager.listBundles) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return {
      plugins: await manager.listPlugins(),
      bundles: await manager.listBundles(),
    };
  }

  private async handleModelDiscover(params: ModelDiscoverParams): Promise<ModelDiscoverResult> {
    const llm = this.ctx.get('llm') as unknown as {
      discoverModels?: (
        settingsNs: string,
        request: object,
        signal: AbortSignal,
      ) => Promise<unknown[]>;
    };
    if (!llm?.discoverModels) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.MODEL_DISCOVERY_FAILED,
        'model discovery is not available',
      );
    }
    try {
      const models = await llm.discoverModels(
        params.settingsNs,
        {
          ...(params.provider === undefined ? {} : { provider: params.provider }),
          ...(params.baseURL === undefined ? {} : { baseURL: params.baseURL }),
          ...(params.api === undefined ? {} : { api: params.api }),
          ...(params.apiKey === undefined ? {} : { apiKey: params.apiKey }),
        },
        new AbortController().signal,
      );
      return { models };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.MODEL_DISCOVERY_FAILED,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handleWebProviders(): Promise<WebProvidersResult> {
    for (let attempt = 0; attempt < 50; attempt++) {
      const web = this.ctx.get('web') as unknown as {
        listProviders?: () => { search?: unknown[]; fetch?: unknown[] };
      };
      if (web?.listProviders) {
        const providers = web.listProviders();
        if ((providers.search?.length ?? 0) + (providers.fetch?.length ?? 0) > 0) {
          return { search: providers.search ?? [], fetch: providers.fetch ?? [] };
        }
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return { search: [], fetch: [] };
  }

  private async handleSubagentList(params: SubagentListParams): Promise<SubagentListResult> {
    const subagents = await this.waitForService<{
      listDescendants?: (sessionId: string, signal?: AbortSignal) => Promise<unknown[]>;
    }>('subagents');
    if (!subagents?.listDescendants) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SUBAGENTS_UNAVAILABLE,
        'subagent service is not available',
      );
    }
    return {
      entries: await subagents.listDescendants(params.sessionId, new AbortController().signal),
    };
  }

  private async handleSubagentPrompt(params: SubagentPromptParams): Promise<SubagentPromptResult> {
    const subagents = await this.waitForService<{
      prompt?: (request: object, signal: AbortSignal) => Promise<{ messageId: string }>;
    }>('subagents');
    if (!subagents?.prompt) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SUBAGENTS_UNAVAILABLE,
        'subagent service is not available',
      );
    }
    try {
      const result = await subagents.prompt(
        {
          requestId: params.requestId ?? crypto.randomUUID(),
          childSessionId: params.childSessionId,
          delivery: params.delivery ?? 'queue',
          content: params.content,
        },
        new AbortController().signal,
      );
      return { messageId: result.messageId };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SUBAGENTS_UNAVAILABLE,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handleSubagentInterrupt(
    params: SubagentInterruptParams,
  ): Promise<SubagentInterruptResult> {
    const subagents = await this.waitForService<{
      interruptByUser?: (childSessionId: string) => { accepted: true };
      interrupt?: (id: string, authority: object) => void;
    }>('subagents');
    if (!subagents) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SUBAGENTS_UNAVAILABLE,
        'subagent service is not available',
      );
    }
    if (subagents.interruptByUser) return subagents.interruptByUser(params.childSessionId);
    const interrupt = subagents.interrupt;
    if (!interrupt) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SUBAGENTS_UNAVAILABLE,
        'subagent interrupt is not available',
      );
    }
    interrupt.call(subagents, params.childSessionId, { kind: 'user' });
    return { accepted: true };
  }

  private async handleTeamGet(params: TeamGetParams): Promise<TeamGetResult> {
    const teams = await this.waitForService<{
      isEnabled: (agent: AgentLike) => boolean;
      listTasks: (agent: AgentLike) => unknown[];
    }>('agentTeams');
    if (!teams) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TEAM_UNAVAILABLE,
        'agent team service is not available',
      );
    }
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    return { enabled: teams.isEnabled(agent), tasks: teams.listTasks(agent) };
  }

  private async handleTeamSet(params: TeamSetParams): Promise<TeamSetResult> {
    const teams = await this.waitForService<{
      isEnabled: (agent: AgentLike) => boolean;
      setEnabled: (agent: AgentLike, enabled: boolean, signal?: AbortSignal) => Promise<void>;
    }>('agentTeams');
    if (!teams) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TEAM_UNAVAILABLE,
        'agent team service is not available',
      );
    }
    const { agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    await teams.setEnabled(agent, params.enabled, new AbortController().signal);
    return { enabled: teams.isEnabled(agent) };
  }

  private async handleCredentialRecordsList(): Promise<CredentialRecordsListResult> {
    const credentials = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (!credentials?.listRecords) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.CREDENTIAL_REJECTED,
        'credential service is not available',
      );
    }
    const entries = await credentials.listRecords();
    const records: CredentialRecordItem[] = [];
    for (const entry of entries) {
      const info = await credentials.describeRecord(entry.key);
      records.push({
        key: entry.key,
        kind: entry.kind,
        configured: info.configured,
        writable: info.writable,
      });
    }
    return { records };
  }

  private async handleCredentialRecordRead(
    params: CredentialRecordReadParams,
  ): Promise<CredentialRecordReadResult> {
    const credentials = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (!credentials?.readRecord) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.CREDENTIAL_REJECTED,
        'credential service is not available',
      );
    }
    const info = await credentials.describeRecord(params.key as never);
    const record = await credentials.readRecord(params.key as never);
    if (!record) {
      return { key: params.key, configured: false, writable: info.writable };
    }
    if (record.kind === 'api-key') {
      return {
        key: params.key,
        configured: info.configured,
        kind: record.kind,
        writable: info.writable,
        hasKey: record.key !== undefined && record.key.length > 0,
        envNames: Object.keys(record.env ?? {}),
      };
    }
    return {
      key: params.key,
      configured: info.configured,
      kind: record.kind,
      writable: info.writable,
      payloadRedacted: true,
    };
  }

  private async handleCredentialRecordModify(
    params: CredentialRecordModifyParams,
  ): Promise<CredentialRecordMutationResult> {
    const credentials = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (!credentials?.modifyRecord) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.CREDENTIAL_REJECTED,
        'credential service is not available',
      );
    }
    await credentials.modifyRecord(params.key as never, async () => params.record as never);
    return {
      metadata: await this.handleCredentialRecordRead({ key: params.key }),
    };
  }

  private async handleCredentialRecordDelete(
    params: CredentialRecordDeleteParams,
  ): Promise<CredentialRecordMutationResult> {
    const credentials = this.ctx.get('credentials') as unknown as BridgeCredentials | undefined;
    if (!credentials?.deleteRecord) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.CREDENTIAL_REJECTED,
        'credential service is not available',
      );
    }
    await credentials.deleteRecord(params.key as never);
    return {
      deleted: true,
      metadata: {
        key: params.key,
        configured: false,
        writable: true,
      },
    };
  }

  private accountService():
    | {
        getState: () => Promise<AccountStatusResult['state']>;
        getProfile: (client: object) => Promise<AccountProfileResult['profile']>;
        getBalance: (client: object) => Promise<AccountBalanceResult['balance']>;
        signOut: (client: object) => Promise<AccountSignOutResult['state']>;
      }
    | undefined {
    return this.ctx.get('deepseekAccount') as
      | {
          getState: () => Promise<AccountStatusResult['state']>;
          getProfile: (client: object) => Promise<AccountProfileResult['profile']>;
          getBalance: (client: object) => Promise<AccountBalanceResult['balance']>;
          signOut: (client: object) => Promise<AccountSignOutResult['state']>;
        }
      | undefined;
  }

  private accountClient(): object {
    return { version: '0.1.0', locale: 'en', timezoneOffsetSeconds: 0 };
  }

  private async handleAccountStatus(): Promise<AccountStatusResult> {
    const account = this.accountService();
    if (!account) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ACCOUNT_UNAVAILABLE,
        'account service is not available',
      );
    }
    return { state: await account.getState() };
  }

  private async handleAccountProfile(): Promise<AccountProfileResult> {
    const account = this.accountService();
    if (!account) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ACCOUNT_UNAVAILABLE,
        'account service is not available',
      );
    }
    return { profile: await account.getProfile(this.accountClient()) };
  }

  private async handleAccountBalance(): Promise<AccountBalanceResult> {
    const account = this.accountService();
    if (!account) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ACCOUNT_UNAVAILABLE,
        'account service is not available',
      );
    }
    return { balance: await account.getBalance(this.accountClient()) };
  }

  private async handleAccountSignOut(): Promise<AccountSignOutResult> {
    const account = this.accountService();
    if (!account) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.ACCOUNT_UNAVAILABLE,
        'account service is not available',
      );
    }
    return { state: await account.signOut(this.accountClient()) };
  }

  private async scheduleService(): Promise<
    | {
        catalog: () => Promise<unknown>;
        list: (request: object) => Promise<unknown>;
        create: (
          request: object,
          actorSessionId?: string,
          signal?: AbortSignal,
        ) => Promise<unknown>;
        edit: (request: object, actorSessionId?: string, signal?: AbortSignal) => Promise<unknown>;
        run: (request: object, signal?: AbortSignal) => Promise<unknown>;
        history: (request: object) => Promise<unknown>;
        deleteInactive: () => Promise<unknown>;
        deliveryLimits: () => unknown;
      }
    | undefined
  > {
    return this.waitForService('schedule');
  }

  private scheduleError(error: unknown): BridgeError {
    const message = error instanceof Error ? error.message : String(error);
    const code = (error as { code?: unknown } | null)?.code;
    if (typeof code !== 'string') {
      return new BridgeError(BRIDGE_ERROR_CODES.SCHEDULE_OPERATION_FAILED, message);
    }
    const details = (error as { details?: unknown }).details;
    return new BridgeError(BRIDGE_ERROR_CODES.SCHEDULE_OPERATION_FAILED, message, {
      code,
      ...(details === undefined ? {} : { details }),
    });
  }

  private async handleScheduleCatalog(): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    return { result: await schedule.catalog() };
  }

  private async handleScheduleCreate(
    params: ScheduleCreateParams,
  ): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    try {
      return {
        result: await schedule.create(
          {
            title: params.title,
            prompt: params.prompt,
            timing: params.timing,
            ...(params.target === undefined ? {} : { target: params.target }),
          },
          undefined,
          new AbortController().signal,
        ),
      };
    } catch (error) {
      throw this.scheduleError(error);
    }
  }

  private async handleScheduleEdit(params: ScheduleEditParams): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    try {
      return {
        result: await schedule.edit(
          {
            id: params.id,
            action: params.action,
            ...(params.patch === undefined ? {} : { patch: params.patch }),
            ...(params.expectedUpdatedAt === undefined
              ? {}
              : { expectedUpdatedAt: params.expectedUpdatedAt }),
          },
          undefined,
          new AbortController().signal,
        ),
      };
    } catch (error) {
      throw this.scheduleError(error);
    }
  }

  private async handleScheduleRun(params: ScheduleRunParams): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    try {
      return {
        result: await schedule.run({ id: params.id }, new AbortController().signal),
      };
    } catch (error) {
      throw this.scheduleError(error);
    }
  }

  private async handleScheduleHistory(
    params: ScheduleHistoryParams,
  ): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    return {
      result: await schedule.history({
        id: params.id,
        limit: Math.min(Math.max(params.limit ?? 50, 1), 100),
        ...(params.before === undefined ? {} : { before: params.before }),
      }),
    };
  }

  private async handleScheduleDelete(params: ScheduleEditParams): Promise<ScheduleOperationResult> {
    return this.handleScheduleEdit({
      id: params.id,
      action: 'delete',
      ...(params.expectedUpdatedAt === undefined
        ? {}
        : { expectedUpdatedAt: params.expectedUpdatedAt }),
    });
  }

  private async handleScheduleDeleteInactive(): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule?.deleteInactive) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    return { result: await schedule.deleteInactive() };
  }

  private async handleScheduleDeliveryLimits(): Promise<ScheduleOperationResult> {
    const schedule = await this.scheduleService();
    if (!schedule?.deliveryLimits) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SCHEDULE_UNAVAILABLE,
        'schedule service is not available',
      );
    }
    return { result: schedule.deliveryLimits() };
  }

  private async terminalService(): Promise<
    | {
        spawn: (
          owner: AgentLike,
          request: object,
          signal?: AbortSignal,
        ) => Promise<{
          sessionId: string;
          type: string;
          pid?: number;
          motd: string;
        }>;
        list: (owner: AgentLike) => Array<{
          sessionId: string;
          type: string;
          pid?: number;
          status: unknown;
        }>;
        startSend: (owner: AgentLike, id: string, request: object) => { done: Promise<unknown> };
        read: (owner: AgentLike, id: string, request?: object) => unknown;
        kill: (owner: AgentLike, id: string, reason?: string) => Promise<boolean>;
      }
    | undefined
  > {
    return this.waitForService('terminals');
  }

  private async terminalAgent(sessionId: string): Promise<AgentLike> {
    const { agent } = await this.resolveSessionEntry(sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${sessionId}`,
      );
    }
    return agent;
  }

  private terminalCwd(session: SessionLike): string {
    const workingDirectory = this.ctx.get('workingDirectory') as unknown as {
      get: (session: SessionLike) => string;
    };
    return workingDirectory?.get(session) ?? session.header.cwd ?? process.cwd();
  }

  private async handleTerminalCreate(params: TerminalCreateParams): Promise<TerminalCreateResult> {
    const terminals = await this.terminalService();
    if (!terminals) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        'terminal service is not available',
      );
    }
    const { session, agent } = await this.resolveSessionEntry(params.sessionId);
    if (!agent) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.SESSION_NOT_FOUND,
        `session not found: ${params.sessionId}`,
      );
    }
    try {
      const spawned = await terminals.spawn(
        agent,
        { type: 'shell', cwd: params.cwd ?? this.terminalCwd(session) },
        new AbortController().signal,
      );
      return {
        terminalId: spawned.sessionId,
        sessionId: params.sessionId,
        pid: spawned.pid ?? 0,
        cwd: this.terminalCwd(session),
      };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handleTerminalList(params: TerminalListParams): Promise<TerminalListResult> {
    const terminals = await this.terminalService();
    if (!terminals) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        'terminal service is not available',
      );
    }
    const agent = await this.terminalAgent(params.sessionId);
    return {
      terminals: terminals.list(agent).map((terminal) => ({
        terminalId: terminal.sessionId,
        pid: terminal.pid ?? 0,
        cwd: this.terminalCwd(agent.session),
        running: true,
      })),
    };
  }

  private async handleTerminalInput(params: TerminalInputParams): Promise<{ result: unknown }> {
    const terminals = await this.terminalService();
    if (!terminals) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        'terminal service is not available',
      );
    }
    const agent = await this.terminalAgent(params.sessionId);
    try {
      const operation = terminals.startSend(agent, params.terminalId, {
        text: params.data,
        submit: params.data.endsWith('\n'),
      });
      return { result: await operation.done };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_NOT_FOUND,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handleTerminalOutput(params: TerminalOutputParams): Promise<TerminalOutputResult> {
    const terminals = await this.terminalService();
    if (!terminals) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        'terminal service is not available',
      );
    }
    const agent = await this.terminalAgent(params.sessionId);
    try {
      return terminals.read(agent, params.terminalId, {
        ...(params.offset === undefined ? {} : { offset: params.offset }),
        ...(params.count === undefined ? {} : { count: params.count }),
      }) as TerminalOutputResult;
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_NOT_FOUND,
        error instanceof Error ? error.message : String(error),
      );
    }
  }

  private async handleTerminalClose(params: TerminalCloseParams): Promise<{ closed: boolean }> {
    const terminals = await this.terminalService();
    if (!terminals) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_UNAVAILABLE,
        'terminal service is not available',
      );
    }
    const agent = await this.terminalAgent(params.sessionId);
    try {
      return { closed: await terminals.kill(agent, params.terminalId) };
    } catch (error) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.TERMINAL_NOT_FOUND,
        error instanceof Error ? error.message : String(error),
      );
    }
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

  private async handlePluginEnable(params: PluginToggleParams): Promise<PluginToggleResult> {
    const manager = this.pluginManager();
    if (!manager?.setPluginEnabled) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginChange(() => manager.setPluginEnabled!(params.id, true));
  }

  private async handlePluginDisable(params: PluginToggleParams): Promise<PluginToggleResult> {
    const manager = this.pluginManager();
    if (!manager?.setPluginEnabled) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginChange(() => manager.setPluginEnabled!(params.id, false));
  }

  private async handlePluginBundleEnable(
    params: PluginBundleToggleParams,
  ): Promise<PluginToggleResult> {
    const manager = this.pluginManager();
    if (!manager?.setBundleEnabled) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginChange(() => manager.setBundleEnabled!(params.name, true));
  }

  private async handlePluginBundleDisable(
    params: PluginBundleToggleParams,
  ): Promise<PluginToggleResult> {
    const manager = this.pluginManager();
    if (!manager?.setBundleEnabled) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin manager is not available',
      );
    }
    return this.pluginChange(() => manager.setBundleEnabled!(params.name, false));
  }

  private async handlePluginRegistryProbe(): Promise<PluginRegistryProbeResult> {
    return { result: await this.registryProbe.fastest() };
  }

  private async pluginMarketService(): Promise<
    | {
        listIndexes?: () => unknown;
        setIndexes?: (urls: string[]) => Promise<void>;
        search?: (url: string, query: object) => Promise<unknown>;
        readEntryDetail?: (
          url: string,
          name: string,
          version: string,
          language: string,
          options?: object,
        ) => Promise<unknown>;
      }
    | undefined
  > {
    return this.waitForService('pluginMarket');
  }

  private async handlePluginMarketListIndexes(): Promise<PluginOperationResult> {
    const market = await this.pluginMarketService();
    if (!market?.listIndexes) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin market is not available',
      );
    }
    return this.pluginOperation(async () => await market.listIndexes!());
  }

  private async handlePluginMarketSetIndexes(
    params: PluginMarketSetIndexesParams,
  ): Promise<PluginOperationResult> {
    const market = await this.pluginMarketService();
    if (!market?.setIndexes) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin market is not available',
      );
    }
    return this.pluginOperation(async () => {
      await market.setIndexes!(params.urls);
      return null;
    });
  }

  private async handlePluginMarketSearch(
    params: PluginMarketSearchParams,
  ): Promise<PluginOperationResult> {
    const market = await this.pluginMarketService();
    if (!market?.search) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin market is not available',
      );
    }
    return this.pluginOperation(() => market.search!(params.url, params.query));
  }

  private async handlePluginMarketReadEntryDetail(
    params: PluginMarketReadEntryDetailParams,
  ): Promise<PluginOperationResult> {
    const market = await this.pluginMarketService();
    if (!market?.readEntryDetail) {
      throw new BridgeError(
        BRIDGE_ERROR_CODES.PLUGIN_OPERATION_FAILED,
        'plugin market is not available',
      );
    }
    return this.pluginOperation(() =>
      market.readEntryDetail!(params.url, params.name, params.version, params.language ?? 'en', {
        ...(params.refresh === undefined ? {} : { refresh: params.refresh }),
        ...(params.npmFileSources === undefined ? {} : { npmFileSources: params.npmFileSources }),
      }),
    );
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
