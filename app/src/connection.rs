use crate::protocol::*;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex};

pub const DEFAULT_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionError {
    Transport(String),
    Protocol(String),
    Timeout,
    Remote {
        code: i64,
        message: String,
        data: Option<serde_json::Value>,
    },
    Closed,
    MaxFrameExceeded(usize),
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(s) => write!(f, "Transport error: {}", s),
            Self::Protocol(s) => write!(f, "Protocol error: {}", s),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Remote { code, message, .. } => write!(f, "Remote error ({}): {}", code, message),
            Self::Closed => write!(f, "Connection is closed"),
            Self::MaxFrameExceeded(max) => write!(f, "Maximum frame size ({}) exceeded", max),
        }
    }
}

impl std::error::Error for ConnectionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected = 0,
    Connecting = 1,
    Connected = 2,
    Closing = 3,
    Closed = 4,
}

impl From<u8> for ConnectionState {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Disconnected,
            1 => Self::Connecting,
            2 => Self::Connected,
            3 => Self::Closing,
            _ => Self::Closed,
        }
    }
}

#[derive(Debug)]
pub struct NdjsonCodec {
    buffer: Vec<u8>,
    max_frame_size: usize,
}

impl NdjsonCodec {
    pub fn new(max_frame_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_frame_size,
        }
    }

    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<String>, ConnectionError> {
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            if pos > self.max_frame_size {
                return Err(ConnectionError::MaxFrameExceeded(self.max_frame_size));
            }

            let line_bytes = &self.buffer[..pos];
            let line_str = std::str::from_utf8(line_bytes)
                .map_err(|e| ConnectionError::Protocol(format!("UTF-8 decode error: {}", e)))?;
            let trimmed = line_str.trim();
            if !trimmed.is_empty() {
                frames.push(trimmed.to_string());
            }

            self.buffer.drain(..=pos);
        }

        if self.buffer.len() > self.max_frame_size {
            return Err(ConnectionError::MaxFrameExceeded(self.max_frame_size));
        }

        Ok(frames)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IncomingNotification {
    SessionEvent(SessionEventNotificationParams),
    AgentStream(AgentStreamNotificationParams),
    SessionStatus(SessionStatusNotificationParams),
    SessionTitle(SessionTitleNotificationParams),
    UserQuestionCancelled(UserQuestionCancelledParams),
    SessionInbox(SessionInboxNotificationParams),
    Unknown {
        method: String,
        params: serde_json::Value,
    },
}

