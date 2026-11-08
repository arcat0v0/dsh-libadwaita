use crate::ui_prefs::TranscriptView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepGrouping {
    Collapsed,
    History,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationPolicy {
    pub fold_completed_turns: bool,
    pub step_grouping: StepGrouping,
    pub live_process_detail: bool,
    pub settled_reasoning_preview: bool,
}

impl PresentationPolicy {
    pub fn for_mode(mode: TranscriptView) -> Self {
        match mode {
            TranscriptView::Concise => Self {
                fold_completed_turns: true,
                step_grouping: StepGrouping::Collapsed,
                live_process_detail: false,
                settled_reasoning_preview: false,
            },
            TranscriptView::Standard => Self {
                fold_completed_turns: true,
                step_grouping: StepGrouping::Collapsed,
                live_process_detail: true,
                settled_reasoning_preview: true,
            },
            TranscriptView::Detailed => Self {
                fold_completed_turns: true,
                step_grouping: StepGrouping::History,
                live_process_detail: true,
                settled_reasoning_preview: true,
            },
            TranscriptView::Expanded => Self {
                fold_completed_turns: false,
                step_grouping: StepGrouping::None,
                live_process_detail: false,
                settled_reasoning_preview: true,
            },
        }
    }

    pub fn groups_intermediate(self) -> bool {
        self.step_grouping != StepGrouping::None
    }
}

pub fn reasoning_summary(reasoning: &str, running: bool) -> Option<String> {
    let line = if running {
        latest_completed_paragraph_first_line(reasoning)
    } else {
        reasoning.lines().next().unwrap_or("")
    };
    let cleaned = line.replace("**", "");
    let summary = cleaned.trim();
    if summary.is_empty() {
        return None;
    }
    Some(truncate(summary, 80))
}

pub fn reasoning_header(
    expanded: bool,
    preview_enabled: bool,
    running: bool,
    reasoning: &str,
) -> String {
    if !expanded && (running || preview_enabled) {
        if let Some(summary) = reasoning_summary(reasoning, running) {
            return format!("Reasoning Process · {summary}");
        }
    }
    "Reasoning Process".to_string()
}

fn latest_completed_paragraph_first_line(text: &str) -> &str {
    let mut summary = "";
    let mut first: Option<&str> = None;
    let mut ended = false;
    for line in text.split_inclusive('\n') {
        let has_terminator = line.ends_with('\n');
        let content = line.trim_end_matches(['\r', '\n']);
        if content.trim().is_empty() {
            if let (Some(first_line), true) = (first, ended) {
                summary = first_line;
            }
            first = None;
            ended = false;
            continue;
        }
        match first {
            None => first = Some(content),
            Some(first_line) => summary = first_line,
        }
        ended = has_terminator;
    }
    summary
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(limit).collect();
    truncated.push('…');
    truncated
}
