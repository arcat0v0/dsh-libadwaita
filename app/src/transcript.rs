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
    },
    AssistantMessage {
        text: String,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCallItem>,
        is_live: bool,
        attempt_id: Option<String>,
        seq: Option<i64>,
    },
    ToolResult {
        call_id: String,
        name: String,
        result: String,
        is_error: bool,
        seq: i64,
    },
    PlanModeBadge {
        active: bool,
        seq: i64,
    },
    TodoBlock {
        items: Vec<TodoItem>,
        seq: i64,
    },
    GenericEvent {
        event_type: String,
        summary: String,
        seq: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveAttemptState {
    pub attempt_id: String,
    pub turn: u32,
    pub step: u32,
    pub revision: u64,
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
    live_attempt: Option<LiveAttemptState>,
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
            live_attempt: None,
        }
    }

    pub fn blocks(&self) -> &[RenderBlock] {
        &self.blocks
    }

    pub fn last_seq(&self) -> i64 {
        self.last_seq
    }

    pub fn set_last_seq(&mut self, seq: i64) {
        self.last_seq = seq;
    }

    pub fn clear(&mut self) {
        self.blocks.clear();
        self.last_seq = -1;
        self.live_attempt = None;
    }

    pub fn apply_baseline(&mut self, baseline: Option<SessionAssistantBaseline>) {
        if let Some(b) = baseline {
            let mut state = LiveAttemptState {
                attempt_id: b.attempt_id.clone(),
                turn: b.turn,
                step: b.step,
                revision: b.revision,
                text: String::new(),
                reasoning: None,
                tool_calls: Vec::new(),
            };

            for chunk in b.chunks {
                Self::accumulate_chunk(&mut state, chunk);
            }

            let block = RenderBlock::AssistantMessage {
                text: state.text.clone(),
                reasoning: state.reasoning.clone(),
                tool_calls: state.tool_calls.clone(),
                is_live: true,
                attempt_id: Some(b.attempt_id),
                seq: None,
            };

            self.live_attempt = Some(state);
            self.blocks.push(block);
        }
    }

    pub fn apply_stream_frame(&mut self, frame: AssistantStreamFrame) {
        match frame {
            AssistantStreamFrame::Start {
                attempt_id,
                revision,
                turn,
                step,
            } => {
                let state = LiveAttemptState {
                    attempt_id: attempt_id.clone(),
                    turn,
                    step,
                    revision,
                    text: String::new(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                };

                let block = RenderBlock::AssistantMessage {
                    text: String::new(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    is_live: true,
                    attempt_id: Some(attempt_id),
                    seq: None,
                };

                self.live_attempt = Some(state);
                self.blocks.push(block);
            }
            AssistantStreamFrame::Chunk {
                attempt_id, chunk, ..
            } => {
                if self.live_attempt.is_none() {
                    self.live_attempt = Some(LiveAttemptState {
                        attempt_id: attempt_id.clone(),
                        turn: 0,
                        step: 0,
                        revision: 0,
                        text: String::new(),
                        reasoning: None,
                        tool_calls: Vec::new(),
                    });
                }

                if let Some(live) = self.live_attempt.as_mut() {
                    Self::accumulate_chunk(live, chunk);

                    if let Some(last_block) = self.blocks.iter_mut().rev().find(|b| match b {
                        RenderBlock::AssistantMessage { is_live, .. } => *is_live,
                        _ => false,
                    }) {
                        if let RenderBlock::AssistantMessage {
                            text,
                            reasoning,
                            tool_calls,
                            ..
                        } = last_block
                        {
                            *text = live.text.clone();
                            *reasoning = live.reasoning.clone();
                            *tool_calls = live.tool_calls.clone();
                        }
                    } else {
                        self.blocks.push(RenderBlock::AssistantMessage {
                            text: live.text.clone(),
                            reasoning: live.reasoning.clone(),
                            tool_calls: live.tool_calls.clone(),
                            is_live: true,
                            attempt_id: Some(attempt_id),
                            seq: None,
                        });
                    }
                }
            }
            AssistantStreamFrame::End { outcome, .. } => match outcome {
                StreamOutcome::Abandoned => {
                    self.blocks.retain(|b| match b {
                        RenderBlock::AssistantMessage { is_live, .. } => !*is_live,
                        _ => true,
                    });
                    self.live_attempt = None;
                }
                StreamOutcome::Committed { seq, .. } => {
                    if let Some(last_live) = self.blocks.iter_mut().rev().find(|b| match b {
                        RenderBlock::AssistantMessage { is_live, .. } => *is_live,
                        _ => false,
                    }) {
                        if let RenderBlock::AssistantMessage {
                            is_live,
                            seq: block_seq,
                            ..
                        } = last_live
                        {
                            *is_live = false;
                            *block_seq = Some(seq);
                        }
                    }
                    self.live_attempt = None;
                }
            },
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
                self.blocks.push(RenderBlock::UserMessage {
                    text,
                    time: event.time,
                    seq: event.seq,
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
                            ..
                        } => {
                            if *is_live || *seq == Some(event.seq) {
                                *is_live = false;
                                *seq = Some(event.seq);
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
                    });
                    self.live_attempt = None;
                    Ok(TranscriptUpdate::Appended(self.blocks.len() - 1))
                }
            }
            "tool/result" => {
                let call_id = event.data["message"]["callId"]
                    .as_str()
                    .or_else(|| event.data["callId"].as_str())
                    .unwrap_or_default()
                    .to_string();
                let name = event.data["name"].as_str().unwrap_or_default().to_string();
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
                    result,
                    is_error,
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
