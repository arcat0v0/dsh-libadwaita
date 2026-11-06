use dsh_libadwaita::protocol::*;
use dsh_libadwaita::session::*;
use dsh_libadwaita::transcript::*;

#[test]
fn test_normal_streaming_and_commit_flow() {
    let mut transcript = ConversationTranscript::new();
    assert_eq!(transcript.last_seq(), -1);
    assert!(transcript.blocks().is_empty());

    let user_event = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 0,
        time: 1000,
        data: serde_json::json!({
            "role": "user",
            "content": [{"type": "text", "text": "Hello assistant"}]
        }),
        ignorable: None,
        surface_op: Some(serde_json::json!("append")),
        source_event_seqs: None,
    };

    let res = transcript.apply_event(user_event).unwrap();
    assert_eq!(res, TranscriptUpdate::Appended(0));
    assert_eq!(transcript.last_seq(), 0);
    assert_eq!(transcript.blocks().len(), 1);

    match &transcript.blocks()[0] {
        RenderBlock::UserMessage { text, seq, .. } => {
            assert_eq!(text, "Hello assistant");
            assert_eq!(*seq, 0);
        }
        _ => panic!("expected UserMessage"),
    }

    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-1".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });

    assert_eq!(transcript.blocks().len(), 2);
    match &transcript.blocks()[1] {
        RenderBlock::AssistantMessage { is_live, text, .. } => {
            assert!(is_live);
            assert_eq!(text, "");
        }
        _ => panic!("expected live AssistantMessage"),
    }

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 1,
        index: 0,
        time: 1010,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "Hello ".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 1,
        index: 1,
        time: 1020,
        chunk: StreamChunk::TextDelta {
            index: 1,
            text: "world!".to_string(),
        },
    });

    assert_eq!(transcript.blocks().len(), 2);
    match &transcript.blocks()[1] {
        RenderBlock::AssistantMessage { is_live, text, .. } => {
            assert!(is_live);
            assert_eq!(text, "Hello world!");
        }
        _ => panic!("expected live AssistantMessage"),
    }

    transcript.apply_stream_frame(AssistantStreamFrame::End {
        attempt_id: "att-1".to_string(),
        revision: 1,
        index: 2,
        outcome: StreamOutcome::Committed {
            event_type: "assistant/message".to_string(),
            seq: 1,
        },
    });

    let assistant_event = SessionEvent {
        event_type: "assistant/message".to_string(),
        seq: 1,
        time: 1030,
        data: serde_json::json!({
            "turn": 1,
            "step": 0,
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello world!"}]
            }
        }),
        ignorable: None,
        surface_op: Some(serde_json::json!("append")),
        source_event_seqs: None,
    };

    let res2 = transcript.apply_event(assistant_event).unwrap();
    assert_eq!(res2, TranscriptUpdate::Updated(1));
    assert_eq!(transcript.last_seq(), 1);
    assert_eq!(transcript.blocks().len(), 2);

    match &transcript.blocks()[1] {
        RenderBlock::AssistantMessage {
            is_live,
            text,
            seq,
            ..
        } => {
            assert!(!is_live);
            assert_eq!(text, "Hello world!");
            assert_eq!(*seq, Some(1));
        }
        _ => panic!("expected finalized AssistantMessage"),
    }
}

#[test]
fn test_abandoned_stream_flow() {
    let mut transcript = ConversationTranscript::new();

    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-fail".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-fail".to_string(),
        revision: 1,
        index: 0,
        time: 1000,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "Partial text that will be abandoned".to_string(),
        },
    });

    assert_eq!(transcript.blocks().len(), 1);

    transcript.apply_stream_frame(AssistantStreamFrame::End {
        attempt_id: "att-fail".to_string(),
        revision: 1,
        index: 1,
        outcome: StreamOutcome::Abandoned,
    });

    assert!(transcript.blocks().is_empty());
}

