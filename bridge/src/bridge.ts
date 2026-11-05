import crypto from 'node:crypto';
import { type Context, Service } from '@deepseek-ai/cordis';
import type { BridgeCredentials } from './credentials.js';
import { BridgeServer } from './server.js';
import {
  type AssistantStreamFrame,
  BRIDGE_ERROR_CODES,
  BridgeError,
  DSH_PINNED_VERSION,
  type ModelCatalogResult,
  type PluginListResult,
  type PluginToggleResult,
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
  type SessionTitleSnapshot,
  type SessionWorkdirGetParams,
  type SessionWorkdirGetResult,
  type SettingsGetParams,
  type SettingsGetResult,
  type SettingsMutateParams,
  type SettingsMutateResult,
  type StreamChunk,
  type TodoItem,
  type WorkspaceChangesParams,
  type WorkspaceChangesResult,
} from './types.js';

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
      case 'libadwaita/model/catalog':
        return this.handleModelCatalog();
      case 'libadwaita/settings/get':
        return this.handleSettingsGet(params as unknown as SettingsGetParams);
      case 'libadwaita/settings/mutate':
        return this.handleSettingsMutate(params as unknown as SettingsMutateParams);
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

  private async handleSettingsGet(params: SettingsGetParams): Promise<SettingsGetResult> {
    const settings = this.ctx.get('settings') as unknown as {
      describe?: (opt: object) => Array<{ ns: string; value: unknown }>;
    };
    if (!settings?.describe) {
      return { settings: {} };
    }

    const descriptors = settings.describe({ redactSecrets: true });
    const dict: Record<string, unknown> = {};
    const keys = params.keys ? new Set(params.keys) : undefined;

    for (const d of descriptors) {
      if (!keys || keys.has(d.ns)) {
        dict[d.ns] = d.value;
      }
    }
    return { settings: dict };
  }

  private async handleSettingsMutate(params: SettingsMutateParams): Promise<SettingsMutateResult> {
    const settings = this.ctx.get('settings') as unknown as {
      update?: (ns: string, patch: object) => Promise<void>;
      describe?: (opt: object) => Array<{ ns: string; value: unknown }>;
    };

    if (settings?.update) {
      for (const [ns, val] of Object.entries(params.patch)) {
        if (val && typeof val === 'object') {
          await settings.update(ns, val as object);
        }
      }
    }

    return this.handleSettingsGet({});
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
