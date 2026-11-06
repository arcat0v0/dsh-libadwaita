use dsh_libadwaita::protocol::*;

#[test]
fn test_jsonrpc_request_roundtrip() {
    let req = JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: RequestId::from("req-1"),
        method: METHOD_INITIALIZE.to_string(),
        params: Some(serde_json::to_value(InitializeParams {
            client_version: "0.1.0".to_string(),
            protocol_version: PROTOCOL_VERSION,
            capabilities: Some(ClientCapabilities {
                user_questions: true,
                approval_requests: true,
                streaming: true,
            }),
        }).unwrap()),
    };

    let serialized = serde_json::to_string(&req).unwrap();
    let parsed: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed.jsonrpc, "2.0");
    assert_eq!(parsed.id, RequestId::from("req-1"));
    assert_eq!(parsed.method, "libadwaita/initialize");
}

#[test]
fn test_jsonrpc_response_success_and_error() {
    let ok_resp = JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: RequestId::from(42),
        result: Some(serde_json::json!({"ok": true})),
        error: None,
    };
    let s_ok = serde_json::to_string(&ok_resp).unwrap();
    let p_ok: JsonRpcResponse = serde_json::from_str(&s_ok).unwrap();
    assert!(p_ok.is_success());
    assert_eq!(p_ok.id, RequestId::from(42));

    let err_resp: JsonRpcResponse = JsonRpcResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: RequestId::from("req-err"),
        result: None,
        error: Some(JsonRpcError {
            code: ERR_SESSION_NOT_FOUND,
            message: "session not found".to_string(),
            data: Some(serde_json::json!({"sessionId": "sess-abc"})),
        }),
    };
    let s_err = serde_json::to_string(&err_resp).unwrap();
    let p_err: JsonRpcResponse = serde_json::from_str(&s_err).unwrap();
    assert!(!p_err.is_success());
    assert_eq!(p_err.error.as_ref().unwrap().code, -32002);
    assert_eq!(
        BridgeErrorCode::from_code(p_err.error.as_ref().unwrap().code),
        BridgeErrorCode::SessionNotFound
    );
}

#[test]
fn test_incoming_message_parsing() {
    let raw_req = r#"{"jsonrpc":"2.0","id":"srv-1","method":"approval/request","params":{"sessionId":"s1","toolName":"bash","callId":"c1"}}"#;
    let msg: IncomingMessage = serde_json::from_str(raw_req).unwrap();
    match msg {
        IncomingMessage::Request { id, method, params } => {
            assert_eq!(id, RequestId::from("srv-1"));
            assert_eq!(method, "approval/request");
            assert!(params.is_some());
        }
        _ => panic!("expected request"),
    }

    let raw_resp = r#"{"jsonrpc":"2.0","id":"req-1","result":{"status":"idle"}}"#;
    let msg2: IncomingMessage = serde_json::from_str(raw_resp).unwrap();
    match msg2 {
        IncomingMessage::Response { id, result, error } => {
            assert_eq!(id, RequestId::from("req-1"));
            assert!(result.is_some());
            assert!(error.is_none());
        }
        _ => panic!("expected response"),
    }

    let raw_notif = r#"{"jsonrpc":"2.0","method":"session.status","params":{"sessionId":"s1","status":"running"}}"#;
    let msg3: IncomingMessage = serde_json::from_str(raw_notif).unwrap();
    match msg3 {
        IncomingMessage::Notification { method, params } => {
            assert_eq!(method, "session.status");
            assert_eq!(params["status"], "running");
        }
        _ => panic!("expected notification"),
    }
}

#[test]
fn test_session_follow_and_projections_roundtrip() {
    let json_str = r#"{
        "header": {
            "version": 4,
            "id": "sess-88a1",
            "createdAt": 1762419200000,
            "cwd": "/home/arcat/Develop/my-project",
            "isSeeded": false
        },
        "latestSeq": 10,
        "status": "running",
        "title": {
            "title": "Test Title",
            "source": {"kind": "human"}
        },
        "projections": {
            "planMode": true,
            "todos": [
                {"id": "t1", "content": "task 1", "status": "completed"},
                {"id": "t2", "content": "task 2", "status": "in_progress"}
            ]
        },
        "assistantBaseline": {
            "attemptId": "att-1",
            "turn": 1,
            "step": 0,
            "revision": 2,
            "chunks": [
                {"type": "text-delta", "index": 0, "text": "Hello"}
            ]
        }
    }"#;

    let res: SessionFollowResult = serde_json::from_str(json_str).unwrap();
    assert_eq!(res.header.id, "sess-88a1");
    assert_eq!(res.latest_seq, 10);
    assert_eq!(res.status, "running");
    assert!(res.projections.plan_mode);
    let todos = res.projections.todos.unwrap();
    assert_eq!(todos.len(), 2);
    assert_eq!(todos[0].status, "completed");

    let baseline = res.assistant_baseline.unwrap();
    assert_eq!(baseline.attempt_id, "att-1");
    assert_eq!(baseline.chunks.len(), 1);
    match &baseline.chunks[0] {
        StreamChunk::TextDelta { text, .. } => assert_eq!(text, "Hello"),
        _ => panic!("expected TextDelta"),
    }
}

