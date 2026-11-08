use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftStore {
    #[serde(default)]
    pub drafts: HashMap<String, String>,
}

impl DraftStore {
    pub fn get(&self, session_id: &str) -> Option<&str> {
        self.drafts.get(session_id).map(|value| value.as_str())
    }

    pub fn text_for(&self, session_id: &str) -> String {
        self.drafts.get(session_id).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, session_id: &str, text: &str) {
        if text.trim().is_empty() {
            self.drafts.remove(session_id);
        } else {
            self.drafts
                .insert(session_id.to_string(), text.to_string());
        }
    }

    pub fn remove(&mut self, session_id: &str) {
        self.drafts.remove(session_id);
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(path, raw)
    }
}

pub fn default_drafts_path() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        }
    };
    base.join("dsh-libadwaita").join("drafts.json")
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PendingEchoState {
    #[default]
    Idle,
    Awaiting { text: String, attempt: u64 },
    Confirmed,
    Failed { text: String, error: String },
}

#[derive(Debug, Clone, Default)]
pub struct PendingEchoTracker {
    pending: Option<String>,
    attempt: u64,
    state: PendingEchoState,
}

impl PendingEchoTracker {
    pub fn submit(&mut self, text: &str) {
        self.attempt = self.attempt.wrapping_add(1);
        self.pending = Some(text.to_string());
        self.state = PendingEchoState::Awaiting {
            text: text.to_string(),
            attempt: self.attempt,
        };
    }

    pub fn confirm(&mut self) -> Option<String> {
        let pending = self.pending.take()?;
        self.state = PendingEchoState::Confirmed;
        Some(pending)
    }

    pub fn fail(&mut self, error: &str) -> Option<String> {
        let pending = self.pending.take()?;
        self.state = PendingEchoState::Failed {
            text: pending.clone(),
            error: error.to_string(),
        };
        Some(pending)
    }

    pub fn state(&self) -> &PendingEchoState {
        &self.state
    }

    pub fn pending_text(&self) -> Option<&str> {
        self.pending.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationEligibility {
    Eligible,
    NotEligible,
}

pub const CONTINUATION_ERROR_CODES: [&str; 6] = [
    "RATE_LIMIT",
    "SERVER",
    "TIMEOUT",
    "TRANSPORT",
    "EMPTY_RESPONSE",
    "STREAM_CLOSED",
];

pub fn continuation_eligibility(
    last_error_code: Option<&str>,
    composer_empty: bool,
    session_running: bool,
) -> ContinuationEligibility {
    if session_running || !composer_empty {
        return ContinuationEligibility::NotEligible;
    }
    match last_error_code {
        Some(code) if CONTINUATION_ERROR_CODES.contains(&code) => ContinuationEligibility::Eligible,
        _ => ContinuationEligibility::NotEligible,
    }
}

pub fn mask_secret_patterns(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(['s', 'S', 'g', 'G']) {
        let candidate = &rest[start..];
        let prefix_len = if candidate.starts_with("sk-") || candidate.starts_with("SK-") {
            3
        } else if candidate.starts_with("ghp_")
            || candidate.starts_with("GHP_")
            || candidate.starts_with("github_pat_")
        {
            candidate
                .find('_')
                .map(|idx| idx + 1)
                .unwrap_or(candidate.len())
        } else {
            0
        };

        if prefix_len == 0 {
            masked.push_str(&rest[..start + 1]);
            rest = &rest[start + 1..];
            continue;
        }

        let tail = &candidate[prefix_len..];
        let token_len = tail
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(tail.len());
        let token = &tail[..token_len];
        if token.len() < 8 {
            masked.push_str(&rest[..start + 1]);
            rest = &rest[start + 1..];
            continue;
        }

        let visible = 8usize.saturating_sub(prefix_len).min(token.len());
        masked.push_str(&rest[..start + prefix_len]);
        masked.push_str(&token[..visible]);
        masked.push_str("...");
        masked.push_str(&token[token.len() - 4..]);
        rest = &tail[token_len..];
    }

    masked.push_str(rest);
    masked
}
