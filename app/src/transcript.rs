use crate::protocol::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallItem {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderBlock {
    UserMessage {
        text: String,
        time: u64,
        seq: i64,
        #[serde(default)]
        images: Vec<UserImageAttachment>,
    },
    QuestionReply {
        summary: String,
        selected: Vec<String>,
        details: serde_json::Value,
        seq: i64,
    },
    AssistantMessage {
        text: String,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCallItem>,
        is_live: bool,
        attempt_id: Option<String>,
        seq: Option<i64>,
        time: u64,
    },
    ToolResult {
        call_id: String,
        name: String,
        arguments: Option<String>,
        result: String,
        is_error: bool,
        seq: i64,
    },
    PlanModeBadge {
        active: bool,
        seq: i64,
    },
    PlanCard {
        plan: String,
        status: String,
        seq: i64,
    },
    TodoBlock {
        items: Vec<TodoItem>,
        seq: i64,
    },
    DeliverablesBlock {
        summary: serde_json::Value,
        diff: Option<serde_json::Value>,
        seq: i64,
    },
    GenericEvent {
        event_type: String,
        summary: String,
        seq: i64,
    },
    CompactionCard {
        summary: String,
        seq: i64,
    },
    TurnError {
        message: String,
        code: Option<String>,
        seq: i64,
    },
    MaxTokensHint {
        seq: i64,
    },
    RetryHint {
        message: String,
        seq: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveAttemptState {
    pub attempt_id: String,
    pub turn: u32,
    pub step: u32,
    pub revision: u64,
    pub next_index: u32,
    pub text: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<ToolCallItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapDetected {
    pub expected: i64,
    pub actual: i64,
}

impl std::fmt::Display for GapDetected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sequence gap detected: expected {}, got {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for GapDetected {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranscriptUpdate {
    Appended(usize),
    Updated(usize),
    Replaced(usize),
    Ignored,
}

#[derive(Debug, Clone)]
pub struct ConversationTranscript {
    blocks: Vec<RenderBlock>,
    last_seq: i64,
    stream_revision: u64,
    live_attempt: Option<LiveAttemptState>,
    pending_tool_calls: std::collections::HashMap<String, (String, String)>,
    pending_user: Option<usize>,
}

impl Default for ConversationTranscript {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationTranscript {
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            last_seq: -1,
            stream_revision: 0,
            live_attempt: None,
            pending_tool_calls: std::collections::HashMap::new(),
            pending_user: None,
        }
    }

    pub fn blocks(&self) -> &[RenderBlock] {
        &self.blocks
    }

    pub fn last_seq(&self) -> i64 {
        self.last_seq
    }

    pub fn latest_snippet(&self) -> Option<String> {
        self.blocks.iter().rev().find_map(|block| match block {
            RenderBlock::UserMessage { text, .. } => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            RenderBlock::AssistantMessage { text, .. } => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            RenderBlock::ToolResult { result, .. } => {
                let trimmed = result.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            RenderBlock::GenericEvent { summary, .. } => {
                let trimmed = summary.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            _ => None,
        })
    }

    pub fn set_last_seq(&mut self, seq: i64) {
        self.last_seq = seq;
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.last_seq = -1;
        self.stream_revision = 0;
        self.live_attempt = None;
        self.pending_tool_calls.clear();
        self.pending_user = None;
    }

    pub fn push_pending_user(&mut self, text: &str) {
        self.pending_user = Some(self.blocks.len());
        self.blocks.push(RenderBlock::UserMessage {
            text: text.to_string(),
            time: 0,
            seq: -2,
            images: Vec::new(),
        });
    }

    pub fn push_pending_user_with_images(
        &mut self,
        text: &str,
        images: &[UserImageAttachment],
    ) {
        self.push_pending_user(text);
        if let Some(index) = self.pending_user {
            if let Some(RenderBlock::UserMessage {
                images: block_images,
                ..
            }) = self.blocks.get_mut(index)
            {
                *block_images = images.to_vec();
            }
        }
    }

    pub fn clear_pending_user(&mut self) {
        if let Some(index) = self.pending_user.take() {
            if index < self.blocks.len() {
                self.blocks.remove(index);
            }
        }
    }

    pub fn first_seq(&self) -> Option<i64> {
        self.blocks
            .iter()
            .filter_map(|block| match block {
                RenderBlock::UserMessage { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::QuestionReply { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::AssistantMessage { seq: Some(seq), .. } if *seq > 0 => Some(*seq),
                RenderBlock::ToolResult { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::PlanModeBadge { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::PlanCard { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::TodoBlock { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::DeliverablesBlock { seq, .. } if *seq > 0 => Some(*seq),
                RenderBlock::GenericEvent { seq, .. } if *seq > 0 => Some(*seq),
                _ => None,
            })
            .min()
    }

    pub fn prepend_events(&mut self, events: Vec<SessionEvent>) {
        if events.is_empty() {
            return;
        }
        let mut temp = ConversationTranscript::new();
        for event in events {
            let _ = temp.apply_event(event);
        }
        let mut combined = temp.blocks().to_vec();
        combined.append(&mut self.blocks);
        self.blocks = combined;
        self.pending_user = None;
    }

    pub fn apply_baseline(&mut self, baseline: Option<SessionAssistantBaseline>) {
        if let Some(b) = baseline {
            let next_index = b.chunks.len() as u32;
            let mut state = LiveAttemptState {
                attempt_id: b.attempt_id.clone(),
                turn: b.turn,
                step: b.step,
                revision: b.revision,
                next_index,
                text: String::new(),
                reasoning: None,
                tool_calls: Vec::new(),
            };

            for chunk in b.chunks {
                Self::accumulate_chunk(&mut state, chunk);
            }

            self.stream_revision = b.revision;

            let block = RenderBlock::AssistantMessage {
                text: state.text.clone(),
                reasoning: state.reasoning.clone(),
                tool_calls: state.tool_calls.clone(),
                is_live: true,
                attempt_id: Some(b.attempt_id),
                seq: None,
                time: 0,
            };

            self.live_attempt = Some(state);
            self.blocks.push(block);
        }
    }

    pub fn apply_stream_frame(&mut self, frame: AssistantStreamFrame) {
        let (revision, is_start) = match &frame {
            AssistantStreamFrame::Start { revision, .. } => (*revision, true),
            AssistantStreamFrame::Chunk { revision, .. } => (*revision, false),
            AssistantStreamFrame::End { revision, .. } => (*revision, false),
        };

        if is_start && revision == 1 && self.stream_revision != 0 {
            self.retire_live_blocks();
            self.live_attempt = None;
            self.stream_revision = 0;
        }
        if revision != self.stream_revision + 1 {
            self.retire_live_blocks();
            self.live_attempt = None;
            self.stream_revision = revision;
            return;
        }
        self.stream_revision = revision;

        match frame {
            AssistantStreamFrame::Start {
                attempt_id,
                revision,
                turn,
                step,
            } => {
                self.retire_live_blocks();
                self.live_attempt = Some(LiveAttemptState {
                    attempt_id: attempt_id.clone(),
                    turn,
                    step,
                    revision,
                    next_index: 0,
                    text: String::new(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                });
                self.blocks.push(RenderBlock::AssistantMessage {
                    text: String::new(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    is_live: true,
                    attempt_id: Some(attempt_id),
                    seq: None,
                    time: 0,
                });
            }
            AssistantStreamFrame::Chunk {
                attempt_id,
                revision,
                index,
                chunk,
                ..
            } => {
                let dense = matches!(
                    &self.live_attempt,
                    Some(live)
                        if live.attempt_id == attempt_id && live.next_index == index
                );
                if !dense {
                    self.retire_live_blocks();
                    self.live_attempt = None;
                    return;
                }
                let Some(live) = self.live_attempt.as_mut() else {
                    return;
                };
                Self::accumulate_chunk(live, chunk);
                live.next_index += 1;
                live.revision = revision;
                let live = live.clone();
                self.update_live_block(&attempt_id, &live);
            }
            AssistantStreamFrame::End {
                attempt_id, outcome, ..
            } => {
                match outcome {
                    StreamOutcome::Abandoned => {
                        self.blocks.retain(|block| {
                            !matches!(
                                block,
                                RenderBlock::AssistantMessage {
                                    is_live: true,
                                    attempt_id: Some(id),
                                    ..
                                } if *id == attempt_id
                            )
                        });
                    }
                    StreamOutcome::Committed { seq, .. } => {
                        if let Some(RenderBlock::AssistantMessage {
                            is_live,
                            seq: block_seq,
                            ..
                        }) = self.blocks.iter_mut().find(|block| {
                            matches!(
                                block,
                                RenderBlock::AssistantMessage {
                                    is_live: true,
                                    attempt_id: Some(id),
                                    ..
                                } if *id == attempt_id
                            )
                        }) {
                            *is_live = false;
                            *block_seq = Some(seq);
                        }
                    }
                }
                self.live_attempt = None;
            }
        }
    }

    fn retire_live_blocks(&mut self) {
        self.blocks.retain(|block| {
            !matches!(
                block,
                RenderBlock::AssistantMessage { is_live: true, .. }
            )
        });
    }

    fn update_live_block(&mut self, attempt_id: &str, live: &LiveAttemptState) {
        let existing = self.blocks.iter_mut().find(|block| {
            matches!(
                block,
                RenderBlock::AssistantMessage {
                    is_live: true,
                    attempt_id: Some(id),
                    ..
                } if id == attempt_id
            )
        });
        if let Some(RenderBlock::AssistantMessage {
            text,
            reasoning,
            tool_calls,
            ..
        }) = existing
        {
            *text = live.text.clone();
            *reasoning = live.reasoning.clone();
            *tool_calls = live.tool_calls.clone();
        } else {
            self.blocks.push(RenderBlock::AssistantMessage {
                text: live.text.clone(),
                reasoning: live.reasoning.clone(),
                tool_calls: live.tool_calls.clone(),
                is_live: true,
                attempt_id: Some(attempt_id.to_string()),
                seq: None,
                time: 0,
            });
        }
    }

    pub fn apply_event(&mut self, event: SessionEvent) -> Result<TranscriptUpdate, GapDetected> {
        if self.last_seq >= 0 && event.seq > self.last_seq + 1 {
            return Err(GapDetected {
                expected: self.last_seq + 1,
                actual: event.seq,
            });
        }

        if self.last_seq >= 0 && event.seq <= self.last_seq {
            return Ok(TranscriptUpdate::Ignored);
        }

        self.last_seq = event.seq;

        match event.event_type.as_str() {
            "user/message" => {
                let text = Self::extract_user_text(&event.data);
                let source_kind = event.data["source"]["kind"].as_str().unwrap_or_default();
                if source_kind == "user-question-reply" {
                    let summary = event.data["source"]["question"]
                        .as_str()
                        .or_else(|| event.data["question"].as_str())
                        .unwrap_or(&text)
                        .to_string();
                    let selected = event.data["selected"]
                        .as_array()
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|value| value.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    self.blocks.push(RenderBlock::QuestionReply {
                        summary,
                        selected,
                        details: event.data.clone(),
                        seq: event.seq,
                    });
                    return Ok(TranscriptUpdate::Appended(self.blocks.len() - 1));
                }
                if let Some(index) = self.pending_user.take() {
                    if let Some(RenderBlock::UserMessage {
                        text: block_text,
                        time,
                        seq,
                        images,
                    }) = self.blocks.get_mut(index)
                    {
                        *block_text = text;
                        *time = event.time;
                        *seq = event.seq;
                        *images = parse_user_images(&event.data);
                        return Ok(TranscriptUpdate::Replaced(index));
                    }
                }
                self.blocks.push(RenderBlock::UserMessage {
                    text,
                    time: event.time,
                    seq: event.seq,
                    images: parse_user_images(&event.data),
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "assistant/message" => {
                let text = Self::extract_assistant_text(&event.data);
                let mut matched_idx = None;

                for (idx, block) in self.blocks.iter_mut().enumerate().rev() {
                    match block {
                        RenderBlock::AssistantMessage {
                            is_live,
                            seq,
                            text: b_text,
                            time: b_time,
                            ..
                        } => {
                            if *is_live || *seq == Some(event.seq) {
                                *is_live = false;
                                *seq = Some(event.seq);
                                *b_time = event.time;
                                if !text.is_empty() {
                                    *b_text = text.clone();
                                }
                                matched_idx = Some(idx);
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(idx) = matched_idx {
                    self.live_attempt = None;
                    Ok(TranscriptUpdate::Updated(idx))
                } else {
                    self.blocks.push(RenderBlock::AssistantMessage {
                        text,
                        reasoning: None,
                        tool_calls: Vec::new(),
                        is_live: false,
                        attempt_id: None,
                        seq: Some(event.seq),
                        time: event.time,
                    });
                    self.live_attempt = None;
                    Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
                }
            }
            "tool/call" => {
                let call_id = event.data["callId"]
                    .as_str()
                    .or_else(|| event.data["call_id"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let name = event.data["name"].as_str().unwrap_or_default().to_string();
                let arguments = event.data["arguments"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                if name == "exit_plan_mode" {
                    let parsed: serde_json::Value =
                        serde_json::from_str(&arguments).unwrap_or_default();
                    let plan = parsed["plan"].as_str().unwrap_or(&arguments).to_string();
                    self.blocks.push(RenderBlock::PlanCard {
                        plan,
                        status: "Plan finalized".to_string(),
                        seq: event.seq,
                    });
                }
                self.pending_tool_calls.insert(call_id, (name, arguments));
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "tool/result" => {
                let call_id = event.data["message"]["callId"]
                    .as_str()
                    .or_else(|| event.data["callId"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let (pending_name, pending_args) = self
                    .pending_tool_calls
                    .remove(&call_id)
                    .unwrap_or_default();
                let name = if let Some(n) = event.data["name"].as_str() {
                    n.to_string()
                } else if !pending_name.is_empty() {
                    pending_name
                } else {
                    String::new()
                };
                let result = if let Some(arr) = event.data["message"]["content"].as_array() {
                    arr.iter()
                        .filter_map(|c| c["text"].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    event.data["result"].as_str().unwrap_or_default().to_string()
                };
                let is_error = event.data["isError"].as_bool().unwrap_or(false);

                self.blocks.push(RenderBlock::ToolResult {
                    call_id,
                    name,
                    arguments: if pending_args.is_empty() {
                        None
                    } else {
                        Some(pending_args)
                    },
                    result,
                    is_error,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "workspace/changes" => {
                let summary = event.data["summary"].clone();
                let diff = event.data.get("diff").cloned();
                self.blocks.push(RenderBlock::DeliverablesBlock {
                    summary,
                    diff,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "plan/mode" => {
                let active = event.data["active"].as_bool().unwrap_or(false);
                self.blocks.push(RenderBlock::PlanModeBadge {
                    active,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "todo/write" => {
                let items: Vec<TodoItem> = serde_json::from_value(event.data["todos"].clone())
                    .unwrap_or_default();
                self.blocks.push(RenderBlock::TodoBlock {
                    items,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "compaction/summary" => {
                let summary = event.data["summary"]
                    .as_str()
                    .or_else(|| event.data["summary"]["text"].as_str())
                    .or_else(|| event.data["text"].as_str())
                    .unwrap_or("Context compacted")
                    .to_string();
                self.blocks.push(RenderBlock::CompactionCard {
                    summary,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            "compaction/start" | "compaction/end" | "llm/retry-started" => {
                Ok(TranscriptUpdate::Ignored)
            }
            "turn/end" => {
                let kind = event.data["reason"]["kind"].as_str().unwrap_or_default();
                match kind {
                    "error" => {
                        let message = event.data["reason"]["error"]["message"]
                            .as_str()
                            .unwrap_or("Turn failed")
                            .to_string();
                        let code = event.data["reason"]["error"]["code"]
                            .as_str()
                            .map(str::to_string);
                        self.blocks.push(RenderBlock::TurnError {
                            message,
                            code,
                            seq: event.seq,
                        });
                        Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
                    }
                    "max-tokens" => {
                        self.blocks.push(RenderBlock::MaxTokensHint { seq: event.seq });
                        Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
                    }
                    _ => Ok(TranscriptUpdate::Ignored),
                }
            }
            "llm/retry" => {
                self.retire_live_blocks();
                self.live_attempt = None;
                let message = event.data["failure"]["message"]
                    .as_str()
                    .unwrap_or("Retrying request")
                    .to_string();
                self.blocks.push(RenderBlock::RetryHint {
                    message,
                    seq: event.seq,
                });
                Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
            }
            _ => {
                if event.is_ignorable() {
                    Ok(TranscriptUpdate::Ignored)
                } else {
                    self.blocks.push(RenderBlock::GenericEvent {
                        event_type: event.event_type.clone(),
                        summary: event.data.to_string(),
                        seq: event.seq,
                    });
                    Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
                }
            }
        }
    }

    fn accumulate_chunk(live: &mut LiveAttemptState, chunk: StreamChunk) {
        match chunk {
            StreamChunk::TextDelta { text, .. } => {
                live.text.push_str(&text);
            }
            StreamChunk::ReasoningDelta { text, .. } => {
                if let Some(r) = live.reasoning.as_mut() {
                    r.push_str(&text);
                } else {
                    live.reasoning = Some(text);
                }
            }
            StreamChunk::ToolCallDelta {
                id,
                name,
                arguments_delta,
                ..
            } => {
                if let Some(existing) = live.tool_calls.iter_mut().find(|tc| tc.id == id) {
                    existing.arguments.push_str(&arguments_delta);
                    if let Some(n) = name {
                        if existing.name.is_empty() {
                            existing.name = n;
                        }
                    }
                } else {
                    live.tool_calls.push(ToolCallItem {
                        id,
                        name: name.unwrap_or_default(),
                        arguments: arguments_delta,
                    });
                }
            }
            _ => {}
        }
    }

    fn extract_user_text(data: &serde_json::Value) -> String {
        if let Some(arr) = data["content"].as_array() {
            let mut parts = Vec::new();
            for block in arr {
                if block["type"] == "text" {
                    if let Some(t) = block["text"].as_str() {
                        parts.push(t);
                    }
                }
            }
            if !parts.is_empty() {
                return parts.join("\n");
            }
        }
        if let Some(text) = data["content"].as_str() {
            return text.to_string();
        }
        if let Some(text) = data["text"].as_str() {
            return text.to_string();
        }
        String::new()
    }

    fn extract_assistant_text(data: &serde_json::Value) -> String {
        if let Some(arr) = data["message"]["content"].as_array() {
            let mut parts = Vec::new();
            for block in arr {
                if block["type"] == "text" {
                    if let Some(t) = block["text"].as_str() {
                        parts.push(t);
                    }
                }
            }
            if !parts.is_empty() {
                return parts.join("\n");
            }
        }
        if let Some(arr) = data["content"].as_array() {
            let mut parts = Vec::new();
            for block in arr {
                if block["type"] == "text" {
                    if let Some(t) = block["text"].as_str() {
                        parts.push(t);
                    }
                }
            }
            if !parts.is_empty() {
                return parts.join("\n");
            }
        }
        if let Some(text) = data["text"].as_str() {
            return text.to_string();
        }
        String::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BashToolInfo {
    pub command: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i32>,
    pub output: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffInfo {
    pub path: String,
    pub diff: String,
    pub is_patch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReadInfo {
    pub path: String,
    pub range: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchToolInfo {
    pub tool: String,
    pub pattern: String,
    pub path: String,
    pub results: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebToolInfo {
    pub tool: String,
    pub query_or_url: String,
    pub status_code: Option<u16>,
    pub content: String,
}

pub fn parse_bash_tool(arguments: &str, result: &str) -> BashToolInfo {
    let parsed: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let command = parsed["command"]
        .as_str()
        .unwrap_or(arguments)
        .to_string();
    let cwd = parsed["cwd"]
        .as_str()
        .or_else(|| parsed["workdir"].as_str())
        .map(|s| s.to_string());

    let mut exit_code = None;
    if let Some(pos) = result.find("[exit code:") {
        let after = &result[pos + 11..];
        if let Some(end) = after.find(']') {
            if let Ok(code) = after[..end].trim().parse::<i32>() {
                exit_code = Some(code);
            }
        }
    }

    BashToolInfo {
        command,
        cwd,
        exit_code,
        output: result.to_string(),
    }
}

pub fn parse_file_diff(arguments: &str, result: &str) -> FileDiffInfo {
    let parsed: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let is_patch = parsed["patch"].is_string();

    let path = parsed["path"]
        .as_str()
        .or_else(|| parsed["file"].as_str())
        .unwrap_or_else(|| if is_patch { "patch" } else { "file" })
        .to_string();

    let diff = parsed["diff"]
        .as_str()
        .or_else(|| parsed["patch"].as_str())
        .unwrap_or(result)
        .to_string();

    FileDiffInfo {
        path,
        diff,
        is_patch,
    }
}

pub fn parse_file_read(arguments: &str, result: &str) -> FileReadInfo {
    let parsed: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let path = parsed["path"]
        .as_str()
        .or_else(|| parsed["file_path"].as_str())
        .unwrap_or_default()
        .to_string();

    let offset = parsed["offset"].as_u64();
    let limit = parsed["limit"].as_u64();

    let range = match (offset, limit) {
        (Some(o), Some(l)) => format!("{}-{}", o, o + l),
        (Some(o), None) => format!("{}-", o),
        (None, Some(l)) => format!("1-{}", l),
        (None, None) => "1-".to_string(),
    };

    FileReadInfo {
        path,
        range,
        content: result.to_string(),
    }
}

pub fn parse_search_tool(tool_name: &str, arguments: &str, result: &str) -> SearchToolInfo {
    let parsed: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let pattern = parsed["pattern"]
        .as_str()
        .or_else(|| parsed["query"].as_str())
        .unwrap_or(arguments)
        .to_string();

    let path = parsed["path"]
        .as_str()
        .or_else(|| parsed["dir"].as_str())
        .unwrap_or(".")
        .to_string();

    SearchToolInfo {
        tool: tool_name.to_string(),
        pattern,
        path,
        results: result.to_string(),
    }
}

pub fn parse_web_tool(tool_name: &str, arguments: &str, result: &str) -> WebToolInfo {
    let parsed: serde_json::Value = serde_json::from_str(arguments).unwrap_or_default();
    let query_or_url = parsed["query"]
        .as_str()
        .or_else(|| parsed["url"].as_str())
        .or_else(|| parsed["queries"].as_str())
        .unwrap_or(arguments)
        .to_string();

    let mut status_code = None;
    if let Some(pos) = result.find("[status:") {
        let after = &result[pos + 8..];
        if let Some(end) = after.find(']') {
            if let Ok(code) = after[..end].trim().parse::<u16>() {
                status_code = Some(code);
            }
        }
    }

    WebToolInfo {
        tool: tool_name.to_string(),
        query_or_url,
        status_code,
        content: result.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TurnStat {
    pub turn: u32,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_read_tokens: u64,
    pub elapsed_ms: u64,
    pub tokens_per_sec: f64,
}

pub fn turn_stats(events: &[SessionEvent]) -> Vec<TurnStat> {
    let mut stats = Vec::new();
    let mut current: Option<(u32, u64, u64, u64, u64, u64)> = None;
    for event in events {
        match event.event_type.as_str() {
            "turn/start" => {
                let turn = event.data["turn"].as_u64().unwrap_or(0) as u32;
                current = Some((turn, event.time, 0, 0, 0, 0));
            }
            "assistant/attempt" => {
                let Some(state) = current.as_mut() else {
                    continue;
                };
                let Some(stream) = event.data["stream"].as_array() else {
                    continue;
                };
                for entry in stream {
                    let chunk = entry.get("chunk").unwrap_or(entry);
                    if chunk["type"].as_str() != Some("usage") {
                        continue;
                    }
                    let usage = &chunk["usage"];
                    let bucket = |key: &str| usage[key].as_u64().unwrap_or(0);
                    state.2 = state.2.saturating_add(bucket("uncachedInputTokens"));
                    state.3 = state.3.saturating_add(bucket("outputTokens"));
                    state.4 = state.4.saturating_add(bucket("reasoningTokens"));
                    state.5 = state.5.saturating_add(bucket("cacheReadTokens"));
                }
            }
            "turn/end" => {
                let Some((turn, start, input, output, reasoning, cache)) = current.take() else {
                    continue;
                };
                let elapsed_ms = event.time.saturating_sub(start);
                let tokens_per_sec = if elapsed_ms > 0 {
                    output as f64 / (elapsed_ms as f64 / 1000.0)
                } else {
                    0.0
                };
                stats.push(TurnStat {
                    turn,
                    input_tokens: input,
                    output_tokens: output,
                    reasoning_tokens: reasoning,
                    cache_read_tokens: cache,
                    elapsed_ms,
                    tokens_per_sec,
                });
            }
            _ => {}
        }
    }
    stats
}
