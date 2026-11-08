use dsh_libadwaita::protocol::SessionEvent;
use dsh_libadwaita::transcript::{
    parse_message_attachments, parse_search_meta, parse_web_meta, normalize_denial_reason,
    parse_auto_review_denial, ConversationTranscript, RenderBlock, ToolErrorInfo,
    TranscriptUpdate,
};

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
fn test_normalize_denial_reason_collapses_and_trims() {
    assert_eq!(
        normalize_denial_reason("  line one\r\nline two \n\u{2028}three  ").as_deref(),
        Some("line one line two  three")
    );
    assert_eq!(normalize_denial_reason("plain reason").as_deref(), Some("plain reason"));
    assert_eq!(normalize_denial_reason("   \n  "), None);
    assert_eq!(normalize_denial_reason(""), None);
}

#[test]
fn test_parse_auto_review_denial_matches_error_identity() {
    let denial = parse_auto_review_denial(&ToolErrorInfo {
        name: "AutoReviewDeniedError".to_string(),
        code: "AUTO_REVIEW_DENIED".to_string(),
        reason: Some("rm -rf outside workspace".to_string()),
    });
    assert_eq!(denial.unwrap().reason.as_deref(), Some("rm -rf outside workspace"));

    let no_reason = parse_auto_review_denial(&ToolErrorInfo {
        name: "AutoReviewDeniedError".to_string(),
        code: "AUTO_REVIEW_DENIED".to_string(),
        reason: None,
    });
    assert!(no_reason.unwrap().reason.is_none());

    for (name, code) in [
        ("AutoReviewFailedError", "AUTO_REVIEW_FAILED"),
        ("Interrupted", "interrupted"),
        ("AutoReviewDeniedError", "OTHER"),
    ] {
        let other = ToolErrorInfo {
            name: name.to_string(),
            code: code.to_string(),
            reason: Some("x".to_string()),
        };
        assert!(parse_auto_review_denial(&other).is_none(), "{name}/{code}");
    }
}

#[test]
fn test_tool_result_captures_error_and_meta() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(
            1,
            100,
            "tool/call",
            serde_json::json!({"callId": "c1", "name": "bash", "arguments": "{\"command\":\"ls\"}"}),
        ))
        .unwrap();
    let update = transcript
        .apply_event(event(
            2,
            200,
            "tool/result",
            serde_json::json!({
                "turn": 1,
                "step": 1,
                "message": {"callId": "c1", "isError": true, "content": [{"type": "text", "text": "boom"}]},
                "error": {"name": "ExecError", "code": "EXEC_FAILED", "reason": "bad command"},
                "meta": {"cwd": "/tmp"}
            }),
        ))
        .unwrap();
    assert!(matches!(update, TranscriptUpdate::Appended(_)));
    let Some(RenderBlock::ToolResult { info, .. }) = transcript.blocks().last() else {
        panic!("expected ToolResult block");
    };
    assert!(info.is_error);
    let error = info.error.as_ref().unwrap();
    assert_eq!(error.name, "ExecError");
    assert_eq!(error.code, "EXEC_FAILED");
    assert_eq!(error.reason.as_deref(), Some("bad command"));
    assert_eq!(info.meta.as_ref().unwrap()["cwd"], "/tmp");
    assert_eq!(info.result, "boom");
}

#[test]
fn test_tool_result_auto_review_denial_derived() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(
            1,
            100,
            "tool/call",
            serde_json::json!({"callId": "c2", "name": "bash", "arguments": "{\"command\":\"rm -rf /\"}"}),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            200,
            "tool/result",
            serde_json::json!({
                "turn": 1,
                "step": 1,
                "message": {"callId": "c2", "isError": true, "content": [{"type": "text", "text": "denied"}]},
                "error": {
                    "name": "AutoReviewDeniedError",
                    "code": "AUTO_REVIEW_DENIED",
                    "reason": "Auto review rejected tool \"bash\"; its body was not executed\nunsafe deletion"
                }
            }),
        ))
        .unwrap();
    let Some(RenderBlock::ToolResult { info, .. }) = transcript.blocks().last() else {
        panic!("expected ToolResult block");
    };
    let denial = info.error.as_ref().unwrap().auto_review_denial().unwrap();
    assert_eq!(
        denial.reason.as_deref(),
        Some("Auto review rejected tool \"bash\"; its body was not executed unsafe deletion")
    );
}