#[test]
fn test_assistant_stream_frame_variants() {
    let start_json = r#"{"type":"start","attemptId":"att-0","revision":1,"turn":1,"step":0}"#;
    let start_frame: AssistantStreamFrame = serde_json::from_str(start_json).unwrap();
    match start_frame {
        AssistantStreamFrame::Start { attempt_id, turn, .. } => {
            assert_eq!(attempt_id, "att-0");
            assert_eq!(turn, 1);
        }
        _ => panic!("expected Start"),
    }

    let chunk_json = r#"{"type":"chunk","attemptId":"att-0","revision":1,"index":0,"time":1762419201200,"chunk":{"type":"text-delta","index":0,"text":"world"}}"#;
    let chunk_frame: AssistantStreamFrame = serde_json::from_str(chunk_json).unwrap();
    match chunk_frame {
        AssistantStreamFrame::Chunk { chunk, .. } => match chunk {
            StreamChunk::TextDelta { text, .. } => assert_eq!(text, "world"),
            _ => panic!("expected TextDelta"),
        },
        _ => panic!("expected Chunk"),
    }

    let end_json = r#"{"type":"end","attemptId":"att-0","revision":1,"index":2,"outcome":{"kind":"committed","eventType":"assistant/message","seq":3}}"#;
    let end_frame: AssistantStreamFrame = serde_json::from_str(end_json).unwrap();
    match end_frame {
        AssistantStreamFrame::End { outcome, .. } => match outcome {
            StreamOutcome::Committed { event_type, seq } => {
                assert_eq!(event_type, "assistant/message");
                assert_eq!(seq, 3);
            }
            _ => panic!("expected Committed"),
        },
        _ => panic!("expected End"),
    }

    let end_abandoned = r#"{"type":"end","attemptId":"att-0","revision":1,"index":2,"outcome":{"kind":"abandoned"}}"#;
    let end_abandoned_frame: AssistantStreamFrame = serde_json::from_str(end_abandoned).unwrap();
    match end_abandoned_frame {
        AssistantStreamFrame::End { outcome, .. } => match outcome {
            StreamOutcome::Abandoned => {}
            _ => panic!("expected Abandoned"),
        },
        _ => panic!("expected End"),
    }
}

#[test]
fn test_approval_request_and_result() {
    let req_json = r#"{"sessionId":"sess-1","toolName":"bash","callId":"call-123","reason":"run git status"}"#;
    let req: ApprovalRequestParams = serde_json::from_str(req_json).unwrap();
    assert_eq!(req.session_id, "sess-1");
    assert_eq!(req.tool_name, "bash");
    assert_eq!(req.call_id, "call-123");

    let allowed = ApprovalRequestResult {
        outcome: ApprovalOutcome::AllowedOnce,
    };
    let s_allowed = serde_json::to_string(&allowed).unwrap();
    assert!(s_allowed.contains("allowed-once"));

    let rejected = ApprovalRequestResult {
        outcome: ApprovalOutcome::Rejected,
    };
    let s_rejected = serde_json::to_string(&rejected).unwrap();
    assert!(s_rejected.contains("rejected"));
}

