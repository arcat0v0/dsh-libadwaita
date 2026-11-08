use dsh_libadwaita::transcript::{
    MessageAttachment, RenderBlock, ToolCallInfo,
};
use dsh_libadwaita::ui::turn_nav::{active_turn, fork_target_index, turn_markers, TurnMarker};

fn user(text: &str) -> RenderBlock {
    RenderBlock::UserMessage {
        text: text.to_string(),
        time: 0,
        seq: 0,
        images: Vec::new(),
    }
}

fn assistant(text: &str, seq: Option<i64>, is_live: bool) -> RenderBlock {
    RenderBlock::AssistantMessage {
        text: text.to_string(),
        reasoning: None,
        tool_calls: Vec::new(),
        attachments: Vec::<MessageAttachment>::new(),
        is_live,
        attempt_id: None,
        seq,
        message_id: seq.map(|value| format!("msg-{}", value)),
        time: 0,
    }
}

fn tool_result(seq: i64) -> RenderBlock {
    RenderBlock::ToolResult {
        call_id: format!("call-{seq}"),
        info: ToolCallInfo {
            name: "read_file".to_string(),
            arguments: None,
            result: String::new(),
            is_error: false,
            error: None,
            meta: None,
            sub_calls: Vec::new(),
        },
        seq,
    }
}

#[test]
fn test_turn_markers_follow_user_messages() {
    let blocks = vec![
        user("First question\nmore detail"),
        assistant("answer", Some(1), false),
        user("Second question"),
        assistant("answer", Some(3), false),
    ];
    let markers = turn_markers(&blocks);
    assert_eq!(
        markers,
        vec![
            TurnMarker {
                turn: 1,
                block_index: 0,
                preview: "First question".to_string(),
            },
            TurnMarker {
                turn: 2,
                block_index: 2,
                preview: "Second question".to_string(),
            },
        ]
    );
}

#[test]
fn test_turn_markers_empty_without_user_messages() {
    assert!(turn_markers(&[assistant("hello", Some(0), false)]).is_empty());
}

#[test]
fn test_turn_preview_is_bounded() {
    let long = "x".repeat(200);
    let markers = turn_markers(&[user(&long)]);
    assert_eq!(markers[0].preview.chars().count(), 81);
    assert!(markers[0].preview.ends_with('…'));
}

#[test]
fn test_active_turn_resolves_viewport_top() {
    let offsets = [(1, 0.0), (2, 100.0), (3, 250.0)];
    assert_eq!(active_turn(&offsets, 0.0), Some(1));
    assert_eq!(active_turn(&offsets, 150.0), Some(2));
    assert_eq!(active_turn(&offsets, 1000.0), Some(3));
    assert_eq!(active_turn(&[], 0.0), None);
}

#[test]
fn test_fork_target_is_last_finalized_assistant_message() {
    let blocks = vec![
        user("question"),
        assistant("answer", Some(1), false),
    ];
    assert_eq!(fork_target_index(&blocks), Some(1));

    let with_tool = vec![
        user("question"),
        assistant("working", Some(2), false),
        tool_result(3),
    ];
    assert_eq!(fork_target_index(&with_tool), None);

    let live = vec![user("question"), assistant("partial", None, true)];
    assert_eq!(fork_target_index(&live), None);

    let later_turn = vec![
        user("one"),
        assistant("answer", Some(1), false),
        user("two"),
    ];
    assert_eq!(fork_target_index(&later_turn), None);

    let no_seq = vec![user("question"), assistant("answer", None, false)];
    assert_eq!(fork_target_index(&no_seq), None);

    assert_eq!(fork_target_index(&[]), None);
}