#[test]
fn test_ptc_dispatch_tree_assembles_nested_sub_calls() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(
            1,
            100,
            "tool/call",
            serde_json::json!({"callId": "root", "name": "run_code", "arguments": "{\"code\":\"...\"}"}),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            110,
            "tool/ptc-dispatch-start",
            serde_json::json!({
                "rootCallId": "root",
                "parentCallId": "root",
                "subCallId": "root:ptc:1",
                "name": "bash",
                "arguments": {"command": "ls"}
            }),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            3,
            120,
            "tool/ptc-dispatch-start",
            serde_json::json!({
                "rootCallId": "root",
                "parentCallId": "root:ptc:1",
                "subCallId": "root:ptc:2",
                "name": "grep",
                "arguments": {"pattern": "x"}
            }),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            4,
            130,
            "tool/ptc-dispatch",
            serde_json::json!({
                "rootCallId": "root",
                "parentCallId": "root",
                "subCallId": "root:ptc:1",
                "name": "bash",
                "arguments": {"command": "ls"},
                "isError": false,
                "content": [{"type": "text", "text": "file.txt"}],
                "meta": {"cwd": "/tmp"}
            }),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            5,
            140,
            "tool/ptc-dispatch",
            serde_json::json!({
                "rootCallId": "root",
                "parentCallId": "root:ptc:1",
                "subCallId": "root:ptc:2",
                "name": "grep",
                "arguments": {"pattern": "x"},
                "isError": true,
                "content": [{"type": "text", "text": "nope"}],
                "error": {"name": "ToolError", "code": "TOOL_FAILED"}
            }),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            6,
            150,
            "tool/result",
            serde_json::json!({
                "turn": 1,
                "step": 1,
                "message": {"callId": "root", "isError": false, "content": [{"type": "text", "text": "done"}]}
            }),
        ))
        .unwrap();
    let Some(RenderBlock::ToolResult { info, .. }) = transcript.blocks().last() else {
        panic!("expected ToolResult block");
    };
    assert_eq!(info.name, "run_code");
    assert_eq!(info.sub_calls.len(), 1);
    let first = &info.sub_calls[0];
    assert_eq!(first.name, "bash");
    assert!(!first.is_error);
    assert_eq!(first.result, "file.txt");
    assert_eq!(first.meta.as_ref().unwrap()["cwd"], "/tmp");
    assert_eq!(first.sub_calls.len(), 1);
    let nested = &first.sub_calls[0];
    assert_eq!(nested.name, "grep");
    assert!(nested.is_error);
    assert_eq!(nested.error.as_ref().unwrap().code, "TOOL_FAILED");
    assert!(nested.sub_calls.is_empty());
}

#[test]
fn test_ptc_dispatch_unsettled_sub_call_kept_as_pending() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(
            1,
            100,
            "tool/call",
            serde_json::json!({"callId": "root", "name": "run_code", "arguments": "{}"}),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            2,
            110,
            "tool/ptc-dispatch-start",
            serde_json::json!({
                "rootCallId": "root",
                "parentCallId": "root",
                "subCallId": "root:ptc:1",
                "name": "bash",
                "arguments": {"command": "sleep"}
            }),
        ))
        .unwrap();
    transcript
        .apply_event(event(
            3,
            150,
            "tool/result",
            serde_json::json!({
                "turn": 1,
                "step": 1,
                "message": {"callId": "root", "isError": true, "content": [{"type": "text", "text": "interrupted"}]},
                "error": {"name": "Interrupted", "code": "interrupted"}
            }),
        ))
        .unwrap();
    let Some(RenderBlock::ToolResult { info, .. }) = transcript.blocks().last() else {
        panic!("expected ToolResult block");
    };
    assert_eq!(info.sub_calls.len(), 1);
    assert!(!info.sub_calls[0].is_error);
    assert_eq!(info.sub_calls[0].result, "");
}

#[test]
fn test_parse_search_meta_matches_shape() {
    let meta = serde_json::json!({
        "shape": "matches",
        "files": [
            {"path": "src/a.rs", "matches": [
                {"lineNumber": 3, "line": "fn main() {}"},
                {"lineNumber": 9, "line": "let x = 1;"}
            ]},
            {"path": "src/b.rs", "matches": [{"lineNumber": 1, "line": "use x;"}]}
        ],
        "total": 3,
        "truncated": false
    });
    let parsed = parse_search_meta(Some(&meta), "").unwrap();
    assert_eq!(parsed.total, 3);
    assert!(!parsed.truncated);
    assert!(parsed.recovery.is_none());
    let files = parsed.files.as_ref().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "src/a.rs");
    assert_eq!(files[0].matches.len(), 2);
    assert_eq!(files[0].matches[0].line_number, 3);
    assert!(parsed.paths.is_none());
}

#[test]
fn test_parse_search_meta_paths_shape_and_truncated_recovery() {
    let meta = serde_json::json!({
        "shape": "paths",
        "paths": ["src/a.rs", "src/b.rs"],
        "total": 12,
        "truncated": true
    });
    let parsed = parse_search_meta(Some(&meta), "raw fallback output").unwrap();
    assert!(parsed.truncated);
    assert_eq!(parsed.total, 12);
    assert_eq!(
        parsed.paths.as_ref().unwrap(),
        &vec!["src/a.rs".to_string(), "src/b.rs".to_string()]
    );
    assert_eq!(parsed.recovery.as_deref(), Some("raw fallback output"));
    assert!(parsed.files.is_none());
}

