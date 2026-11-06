use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

pub const JSONRPC_VERSION: &str = "2.0";
pub const PROTOCOL_VERSION: u32 = 1;

pub const METHOD_INITIALIZE: &str = "libadwaita/initialize";
pub const METHOD_SHUTDOWN: &str = "libadwaita/shutdown";
pub const METHOD_SESSION_CREATE: &str = "libadwaita/session/create";
pub const METHOD_SESSION_LIST: &str = "libadwaita/session/list";
pub const METHOD_SESSION_FOLLOW: &str = "libadwaita/session/follow";
pub const METHOD_SESSION_PAGE: &str = "libadwaita/session/page";
pub const METHOD_SESSION_PROMPT: &str = "libadwaita/session/prompt";
pub const METHOD_SESSION_CANCEL: &str = "libadwaita/session/cancel";
pub const METHOD_SESSION_RELEASE: &str = "libadwaita/session/release";
pub const METHOD_SESSION_ARCHIVE: &str = "libadwaita/session/archive";
pub const METHOD_SESSION_RENAME: &str = "libadwaita/session/rename";
pub const METHOD_SESSION_FORK: &str = "libadwaita/session/fork";
pub const METHOD_QUEUE_LIST: &str = "libadwaita/session/queue/list";
pub const METHOD_QUEUE_EDIT: &str = "libadwaita/session/queue/edit";
pub const METHOD_QUEUE_REMOVE: &str = "libadwaita/session/queue/remove";
pub const METHOD_QUEUE_STEER: &str = "libadwaita/session/queue/steer";
pub const METHOD_SESSION_WORKDIR_GET: &str = "libadwaita/session/workdir/get";
pub const METHOD_SESSION_MODEL_GET: &str = "libadwaita/session/model/get";
pub const METHOD_SESSION_MODEL_SET: &str = "libadwaita/session/model/set";
pub const METHOD_MODEL_CATALOG: &str = "libadwaita/model/catalog";
pub const METHOD_SETTINGS_GET: &str = "libadwaita/settings/get";
pub const METHOD_SETTINGS_MUTATE: &str = "libadwaita/settings/mutate";
pub const METHOD_COMMAND_LIST: &str = "libadwaita/command/list";
pub const METHOD_COMMAND_EXECUTE: &str = "libadwaita/command/execute";
pub const METHOD_SKILL_LIST: &str = "libadwaita/skill/list";
pub const METHOD_REFERENCE_LIST: &str = "libadwaita/reference/list";
pub const METHOD_PLUGIN_LIST: &str = "libadwaita/plugin/list";
pub const METHOD_PLUGIN_ENABLE: &str = "libadwaita/plugin/enable";
pub const METHOD_PLUGIN_DISABLE: &str = "libadwaita/plugin/disable";
pub const METHOD_CREDENTIALS_STATUS: &str = "libadwaita/credentials/status";
pub const METHOD_CREDENTIALS_INJECT: &str = "libadwaita/credentials/inject";
pub const METHOD_CREDENTIALS_ROTATE: &str = "libadwaita/credentials/rotate";
pub const METHOD_CREDENTIALS_REMOVE: &str = "libadwaita/credentials/remove";
pub const METHOD_WORKSPACE_CHANGES: &str = "libadwaita/workspace/changes";

pub const METHOD_APPROVAL_REQUEST: &str = "approval/request";
pub const METHOD_USER_QUESTION_ASK: &str = "user-question/ask";

pub const NOTIFICATION_SESSION_EVENT: &str = "session.event";
pub const NOTIFICATION_AGENT_STREAM: &str = "agent.stream";
pub const NOTIFICATION_SESSION_STATUS: &str = "session.status";
pub const NOTIFICATION_SESSION_TITLE: &str = "session.title";
pub const NOTIFICATION_USER_QUESTION_CANCELLED: &str = "user-question.cancelled";
pub const NOTIFICATION_SESSION_INBOX: &str = "session.inbox";

pub const ERR_PARSE_ERROR: i64 = -32700;
pub const ERR_INVALID_REQUEST: i64 = -32600;
pub const ERR_METHOD_NOT_FOUND: i64 = -32601;
pub const ERR_INVALID_PARAMS: i64 = -32602;
pub const ERR_INTERNAL_ERROR: i64 = -32603;

