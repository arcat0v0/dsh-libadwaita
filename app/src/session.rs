use crate::protocol::*;
use crate::transcript::*;
use std::collections::VecDeque;

pub const MAX_RAW_EVENTS: usize = 2000;

pub fn terminal_error_code(events: &[SessionEvent]) -> Option<String> {
    for event in events.iter().rev() {
        if event.event_type == "user/message" {
            return None;
        }
        if event.event_type.contains("error") {
            if let Some(code) = event.data["code"].as_str() {
                return Some(code.to_string());
            }
            if let Some(code) = event.data["error"]["code"].as_str() {
                return Some(code.to_string());
            }
        }
        if event.event_type == "turn/end" {
            if let Some(code) = event.data["reason"]["code"].as_str() {
                return Some(code.to_string());
            }
            if let Some(code) = event.data["outcome"]["code"].as_str() {
                return Some(code.to_string());
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Idle,
    Running,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl From<&str> for SessionStatus {
    fn from(s: &str) -> Self {
        match s {
            "running" => Self::Running,
            _ => Self::Idle,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionModel {
    pub id: String,
    pub cwd: String,
    pub title: String,
    pub status: SessionStatus,
    pub current_model: Option<String>,
    pub current_provider: Option<String>,
    pub transcript: ConversationTranscript,
    pub plan_mode: bool,
    pub todos: Vec<TodoItem>,
    pub last_error_code: Option<String>,
    pub raw_events: VecDeque<SessionEvent>,
}

impl SessionModel {
    pub fn new(id: impl Into<String>, cwd: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            cwd: cwd.into(),
            title: "New Session".to_string(),
            status: SessionStatus::Idle,
            current_model: None,
            current_provider: None,
            transcript: ConversationTranscript::new(),
            plan_mode: false,
            todos: Vec::new(),
            last_error_code: None,
            raw_events: VecDeque::new(),
        }
    }

    pub fn from_follow(follow: SessionFollowResult) -> Self {
        let mut transcript = ConversationTranscript::new();
        transcript.set_last_seq(follow.latest_seq);
        transcript.apply_baseline(follow.assistant_baseline);

        let title = follow
            .title
            .map(|t| t.title)
            .unwrap_or_else(|| "New Session".to_string());

        Self {
            id: follow.header.id,
            cwd: follow.header.cwd,
            title,
            status: SessionStatus::from(follow.status.as_str()),
            current_model: None,
            current_provider: None,
            transcript,
            plan_mode: follow.projections.plan_mode,
            todos: follow.projections.todos.unwrap_or_default(),
            last_error_code: None,
            raw_events: VecDeque::new(),
        }
    }
    pub fn raw_events(&self) -> &VecDeque<SessionEvent> {
        &self.raw_events
    }

    pub fn record_older_events(&mut self, events: Vec<SessionEvent>) {
        let front_seq = self.raw_events.front().map(|event| event.seq);
        let older: Vec<SessionEvent> = events
            .into_iter()
            .filter(|event| front_seq.map(|seq| event.seq < seq).unwrap_or(true))
            .collect();
        for event in older.into_iter().rev() {
            self.raw_events.push_front(event);
        }
        while self.raw_events.len() > MAX_RAW_EVENTS {
            self.raw_events.pop_back();
        }
    }

    pub fn handle_event(&mut self, event: SessionEvent) -> Result<TranscriptUpdate, GapDetected> {
        if self
            .raw_events
            .back()
            .map(|existing| existing.seq != event.seq)
            .unwrap_or(true)
        {
            self.raw_events.push_back(event.clone());
            while self.raw_events.len() > MAX_RAW_EVENTS {
                self.raw_events.pop_front();
            }
        }
        if event.event_type == "user/message" {
            self.last_error_code = None;
        }
        if event.event_type.contains("error") {
            if let Some(code) = event.data["code"].as_str() {
                self.last_error_code = Some(code.to_string());
            } else if let Some(code) = event.data["error"]["code"].as_str() {
                self.last_error_code = Some(code.to_string());
            }
        }
        if event.event_type == "plan/mode" {
            if let Ok(data) = event.parse_data::<PlanModeData>() {
                self.plan_mode = data.active;
            } else if let Some(active) = event.data["active"].as_bool() {
                self.plan_mode = active;
            }
        } else if event.event_type == "todo/write" {
            if let Ok(data) = event.parse_data::<TodoWriteData>() {
                self.todos = data.todos;
            } else if let Ok(items) =
                serde_json::from_value::<Vec<TodoItem>>(event.data["todos"].clone())
            {
                self.todos = items;
            }
        } else if event.event_type == "session/title" {
            if let Some(t) = event.data["title"].as_str() {
                self.title = t.to_string();
            }
        }
        self.transcript.apply_event(event)
    }

    pub fn handle_stream_frame(&mut self, frame: AssistantStreamFrame) {
        self.transcript.apply_stream_frame(frame);
    }

    pub fn handle_status(&mut self, status: &str) {
        self.status = SessionStatus::from(status);
    }

    pub fn handle_title(&mut self, title: &str) {
        self.title = title.to_string();
    }
}
