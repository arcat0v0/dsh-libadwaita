use crate::protocol::{build_question_answer, AskUserQuestionItem, SessionEvent, SdkUserQuestionRequest, UserQuestionAnswer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbiterKey {
    Enter,
    Escape,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusContext {
    pub in_card: bool,
    pub is_text_input: bool,
    pub is_button: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitratedDecision {
    Allow,
    Reject,
    Pass,
}

pub fn arbitrate_key(
    key: ArbiterKey,
    has_modifiers: bool,
    focus: FocusContext,
) -> ArbitratedDecision {
    if !focus.in_card || focus.is_text_input {
        return ArbitratedDecision::Pass;
    }
    let key = match key {
        ArbiterKey::Enter | ArbiterKey::Escape => key,
        ArbiterKey::Other => return ArbitratedDecision::Pass,
    };
    if has_modifiers {
        return ArbitratedDecision::Pass;
    }
    match key {
        ArbiterKey::Enter if focus.is_button => ArbitratedDecision::Pass,
        ArbiterKey::Enter => ArbitratedDecision::Allow,
        _ => ArbitratedDecision::Reject,
    }
}

pub fn parse_recommended_label(label: &str) -> (String, bool) {
    const SUFFIXES: [&str; 4] = ["(recommended)", "（推荐）", "(推荐)", "（recommended）"];
    let trimmed = label.trim_end();
    let lowered = trimmed.to_lowercase();
    for suffix in SUFFIXES {
        if lowered.ends_with(suffix) {
            let cut = trimmed.len() - suffix.len();
            return (trimmed[..cut].trim_end().to_string(), true);
        }
    }
    (label.to_string(), false)
}

pub fn recommended_labels(question: &AskUserQuestionItem) -> Vec<String> {
    let Some(options) = question.options.as_ref() else {
        return Vec::new();
    };
    let mut recommended: Vec<String> = options
        .iter()
        .filter(|option| parse_recommended_label(&option.label).1)
        .map(|option| option.label.clone())
        .collect();
    if question.multi_select != Some(true) {
        recommended.truncate(1);
    }
    recommended
}

pub fn approval_command(events: &[SessionEvent], call_id: Option<&str>) -> Option<String> {
    let call_id = call_id?;
    for event in events {
        if event.event_type != "tool/call" {
            continue;
        }
        if event.data.get("callId").and_then(|value| value.as_str()) != Some(call_id) {
            continue;
        }
        let arguments = event.data.get("arguments").and_then(|value| value.as_str())?;
        let parsed: serde_json::Value = serde_json::from_str(arguments).ok()?;
        if let Some(command) = parsed.get("command").and_then(|value| value.as_str()) {
            return Some(command.to_string());
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionDraft {
    pub selected: Vec<Vec<String>>,
    pub custom: Vec<String>,
    pub page: usize,
}

impl QuestionDraft {
    pub fn new(request: &SdkUserQuestionRequest) -> Self {
        let mut selected = Vec::with_capacity(request.questions.len());
        let mut custom = Vec::with_capacity(request.questions.len());
        for question in &request.questions {
            selected.push(recommended_labels(question));
            custom.push(String::new());
        }
        Self {
            selected,
            custom,
            page: 0,
        }
    }

    pub fn answers(&self, request: &SdkUserQuestionRequest) -> Vec<UserQuestionAnswer> {
        request
            .questions
            .iter()
            .enumerate()
            .map(|(index, question)| {
                build_question_answer(
                    &question.id,
                    self.selected.get(index).cloned().unwrap_or_default(),
                    self.custom.get(index).map(String::as_str).unwrap_or(""),
                )
            })
            .collect()
    }
}
