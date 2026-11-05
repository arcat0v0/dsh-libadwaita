export const PROTOCOL_VERSION = 1;
export const DSH_PINNED_VERSION = '0.2.2-rc.1';

export const BRIDGE_ERROR_CODES = {
  INTERNAL_ERROR: -32000,
  PROTOCOL_VERSION_MISMATCH: -32001,
  SESSION_NOT_FOUND: -32002,
  SESSION_BUSY: -32003,
  TURN_NOT_CANCELLABLE: -32004,
  QUESTION_ALREADY_SETTLED: -32005,
  APPROVAL_FAIL_CLOSED: -32006,
  CREDENTIAL_REJECTED: -32007,
  MODEL_NOT_CONFIGURED: -32008,
  SEQUENCE_GAP_UNRECOVERABLE: -32009,
  RESTART_REQUIRED: -32010,
  WORKDIR_UNAVAILABLE: -32011,
} as const;

export class BridgeError extends Error {
  readonly code: number;
  readonly data: unknown;

  constructor(code: number, message: string, data?: unknown) {
    super(message);
    this.name = 'BridgeError';
    this.code = code;
    this.data = data;
  }
}

export interface ClientCapabilities {
  userQuestions?: boolean;
  approvalRequests?: boolean;
  streaming?: boolean;
}

export interface InitializeParams {
  clientVersion: string;
  protocolVersion: number;
  capabilities?: ClientCapabilities;
}

export interface InitializeResult {
  protocolVersion: number;
  dshVersion: string;
  profileName: string;
  generationId: string;
  serverInfo: {
    name: string;
    version: string;
  };
}

export interface SessionCreateParams {
  sessionId?: string;
  cwd: string;
  provider?: string;
  model?: string;
  agentPreset?: string;
}

export interface SessionHeader {
  version: number;
  id: string;
  createdAt: number;
  cwd?: string;
  parentSession?: string;
  isSeeded: boolean;
  origin?: 'subagent';
}

export interface SessionCreateResult {
  sessionId: string;
  header: SessionHeader;
}

export interface SessionListParams {
  archived?: boolean;
  limit?: number;
}

export interface SessionListItem {
  id: string;
  createdAt: number;
  updatedAt: number;
  cwd: string;
  title: string;
  turnCount: number;
  isArchived: boolean;
}

export interface SessionListResult {
  sessions: SessionListItem[];
}

export interface SessionTitleSnapshot {
  title: string;
  source: {
    kind: string;
    provider?: string;
  };
}

