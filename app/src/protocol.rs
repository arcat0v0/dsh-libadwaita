use crate::schedule_model::{
    ScheduleCatalogEntry, ScheduleDeliveryHistoryPage, SchedulePatch, ScheduleTargetRequest,
    ScheduleTiming,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::collections::HashMap;
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
pub const METHOD_SETTINGS_DESCRIBE: &str = "libadwaita/settings/describe";
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
pub const METHOD_WORKSPACE_LIST: &str = "libadwaita/workspace/list";
pub const METHOD_WORKSPACE_CREATE: &str = "libadwaita/workspace/create";
pub const METHOD_WORKSPACE_RENAME: &str = "libadwaita/workspace/rename";
pub const METHOD_WORKSPACE_REMOVE: &str = "libadwaita/workspace/remove";
pub const METHOD_PERMISSION_CATALOG: &str = "libadwaita/permission/catalog";
pub const METHOD_PERMISSION_GET: &str = "libadwaita/permission/get";
pub const METHOD_PERMISSION_SET: &str = "libadwaita/permission/set";
pub const METHOD_GOAL_GET: &str = "libadwaita/goal/get";
pub const METHOD_GOAL_EDIT: &str = "libadwaita/goal/edit";
pub const METHOD_GOAL_PAUSE: &str = "libadwaita/goal/pause";
pub const METHOD_GOAL_RESUME: &str = "libadwaita/goal/resume";
pub const METHOD_GOAL_CLEAR: &str = "libadwaita/goal/clear";
pub const METHOD_FILES_LIST: &str = "libadwaita/files/list";
pub const METHOD_FILES_STAT: &str = "libadwaita/files/stat";
pub const METHOD_FILES_READ: &str = "libadwaita/files/read";
pub const METHOD_FILES_READ_BYTES: &str = "libadwaita/files/readBytes";
pub const METHOD_APPROVAL_REQUEST: &str = "approval/request";
pub const METHOD_USER_QUESTION_ASK: &str = "user-question/ask";

pub const NOTIFICATION_SESSION_EVENT: &str = "session.event";
pub const NOTIFICATION_AGENT_STREAM: &str = "agent.stream";
pub const NOTIFICATION_SESSION_STATUS: &str = "session.status";
pub const NOTIFICATION_SESSION_TITLE: &str = "session.title";
pub const NOTIFICATION_USER_QUESTION_CANCELLED: &str = "user-question.cancelled";
pub const NOTIFICATION_SESSION_INBOX: &str = "session.inbox";
pub const NOTIFICATION_SETTINGS_UPDATED: &str = "settings.updated";
pub const NOTIFICATION_PERMISSION_UPDATED: &str = "permission.updated";
pub const NOTIFICATION_GOAL_UPDATED: &str = "goal.updated";

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
pub const ERR_SETTINGS_CONFLICT: i64 = -32020;
pub const ERR_WORKSPACE_NOT_FOUND: i64 = -32021;
pub const ERR_WORKSPACE_NAME_CONFLICT: i64 = -32022;
pub const ERR_WORKSPACE_UNAVAILABLE: i64 = -32023;
pub const ERR_PERMISSIONS_UNAVAILABLE: i64 = -32024;
pub const ERR_GOAL_UNAVAILABLE: i64 = -32025;
pub const ERR_GOAL_INVALID: i64 = -32026;
pub const ERR_FILE_NOT_FOUND: i64 = -32027;
pub const ERR_FILE_ACCESS_DENIED: i64 = -32028;
pub const ERR_FILE_TOO_LARGE: i64 = -32029;
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
    SettingsConflict,
    WorkspaceNotFound,
    WorkspaceNameConflict,
    WorkspaceUnavailable,
    PermissionsUnavailable,
    GoalUnavailable,
    GoalInvalid,
    FileNotFound,
    FileAccessDenied,
    FileTooLarge,
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
            ERR_SETTINGS_CONFLICT => Self::SettingsConflict,
            ERR_WORKSPACE_NOT_FOUND => Self::WorkspaceNotFound,
            ERR_WORKSPACE_NAME_CONFLICT => Self::WorkspaceNameConflict,
            ERR_WORKSPACE_UNAVAILABLE => Self::WorkspaceUnavailable,
            ERR_PERMISSIONS_UNAVAILABLE => Self::PermissionsUnavailable,
            ERR_GOAL_UNAVAILABLE => Self::GoalUnavailable,
            ERR_GOAL_INVALID => Self::GoalInvalid,
            ERR_FILE_NOT_FOUND => Self::FileNotFound,
            ERR_FILE_ACCESS_DENIED => Self::FileAccessDenied,
            ERR_FILE_TOO_LARGE => Self::FileTooLarge,
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
            Self::SettingsConflict => ERR_SETTINGS_CONFLICT,
            Self::WorkspaceNotFound => ERR_WORKSPACE_NOT_FOUND,
            Self::WorkspaceNameConflict => ERR_WORKSPACE_NAME_CONFLICT,
            Self::WorkspaceUnavailable => ERR_WORKSPACE_UNAVAILABLE,
            Self::PermissionsUnavailable => ERR_PERMISSIONS_UNAVAILABLE,
            Self::GoalUnavailable => ERR_GOAL_UNAVAILABLE,
            Self::GoalInvalid => ERR_GOAL_INVALID,
            Self::FileNotFound => ERR_FILE_NOT_FOUND,
            Self::FileAccessDenied => ERR_FILE_ACCESS_DENIED,
            Self::FileTooLarge => ERR_FILE_TOO_LARGE,
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
    pub fn new(
        id: impl Into<RequestId>,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
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
            Err(serde::de::Error::custom(
                "unrecognized JSON-RPC message framing",
            ))
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
    TextDelta { index: u32, text: String },
    #[serde(rename = "reasoning-delta")]
    ReasoningDelta { index: u32, text: String },
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
    Usage { usage: serde_json::Value },
    #[serde(rename = "finish")]
    Finish { reason: String },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_localized_text",
        skip_serializing_if = "Option::is_none"
    )]
    pub display_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LocalizedText {
    Text(String),
    Localized(HashMap<String, String>),
}

