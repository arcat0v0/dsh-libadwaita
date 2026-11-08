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
        revision: 2,
        index: 0,
        time: 1010,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "Hello ".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 3,
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
        revision: 4,
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
        revision: 2,
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
        revision: 3,
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
        revision: 2,
        index: 0,
        time: 1000,
        chunk: StreamChunk::ReasoningDelta {
            index: 0,
            text: "Thinking ".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-reason".to_string(),
        revision: 3,
        index: 1,
        time: 1010,
        chunk: StreamChunk::ReasoningDelta {
            index: 1,
            text: "step by step".to_string(),
        },
    });

    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-reason".to_string(),
        revision: 4,
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

#[test]
fn test_pending_user_echo_replacement() {
    let mut transcript = ConversationTranscript::new();
    transcript.push_pending_user("hello");
    assert_eq!(transcript.blocks().len(), 1);
    assert_eq!(transcript.first_seq(), None);

    let event = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 4,
        time: 100,
        data: serde_json::json!({"content": [{"type": "text", "text": "hello"}]}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    let update = transcript.apply_event(event).unwrap();
    assert_eq!(update, TranscriptUpdate::Replaced(0));
    assert_eq!(transcript.blocks().len(), 1);
    assert_eq!(transcript.first_seq(), Some(4));
}

#[test]
fn test_prepend_events_preserves_existing_blocks() {
    let mut transcript = ConversationTranscript::new();
    transcript.set_last_seq(9);
    let first = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 10,
        time: 100,
        data: serde_json::json!({"content": [{"type": "text", "text": "new"}]}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    transcript.apply_event(first).unwrap();

    let older = vec![SessionEvent {
        event_type: "user/message".to_string(),
        seq: 3,
        time: 50,
        data: serde_json::json!({"content": [{"type": "text", "text": "old"}]}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    }];
    transcript.prepend_events(older);
    assert_eq!(transcript.blocks().len(), 2);
    assert_eq!(transcript.first_seq(), Some(3));
}

#[test]
fn test_question_reply_renders_readonly_block() {
    let mut transcript = ConversationTranscript::new();
    let event = SessionEvent {
        event_type: "user/message".to_string(),
        seq: 1,
        time: 10,
        data: serde_json::json!({
            "source": {"kind": "user-question-reply", "question": "Proceed?"},
            "selected": ["Yes"],
            "content": [{"type": "text", "text": "Yes"}]
        }),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    transcript.apply_event(event).unwrap();
    match &transcript.blocks()[0] {
        RenderBlock::QuestionReply {
            summary, selected, ..
        } => {
            assert_eq!(summary, "Proceed?");
            assert_eq!(selected, &vec!["Yes".to_string()]);
        }
        other => panic!("unexpected block: {:?}", other),
    }
}

#[test]
fn test_stream_rejects_foreign_attempt_at_dense_revision() {
    let mut transcript = ConversationTranscript::new();
    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-1".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 2,
        index: 0,
        time: 1000,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "first attempt".to_string(),
        },
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-2".to_string(),
        revision: 3,
        index: 1,
        time: 1010,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "foreign attempt".to_string(),
        },
    });
    assert!(
        transcript.blocks().is_empty(),
        "a foreign attempt must retire the live block"
    );

    transcript.apply_stream_frame(AssistantStreamFrame::End {
        attempt_id: "att-2".to_string(),
        revision: 4,
        index: 2,
        outcome: StreamOutcome::Committed {
            event_type: "assistant/message".to_string(),
            seq: 1,
        },
    });
    assert!(transcript.blocks().is_empty());
}

#[test]
fn test_stream_start_replaces_superseded_attempt() {
    let mut transcript = ConversationTranscript::new();
    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-1".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 2,
        index: 0,
        time: 1000,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "superseded".to_string(),
        },
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-2".to_string(),
        revision: 1,
        turn: 1,
        step: 1,
    });
    assert_eq!(transcript.blocks().len(), 1);
    match &transcript.blocks()[0] {
        RenderBlock::AssistantMessage {
            is_live,
            text,
            attempt_id,
            ..
        } => {
            assert!(is_live);
            assert_eq!(text, "");
            assert_eq!(attempt_id.as_deref(), Some("att-2"));
        }
        _ => panic!("expected the replacement attempt"),
    }
}

#[test]
fn test_stream_chunk_index_must_be_dense() {
    let mut transcript = ConversationTranscript::new();
    transcript.apply_stream_frame(AssistantStreamFrame::Start {
        attempt_id: "att-1".to_string(),
        revision: 1,
        turn: 1,
        step: 0,
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 2,
        index: 0,
        time: 1000,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "kept".to_string(),
        },
    });
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-1".to_string(),
        revision: 3,
        index: 2,
        time: 1010,
        chunk: StreamChunk::TextDelta {
            index: 0,
            text: "out of order".to_string(),
        },
    });
    assert!(
        transcript.blocks().is_empty(),
        "a non-dense chunk must retire the live attempt"
    );
}

#[test]
fn test_baseline_stream_continues_at_next_revision() {
    let mut transcript = ConversationTranscript::new();
    transcript.apply_baseline(Some(SessionAssistantBaseline {
        attempt_id: "att-base".to_string(),
        turn: 2,
        step: 0,
        revision: 3,
        chunks: vec![StreamChunk::TextDelta {
            index: 0,
            text: "base".to_string(),
        }],
    }));
    transcript.apply_stream_frame(AssistantStreamFrame::Chunk {
        attempt_id: "att-base".to_string(),
        revision: 4,
        index: 1,
        time: 1000,
        chunk: StreamChunk::TextDelta {
            index: 1,
            text: " continued".to_string(),
        },
    });
    match &transcript.blocks()[0] {
        RenderBlock::AssistantMessage { text, .. } => assert_eq!(text, "base continued"),
        _ => panic!("expected baseline attempt"),
    }
}

#[test]
fn test_terminal_error_code_scan() {
    let event = |event_type: &str, seq: i64, data: serde_json::Value| SessionEvent {
        event_type: event_type.to_string(),
        seq,
        time: 1000 + seq as u64,
        data,
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    };
    let error = event("turn/end", 2, serde_json::json!({"turn": 1, "reason": {"code": "RATE_LIMIT"}}));
    let prompt = event("user/message", 3, serde_json::json!({"content": "next"}));
    assert_eq!(
        terminal_error_code(&[event("turn/start", 1, serde_json::json!({})), error.clone()]),
        Some("RATE_LIMIT".to_string())
    );
    assert_eq!(terminal_error_code(&[error, prompt]), None);
    assert_eq!(
        terminal_error_code(&[event("stream/error", 4, serde_json::json!({"error": {"code": "STREAM_CLOSED"}}))]),
        Some("STREAM_CLOSED".to_string())
    );
}