export interface TodoItem {
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export interface SessionProjectionsSnapshot {
  planMode: boolean;
  todos: TodoItem[] | null;
}

export interface StreamChunk {
  type: string;
  [key: string]: unknown;
}

export interface SessionAssistantBaseline {
  attemptId: string;
  turn: number;
  step: number;
  revision: number;
  chunks: StreamChunk[];
}

export interface SessionFollowParams {
  sessionId: string;
  fromSeq?: number;
  assistantStream?: boolean;
}

export interface SessionFollowResult {
  header: SessionHeader;
  latestSeq: number;
  status: 'idle' | 'running';
  title?: SessionTitleSnapshot | null;
  projections: SessionProjectionsSnapshot;
  assistantBaseline?: SessionAssistantBaseline | null;
}

export interface SessionPageParams {
  sessionId: string;
  fromSeq: number;
  toSeq?: number;
  limit?: number;
}

export interface SessionEvent {
  type: string;
  seq: number;
  time: number;
  data: unknown;
  ignorable?: true;
  surfaceOp?: unknown;
  sourceEventSeqs?: number[];
}

export interface SessionPageResult {
  events: SessionEvent[];
  hasMore: boolean;
}

export interface SdkEncodedImageBlock {
  type: 'image';
  data: string;
  mimeType: 'image/png' | 'image/jpeg' | 'image/webp' | 'image/gif';
  name?: string;
}

export interface TextContentBlock {
  type: 'text';
  text: string;
}

export type SdkPromptContentBlock =
  | TextContentBlock
  | SdkEncodedImageBlock
  | { type: string; [key: string]: unknown };

export interface SessionPromptParams {
  sessionId: string;
  contentBlocks: SdkPromptContentBlock[];
  mode?: 'prompt' | 'steer';
}

export interface SessionPromptResult {
  messageId: string;
}

export interface SessionCancelParams {
  sessionId: string;
  reason?: string;
}

export interface SessionCancelResult {
  cancelled: boolean;
}

export interface SessionArchiveParams {
  sessionId: string;
  archived: boolean;
}

export interface SessionArchiveResult {
  sessionId: string;
  archived: boolean;
}

export interface SessionWorkdirGetParams {
  sessionId: string;
}

export interface SessionWorkdirGetResult {
  cwd: string;
}

export interface SessionModelGetParams {
  sessionId: string;
}

export interface SessionModelGetResult {
  provider: string;
  model: string;
  reasoningEffort?: string;
}

export interface SessionModelSetParams {
  sessionId: string;
  provider: string;
  model: string;
  reasoningEffort?: string;
}

export interface ModelDescriptor {
  id: string;
  displayName: string;
  contextWindow?: number;
  supportsReasoning: boolean;
}

export interface ModelProviderInfo {
  id: string;
  name: string;
  configured: boolean;
  models: ModelDescriptor[];
}

export interface ModelCatalogResult {
  defaultProvider: string;
  defaultModel: string;
  providers: ModelProviderInfo[];
}

export interface SettingsGetParams {
  keys?: string[];
}

export interface SettingsGetResult {
  settings: Record<string, unknown>;
}

export interface SettingsMutateParams {
  patch: Record<string, unknown>;
}

export interface SettingsMutateResult {
  settings: Record<string, unknown>;
}

export interface PluginInfoItem {
  id: string;
  name: string;
  enabled: boolean;
  restartRequired: boolean;
  description?: string;
}

export interface PluginListResult {
  plugins: PluginInfoItem[];
}

export interface PluginToggleParams {
  id: string;
}

export interface PluginToggleResult {
  id: string;
  enabled: boolean;
  status: string;
  restartRequired: boolean;
}

export interface CredentialStatusItem {
  provider: string;
  configured: boolean;
  masked?: string;
}

export interface CredentialsStatusResult {
  items: CredentialStatusItem[];
}

export interface CredentialsInjectParams {
  provider: string;
  key: string;
  baseURL?: string;
}

export interface CredentialsInjectResult {
  provider: string;
  configured: boolean;
}

export interface CredentialsRotateParams {
  provider: string;
  key: string;
}

export interface CredentialsRotateResult {
  provider: string;
  configured: boolean;
}

export interface CredentialsRemoveParams {
  provider: string;
}

export interface CredentialsRemoveResult {
  provider: string;
  configured: boolean;
}

export interface WorkspaceChangesParams {
  sessionId: string;
  turn?: number;
  fileIndex?: number;
}

export interface WorkspaceChangesResult {
  summary: unknown | null;
  diff: unknown | null;
}

export interface ApprovalRequestParams {
  sessionId: string;
  toolName: string;
  callId?: string;
  reason?: string;
}

export interface ApprovalRequestResult {
  outcome: 'allowed-once' | 'rejected' | 'cancelled';
}

export interface AskUserQuestionOption {
  label: string;
  description?: string;
}

export interface AskUserQuestionItem {
  id: string;
  question: string;
  detail?: string;
  header?: string;
  options?: AskUserQuestionOption[];
  multiSelect?: boolean;
  intent?: {
    kind: string;
    approve?: string;
    callId?: string;
  };
}

export interface SdkUserQuestionRequest {
  requestId: string;
  sessionId: string;
  questions: AskUserQuestionItem[];
}

export interface AskUserQuestionAnswerItem {
  id: string;
  selected: string[];
  custom?: string;
}

export type SdkUserQuestionResult = { answers: AskUserQuestionAnswerItem[] } | { cancelled: true };

export interface SdkUserQuestionCancelled {
  requestId: string;
  sessionId: string;
}

export interface AssistantStreamFrame {
  type: 'start' | 'chunk' | 'end';
  attemptId: string;
  revision: number;
  [key: string]: unknown;
}

declare module '@deepseek-ai/cordis' {
  interface Events {
    'approval/request'(req: unknown, next: () => Promise<unknown>): Promise<unknown>;
    'user-questions/request'(req: unknown, next: () => Promise<unknown>): Promise<unknown>;
    'agent/assistant-stream'(payload: unknown): void;
    'session/event'(session: unknown, event: unknown): void;
    'agent/status'(payload: unknown): void;
  }
}