#[test]
fn test_reasoning_deltas_accumulation() {
    let mut transcript = ConversationTranscript::new();

    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-reason".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-reason".to_string(),
        revision: 1,
        index: 0,
        time: 1000,
        chunk: StreamChunk::ReasoningDelta {
            index: 0,
            text: "Thinking ".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-reason".to_string(),
        revision: 1,
        index: 1,
        time: 1010,
        chunk: StreamChunk::ReasoningDelta {
            index: 1,
            text: "step by step".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-reason".to_string(),
        revision: 1,
        index: 2,
        time: 1020,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "Final answer".to_string(),
        },
    });

    match &transcript.blocks()[0] {
        RenderBlock::AssistantMessage {
            reasoning, text, ..
        } => {
            assert_eq!(reasoning.as_deref(), Some("Thinking step by step"));
            assert_eq!(text, "Final answer");
        }
        _ => panic!("expected AssistantMessage"),
    }
}

#[test]
fn test_gap_detection() {
    let mut transcript = ConversationTranscript::new();

    let ev0 = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 0,
        time: 1000,
        data: serde_json::json!({"content": "Hello"}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    transcript.apply_event(ev0).unwrap();
    assert_eq!(transcript.last_seq(), 0);

    let ev3 = SessionEvent {
        event_type: "tool/call".to_string(),
        seq: 3,
        time: 1050,
        data: serde_json::json!({
            "callId": "call-1",
            "name": "bash",
            "arguments": "git status"
        }),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };

    let err = transcript.apply_event(ev3).unwrap_err();
    assert_eq!(err, GapDetected { expected: 1, actual: 3 });
}

#[test]
fn test_duplicate_and_replay_ignored() {
    let mut transcript = ConversationTranscript::new();

    let ev0 = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 0,
        time: 1000,
        data: serde_json::json!({"content": "Hello"}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    transcript.apply_event(ev0.clone()).unwrap();
    assert_eq!(transcript.blocks().len(), 1);

    let res = transcript.apply_event(ev0).unwrap();
    assert_eq!(res, TranscriptUpdate::Ignored);
    assert_eq!(transcript.blocks().len(), 1);
}

#[test]
fn test_baseline_initialization() {
    let mut transcript = ConversationTranscript::new();

    let baseline = SessionAssistantBaseline {
        attempt_id: "att-base".to_string(),
        turn: 2,
        step: 0,
        revision: 3,
        chunks: vec![
            StreamChunk::ReasoningDelta {
                index: 0,
                text: "Reasoning from baseline".to_string(),
            },
            StreamChunk::TextDelta {
                index: 0,
                text: "Response from baseline".to_string(),
            },
        ],
    };

    transcript.apply_baseline(Some(baseline));
    assert_eq!(transcript.blocks().len(), 1);
    match &transcript.blocks()[0] {
        RenderBlock::AssistantMessage {
            is_live,
            reasoning,
            text,
            attempt_id,
            ..
        } => {
            assert!(is_live);
            assert_eq!(attempt_id.as_deref(), Some("att-base"));
            assert_eq!(reasoning.as_deref(), Some("Reasoning from baseline"));
            assert_eq!(text, "Response from baseline");
        }
        _ => panic!("expected AssistantMessage from baseline"),
    }
}

#[test]
fn test_session_model_updates() {
    let mut session = SessionModel::new("sess-1", "/home/project");
    assert_eq!(session.status, SessionStatus::Idle);
    assert_eq!(session.title, "New Session");

    session.handle_status("running");
    assert_eq!(session.status, SessionStatus::Running);

    session.handle_title("Implementing Feature");
    assert_eq!(session.title, "Implementing Feature");

    let ev = SessionEvent {
        event_type: "plan/mode".to_string(),
        seq: 0,
        time: 1000,
        data: serde_json::json!({"active": true}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    session.handle_event(ev).unwrap();
    assert!(session.plan_mode);
}
