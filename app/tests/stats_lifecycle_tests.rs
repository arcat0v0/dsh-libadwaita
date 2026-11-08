use dsh_libadwaita::protocol::SessionEvent;
use dsh_libadwaita::transcript::{turn_stats, ConversationTranscript, RenderBlock, TranscriptUpdate};

fn event(seq: i64, time: u64, event_type: &str, data: serde_json::Value) -> SessionEvent {
    SessionEvent {
        event_type: event_type.to_string(),
        seq,
        time,
        data,
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    }
}

#[test]
fn test_turn_stats_accumulates_usage_and_rate() {
    let events = vec![
        event(1, 1000, "turn/start", serde_json::json!({"turn": 1})),
        event(
            2,
            1500,
            "assistant/attempt",
            serde_json::json!({
                "turn": 1,
                "step": 1,
                "stream": [
                    {"type": "chunk", "chunk": {"type": "usage", "usage": {
                        "uncachedInputTokens": 100,
                        "outputTokens": 200,
                        "reasoningTokens": 50,
                        "cacheReadTokens": 30
                    }}}
                ]
            }),
        ),
        event(3, 3000, "turn/end", serde_json::json!({"turn": 1, "reason": {"kind": "completed"}})),
    ];
    let stats = turn_stats(&events);
    assert_eq!(stats.len(), 1);
    let stat = &stats[0];
    assert_eq!(stat.turn, 1);
    assert_eq!(stat.input_tokens, 100);
    assert_eq!(stat.output_tokens, 200);
    assert_eq!(stat.reasoning_tokens, 50);
    assert_eq!(stat.cache_read_tokens, 30);
    assert_eq!(stat.elapsed_ms, 2000);
    assert!((stat.tokens_per_sec - 100.0).abs() < 0.01);
}

#[test]
fn test_turn_stats_sums_multiple_attempts() {
    let events = vec![
        event(1, 0, "turn/start", serde_json::json!({"turn": 2})),
        event(
            2,
            100,
            "assistant/attempt",
            serde_json::json!({"stream": [{"chunk": {"type": "usage", "usage": {"outputTokens": 10}}}]}),
        ),
        event(
            3,
            200,
            "assistant/attempt",
            serde_json::json!({"stream": [{"chunk": {"type": "usage", "usage": {"outputTokens": 20}}}]}),
        ),
        event(4, 1000, "turn/end", serde_json::json!({"turn": 2, "reason": {"kind": "completed"}})),
    ];
    let stats = turn_stats(&events);
    assert_eq!(stats[0].output_tokens, 30);
    assert_eq!(stats[0].elapsed_ms, 1000);
}

#[test]
fn test_turn_stats_zero_elapsed_has_zero_rate() {
    let events = vec![
        event(1, 500, "turn/start", serde_json::json!({"turn": 1})),
        event(2, 500, "turn/end", serde_json::json!({"turn": 1, "reason": {"kind": "completed"}})),
    ];
    let stats = turn_stats(&events);
    assert_eq!(stats[0].elapsed_ms, 0);
    assert_eq!(stats[0].tokens_per_sec, 0.0);
}

#[test]
fn test_compaction_summary_renders_card() {
    let mut transcript = ConversationTranscript::new();
    let update = transcript
        .apply_event(event(
            1,
            10,
            "compaction/summary",
            serde_json::json!({"summary": "Compacted 12 earlier messages"}),
        ))
        .unwrap();
    assert!(matches!(update, TranscriptUpdate::Appended(_)));
    assert!(matches!(
        transcript.blocks().last(),
        Some(RenderBlock::CompactionCard { summary, .. }) if summary == "Compacted 12 earlier messages"
    ));
}

#[test]
fn test_turn_end_error_renders_error_row() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(1, 10, "turn/start", serde_json::json!({"turn": 1})))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            20,
            "turn/end",
            serde_json::json!({"turn": 1, "reason": {"kind": "error", "error": {"message": "rate limited", "code": "RATE_LIMIT"}}}),
        ))
        .unwrap();
    assert!(matches!(
        transcript.blocks().last(),
        Some(RenderBlock::TurnError { message, code, .. })
            if message == "rate limited" && code.as_deref() == Some("RATE_LIMIT")
    ));
}

#[test]
fn test_turn_end_max_tokens_renders_hint() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(1, 10, "turn/start", serde_json::json!({"turn": 1})))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            20,
            "turn/end",
            serde_json::json!({"turn": 1, "reason": {"kind": "max-tokens"}}),
        ))
        .unwrap();
    assert!(matches!(
        transcript.blocks().last(),
        Some(RenderBlock::MaxTokensHint { .. })
    ));
}

#[test]
fn test_turn_end_completed_is_ignored() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(1, 10, "turn/start", serde_json::json!({"turn": 1})))
        .unwrap();
    let update = transcript
        .apply_event(event(
            2,
            20,
            "turn/end",
            serde_json::json!({"turn": 1, "reason": {"kind": "completed"}}),
        ))
        .unwrap();
    assert_eq!(update, TranscriptUpdate::Ignored);
}

#[test]
fn test_llm_retry_clears_live_attempt_and_adds_hint() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(1, 10, "turn/start", serde_json::json!({"turn": 1})))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            20,
            "llm/retry",
            serde_json::json!({"turn": 1, "retry": 1, "failure": {"message": "server error"}}),
        ))
        .unwrap();
    assert!(matches!(
        transcript.blocks().last(),
        Some(RenderBlock::RetryHint { message, .. }) if message == "server error"
    ));
}