#[test]
fn test_parse_search_meta_rejects_invalid() {
    assert!(parse_search_meta(None, "").is_none());
    assert!(parse_search_meta(Some(&serde_json::json!({})), "").is_none());
    assert!(parse_search_meta(
        Some(&serde_json::json!({"shape": "matches", "files": [{"path": "a"}], "total": 1, "truncated": false})),
        ""
    )
    .is_none());
    assert!(parse_search_meta(
        Some(&serde_json::json!({"shape": "other", "total": 1, "truncated": false})),
        ""
    )
    .is_none());
}

#[test]
fn test_parse_web_meta_search_sources() {
    let meta = serde_json::json!({
        "sources": [
            {"url": "https://a.example", "title": "A", "snippet": "first", "publishedAt": "2026-01-01"},
            {"url": "https://b.example"}
        ],
        "answer": "synthesized",
        "truncated": true
    });
    let parsed = parse_web_meta("web_search", Some(&meta)).unwrap();
    let dsh_libadwaita::transcript::WebCardKind::Search { answer, sources, truncated } = parsed
    else {
        panic!("expected search kind");
    };
    assert_eq!(answer.as_deref(), Some("synthesized"));
    assert!(truncated);
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].url, "https://a.example");
    assert_eq!(sources[0].title.as_deref(), Some("A"));
    assert_eq!(sources[1].title, None);
}

#[test]
fn test_parse_web_meta_fetch_status() {
    let meta = serde_json::json!({"url": "https://x.example/page", "statusCode": 200, "truncated": false});
    let parsed = parse_web_meta("web_fetch", Some(&meta)).unwrap();
    let dsh_libadwaita::transcript::WebCardKind::Fetch { url, status_code, truncated } = parsed
    else {
        panic!("expected fetch kind");
    };
    assert_eq!(url, "https://x.example/page");
    assert_eq!(status_code, 200);
    assert!(!truncated);
}

#[test]
fn test_parse_web_meta_rejects_invalid() {
    assert!(parse_web_meta("web_search", None).is_none());
    assert!(parse_web_meta("web_search", Some(&serde_json::json!({"sources": "x", "truncated": false}))).is_none());
    assert!(parse_web_meta("web_fetch", Some(&serde_json::json!({"statusCode": "200", "truncated": false}))).is_none());
    assert!(parse_web_meta("grep", Some(&serde_json::json!({"truncated": false}))).is_none());
}

#[test]
fn test_parse_message_attachments_images_and_files() {
    let data = serde_json::json!({
        "content": [
            {"type": "text", "text": "look at this"},
            {"type": "image", "attachment": {"attachmentId": "img1", "mediaType": "image/png", "bytes": 10, "width": 32, "height": 32, "name": "shot.png"}},
            {"type": "file", "attachment": {"attachmentId": "file1", "name": "report.pdf", "bytes": 2048}}
        ]
    });
    let attachments = parse_message_attachments(&data);
    assert_eq!(attachments.len(), 2);
    let dsh_libadwaita::transcript::MessageAttachment::Image(image) = &attachments[0] else {
        panic!("expected image attachment");
    };
    assert_eq!(image.attachment_id.as_deref(), Some("img1"));
    let reference = image.reference.as_ref().expect("durable image reference");
    assert_eq!(reference.media_type, "image/png");
    assert_eq!(reference.bytes, 10);
    assert_eq!(reference.width, 32);
    assert_eq!(reference.height, 32);
    let dsh_libadwaita::transcript::MessageAttachment::File(file) = &attachments[1] else {
        panic!("expected file attachment");
    };
    assert_eq!(file.name, "report.pdf");
    assert_eq!(file.bytes, Some(2048));
}

#[test]
fn test_assistant_message_carries_attachments() {
    let mut transcript = ConversationTranscript::new();
    transcript
        .apply_event(event(
            1,
            100,
            "assistant/message",
            serde_json::json!({
                "message": {"content": [
                    {"type": "text", "text": "here you go"},
                    {"type": "image", "attachment": {"attachmentId": "img1", "mediaType": "image/png", "bytes": 10, "width": 32, "height": 32}},
                    {"type": "file", "attachment": {"attachmentId": "file1", "name": "a.txt", "bytes": 3}}
                ]}
            }),
        ))
        .unwrap();
    let Some(RenderBlock::AssistantMessage { attachments, .. }) = transcript.blocks().last() else {
        panic!("expected AssistantMessage block");
    };
    assert_eq!(attachments.len(), 2);
}
