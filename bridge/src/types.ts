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
  TITLE_INVALID: -32012,
  QUEUE_ITEM_NOT_FOUND: -32013,
  QUEUE_STEER_UNAVAILABLE: -32014,
  COMMANDS_UNAVAILABLE: -32015,
  COMMAND_NOT_FOUND: -32016,
  SKILLS_UNAVAILABLE: -32017,
  REFERENCES_UNAVAILABLE: -32018,
  FORK_UNAVAILABLE: -32019,
  SETTINGS_CONFLICT: -32020,
  WORKSPACE_NOT_FOUND: -32021,
  WORKSPACE_NAME_CONFLICT: -32022,
  WORKSPACE_UNAVAILABLE: -32023,
  PERMISSIONS_UNAVAILABLE: -32024,
  GOAL_UNAVAILABLE: -32025,
  GOAL_INVALID: -32026,
  FILE_NOT_FOUND: -32027,
  FILE_ACCESS_DENIED: -32028,
  FILE_TOO_LARGE: -32029,
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
  mode?: 'queue' | 'steer' | 'prompt';
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
  namespace?: string;
  patch: Record<string, unknown>;
  expectedRevision?: number;
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

export interface SessionRenameParams {
  sessionId: string;
  title: string;
}

export interface SessionRenameResult {
  sessionId: string;
  title: string;
  source: {
    kind: string;
    provider?: string;
  };
}

export interface SessionForkParams {
  sessionId: string;
  atSeq?: number;
  increaseTitle?: boolean;
}

export interface SessionForkResult {
  sessionId: string;
  header: SessionHeader;
}

export type QueuePlacement = 'queued' | 'steering' | 'context';

export interface QueueItem {
  id: string;
  placement: QueuePlacement;
  content: unknown[];
}

export interface QueueListParams {
  sessionId: string;
}

export interface QueueListResult {
  items: QueueItem[];
}

export interface QueueEditParams {
  sessionId: string;
  itemId: string;
  content: TextContentBlock[];
}

export interface QueueRemoveParams {
  sessionId: string;
  itemId: string;
}

export interface QueueSteerParams {
  sessionId: string;
  itemId: string;
}

export interface QueueMutationResult {
  accepted: true;
}

export interface CommandListParams {
  sessionId: string;
}

export interface CommandDescriptor {
  name: string;
  description: string;
  input?: {
    hint: string;
    attachments?: boolean;
  };
}

export interface CommandListResult {
  commands: CommandDescriptor[];
}

export interface CommandExecuteParams {
  sessionId: string;
  command: string;
  args?: string;
}

export interface CommandExecuteResult {
  commandId?: string;
  kind: 'success' | 'error';
  text?: string;
  sourceEventSeq?: number;
}

export interface SkillListParams {
  sessionId?: string;
  cwd?: string;
}

export interface SkillSummaryItem {
  name: string;
  description: string;
  whenToUse?: string;
  path?: string;
  source: string;
  provider: string;
  invocation: {
    modelInvocable: boolean;
    userInvocable: boolean;
  };
}

export interface SkillListResult {
  skills: SkillSummaryItem[];
  complete: boolean;
}

export interface ReferenceListParams {
  sessionId: string;
  query: string;
}

export interface ReferenceListResult {
  candidates: Array<{
    path: string;
    kind: 'file' | 'directory';
  }>;
}

export interface SessionInboxNotification {
  sessionId: string;
  items: QueueItem[];
}

export interface WorkspaceView {
  id: string;
  path: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  sessionIds: string[];
  status?: 'ok' | 'missing-dir';
}

export interface WorkspaceListResult {
  workspaces: WorkspaceView[];
}

export interface WorkspaceCreateParams {
  path: string;
  title?: string;
}

export interface WorkspaceCreateResult {
  workspace: WorkspaceView;
  created: boolean;
}

export interface WorkspaceRenameParams {
  workspaceId: string;
  title: string;
}

export interface WorkspaceRenameResult {
  workspace: WorkspaceView;
}

export interface WorkspaceRemoveParams {
  workspaceId: string;
}

export interface WorkspaceRemoveResult {
  deleted: boolean;
}

export interface SettingsDescriptor {
  ns: string;
  autoGenerate: boolean;
  schema: unknown;
  value: unknown;
  base?: unknown;
  user?: unknown;
  revision: number;
  secrets?: unknown;
}

export interface SettingsDescribeParams {
  keys?: string[];
}

export interface SettingsDescribeResult {
  namespaces: SettingsDescriptor[];
}

export interface SettingsUpdatedNotification {
  namespace: string;
  revision: number;
}

export interface PresetOptionItem {
  value: string;
  name: string;
  description?: string;
}

export interface PermissionCatalogResult {
  options: PresetOptionItem[];
  defaultOptions: PresetOptionItem[];
  defaultPreset: string;
}

export interface PermissionGetParams {
  sessionId: string;
}

export interface PermissionGetResult {
  currentValue: string;
  catalog: PermissionCatalogResult;
}

export interface PermissionSetParams {
  sessionId: string;
  preset: string;
}

export interface PermissionSetResult {
  currentValue: string;
}

export interface PermissionUpdatedNotification {
  sessionId: string;
  currentValue: string;
}

export interface GoalRef {
  id: string;
  revision: number;
}

export interface GoalView extends GoalRef {
  objective: string;
  phase: 'active' | 'paused' | 'blocked' | 'complete';
  blockedReason?: { code: string; message: string };
  maxGoalRounds: number;
  roundsStarted: number;
  createdAt: number;
  updatedAt: number;
  activation: 'armed' | 'disarmed';
}

export interface GoalGetParams {
  sessionId: string;
}

export interface GoalGetResult {
  goal: GoalView | null;
}

export interface GoalEditParams {
  sessionId: string;
  ref: GoalRef;
  objective?: string;
  maxGoalRounds?: number;
}

export interface GoalMutationParams {
  sessionId: string;
  ref: GoalRef;
}

export interface GoalMutationResult {
  goal: GoalView | null;
}

export interface GoalUpdatedNotification {
  sessionId: string;
  goal: GoalView | null;
}

export interface FilesListParams {
  sessionId: string;
  path?: string;
}

export interface FileEntry {
  name: string;
  path: string;
  kind: 'file' | 'directory' | 'symlink' | 'other';
  size: number;
}

export interface FilesListResult {
  path: string;
  entries: FileEntry[];
}

export interface FilesStatParams {
  sessionId: string;
  path: string;
}

export interface FilesStatResult {
  path: string;
  kind: 'file' | 'directory' | 'symlink' | 'other';
  size: number;
  mtime: number;
}

export interface FilesReadParams {
  sessionId: string;
  path: string;
  maxBytes?: number;
}

export type FilesReadResult =
  | {
      kind: 'text';
      path: string;
      text: string;
      size: number;
      truncated: boolean;
    }
  | {
      kind: 'binary';
      path: string;
      size: number;
    };

export interface FilesReadBytesParams {
  sessionId: string;
  path: string;
  maxBytes?: number;
}

export interface FilesReadBytesResult {
  path: string;
  data: string;
  size: number;
}
declare module '@deepseek-ai/cordis' {
  interface Events {
    'approval/request'(req: unknown, next: () => Promise<unknown>): Promise<unknown>;
    'user-questions/request'(req: unknown, next: () => Promise<unknown>): Promise<unknown>;
    'agent/assistant-stream'(payload: unknown): void;
    'session/event'(session: unknown, event: unknown): void;
    'agent/status'(payload: unknown): void;
    'settings/document-updated'(namespace: string, revision: number): void;
    'goal/activation-changed'(payload: unknown): void;
  }
}