pub const ERR_BRIDGE_INTERNAL_ERROR: i64 = -32000;
pub const ERR_PROTOCOL_VERSION_MISMATCH: i64 = -32001;
pub const ERR_SESSION_NOT_FOUND: i64 = -32002;
pub const ERR_SESSION_BUSY: i64 = -32003;
pub const ERR_TURN_NOT_CANCELLABLE: i64 = -32004;
pub const ERR_QUESTION_ALREADY_SETTLED: i64 = -32005;
pub const ERR_APPROVAL_FAIL_CLOSED: i64 = -32006;
pub const ERR_CREDENTIAL_REJECTED: i64 = -32007;
pub const ERR_MODEL_NOT_CONFIGURED: i64 = -32008;
pub const ERR_SEQUENCE_GAP_UNRECOVERABLE: i64 = -32009;
pub const ERR_RESTART_REQUIRED: i64 = -32010;
pub const ERR_WORKDIR_UNAVAILABLE: i64 = -32011;
pub const ERR_TITLE_INVALID: i64 = -32012;
pub const ERR_QUEUE_ITEM_NOT_FOUND: i64 = -32013;
pub const ERR_QUEUE_STEER_UNAVAILABLE: i64 = -32014;
pub const ERR_COMMANDS_UNAVAILABLE: i64 = -32015;
pub const ERR_COMMAND_NOT_FOUND: i64 = -32016;
pub const ERR_SKILLS_UNAVAILABLE: i64 = -32017;
pub const ERR_REFERENCES_UNAVAILABLE: i64 = -32018;
pub const ERR_FORK_UNAVAILABLE: i64 = -32019;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeErrorCode {
    ParseError,
    InvalidRequest,
    MethodNotFound,
    InvalidParams,
    InternalError,
    BridgeInternalError,
    ProtocolVersionMismatch,
    SessionNotFound,
    SessionBusy,
    TurnNotCancellable,
    QuestionAlreadySettled,
    ApprovalFailClosed,
    CredentialRejected,
    ModelNotConfigured,
    SequenceGapUnrecoverable,
    RestartRequired,
    WorkdirUnavailable,
    TitleInvalid,
    QueueItemNotFound,
    QueueSteerUnavailable,
    CommandsUnavailable,
    CommandNotFound,
    SkillsUnavailable,
    ReferencesUnavailable,
    ForkUnavailable,
    Unknown(i64),
}

impl BridgeErrorCode {
    pub fn from_code(code: i64) -> Self {
        match code {
            ERR_PARSE_ERROR => Self::ParseError,
            ERR_INVALID_REQUEST => Self::InvalidRequest,
            ERR_METHOD_NOT_FOUND => Self::MethodNotFound,
            ERR_INVALID_PARAMS => Self::InvalidParams,
            ERR_INTERNAL_ERROR => Self::InternalError,
            ERR_BRIDGE_INTERNAL_ERROR => Self::BridgeInternalError,
            ERR_PROTOCOL_VERSION_MISMATCH => Self::ProtocolVersionMismatch,
            ERR_SESSION_NOT_FOUND => Self::SessionNotFound,
            ERR_SESSION_BUSY => Self::SessionBusy,
            ERR_TURN_NOT_CANCELLABLE => Self::TurnNotCancellable,
            ERR_QUESTION_ALREADY_SETTLED => Self::QuestionAlreadySettled,
            ERR_APPROVAL_FAIL_CLOSED => Self::ApprovalFailClosed,
            ERR_CREDENTIAL_REJECTED => Self::CredentialRejected,
            ERR_MODEL_NOT_CONFIGURED => Self::ModelNotConfigured,
            ERR_SEQUENCE_GAP_UNRECOVERABLE => Self::SequenceGapUnrecoverable,
            ERR_RESTART_REQUIRED => Self::RestartRequired,
            ERR_WORKDIR_UNAVAILABLE => Self::WorkdirUnavailable,
            ERR_TITLE_INVALID => Self::TitleInvalid,
            ERR_QUEUE_ITEM_NOT_FOUND => Self::QueueItemNotFound,
            ERR_QUEUE_STEER_UNAVAILABLE => Self::QueueSteerUnavailable,
            ERR_COMMANDS_UNAVAILABLE => Self::CommandsUnavailable,
            ERR_COMMAND_NOT_FOUND => Self::CommandNotFound,
            ERR_SKILLS_UNAVAILABLE => Self::SkillsUnavailable,
            ERR_REFERENCES_UNAVAILABLE => Self::ReferencesUnavailable,
            ERR_FORK_UNAVAILABLE => Self::ForkUnavailable,
            other => Self::Unknown(other),
        }
    }