fn deserialize_localized_text<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<LocalizedText>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        LocalizedText::Text(text) => Some(text),
        LocalizedText::Localized(mut map) => map
            .remove("zh")
            .or_else(|| map.remove("en"))
            .or_else(|| map.into_iter().next().map(|(_, text)| text)),
    }))
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
    Text { text: String },
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
#[serde(rename_all = "camelCase")]
pub struct SettingsMutateParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub patch: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDescriptor {
    pub ns: String,
    #[serde(default)]
    pub auto_generate: bool,
    pub schema: serde_json::Value,
    pub value: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<serde_json::Value>,
    pub revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettingsDescribeResult {
    pub namespaces: Vec<SettingsDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: String,
    pub path: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub session_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceListResult {
    pub workspaces: Vec<WorkspaceView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCreateParams {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCreateResult {
    pub workspace: WorkspaceView,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRenameParams {
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRenameResult {
    pub workspace: WorkspaceView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRemoveParams {
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceRemoveResult {
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetOptionItem {
    pub value: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionCatalogResult {
    pub options: Vec<PresetOptionItem>,
    pub default_options: Vec<PresetOptionItem>,
    pub default_preset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionGetParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionGetResult {
    pub current_value: String,
    pub catalog: PermissionCatalogResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSetParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub preset: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSetResult {
    pub current_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalRef {
    pub id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalView {
    pub id: String,
    pub revision: u64,
    pub objective: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<serde_json::Value>,
    pub max_goal_rounds: u64,
    pub rounds_started: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub activation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalGetParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalGetResult {
    pub goal: Option<GoalView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalEditParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub r#ref: GoalRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub objective: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_goal_rounds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalActionParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub r#ref: GoalRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalActionResult {
    pub goal: Option<GoalView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesListParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesListResult {
    pub path: String,
    pub entries: Vec<FileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesStatParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesStatResult {
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub mtime: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesReadParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesReadResult {
    pub kind: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesReadBytesParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilesReadBytesResult {
    pub path: String,
    pub data: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsUpdatedNotificationParams {
    pub namespace: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionUpdatedNotificationParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub current_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalUpdatedNotificationParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub goal: Option<GoalView>,
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
pub struct PluginBundleToggleParams {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompatiblePlugin {
    pub name: String,
    pub version: String,
    #[serde(rename = "runtimeVersion")]
    pub runtime_version: String,
    pub requirements: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginChangeError {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incompatible: Option<Vec<IncompatiblePlugin>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginPackageResult {
    #[serde(rename = "exitCode")]
    pub exit_code: i64,
    pub output: String,
    pub truncated: bool,
    #[serde(rename = "logPath")]
    pub log_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginChangeResult {
    pub changed: bool,
    pub application: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restart_reason: Option<String>,
    pub stage: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PluginChangeError>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_result: Option<PluginPackageResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_builds: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_builds: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registries: Option<Vec<Option<String>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<String>,
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

pub const METHOD_PLUGIN_INVENTORY: &str = "libadwaita/plugin/inventory";
pub const METHOD_PLUGIN_DETAIL: &str = "libadwaita/plugin/detail";
pub const METHOD_PLUGIN_BUNDLE_ENABLE: &str = "libadwaita/plugin/bundle/enable";
pub const METHOD_PLUGIN_BUNDLE_DISABLE: &str = "libadwaita/plugin/bundle/disable";
pub const METHOD_PLUGIN_REGISTRY_PROBE: &str = "libadwaita/plugin/registry-probe";
pub const METHOD_PLUGIN_MARKET_LIST_INDEXES: &str = "libadwaita/plugin-market/list-indexes";
pub const METHOD_PLUGIN_MARKET_SET_INDEXES: &str = "libadwaita/plugin-market/set-indexes";
pub const METHOD_PLUGIN_MARKET_SEARCH: &str = "libadwaita/plugin-market/search";
pub const METHOD_PLUGIN_MARKET_READ_ENTRY_DETAIL: &str =
    "libadwaita/plugin-market/read-entry-detail";
pub const METHOD_PLUGIN_INSPECT: &str = "libadwaita/plugin/inspect";
pub const METHOD_PLUGIN_INSTALL: &str = "libadwaita/plugin/install";
pub const METHOD_PLUGIN_CANCEL_INSTALL: &str = "libadwaita/plugin/cancel-install";
pub const METHOD_PLUGIN_WAIT_INSTALL: &str = "libadwaita/plugin/wait-install";
pub const METHOD_PLUGIN_REMOVE: &str = "libadwaita/plugin/remove";
pub const METHOD_PLUGIN_VERSIONS: &str = "libadwaita/plugin/versions";
pub const METHOD_PLUGIN_UPDATE: &str = "libadwaita/plugin/update";
pub const METHOD_PLUGIN_REGISTRIES: &str = "libadwaita/plugin/registries";
pub const METHOD_PLUGIN_RETRY: &str = "libadwaita/plugin/retry";
pub const METHOD_MODEL_DISCOVER: &str = "libadwaita/model/discover";
pub const METHOD_WEB_SEARCH_PROVIDERS: &str = "libadwaita/web-search/providers";
pub const METHOD_SUBAGENT_LIST: &str = "libadwaita/subagent/list";
pub const METHOD_SUBAGENT_PROMPT: &str = "libadwaita/subagent/prompt";
pub const METHOD_SUBAGENT_INTERRUPT: &str = "libadwaita/subagent/interrupt";
pub const METHOD_AGENT_TEAM_GET: &str = "libadwaita/agent-team/get";
pub const METHOD_AGENT_TEAM_SET_ENABLED: &str = "libadwaita/agent-team/set-enabled";
pub const METHOD_CREDENTIALS_RECORDS_LIST: &str = "libadwaita/credentials/records/list";
pub const METHOD_CREDENTIALS_RECORDS_READ: &str = "libadwaita/credentials/records/read";
pub const METHOD_CREDENTIALS_RECORDS_MODIFY: &str = "libadwaita/credentials/records/modify";
pub const METHOD_CREDENTIALS_RECORDS_DELETE: &str = "libadwaita/credentials/records/delete";
pub const METHOD_ACCOUNT_STATUS: &str = "libadwaita/account/status";
pub const METHOD_ACCOUNT_PROFILE: &str = "libadwaita/account/profile";
pub const METHOD_ACCOUNT_BALANCE: &str = "libadwaita/account/balance";
pub const METHOD_ACCOUNT_SIGN_OUT: &str = "libadwaita/account/sign-out";
pub const METHOD_MESSAGE_FEEDBACK_LIST: &str = "libadwaita/message-feedback/list";
pub const METHOD_MESSAGE_FEEDBACK_PUT: &str = "libadwaita/message-feedback/put";
pub const METHOD_MESSAGE_FEEDBACK_DELETE: &str = "libadwaita/message-feedback/delete";
pub const METHOD_SESSION_FEEDBACK_RECORD: &str = "libadwaita/session-feedback/record";
pub const METHOD_QUESTION_REOPEN: &str = "libadwaita/question/reopen";
pub const METHOD_JOB_LIST: &str = "libadwaita/job/list";
pub const METHOD_JOB_FOLLOW: &str = "libadwaita/job/follow";
pub const METHOD_JOB_KILL: &str = "libadwaita/job/kill";

pub const NOTIFICATION_JOB_OUTPUT: &str = "job.output";
pub const NOTIFICATION_PLUGIN_INSTALL_STATE: &str = "plugin.install-state";
pub const NOTIFICATION_PLUGIN_INSTALL_LOG: &str = "plugin.install-log";
pub const NOTIFICATION_PLUGIN_CHANGED: &str = "plugin.changed";
pub const NOTIFICATION_PLUGIN_UPDATE_STATE: &str = "plugin.update-state";
pub const NOTIFICATION_PLUGIN_MARKET_CHANGED: &str = "plugin.market-changed";

pub const ERR_QUESTION_NOT_PENDING: i64 = -32030;
pub const ERR_FEEDBACK_CONFLICT: i64 = -32031;
pub const ERR_FEEDBACK_TARGET_NOT_FOUND: i64 = -32032;
pub const ERR_JOB_NOT_FOUND: i64 = -32033;
pub const ERR_ATTACHMENT_NOT_FOUND: i64 = -32035;
pub const ERR_ATTACHMENT_TOO_LARGE: i64 = -32036;
pub const ERR_ATTACHMENT_INVALID: i64 = -32037;
pub const ERR_AGENT_TEAM_UNAVAILABLE: i64 = -32040;
pub const ERR_MODEL_DISCOVERY_UNSUPPORTED: i64 = -32044;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInventoryResult {
    pub plugins: Vec<serde_json::Value>,
    pub bundles: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginOperationResult {
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDetailParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_file_sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginBundleDetail {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRegistries {
    pub registry: Option<String>,
    #[serde(rename = "fallbackRegistries", default)]
    pub fallback_registries: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRegistriesResult {
    pub result: PluginRegistries,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRegistryProbeResult {
    pub result: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginMarketListIndexesResult {
    pub result: Vec<MarketIndexSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginInspectParams {
    pub spec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PluginSpecInspection {
    #[serde(rename = "accepted")]
    Accepted {
        kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bundle: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host: Option<String>,
    },
    #[serde(rename = "refused")]
    Refused {
        problem: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registries: Option<Vec<Option<String>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCancellationResult {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryAttempt {
    pub registry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersionItem {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
    #[serde(default)]
    pub bundle: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incompatible: Option<IncompatiblePlugin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum PluginVersionList {
    #[serde(rename = "listed")]
    Listed {
        installed: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
        #[serde(default)]
        attempts: Vec<RegistryAttempt>,
        #[serde(default)]
        versions: Vec<PluginVersionItem>,
        #[serde(default)]
        omitted: usize,
    },
    #[serde(rename = "refused")]
    Refused {
        problem: String,
        reason: String,
        #[serde(default)]
        attempts: Vec<RegistryAttempt>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginUpdateResult {
    pub from: String,
    pub to: String,
    pub step: String,
    pub result: PluginChangeResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore: Option<PluginChangeResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallParams {
    pub spec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_exact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_builds: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRequestParams {
    pub request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRemoveParams {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersionsParams {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdateParams {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallStateNotificationParams {
    pub request_id: String,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallLogNotificationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub job_id: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub stream: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginChangedNotificationParams {
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdateStateNotificationParams {
    pub request_id: String,
    pub step: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketChangedNotificationParams {
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketIndexSummary {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarketSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MarketSort {
    #[serde(rename = "index")]
    Index,
    #[serde(rename = "relevance")]
    Relevance,
    #[serde(rename = "published")]
    Published { direction: MarketSortDirection },
    #[serde(rename = "key")]
    Key {
        id: String,
        direction: MarketSortDirection,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketDeclaration {
    #[serde(default)]
    pub title: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSortKeyDeclaration {
    #[serde(default)]
    pub title: serde_json::Value,
    pub order: MarketSortDirection,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketIndexMetadata {
    #[serde(default)]
    pub categories: HashMap<String, MarketDeclaration>,
    #[serde(default)]
    pub labels: HashMap<String, MarketDeclaration>,
    #[serde(default, rename = "sortKeys")]
    pub sort_keys: HashMap<String, MarketSortKeyDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketRepository {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MarketSource {
    #[serde(rename = "npm")]
    Npm {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        registry: Option<String>,
    },
    #[serde(rename = "git")]
    Git {
        url: String,
        commit: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketCompatibility {
    pub compatible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_version: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub requirements: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exempted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketInstallTarget {
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketListingVersion {
    pub version: String,
    pub publish_timestamp: u64,
    pub source: MarketSource,
    #[serde(default)]
    pub engines: HashMap<String, String>,
    pub install: MarketInstallTarget,
    pub compatibility: MarketCompatibility,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPluginMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default, rename = "sortKeys")]
    pub sort_keys: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketInstallation {
    #[serde(default)]
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketListingEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<MarketRepository>,
    #[serde(default)]
    pub metadata: MarketPluginMetadata,
    #[serde(default)]
    pub versions: Vec<MarketListingVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_version: Option<String>,
    #[serde(default)]
    pub installation: MarketInstallation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSearchResult {
    pub url: String,
    #[serde(default)]
    pub title: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<serde_json::Value>,
    #[serde(default)]
    pub metadata: MarketIndexMetadata,
    pub fetched_at: u64,
    #[serde(default)]
    pub entries: Vec<MarketListingEntry>,
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub next_offset: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
    pub sort: MarketSort,
    pub offset: u32,
    pub limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketSearchParams {
    pub url: String,
    pub query: MarketQuery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketSetIndexesParams {
    pub urls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketReadEntryDetailParams {
    pub url: String,
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npm_file_sources: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginMarketSearchResult {
    pub result: MarketSearchResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginMarketReadEntryDetailResult {
    pub result: PluginMarketDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketDetail {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentListParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubagentListResult {
    pub entries: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPromptParams {
    pub session_id: String,
    pub child_session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<String>,
    pub content: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentPromptResult {
    pub message_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentInterruptParams {
    pub session_id: String,
    pub child_session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentInterruptResult {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTeamGetParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentTeamGetResult {
    pub enabled: bool,
    pub tasks: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTeamSetParams {
    pub session_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTeamSetResult {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecordItem {
    pub key: String,
    pub kind: String,
    pub configured: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecordsListResult {
    pub records: Vec<CredentialRecordItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecordReadParams {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRecordReadResult {
    pub key: String,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub writable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env_names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_redacted: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialRecordModifyParams {
    pub key: String,
    pub record: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecordDeleteParams {
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRecordMutationResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    pub metadata: CredentialRecordReadResult,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountStatusResult {
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountProfileResult {
    pub profile: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountBalanceResult {
    pub balance: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountSignOutResult {
    pub state: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFeedbackItem {
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub rating: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub version: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
    #[serde(rename = "updatedAt")]
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageFeedbackListParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFeedbackListResult {
    pub items: Vec<MessageFeedbackItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageFeedbackPutParams {
    pub session_id: String,
    pub message_id: String,
    pub rating: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub if_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFeedbackPutResult {
    pub item: MessageFeedbackItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageFeedbackDeleteParams {
    pub session_id: String,
    pub message_id: String,
    pub if_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFeedbackDeleteResult {
    pub absent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFeedbackRecordParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFeedbackRecordResult {
    pub recorded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionReopenParams {
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionReopenResult {
    pub request: SdkUserQuestionRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobWireView {
    pub id: String,
    pub kind: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub started_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
    pub output: JobOutputState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutputState {
    pub total: u64,
    pub earliest: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spill_paths: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobChunkWire {
    pub at: u64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_before: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobListParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobListResult {
    pub jobs: Vec<JobWireView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobFollowParams {
    pub session_id: String,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobFollowResult {
    pub job: JobWireView,
    pub chunks: Vec<JobChunkWire>,
    pub next: u64,
    pub lossy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobKillParams {
    pub session_id: String,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobKillResult {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobOutputNotificationParams {
    pub session_id: String,
    pub job_id: String,
    pub job: JobWireView,
    pub chunks: Vec<JobChunkWire>,
    pub next: u64,
    pub lossy: bool,
}

pub fn json_text(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

pub const METHOD_TERMINAL_CREATE: &str = "libadwaita/terminal/create";
pub const METHOD_TERMINAL_LIST: &str = "libadwaita/terminal/list";
pub const METHOD_TERMINAL_INPUT: &str = "libadwaita/terminal/input";
pub const METHOD_TERMINAL_OUTPUT: &str = "libadwaita/terminal/output";
pub const METHOD_TERMINAL_CLOSE: &str = "libadwaita/terminal/close";
pub const METHOD_SCHEDULE_CATALOG: &str = "libadwaita/schedule/catalog";
pub const METHOD_SCHEDULE_CREATE: &str = "libadwaita/schedule/create";
pub const METHOD_SCHEDULE_EDIT: &str = "libadwaita/schedule/edit";
pub const METHOD_SCHEDULE_RUN: &str = "libadwaita/schedule/run";
pub const METHOD_SCHEDULE_HISTORY: &str = "libadwaita/schedule/history";
pub const METHOD_SCHEDULE_DELETE: &str = "libadwaita/schedule/delete";
pub const METHOD_SCHEDULE_DELETE_INACTIVE: &str = "libadwaita/schedule/delete-inactive";
pub const METHOD_SCHEDULE_DELIVERY_LIMITS: &str = "libadwaita/schedule/delivery-limits";
pub const NOTIFICATION_SCHEDULE_CHANGED: &str = "schedule.changed";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreateParams {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreateResult {
    pub terminal_id: String,
    pub session_id: String,
    pub pid: u32,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInfo {
    pub terminal_id: String,
    pub pid: u32,
    pub cwd: String,
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalListParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalListResult {
    pub terminals: Vec<TerminalInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalInputParams {
    pub session_id: String,
    pub terminal_id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputParams {
    pub session_id: String,
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputResult {
    pub text: String,
    pub total_lines: u64,
    pub line_begin: u64,
    pub line_end: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCloseParams {
    pub session_id: String,
    pub terminal_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalCloseResult {
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleCatalogResult {
    pub result: Vec<ScheduleCatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCreateParams {
    pub title: String,
    pub prompt: String,
    pub timing: ScheduleTiming,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ScheduleTargetRequest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleEditParams {
    pub id: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<SchedulePatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRunParams {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleHistoryParams {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScheduleHistoryOutcome {
    Page(ScheduleDeliveryHistoryPage),
    Miss { code: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleHistoryResult {
    pub result: ScheduleHistoryOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDeleteParams {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleOperationResult {
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleChangedNotificationParams {
    pub changed: bool,
}

pub fn build_question_answer(
    id: &str,
    selected: Vec<String>,
    free_text: &str,
) -> UserQuestionAnswer {
    let mut selected = selected;
    if selected.is_empty() {
        let trimmed = free_text.trim();
        if !trimmed.is_empty() {
            selected.push(trimmed.to_string());
        }
    }
    UserQuestionAnswer {
        id: id.to_string(),
        selected,
    }
}

pub const METHOD_ATTACHMENT_UPLOAD: &str = "libadwaita/attachment/upload";
pub const METHOD_DIAGNOSTICS_RECENT: &str = "libadwaita/diagnostics/recent";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRecentResult {
    pub entries: Vec<String>,
}pub const METHOD_ATTACHMENT_READ: &str = "libadwaita/attachment/read";
pub const METHOD_SESSION_SEARCH: &str = "libadwaita/session/search";
pub const METHOD_SESSION_PROJECTIONS: &str = "libadwaita/session/projections";
pub const NOTIFICATION_SESSION_PROJECTION: &str = "session.projection";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadParams {
    pub session_id: String,
    pub name: String,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadResult {
    pub attachment_id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentReadParams {
    pub session_id: String,
    pub attachment_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentReadResult {
    pub attachment_id: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub encoding: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchItem {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchResult {
    pub items: Vec<SessionSearchItem>,
    pub has_more: bool,
    pub full_text: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjectionsParams {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjectionsResult {
    pub as_of_seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_outline: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_stats: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_pressure: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_breakdown: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goal: Option<GoalView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjectionNotificationParams {
    pub session_id: String,
    pub kind: String,
    pub value: serde_json::Value,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserImageAttachment {
    pub name: Option<String>,
    pub mime_type: String,
    pub data: Option<String>,
    pub attachment_id: Option<String>,
}

pub fn parse_user_images(data: &serde_json::Value) -> Vec<UserImageAttachment> {
    let mut images = Vec::new();
    let Some(blocks) = data["content"].as_array() else {
        return images;
    };
    for block in blocks {
        if block["type"] != "image" {
            continue;
        }
        let attachment_id = block["attachment"]
            .as_str()
            .map(str::to_string)
            .or_else(|| block["attachment"]["id"].as_str().map(str::to_string))
            .or_else(|| block["attachmentId"].as_str().map(str::to_string));
        images.push(UserImageAttachment {
            name: block["name"].as_str().map(str::to_string),
            mime_type: block["mimeType"]
                .as_str()
                .unwrap_or("image/png")
                .to_string(),
            data: block["data"].as_str().map(str::to_string),
            attachment_id,
        });
    }
    images
}
