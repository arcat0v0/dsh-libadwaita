use crate::protocol::*;
use crate::transcript::*;

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
        }
    }

    pub fn handle_event(&mut self, event: SessionEvent) -> Result<TranscriptUpdate, GapDetected> {
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