    pub fn to_code(self) -> i64 {
        match self {
            Self::ParseError => ERR_PARSE_ERROR,
            Self::InvalidRequest => ERR_INVALID_REQUEST,
            Self::MethodNotFound => ERR_METHOD_NOT_FOUND,
            Self::InvalidParams => ERR_INVALID_PARAMS,
            Self::InternalError => ERR_INTERNAL_ERROR,
            Self::BridgeInternalError => ERR_BRIDGE_INTERNAL_ERROR,
            Self::ProtocolVersionMismatch => ERR_PROTOCOL_VERSION_MISMATCH,
            Self::SessionNotFound => ERR_SESSION_NOT_FOUND,
            Self::SessionBusy => ERR_SESSION_BUSY,
            Self::TurnNotCancellable => ERR_TURN_NOT_CANCELLABLE,
            Self::QuestionAlreadySettled => ERR_QUESTION_ALREADY_SETTLED,
            Self::ApprovalFailClosed => ERR_APPROVAL_FAIL_CLOSED,
            Self::CredentialRejected => ERR_CREDENTIAL_REJECTED,
            Self::ModelNotConfigured => ERR_MODEL_NOT_CONFIGURED,
            Self::SequenceGapUnrecoverable => ERR_SEQUENCE_GAP_UNRECOVERABLE,
            Self::RestartRequired => ERR_RESTART_REQUIRED,
            Self::WorkdirUnavailable => ERR_WORKDIR_UNAVAILABLE,
            Self::TitleInvalid => ERR_TITLE_INVALID,
            Self::QueueItemNotFound => ERR_QUEUE_ITEM_NOT_FOUND,
            Self::QueueSteerUnavailable => ERR_QUEUE_STEER_UNAVAILABLE,
            Self::CommandsUnavailable => ERR_COMMANDS_UNAVAILABLE,
            Self::CommandNotFound => ERR_COMMAND_NOT_FOUND,
            Self::SkillsUnavailable => ERR_SKILLS_UNAVAILABLE,
            Self::ReferencesUnavailable => ERR_REFERENCES_UNAVAILABLE,
            Self::ForkUnavailable => ERR_FORK_UNAVAILABLE,
            Self::Unknown(code) => code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RequestId {
    String(String),
    Integer(i64),
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{}", s),
            Self::Integer(i) => write!(f, "{}", i),
        }
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i64> for RequestId {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<u64> for RequestId {
    fn from(i: u64) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<i32> for RequestId {
    fn from(i: i32) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<u32> for RequestId {
    fn from(i: u32) -> Self {
        Self::Integer(i as i64)
    }
}

impl Serialize for RequestId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::String(s) => serializer.serialize_str(s),
            Self::Integer(i) => serializer.serialize_i64(*i),
        }
    }
}

impl<'de> Deserialize<'de> for RequestId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            String(String),
            Integer(i64),
        }
        match Repr::deserialize(deserializer)? {
            Repr::String(s) => Ok(RequestId::String(s)),
            Repr::Integer(i) => Ok(RequestId::Integer(i)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl JsonRpcRequest<serde_json::Value> {
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse<T = serde_json::Value> {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl<T> JsonRpcResponse<T> {
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification<T = serde_json::Value> {
    pub jsonrpc: String,
    pub method: String,
    pub params: T,
}

impl JsonRpcNotification<serde_json::Value> {
    pub fn new(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawIncomingFrame {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<RequestId>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncomingMessage {
    Request {
        id: RequestId,
        method: String,
        params: Option<serde_json::Value>,
    },
    Response {
        id: RequestId,
        result: Option<serde_json::Value>,
        error: Option<JsonRpcError>,
    },
    Notification {
        method: String,
        params: serde_json::Value,
    },
}

impl<'de> Deserialize<'de> for IncomingMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawIncomingFrame::deserialize(deserializer)?;
        if let Some(id) = raw.id {
            if let Some(method) = raw.method {
                Ok(IncomingMessage::Request {
                    id,
                    method,
                    params: raw.params,
                })
            } else {
                Ok(IncomingMessage::Response {
                    id,
                    result: raw.result,
                    error: raw.error,
                })
            }
        } else if let Some(method) = raw.method {
            Ok(IncomingMessage::Notification {
                method,
                params: raw.params.unwrap_or(serde_json::Value::Null),
            })
        } else {
            Err(serde::de::Error::custom("unrecognized JSON-RPC message framing"))
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(default = "default_true")]
    pub user_questions: bool,
    #[serde(default = "default_true")]
    pub approval_requests: bool,
    #[serde(default = "default_true")]
    pub streaming: bool,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            user_questions: true,
            approval_requests: true,
            streaming: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub client_version: String,
    pub protocol_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<ClientCapabilities>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: u32,
    pub dsh_version: String,
    pub profile_name: String,
    pub generation_id: String,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShutdownResult {
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    pub version: u32,
    pub id: String,
    pub created_at: u64,
    pub cwd: String,
    #[serde(default)]
    pub is_seeded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub cwd: String,
    pub title: String,
    pub turn_count: u32,
    pub is_archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionListResult {
    pub sessions: Vec<SessionListItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTitleSnapshot {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub content: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjectionsSnapshot {
    pub plan_mode: bool,
    pub todos: Option<Vec<TodoItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamChunk {
    #[serde(rename = "text-delta")]
    TextDelta {
        index: u32,
        text: String,
    },
    #[serde(rename = "reasoning-delta")]
    ReasoningDelta {
        index: u32,
        text: String,
    },
    #[serde(rename = "tool-call-delta")]
    ToolCallDelta {
        index: u32,
        id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(rename = "argumentsDelta")]
        arguments_delta: String,
    },
    #[serde(rename = "block-start")]
    BlockStart {
        index: u32,
        #[serde(rename = "blockType")]
        block_type: String,
    },
    #[serde(rename = "block-end")]
    BlockEnd {
        index: u32,
        block: serde_json::Value,
    },
    #[serde(rename = "usage")]
    Usage {
        usage: serde_json::Value,
    },
    #[serde(rename = "finish")]
    Finish {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StreamOutcome {
    #[serde(rename = "committed")]
    Committed {
        #[serde(rename = "eventType")]
        event_type: String,
        seq: i64,
    },
    #[serde(rename = "abandoned")]
    Abandoned,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AssistantStreamFrame {
    #[serde(rename = "start", rename_all = "camelCase")]
    Start {
        attempt_id: String,
        revision: u64,
        turn: u32,
        step: u32,
    },
    #[serde(rename = "chunk", rename_all = "camelCase")]
    Chunk {
        attempt_id: String,
        revision: u64,
        index: u32,
        time: u64,
        chunk: StreamChunk,
    },
    #[serde(rename = "end", rename_all = "camelCase")]
    End {
        attempt_id: String,
        revision: u64,
        index: u32,
        outcome: StreamOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionAssistantBaseline {
    pub attempt_id: String,
    pub turn: u32,
    pub step: u32,
    pub revision: u64,
    pub chunks: Vec<StreamChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFollowParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFollowResult {
    pub header: SessionHeader,
    pub latest_seq: i64,
    pub status: String,
    pub title: Option<SessionTitleSnapshot>,
    pub projections: SessionProjectionsSnapshot,
    pub assistant_baseline: Option<SessionAssistantBaseline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPageParams {
    pub session_id: String,
    pub from_seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_seq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPageResult {
    pub events: Vec<SessionEvent>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub seq: i64,
    pub time: u64,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignorable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_op: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_event_seqs: Option<Vec<i64>>,
}

impl SessionEvent {
    pub fn is_ignorable(&self) -> bool {
        self.ignorable.unwrap_or(false)
    }

    pub fn parse_data<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_value(self.data.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventNotificationParams {
    pub session_id: String,
    pub event: SessionEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStreamNotificationParams {
    pub session_id: String,
    pub frame: AssistantStreamFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusNotificationParams {
    pub session_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTitleNotificationParams {
    pub session_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserQuestionCancelledParams {
    pub request_id: String,
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequestParams {
    pub session_id: String,
    pub tool_name: String,
    pub call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalOutcome {
    #[serde(rename = "allowed-once")]
    AllowedOnce,
    #[serde(rename = "rejected")]
    Rejected,
    #[serde(rename = "cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequestResult {
    pub outcome: ApprovalOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserQuestionOption {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum UserQuestionIntent {
    #[serde(rename = "plan-review")]
    PlanReview {
        approve: String,
        #[serde(rename = "callId", skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskUserQuestionItem {
    pub id: String,
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<UserQuestionIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<UserQuestionOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multi_select: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkUserQuestionRequest {
    pub request_id: String,
    pub session_id: String,
    pub questions: Vec<AskUserQuestionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserQuestionAnswer {
    pub id: String,
    pub selected: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkUserQuestionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answers: Option<Vec<UserQuestionAnswer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<bool>,
}

impl SdkUserQuestionResult {
    pub fn answers(answers: Vec<UserQuestionAnswer>) -> Self {
        Self {
            answers: Some(answers),
            cancelled: None,
        }
    }

    pub fn cancelled() -> Self {
        Self {
            answers: None,
            cancelled: Some(true),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PromptContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
    },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPromptParams {
    pub session_id: String,
    pub content_blocks: Vec<PromptContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPromptResult {
    pub message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_preset: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateResult {
    pub session_id: String,
    pub header: SessionHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCancelParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionCancelResult {
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReleaseParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReleaseResult {
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionArchiveParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionArchiveResult {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRenameParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRenameResult {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub title: String,
    pub source: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionForkParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_seq: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increase_title: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionForkResult {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub header: SessionHeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QueuePlacement {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "steering")]
    Steering,
    #[serde(rename = "context")]
    Context,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub placement: QueuePlacement,
    pub content: Vec<serde_json::Value>,
}

impl QueueItem {
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        for block in &self.content {
            if block["type"] == "text" {
                if let Some(text) = block["text"].as_str() {
                    parts.push(text);
                }
            }
        }
        parts.join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueListParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueListResult {
    pub items: Vec<QueueItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEditParams {
    pub session_id: String,
    pub item_id: String,
    pub content: Vec<PromptContentBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueRemoveParams {
    pub session_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueSteerParams {
    pub session_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueMutationResult {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInput {
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<CommandInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandListParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandListResult {
    pub commands: Vec<CommandDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecuteParams {
    pub session_id: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandExecuteResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_event_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInvocation {
    pub model_invocable: bool,
    pub user_invocable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummaryItem {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub source: String,
    pub provider: String,
    pub invocation: SkillInvocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillListResult {
    pub skills: Vec<SkillSummaryItem>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceListParams {
    pub session_id: String,
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCandidate {
    pub path: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceListResult {
    pub candidates: Vec<ReferenceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInboxNotificationParams {
    pub session_id: String,
    pub items: Vec<QueueItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorkdirGetParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorkdirResult {
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionModelGetParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelGetResult {
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelSetParams {
    pub session_id: String,
    pub provider: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDescriptor {
    pub id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    pub supports_reasoning: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProviderInfo {
    pub id: String,
    pub name: String,
    pub configured: bool,
    pub models: Vec<ModelDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogResult {
    pub default_provider: String,
    pub default_model: String,
    pub providers: Vec<ModelProviderInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsGetParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsResult {
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsMutateParams {
    pub patch: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfoItem {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub restart_required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginListResult {
    pub plugins: Vec<PluginInfoItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginToggleParams {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginToggleResult {
    pub id: String,
    pub enabled: bool,
    pub status: String,
    pub restart_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialStatusItem {
    pub provider: String,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masked: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialsStatusResult {
    pub items: Vec<CredentialStatusItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsInjectParams {
    pub provider: String,
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialsInjectResult {
    pub provider: String,
    pub configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialsRotateParams {
    pub provider: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialsRemoveParams {
    pub provider: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceChangesParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceChangesResult {
    pub summary: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallData {
    pub turn: u32,
    pub step: u32,
    #[serde(rename = "callId")]
    pub call_id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultData {
    pub turn: u32,
    pub step: u32,
    pub message: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanModeData {
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnStartData {
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnEndData {
    pub turn: u32,
    pub reason: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionTitleData {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoWriteData {
    pub todos: Vec<TodoItem>,
}