#[test]
fn test_user_question_with_plan_review_intent() {
    let req_json = r#"{
        "requestId": "q-req-1",
        "sessionId": "sess-uuid",
        "questions": [
            {
                "id": "plan-review",
                "question": "审查执行计划",
                "detail": "变更计划详情",
                "intent": {
                    "kind": "plan-review",
                    "approve": "同意执行",
                    "callId": "call-exit-plan"
                },
                "options": [
                    {"label": "同意执行", "description": "退出计划模式并开始执行"},
                    {"label": "要求修改", "description": "留在计划模式要求修改"}
                ],
                "multiSelect": false
            }
        ]
    }"#;

    let req: SdkUserQuestionRequest = serde_json::from_str(req_json).unwrap();
    assert_eq!(req.request_id, "q-req-1");
    assert_eq!(req.questions.len(), 1);
    let q = &req.questions[0];
    assert_eq!(q.id, "plan-review");
    match q.intent.as_ref().unwrap() {
        UserQuestionIntent::PlanReview { approve, call_id } => {
            assert_eq!(approve, "同意执行");
            assert_eq!(call_id.as_deref(), Some("call-exit-plan"));
        }
    }

    let ans = SdkUserQuestionResult::answers(vec![UserQuestionAnswer {
        id: "plan-review".to_string(),
        selected: vec!["同意执行".to_string()],
    }]);
    let s_ans = serde_json::to_string(&ans).unwrap();
    assert!(s_ans.contains("answers"));

    let cancel = SdkUserQuestionResult::cancelled();
    let s_cancel = serde_json::to_string(&cancel).unwrap();
    assert!(s_cancel.contains(r#""cancelled":true"#));
}

#[test]
fn test_error_code_mapping() {
    assert_eq!(BridgeErrorCode::from_code(-32700), BridgeErrorCode::ParseError);
    assert_eq!(BridgeErrorCode::from_code(-32001), BridgeErrorCode::ProtocolVersionMismatch);
    assert_eq!(BridgeErrorCode::from_code(-32006), BridgeErrorCode::ApprovalFailClosed);
    assert_eq!(BridgeErrorCode::from_code(-32010), BridgeErrorCode::RestartRequired);
    assert_eq!(BridgeErrorCode::from_code(-99999), BridgeErrorCode::Unknown(-99999));
    assert_eq!(BridgeErrorCode::ApprovalFailClosed.to_code(), -32006);
}

#[test]
fn test_batch_b_protocol_roundtrips() {
    let workspace = WorkspaceView {
        id: "ws-1".to_string(),
        path: "/tmp/project".to_string(),
        title: "project".to_string(),
        created_at: "2026-11-06T00:00:00Z".to_string(),
        updated_at: "2026-11-06T00:01:00Z".to_string(),
        session_ids: vec!["s-1".to_string()],
        status: Some("ready".to_string()),
    };
    let list = WorkspaceListResult {
        workspaces: vec![workspace],
    };
    let json = serde_json::to_string(&list).unwrap();
    let decoded: WorkspaceListResult = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, list);

    let permission = PermissionGetResult {
        current_value: "auto".to_string(),
        catalog: PermissionCatalogResult {
            options: vec![PresetOptionItem {
                value: "auto".to_string(),
                name: "Auto".to_string(),
                description: Some("Approve safe tools".to_string()),
            }],
            default_options: vec![],
            default_preset: "default".to_string(),
        },
    };
    assert_eq!(
        dsh_libadwaita::ui::controller::permission_display_label(&permission),
        "Auto"
    );

    let file_text = FilesReadResult {
        kind: "text".to_string(),
        path: "src/lib.rs".to_string(),
        text: Some("pub fn ok() {}\n".to_string()),
        size: 15,
        truncated: Some(false),
    };
    assert_eq!(
        dsh_libadwaita::ui::files_panel::read_text_result(&file_text),
        Some(("pub fn ok() {}\n".to_string(), false))
    );
    let binary_text = FilesReadResult {
        kind: "binary".to_string(),
        path: "bin".to_string(),
        text: None,
        size: 128,
        truncated: None,
    };
    assert_eq!(dsh_libadwaita::ui::files_panel::read_text_result(&binary_text), None);
    assert!(dsh_libadwaita::ui::files_panel::is_image_path("diagram.svg"));
    assert!(!dsh_libadwaita::ui::files_panel::is_image_path("notes.rs"));

    let goal_notif = GoalUpdatedNotificationParams {
        session_id: "s-1".to_string(),
        goal: Some(GoalView {
            id: "g-1".to_string(),
            revision: 3,
            objective: "Ship".to_string(),
            phase: "active".to_string(),
            blocked_reason: None,
            max_goal_rounds: 8,
            rounds_started: 2,
            created_at: 100,
            updated_at: 120,
            activation: "manual".to_string(),
        }),
    };
    let raw = serde_json::to_value(&goal_notif).unwrap();
    assert_eq!(
        dsh_libadwaita::connection::IncomingNotification::from_raw(NOTIFICATION_GOAL_UPDATED, raw),
        dsh_libadwaita::connection::IncomingNotification::GoalUpdated(goal_notif)
    );
}
