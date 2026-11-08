use dsh_libadwaita::interaction::*;
use dsh_libadwaita::protocol::*;

fn question(options: Vec<&str>, multi: bool) -> AskUserQuestionItem {
    AskUserQuestionItem {
        id: "q1".to_string(),
        question: "Pick".to_string(),
        detail: None,
        intent: None,
        options: Some(
            options
                .into_iter()
                .map(|label| UserQuestionOption {
                    label: label.to_string(),
                    description: None,
                })
                .collect(),
        ),
        multi_select: Some(multi),
    }
}

#[test]
fn test_recommended_suffix_is_stripped_for_display_only() {
    assert_eq!(
        parse_recommended_label("Use the fast path (recommended)"),
        ("Use the fast path".to_string(), true)
    );
    assert_eq!(
        parse_recommended_label("快速方案（推荐）"),
        ("快速方案".to_string(), true)
    );
    assert_eq!(
        parse_recommended_label("快速方案 (推荐)"),
        ("快速方案".to_string(), true)
    );
    assert_eq!(
        parse_recommended_label("Thorough (Recommended)"),
        ("Thorough".to_string(), true)
    );
    assert_eq!(
        parse_recommended_label("Plain option"),
        ("Plain option".to_string(), false)
    );
}

#[test]
fn test_recommended_labels_preselect_first_for_single_select() {
    let item = question(
        vec!["A", "B (recommended)", "C (recommended)"],
        false,
    );
    assert_eq!(recommended_labels(&item), vec!["B (recommended)".to_string()]);
}

#[test]
fn test_recommended_labels_preselect_all_for_multi_select() {
    let item = question(vec!["A", "B (recommended)", "C（推荐）"], true);
    assert_eq!(
        recommended_labels(&item),
        vec!["B (recommended)".to_string(), "C（推荐）".to_string()]
    );
}

#[test]
fn test_recommended_labels_empty_without_marker() {
    let item = question(vec!["A", "B"], false);
    assert!(recommended_labels(&item).is_empty());
}

#[test]
fn test_approval_command_from_correlated_tool_call() {
    let events = vec![
        SessionEvent {
            event_type: "tool/call".to_string(),
            seq: 1,
            time: 10,
            data: serde_json::json!({
                "turn": 1,
                "step": 1,
                "callId": "call-1",
                "name": "bash",
                "arguments": "{\"command\":\"rm -rf build\"}"
            }),
            ignorable: None,
            surface_op: None,
            source_event_seqs: None,
        },
        SessionEvent {
            event_type: "tool/call".to_string(),
            seq: 2,
            time: 11,
            data: serde_json::json!({
                "turn": 1,
                "step": 1,
                "callId": "call-2",
                "name": "read_file",
                "arguments": "{\"path\":\"x\"}"
            }),
            ignorable: None,
            surface_op: None,
            source_event_seqs: None,
        },
    ];
    assert_eq!(
        approval_command(&events, Some("call-1")),
        Some("rm -rf build".to_string())
    );
    assert_eq!(approval_command(&events, Some("call-2")), None);
    assert_eq!(approval_command(&events, Some("missing")), None);
    assert_eq!(approval_command(&events, None), None);
}

#[test]
fn test_approval_command_ignores_malformed_arguments() {
    let events = vec![SessionEvent {
        event_type: "tool/call".to_string(),
        seq: 1,
        time: 10,
        data: serde_json::json!({
            "callId": "call-9",
            "arguments": "not json"
        }),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    }];
    assert_eq!(approval_command(&events, Some("call-9")), None);
}

#[test]
fn test_arbiter_allows_enter_and_escape_inside_card() {
    let plain = FocusContext {
        in_card: true,
        is_text_input: false,
        is_button: false,
    };
    assert_eq!(
        arbitrate_key(ArbiterKey::Enter, false, plain),
        ArbitratedDecision::Allow
    );
    assert_eq!(
        arbitrate_key(ArbiterKey::Escape, false, plain),
        ArbitratedDecision::Reject
    );
}

#[test]
fn test_arbiter_passes_when_focus_is_outside_card() {
    let outside = FocusContext {
        in_card: false,
        is_text_input: false,
        is_button: false,
    };
    assert_eq!(
        arbitrate_key(ArbiterKey::Enter, false, outside),
        ArbitratedDecision::Pass
    );
    assert_eq!(
        arbitrate_key(ArbiterKey::Escape, false, outside),
        ArbitratedDecision::Pass
    );
}

#[test]
fn test_arbiter_passes_for_text_inputs_and_buttons() {
    let text = FocusContext {
        in_card: true,
        is_text_input: true,
        is_button: false,
    };
    let button = FocusContext {
        in_card: true,
        is_text_input: false,
        is_button: true,
    };
    assert_eq!(
        arbitrate_key(ArbiterKey::Enter, false, text),
        ArbitratedDecision::Pass
    );
    assert_eq!(
        arbitrate_key(ArbiterKey::Enter, false, button),
        ArbitratedDecision::Pass
    );
    assert_eq!(
        arbitrate_key(ArbiterKey::Escape, false, button),
        ArbitratedDecision::Reject
    );
}

#[test]
fn test_arbiter_passes_with_modifiers_and_other_keys() {
    let plain = FocusContext {
        in_card: true,
        is_text_input: false,
        is_button: false,
    };
    assert_eq!(
        arbitrate_key(ArbiterKey::Enter, true, plain),
        ArbitratedDecision::Pass
    );
    assert_eq!(
        arbitrate_key(ArbiterKey::Other, false, plain),
        ArbitratedDecision::Pass
    );
}

#[test]
fn test_question_draft_snapshot_collects_answers() {
    let req = SdkUserQuestionRequest {
        request_id: "r1".to_string(),
        session_id: "s1".to_string(),
        questions: vec![question(vec!["A", "B"], false), question(vec!["C"], true)],
    };
    let mut draft = QuestionDraft::new(&req);
    draft.selected[0] = vec!["A".to_string()];
    draft.custom[1] = "  free text  ".to_string();
    let answers = draft.answers(&req);
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].selected, vec!["A".to_string()]);
    assert_eq!(answers[1].selected, vec!["free text".to_string()]);
}

#[test]
fn test_question_draft_skip_keeps_empty_selection() {
    let req = SdkUserQuestionRequest {
        request_id: "r2".to_string(),
        session_id: "s1".to_string(),
        questions: vec![question(vec!["A"], false)],
    };
    let draft = QuestionDraft::new(&req);
    let answers = draft.answers(&req);
    assert_eq!(answers[0].selected.len(), 0);
}

#[test]
fn test_question_draft_answers_prefer_custom_text_on_empty_selection() {
    let req = SdkUserQuestionRequest {
        request_id: "r3".to_string(),
        session_id: "s1".to_string(),
        questions: vec![question(vec!["A"], false)],
    };
    let mut draft = QuestionDraft::new(&req);
    draft.custom[0] = "typed".to_string();
    assert_eq!(draft.answers(&req)[0].selected, vec!["typed".to_string()]);
}