impl IncomingNotification {
    pub fn from_raw(method: &str, params: serde_json::Value) -> Self {
        match method {
            NOTIFICATION_SESSION_EVENT => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::SessionEvent(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            NOTIFICATION_AGENT_STREAM => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::AgentStream(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            NOTIFICATION_SESSION_STATUS => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::SessionStatus(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            NOTIFICATION_SESSION_TITLE => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::SessionTitle(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            NOTIFICATION_USER_QUESTION_CANCELLED => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::UserQuestionCancelled(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            NOTIFICATION_SESSION_INBOX => {
                if let Ok(parsed) = serde_json::from_value(params.clone()) {
                    Self::SessionInbox(parsed)
                } else {
                    Self::Unknown {
                        method: method.to_string(),
                        params,
                    }
                }
            }
            _ => Self::Unknown {
                method: method.to_string(),
                params,
            },
        }
    }
}
pub trait ApprovalResponder: Send + Sync {
    fn respond(&self, req: ApprovalRequestParams) -> Pin<Box<dyn Future<Output = ApprovalOutcome> + Send>>;
}

impl<F, Fut> ApprovalResponder for F
where
    F: Fn(ApprovalRequestParams) -> Fut + Send + Sync,
    Fut: Future<Output = ApprovalOutcome> + Send + 'static,
{
    fn respond(&self, req: ApprovalRequestParams) -> Pin<Box<dyn Future<Output = ApprovalOutcome> + Send>> {
        Box::pin((self)(req))
    }
}

pub trait UserQuestionResponder: Send + Sync {
    fn respond(&self, req: SdkUserQuestionRequest) -> Pin<Box<dyn Future<Output = SdkUserQuestionResult> + Send>>;
}

impl<F, Fut> UserQuestionResponder for F
where
    F: Fn(SdkUserQuestionRequest) -> Fut + Send + Sync,
    Fut: Future<Output = SdkUserQuestionResult> + Send + 'static,
{
    fn respond(&self, req: SdkUserQuestionRequest) -> Pin<Box<dyn Future<Output = SdkUserQuestionResult> + Send>> {
        Box::pin((self)(req))
    }
}

struct Inner {
    state: AtomicU8,
    next_request_id: AtomicU64,
    pending_requests: Mutex<HashMap<RequestId, oneshot::Sender<Result<serde_json::Value, ConnectionError>>>>,
    outbound_tx: mpsc::Sender<String>,
    notification_tx: broadcast::Sender<IncomingNotification>,
    closed_tx: broadcast::Sender<()>,
    approval_responder: RwLock<Option<Arc<dyn ApprovalResponder>>>,
    user_question_responder: RwLock<Option<Arc<dyn UserQuestionResponder>>>,
    default_timeout: Duration,
}

#[derive(Clone)]
pub struct BridgeConnection {
    inner: Arc<Inner>,
}

impl BridgeConnection {
    pub async fn connect_unix(path: impl AsRef<Path>) -> Result<Self, ConnectionError> {
        Self::connect_unix_with_timeout(path, DEFAULT_TIMEOUT).await
    }

    pub async fn connect_unix_with_timeout(
        path: impl AsRef<Path>,
        timeout: Duration,
    ) -> Result<Self, ConnectionError> {
        let stream = UnixStream::connect(path.as_ref())
            .await
            .map_err(|e| ConnectionError::Transport(e.to_string()))?;
        Ok(Self::from_stream_with_timeout(stream, timeout))
    }

    pub fn from_stream<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        Self::from_stream_with_timeout(stream, DEFAULT_TIMEOUT)
    }

    pub fn from_stream_with_timeout<S>(stream: S, default_timeout: Duration) -> Self
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (outbound_tx, outbound_rx) = mpsc::channel(128);
        let (notification_tx, _) = broadcast::channel(512);
        let (closed_tx, _) = broadcast::channel(8);

        let inner = Arc::new(Inner {
            state: AtomicU8::new(ConnectionState::Connected as u8),
            next_request_id: AtomicU64::new(1),
            pending_requests: Mutex::new(HashMap::new()),
            outbound_tx,
            notification_tx,
            closed_tx,
            approval_responder: RwLock::new(None),
            user_question_responder: RwLock::new(None),
            default_timeout,
        });

        let conn = Self { inner };
        conn.spawn_io_loops(stream, outbound_rx);
        conn
    }

    pub fn state(&self) -> ConnectionState {
        ConnectionState::from(self.inner.state.load(Ordering::Acquire))
    }

    pub fn subscribe_closed(&self) -> broadcast::Receiver<()> {
        self.inner.closed_tx.subscribe()
    }

    pub fn set_approval_responder<R: ApprovalResponder + 'static>(&self, responder: R) {
        *self.inner.approval_responder.write().unwrap() = Some(Arc::new(responder));
    }

    pub fn set_user_question_responder<R: UserQuestionResponder + 'static>(&self, responder: R) {
        *self.inner.user_question_responder.write().unwrap() = Some(Arc::new(responder));
    }

    pub fn subscribe_notifications(&self) -> broadcast::Receiver<IncomingNotification> {
        self.inner.notification_tx.subscribe()
    }

    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: P,
        timeout: Duration,
    ) -> Result<R, ConnectionError> {
        if self.state() == ConnectionState::Closed {
            return Err(ConnectionError::Closed);
        }

        let req_id_num = self.inner.next_request_id.fetch_add(1, Ordering::SeqCst);
        let req_id = RequestId::from(format!("req-{}", req_id_num));

        let params_val = serde_json::to_value(params)
            .map_err(|e| ConnectionError::Protocol(format!("params serialization failed: {}", e)))?;

        let req = JsonRpcRequest::new(req_id.clone(), method, Some(params_val));
        let serialized = serde_json::to_string(&req)
            .map_err(|e| ConnectionError::Protocol(format!("request serialization failed: {}", e)))?;

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.inner.pending_requests.lock().await;
            pending.insert(req_id.clone(), tx);
        }

        if let Err(e) = self.inner.outbound_tx.send(serialized).await {
            let mut pending = self.inner.pending_requests.lock().await;
            pending.remove(&req_id);
            return Err(ConnectionError::Transport(format!("failed to enqueue request: {}", e)));
        }

