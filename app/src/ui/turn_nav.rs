use crate::transcript::RenderBlock;

const PREVIEW_LIMIT: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnMarker {
    pub turn: u32,
    pub block_index: usize,
    pub preview: String,
}

pub fn turn_markers(blocks: &[RenderBlock]) -> Vec<TurnMarker> {
    let mut markers = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        if let RenderBlock::UserMessage { text, .. } = block {
            markers.push(TurnMarker {
                turn: markers.len() as u32 + 1,
                block_index: index,
                preview: preview_line(text),
            });
        }
    }
    markers
}

pub fn active_turn(offsets: &[(u32, f64)], top: f64) -> Option<u32> {
    let mut active = offsets.first().map(|(turn, _)| *turn);
    for (turn, offset) in offsets {
        if *offset <= top + 0.5 {
            active = Some(*turn);
        }
    }
    active
}

pub fn fork_target_index(blocks: &[RenderBlock]) -> Option<usize> {
    match blocks.last()? {
        RenderBlock::AssistantMessage {
            is_live: false,
            seq: Some(_),
            ..
        } => Some(blocks.len() - 1),
        _ => None,
    }
}

fn preview_line(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    if line.chars().count() <= PREVIEW_LIMIT {
        line.to_string()
    } else {
        let mut preview: String = line.chars().take(PREVIEW_LIMIT).collect();
        preview.push('…');
        preview
    }
}