        let res_result = match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(val)) => val,
            Ok(Err(_)) => {
                let mut pending = self.inner.pending_requests.lock().await;
                pending.remove(&req_id);
                return Err(ConnectionError::Closed);
            }
            Err(_) => {
                let mut pending = self.inner.pending_requests.lock().await;
                pending.remove(&req_id);
                return Err(ConnectionError::Timeout);
            }
        };

        match res_result {
            Ok(json_val) => {
                let parsed: R = serde_json::from_value(json_val)
                    .map_err(|e| ConnectionError::Protocol(format!("result deserialization failed: {}", e)))?;
                Ok(parsed)
            }
            Err(e) => Err(e),
        }
    }

    pub async fn send_notification<P: Serialize>(&self, method: &str, params: P) -> Result<(), ConnectionError> {
        if self.state() == ConnectionState::Closed {
            return Err(ConnectionError::Closed);
        }

        let params_val = serde_json::to_value(params)
            .map_err(|e| ConnectionError::Protocol(format!("notification params serialization failed: {}", e)))?;

        let notif = JsonRpcNotification::new(method, params_val);
        let serialized = serde_json::to_string(&notif)
            .map_err(|e| ConnectionError::Protocol(format!("notification serialization failed: {}", e)))?;

        self.inner
            .outbound_tx
            .send(serialized)
            .await
            .map_err(|e| ConnectionError::Transport(format!("failed to send notification: {}", e)))?;

        Ok(())
    }

    pub async fn close(&self) -> Result<(), ConnectionError> {
        self.inner.state.store(ConnectionState::Closing as u8, Ordering::Release);
        let _ = self.inner.closed_tx.send(());
        let mut pending = self.inner.pending_requests.lock().await;
        for (_, sender) in pending.drain() {
            let _ = sender.send(Err(ConnectionError::Closed));
        }
        self.inner.state.store(ConnectionState::Closed as u8, Ordering::Release);
        Ok(())
    }

    fn spawn_io_loops<S>(&self, stream: S, mut outbound_rx: mpsc::Receiver<String>)
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (mut reader, mut writer) = tokio::io::split(stream);

        tokio::spawn(async move {
            while let Some(frame) = outbound_rx.recv().await {
                let mut line = frame;
                line.push('\n');
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.flush().await.is_err() {
                    break;
                }
            }
            let _ = writer.shutdown().await;
        });

        let inner_clone = self.inner.clone();
        tokio::spawn(async move {
            let mut codec = NdjsonCodec::new(DEFAULT_MAX_FRAME_SIZE);
            let mut read_buf = vec![0u8; 8192];

            loop {
                let n = match reader.read(&mut read_buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };

                let frames = match codec.push_bytes(&read_buf[..n]) {
                    Ok(f) => f,
                    Err(_) => break,
                };

                for frame_str in frames {
                    let msg: Result<IncomingMessage, _> = serde_json::from_str(&frame_str);
                    match msg {
                        Ok(IncomingMessage::Response { id, result, error }) => {
                            let sender = {
                                let mut pending = inner_clone.pending_requests.lock().await;
                                pending.remove(&id)
                            };

                            if let Some(tx) = sender {
                                if let Some(err) = error {
                                    let _ = tx.send(Err(ConnectionError::Remote {
                                        code: err.code,
                                        message: err.message,
                                        data: err.data,
                                    }));
                                } else {
                                    let val = result.unwrap_or(serde_json::Value::Null);
                                    let _ = tx.send(Ok(val));
                                }
                            }
                        }
                        Ok(IncomingMessage::Request { id, method, params }) => {
                            let inner_for_req = inner_clone.clone();
                            tokio::spawn(async move {
                                Self::handle_server_request(inner_for_req, id, method, params).await;
                            });
                        }
                        Ok(IncomingMessage::Notification { method, params }) => {
                            let notif = IncomingNotification::from_raw(&method, params);
                            let _ = inner_clone.notification_tx.send(notif);
                        }
                        Err(_) => {}
                    }
                }
            }

            inner_clone.state.store(ConnectionState::Closed as u8, Ordering::Release);
            let _ = inner_clone.closed_tx.send(());
            let mut pending = inner_clone.pending_requests.lock().await;
            for (_, sender) in pending.drain() {
                let _ = sender.send(Err(ConnectionError::Closed));
            }
        });
    }

    async fn handle_server_request(
        inner: Arc<Inner>,
        id: RequestId,
        method: String,
        params: Option<serde_json::Value>,
    ) {
        match method.as_str() {
            METHOD_APPROVAL_REQUEST => {
                let outcome = match params.and_then(|p| serde_json::from_value::<ApprovalRequestParams>(p).ok()) {
                    Some(req_params) => {
                        let responder = { inner.approval_responder.read().unwrap().clone() };
                        if let Some(resp) = responder {
                            match tokio::time::timeout(inner.default_timeout, resp.respond(req_params)).await {
                                Ok(out) => out,
                                Err(_) => ApprovalOutcome::Rejected,
                            }
                        } else {
                            ApprovalOutcome::Rejected
                        }
                    }
                    None => ApprovalOutcome::Rejected,
                };

                let resp = JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: Some(serde_json::json!(ApprovalRequestResult { outcome })),
                    error: None,
                };
                if let Ok(line) = serde_json::to_string(&resp) {
                    let _ = inner.outbound_tx.send(line).await;
                }
            }
            METHOD_USER_QUESTION_ASK => {
                let result = match params.and_then(|p| serde_json::from_value::<SdkUserQuestionRequest>(p).ok()) {
                    Some(req_params) => {
                        let responder = { inner.user_question_responder.read().unwrap().clone() };
                        if let Some(resp) = responder {
                            match tokio::time::timeout(inner.default_timeout, resp.respond(req_params)).await {
                                Ok(res) => res,
                                Err(_) => SdkUserQuestionResult::cancelled(),
                            }
                        } else {
                            SdkUserQuestionResult::cancelled()
                        }
                    }
                    None => SdkUserQuestionResult::cancelled(),
                };

                let resp = JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: Some(serde_json::to_value(result).unwrap_or(serde_json::Value::Null)),
                    error: None,
                };
                if let Ok(line) = serde_json::to_string(&resp) {
                    let _ = inner.outbound_tx.send(line).await;
                }
            }
            _ => {
                let resp: JsonRpcResponse<serde_json::Value> = JsonRpcResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: ERR_METHOD_NOT_FOUND,
                        message: format!("Method '{}' not found", method),
                        data: None,
                    }),
                };
                if let Ok(line) = serde_json::to_string(&resp) {
                    let _ = inner.outbound_tx.send(line).await;
                }
            }
        }
    }
}
