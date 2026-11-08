use dsh_libadwaita::connection::IncomingNotification;
use dsh_libadwaita::protocol::*;
use dsh_libadwaita::schedule_model::{
    ScheduleDailyInput, ScheduleSessionCreateSpec, ScheduleTargetRequest, ScheduleTiming,
};

#[test]
fn test_jsonrpc_request_roundtrip() {
    let req = JsonRpcRequest {
        jsonrpc: JSONRPC_VERSION.to_string(),
        id: RequestId::from("req-1"),
        method: METHOD_INITIALIZE.to_string(),
        params: Some(
            serde_json::to_value(InitializeParams {
                client_version: "0.1.0".to_string(),
                protocol_version: PROTOCOL_VERSION,
                capabilities: Some(ClientCapabilities {
                    user_questions: true,
                    approval_requests: true,
                    streaming: true,
                }),
            })
            .unwrap(),
        ),
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
        AssistantStreamFrame::Start {
            attempt_id, turn, ..
        } => {
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
    let req_json =
        r#"{"sessionId":"sess-1","toolName":"bash","callId":"call-123","reason":"run git status"}"#;
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
    assert_eq!(
        BridgeErrorCode::from_code(-32700),
        BridgeErrorCode::ParseError
    );
    assert_eq!(
        BridgeErrorCode::from_code(-32001),
        BridgeErrorCode::ProtocolVersionMismatch
    );
    assert_eq!(
        BridgeErrorCode::from_code(-32006),
        BridgeErrorCode::ApprovalFailClosed
    );
    assert_eq!(
        BridgeErrorCode::from_code(-32010),
        BridgeErrorCode::RestartRequired
    );
    assert_eq!(
        BridgeErrorCode::from_code(-99999),
        BridgeErrorCode::Unknown(-99999)
    );
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
        "自动审查"
    );

    let decoded_permission: PermissionGetResult = serde_json::from_value(serde_json::json!({
        "currentValue": "workspace-write",
        "catalog": {
            "options": [{"value": "workspace-write", "name": "Workspace Write"}],
            "defaultOptions": [{"value": "read-only", "name": "Read Only"}],
            "defaultPreset": "workspace-write"
        }
    }))
    .unwrap();
    assert_eq!(decoded_permission.current_value, "workspace-write");
    assert_eq!(decoded_permission.catalog.default_preset, "workspace-write");
    assert_eq!(decoded_permission.catalog.default_options.len(), 1);

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
    assert_eq!(
        dsh_libadwaita::ui::files_panel::read_text_result(&binary_text),
        None
    );
    assert!(dsh_libadwaita::ui::files_panel::is_image_path(
        "diagram.svg"
    ));
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

#[test]
fn test_batch_d_notifications_and_types() {
    let install = serde_json::json!({
        "requestId": "req-1",
        "jobId": "job-1",
        "argv": ["pnpm", "add", "file:/tmp/plugin"],
        "cwd": "/tmp/profile",
        "stream": "stdout",
        "text": "Progress",
        "exitCode": 0
    });
    match dsh_libadwaita::connection::IncomingNotification::from_raw(
        NOTIFICATION_PLUGIN_INSTALL_LOG,
        install,
    ) {
        dsh_libadwaita::connection::IncomingNotification::PluginInstallLog(log) => {
            assert_eq!(log.job_id, "job-1");
            assert_eq!(log.text, "Progress");
        }
        other => panic!("unexpected notification: {:?}", other),
    }

    let job = serde_json::json!({
        "sessionId": "s1",
        "jobId": "job-1",
        "job": {
            "id": "job-1",
            "kind": "bash",
            "label": "sleep 3",
            "status": "running",
            "startedAt": 1,
            "output": {"total": 5, "earliest": 0}
        },
        "chunks": [{"at": 0, "text": "hello\n", "channel": "stdout"}],
        "next": 5,
        "lossy": false
    });
    match dsh_libadwaita::connection::IncomingNotification::from_raw(NOTIFICATION_JOB_OUTPUT, job) {
        dsh_libadwaita::connection::IncomingNotification::JobOutput(output) => {
            assert_eq!(output.chunks[0].text, "hello\n");
            assert_eq!(output.next, 5);
        }
        other => panic!("unexpected notification: {:?}", other),
    }

    let feedback = MessageFeedbackItem {
        message_id: "m1".to_string(),
        rating: "positive".to_string(),
        note: None,
        category: None,
        version: "v1".to_string(),
        created_at: 1,
        updated_at: 2,
    };
    assert_eq!(feedback.rating, "positive");
}

#[test]
fn test_terminal_and_schedule_protocol_shapes() {
    let created: TerminalCreateResult =
        serde_json::from_str(r#"{"terminalId":"t1","sessionId":"s1","pid":42,"cwd":"/tmp"}"#)
            .unwrap();
    assert_eq!(created.terminal_id, "t1");

    let output: TerminalOutputResult = serde_json::from_str(
        r#"{"text":"hi\n","totalLines":1,"lineBegin":0,"lineEnd":1,"truncated":false}"#,
    )
    .unwrap();
    assert_eq!(output.text, "hi\n");

    let catalog: ScheduleCatalogResult = serde_json::from_value(serde_json::json!({
        "result": [{
            "id": "sch1",
            "kind": "rrule",
            "title": "Nightly",
            "prompt": "run nightly",
            "scheduledAt": "2026-09-19T15:51:00Z",
            "rrule": "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=1",
            "target": {"kind": "session", "sessionId": "s1"},
            "involvedSessionIds": ["s1"],
            "status": "active"
        }]
    }))
    .unwrap();
    assert_eq!(catalog.result[0].title, "Nightly");
    assert_eq!(
        catalog.result[0].status,
        dsh_libadwaita::schedule_model::ScheduleStatus::Active
    );

    let changed = serde_json::json!({"changed": true});
    match IncomingNotification::from_raw(NOTIFICATION_SCHEDULE_CHANGED, changed) {
        IncomingNotification::ScheduleChanged(p) => assert!(p.changed),
        other => panic!("unexpected notification: {:?}", other),
    }
}

#[test]
fn test_plugin_change_result_parsing() {
    let raw = r#"{
        "changed": true,
        "application": "restart-required",
        "restartReason": "no-hmr",
        "stage": "enable",
        "target": "@deepseek-ai/dsh-experimental-auto-review",
        "enabled": true
    }"#;
    let result: PluginChangeResult = serde_json::from_str(raw).unwrap();
    assert_eq!(result.application, "restart-required");
    assert_eq!(result.restart_reason.as_deref(), Some("no-hmr"));
    assert_eq!(result.stage, "enable");
    assert_eq!(result.target, "@deepseek-ai/dsh-experimental-auto-review");
    assert_eq!(result.enabled, Some(true));
    assert!(result.error.is_none());

    let failed: PluginChangeResult = serde_json::from_str(
        r#"{
            "changed": false,
            "application": "failed",
            "stage": "install",
            "target": "@community/whale-pet",
            "error": {
                "code": "operation-error",
                "diagnostic": "pnpm exited with 1",
                "incompatible": [
                    {"name": "@community/whale-pet", "version": "9.0.0", "runtimeVersion": "0.4.0", "requirements": {"dsh": "^0.4"}}
                ]
            },
            "packageResult": {"exitCode": 1, "output": "ERR", "truncated": false, "logPath": "/tmp/pnpm.log", "kind": "network"},
            "failedAt": "registry",
            "registries": [null, "https://registry.npmmirror.com/"]
        }"#,
    )
    .unwrap();
    let error = failed.error.as_ref().unwrap();
    assert_eq!(error.code, "operation-error");
    assert_eq!(error.diagnostic.as_deref(), Some("pnpm exited with 1"));
    let incompatible = error.incompatible.as_ref().unwrap();
    assert_eq!(incompatible[0].runtime_version, "0.4.0");
    assert_eq!(incompatible[0].requirements["dsh"], "^0.4");
    let package = failed.package_result.as_ref().unwrap();
    assert_eq!(package.exit_code, 1);
    assert_eq!(package.kind.as_deref(), Some("network"));
    assert_eq!(failed.failed_at.as_deref(), Some("registry"));
    assert_eq!(
        failed.registries.as_ref().unwrap()[1].as_deref(),
        Some("https://registry.npmmirror.com/")
    );
}

#[test]
fn test_plugin_bundle_toggle_params_serialize() {
    let params = PluginBundleToggleParams {
        name: "@deepseek-ai/dsh-experimental-auto-review".to_string(),
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["name"], "@deepseek-ai/dsh-experimental-auto-review");
}

#[test]
fn test_plugin_detail_result_parsing() {
    let found: PluginBundleDetail = serde_json::from_str(
        r##"{"status": "found", "origin": "locale", "markdown": "# Demo\n\nBody", "path": "/tmp/x"}"##,
    )
    .unwrap();
    assert_eq!(found.status, "found");
    assert_eq!(found.origin.as_deref(), Some("locale"));
    assert_eq!(found.markdown.as_deref(), Some("# Demo\n\nBody"));

    let none: PluginBundleDetail = serde_json::from_str(r#"{"status": "none"}"#).unwrap();
    assert_eq!(none.status, "none");
    assert!(none.markdown.is_none());
}

#[test]
fn test_plugin_registries_and_probe_shapes() {
    let registries: PluginRegistriesResult = serde_json::from_str(
        r#"{"result": {"registry": null, "fallbackRegistries": ["https://registry.npmmirror.com/"], "resolved": "https://registry.npmjs.org/"}}"#,
    )
    .unwrap();
    assert!(registries.result.registry.is_none());
    assert_eq!(registries.result.fallback_registries.len(), 1);
    assert_eq!(
        registries.result.resolved.as_deref(),
        Some("https://registry.npmjs.org/")
    );

    let probe: PluginRegistryProbeResult =
        serde_json::from_str(r#"{"result": "https://registry.npmmirror.com/"}"#).unwrap();
    assert_eq!(
        probe.result.as_deref(),
        Some("https://registry.npmmirror.com/")
    );
    let probe_null: PluginRegistryProbeResult =
        serde_json::from_str(r#"{"result": null}"#).unwrap();
    assert!(probe_null.result.is_none());
}

#[test]
fn test_plugin_lifecycle_notifications_parse() {
    let changed = serde_json::json!({"reason": "bundle"});
    match IncomingNotification::from_raw(NOTIFICATION_PLUGIN_CHANGED, changed) {
        IncomingNotification::PluginChanged(p) => assert_eq!(p.reason, "bundle"),
        other => panic!("unexpected notification: {:?}", other),
    }
    let update = serde_json::json!({"requestId": "req-9", "step": "removing"});
    match IncomingNotification::from_raw(NOTIFICATION_PLUGIN_UPDATE_STATE, update) {
        IncomingNotification::PluginUpdateState(p) => {
            assert_eq!(p.request_id, "req-9");
            assert_eq!(p.step, "removing");
        }
        other => panic!("unexpected notification: {:?}", other),
    }
    let market = serde_json::json!({"changed": true});
    match IncomingNotification::from_raw(NOTIFICATION_PLUGIN_MARKET_CHANGED, market) {
        IncomingNotification::PluginMarketChanged(p) => assert!(p.changed),
        other => panic!("unexpected notification: {:?}", other),
    }
}

#[test]
fn test_plugin_market_indexes_parse() {
    let indexes: PluginMarketListIndexesResult = serde_json::from_str(
        r#"{"result": [{"url": "https://market.example/index.json", "title": "Example"}]}"#,
    )
    .unwrap();
    assert_eq!(indexes.result.len(), 1);
    assert_eq!(
        indexes.result[0].url,
        "https://market.example/index.json"
    );
}

#[test]
fn test_plugin_spec_inspection_parse() {
    let accepted: PluginSpecInspection = serde_json::from_str(
        r#"{"status": "accepted", "kind": "registry", "name": "dsh-plugin-whale-pet", "version": "1.0.0", "description": "Whale pet", "bundle": true, "registry": "https://registry.npmjs.org/"}"#,
    )
    .unwrap();
    match accepted {
        PluginSpecInspection::Accepted {
            kind,
            name,
            version,
            bundle,
            ..
        } => {
            assert_eq!(kind, "registry");
            assert_eq!(name.as_deref(), Some("dsh-plugin-whale-pet"));
            assert_eq!(version.as_deref(), Some("1.0.0"));
            assert_eq!(bundle, Some(true));
        }
        PluginSpecInspection::Refused { .. } => panic!("expected accepted"),
    }

    let refused: PluginSpecInspection = serde_json::from_str(
        r#"{"status": "refused", "problem": "network", "reason": "fetch failed", "registries": ["https://registry.npmjs.org/", "https://registry.npmmirror.com/"]}"#,
    )
    .unwrap();
    match refused {
        PluginSpecInspection::Refused {
            problem,
            reason,
            registries,
        } => {
            assert_eq!(problem, "network");
            assert_eq!(reason, "fetch failed");
            assert_eq!(registries.unwrap().len(), 2);
        }
        PluginSpecInspection::Accepted { .. } => panic!("expected refused"),
    }
}

#[test]
fn test_plugin_versions_and_update_result_parse() {
    let listed: PluginVersionList = serde_json::from_str(
        r#"{
            "status": "listed",
            "installed": "1.0.0",
            "registry": "https://registry.npmjs.org/",
            "attempts": [{"registry": "https://registry.npmjs.org/"}],
            "versions": [
                {
                    "version": "1.1.0",
                    "publishedAt": "2026-05-01T00:00:00.000Z",
                    "tags": ["latest"],
                    "prerelease": false,
                    "bundle": true
                },
                {
                    "version": "2.0.0-beta.1",
                    "tags": ["next"],
                    "prerelease": true,
                    "bundle": true
                }
            ],
            "omitted": 0
        }"#,
    )
    .unwrap();
    match listed {
        PluginVersionList::Listed {
            installed,
            versions,
            ..
        } => {
            assert_eq!(installed, "1.0.0");
            assert_eq!(versions.len(), 2);
            assert_eq!(versions[0].version, "1.1.0");
            assert!(!versions[0].prerelease);
            assert!(versions[1].prerelease);
        }
        PluginVersionList::Refused { .. } => panic!("expected listed"),
    }

    let update_res: PluginUpdateResult = serde_json::from_str(
        r#"{
            "from": "1.0.0",
            "to": "1.1.0",
            "step": "install",
            "result": {
                "changed": true,
                "application": "applied",
                "stage": "install",
                "target": "dsh-plugin-whale-pet"
            }
        }"#,
    )
    .unwrap();
    assert_eq!(update_res.from, "1.0.0");
    assert_eq!(update_res.to, "1.1.0");
    assert_eq!(update_res.step, "install");
    assert_eq!(update_res.result.application, "applied");
    assert!(update_res.restore.is_none());

    let cancel: PluginCancellationResult =
        serde_json::from_str(r#"{"status": "cancelled"}"#).unwrap();
    assert_eq!(cancel.status, "cancelled");
}

#[test]
fn test_plugin_market_rpc_param_shapes() {
    let params = PluginMarketSetIndexesParams {
        urls: vec!["https://market.example/index.json".to_string()],
    };
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["urls"][0], "https://market.example/index.json");

    let search = PluginMarketSearchParams {
        url: "https://market.example/index.json".to_string(),
        query: MarketQuery {
            text: Some("whale".to_string()),
            category: Some("tools".to_string()),
            labels: Some(vec!["official".to_string()]),
            sort: MarketSort::Key {
                id: "stars".to_string(),
                direction: MarketSortDirection::Desc,
            },
            offset: 50,
            limit: 50,
            refresh: Some(true),
            fetched_at: Some(1234),
        },
    };
    let json = serde_json::to_value(&search).unwrap();
    assert_eq!(json["url"], "https://market.example/index.json");
    assert_eq!(json["query"]["text"], "whale");
    assert_eq!(json["query"]["category"], "tools");
    assert_eq!(json["query"]["labels"][0], "official");
    assert_eq!(
        json["query"]["sort"],
        serde_json::json!({"kind": "key", "id": "stars", "direction": "desc"})
    );
    assert_eq!(json["query"]["offset"], 50);
    assert_eq!(json["query"]["limit"], 50);
    assert_eq!(json["query"]["refresh"], true);
    assert_eq!(json["query"]["fetchedAt"], 1234);

    let detail = PluginMarketReadEntryDetailParams {
        url: "https://market.example/index.json".to_string(),
        name: "@community/whale-pet".to_string(),
        version: "1.1.0".to_string(),
        language: Some("zh".to_string()),
        refresh: Some(true),
        npm_file_sources: Some(vec!["jsdelivr".to_string(), "npmmirror".to_string()]),
    };
    let json = serde_json::to_value(&detail).unwrap();
    assert_eq!(json["url"], "https://market.example/index.json");
    assert_eq!(json["name"], "@community/whale-pet");
    assert_eq!(json["version"], "1.1.0");
    assert_eq!(json["language"], "zh");
    assert_eq!(json["refresh"], true);
    assert_eq!(json["npmFileSources"][1], "npmmirror");

    let bare = PluginMarketReadEntryDetailParams {
        url: "u".to_string(),
        name: "n".to_string(),
        version: "v".to_string(),
        language: None,
        refresh: None,
        npm_file_sources: None,
    };
    let json = serde_json::to_value(&bare).unwrap();
    assert!(json.get("language").is_none());
    assert!(json.get("refresh").is_none());
    assert!(json.get("npmFileSources").is_none());
}

#[test]
fn test_plugin_market_list_indexes_result() {
    let result: PluginMarketListIndexesResult = serde_json::from_value(serde_json::json!({
        "result": [
            {
                "url": "https://market.example/index.json",
                "title": {"en": "Example", "zh": "示例市场"},
                "description": {"zh": "社区插件索引"}
            },
            {
                "url": "https://other.example/index.json",
                "error": "Could not fetch https://other.example/index.json: net down"
            }
        ]
    }))
    .unwrap();
    assert_eq!(result.result.len(), 2);
    assert_eq!(result.result[0].url, "https://market.example/index.json");
    assert_eq!(result.result[0].title.as_ref().unwrap()["zh"], "示例市场");
    assert!(result.result[1].title.is_none());
    assert!(result.result[1].error.as_deref().unwrap().contains("Could not fetch"));
}

#[test]
fn test_plugin_market_search_result() {
    let result: PluginMarketSearchResult = serde_json::from_value(serde_json::json!({
        "result": {
            "url": "https://market.example/index.json",
            "title": {"zh": "示例市场"},
            "description": {"zh": "社区插件索引"},
            "metadata": {
                "categories": {"tools": {"title": {"zh": "工具"}}},
                "labels": {"official": {"title": {"zh": "官方"}}},
                "sortKeys": {"stars": {"title": {"zh": "星标"}, "order": "desc"}}
            },
            "fetchedAt": 1762419200123_u64,
            "entries": [{
                "name": "@community/whale-pet",
                "title": {"zh": "鲸鱼宠物"},
                "description": {"zh": "一只鲸鱼宠物。"},
                "icon": "data:image/png;base64,AA==",
                "author": "arcat",
                "homepage": "https://example.com/pet",
                "repository": {"url": "https://github.com/community/whale-pet", "directory": "packages/pet"},
                "metadata": {"category": "tools", "keywords": ["pet"], "labels": ["official"], "sortKeys": {"stars": 5}},
                "versions": [
                    {
                        "version": "1.1.0",
                        "publishTimestamp": 1762419200,
                        "source": {"type": "npm", "registry": "https://registry.npmmirror.com/"},
                        "engines": {"dsh": "^0.4"},
                        "install": {"spec": "@community/whale-pet@1.1.0"},
                        "compatibility": {"compatible": true}
                    },
                    {
                        "version": "1.0.0-beta.1",
                        "publishTimestamp": 1700000000,
                        "source": {"type": "git", "url": "https://github.com/community/whale-pet", "commit": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2", "path": "packages/pet"},
                        "engines": {},
                        "install": {"spec": "git+https://github.com/community/whale-pet#a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"},
                        "compatibility": {"compatible": false, "runtimeVersion": "0.4.0", "requirements": {"dsh": "^0.4"}, "exempted": true}
                    }
                ],
                "defaultVersion": "1.1.0",
                "installation": {"status": "update-available", "installedVersion": "1.0.0"}
            }],
            "total": 51,
            "nextOffset": 50
        }
    }))
    .unwrap();
    let page = result.result;
    assert_eq!(page.url, "https://market.example/index.json");
    assert_eq!(page.fetched_at, 1762419200123);
    assert_eq!(page.total, 51);
    assert_eq!(page.next_offset, Some(50));
    assert_eq!(page.metadata.sort_keys["stars"].order, MarketSortDirection::Desc);
    let entry = &page.entries[0];
    assert_eq!(entry.name, "@community/whale-pet");
    assert_eq!(entry.installation.status, "update-available");
    assert_eq!(entry.installation.installed_version.as_deref(), Some("1.0.0"));
    assert_eq!(entry.default_version.as_deref(), Some("1.1.0"));
    assert_eq!(entry.metadata.category.as_deref(), Some("tools"));
    assert_eq!(entry.metadata.labels, vec!["official".to_string()]);
    assert_eq!(entry.repository.as_ref().unwrap().directory.as_deref(), Some("packages/pet"));
    match &entry.versions[0].source {
        MarketSource::Npm { registry } => {
            assert_eq!(registry.as_deref(), Some("https://registry.npmmirror.com/"));
        }
        other => panic!("expected npm source: {other:?}"),
    }
    assert!(entry.versions[0].compatibility.compatible);
    assert!(entry.versions[0].compatibility.requirements.is_empty());
    match &entry.versions[1].source {
        MarketSource::Git { url, commit, path } => {
            assert_eq!(url, "https://github.com/community/whale-pet");
            assert_eq!(commit, "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2");
            assert_eq!(path.as_deref(), Some("packages/pet"));
        }
        other => panic!("expected git source: {other:?}"),
    }
    let compatibility = &entry.versions[1].compatibility;
    assert!(!compatibility.compatible);
    assert!(compatibility.exempted);
    assert_eq!(compatibility.runtime_version.as_deref(), Some("0.4.0"));
    assert_eq!(compatibility.requirements["dsh"], "^0.4");
    assert_eq!(entry.versions[1].install.spec, "git+https://github.com/community/whale-pet#a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2");
}

#[test]
fn test_plugin_market_read_entry_detail_result() {
    let found: PluginMarketReadEntryDetailResult = serde_json::from_value(serde_json::json!({
        "result": {
            "status": "found",
            "origin": "readme",
            "markdown": "# Whale Pet\n\nA pet.",
            "document": "doc-1"
        }
    }))
    .unwrap();
    assert_eq!(found.result.status, "found");
    assert_eq!(found.result.origin.as_deref(), Some("readme"));
    assert_eq!(found.result.markdown.as_deref(), Some("# Whale Pet\n\nA pet."));
    assert_eq!(found.result.document.as_deref(), Some("doc-1"));

    let none: PluginMarketReadEntryDetailResult =
        serde_json::from_value(serde_json::json!({"result": {"status": "none"}})).unwrap();
    assert_eq!(none.result.status, "none");
    assert!(none.result.markdown.is_none());
    assert!(none.result.origin.is_none());
    assert!(none.result.document.is_none());
}

#[test]
fn test_plugin_market_changed_notification() {
    let raw = serde_json::json!({"changed": true});
    match IncomingNotification::from_raw(NOTIFICATION_PLUGIN_MARKET_CHANGED, raw) {
        IncomingNotification::PluginMarketChanged(params) => assert!(params.changed),
        other => panic!("unexpected notification: {other:?}"),
    }
}

#[test]
fn test_settings_descriptor_parses_camel_case() {
    let raw = serde_json::json!({
        "ns": "agent-loop",
        "autoGenerate": true,
        "schema": {"type": "object"},
        "value": {"maxParallelToolCalls": 4},
        "revision": 3
    });
    let descriptor: SettingsDescriptor = serde_json::from_value(raw).expect("descriptor parses");
    assert_eq!(descriptor.ns, "agent-loop");
    assert!(descriptor.auto_generate);
    assert_eq!(descriptor.revision, 3);
}

#[test]
fn test_settings_mutate_params_serialize_camel_case_revision() {
    let params = SettingsMutateParams {
        namespace: Some("agent-loop".into()),
        patch: serde_json::json!({"maxParallelToolCalls": 2}),
        expected_revision: Some(7),
    };
    let value = serde_json::to_value(&params).expect("params serialize");
    assert_eq!(value["expectedRevision"], 7);
    assert_eq!(value["namespace"], "agent-loop");
}

#[test]
fn test_schedule_params_wire_shapes_camel_case() {
    let delete = ScheduleDeleteParams {
        id: "task-1".to_string(),
        expected_updated_at: Some("2026-09-19T15:51:00Z".to_string()),
    };
    let value = serde_json::to_value(&delete).unwrap();
    assert_eq!(value["expectedUpdatedAt"], "2026-09-19T15:51:00Z");
    assert!(value.get("expected_updated_at").is_none());

    let edit = ScheduleEditParams {
        id: "task-1".to_string(),
        action: "delete".to_string(),
        patch: None,
        expected_updated_at: Some("2026-09-19T15:51:00Z".to_string()),
    };
    let value = serde_json::to_value(&edit).unwrap();
    assert_eq!(value["expectedUpdatedAt"], "2026-09-19T15:51:00Z");

    let run: serde_json::Value = serde_json::to_value(ScheduleRunParams {
        id: "task-1".to_string(),
    })
    .unwrap();
    assert_eq!(run["id"], "task-1");

    let history: serde_json::Value = serde_json::to_value(ScheduleHistoryParams {
        id: "task-1".to_string(),
        limit: Some(20),
        before: Some("msg-9".to_string()),
    })
    .unwrap();
    assert_eq!(history["id"], "task-1");
    assert_eq!(history["limit"], 20);
    assert_eq!(history["before"], "msg-9");

    let spec: serde_json::Value = serde_json::to_value(ScheduleSessionCreateSpec {
        cwd: Some("/tmp/project".to_string()),
        agent_preset: Some("cordis".to_string()),
    })
    .unwrap();
    assert_eq!(spec["agentPreset"], "cordis");
    assert_eq!(spec["cwd"], "/tmp/project");
    assert!(spec.get("agent_preset").is_none());

    let target: serde_json::Value = serde_json::to_value(ScheduleTargetRequest::Session {
        session_id: "sess-1".to_string(),
    })
    .unwrap();
    assert_eq!(target["sessionId"], "sess-1");
    assert_eq!(target["kind"], "session");
}

#[test]
fn test_schedule_timing_input_keeps_host_snake_case_fields() {
    let after: serde_json::Value = serde_json::to_value(ScheduleTiming::After {
        after_seconds: 3600,
    })
    .unwrap();
    assert_eq!(after["after_seconds"], 3600);

    let daily: serde_json::Value = serde_json::to_value(ScheduleTiming::Daily {
        daily: ScheduleDailyInput {
            time: "08:30:00".to_string(),
            time_zone: "Asia/Shanghai".to_string(),
        },
    })
    .unwrap();
    assert_eq!(daily["daily"]["time_zone"], "Asia/Shanghai");
}

#[test]
fn test_schedule_history_outcome_parses_host_shapes() {
    let page: ScheduleHistoryResult = serde_json::from_value(serde_json::json!({
        "result": {
            "id": "task-1",
            "records": [{
                "scheduledAt": "2026-09-19T15:51:00Z",
                "deliveredAt": "2026-09-19T15:51:01Z",
                "messageId": "msg-1",
                "targetSessionId": "sess-1"
            }],
            "earlierRecordsUnavailable": false,
            "earlierRecordsPruned": false,
            "retention": {"days": 30, "records": 200},
            "nextBefore": "msg-1"
        }
    }))
    .unwrap();
    match page.result {
        ScheduleHistoryOutcome::Page(p) => {
            assert_eq!(p.records.len(), 1);
            assert_eq!(p.retention.days, 30);
        }
        other => panic!("expected page, got {other:?}"),
    }

    let miss: ScheduleHistoryResult = serde_json::from_value(serde_json::json!({
        "result": {"id": "task-1", "code": "delivery_cursor_not_found"}
    }))
    .unwrap();
    match miss.result {
        ScheduleHistoryOutcome::Miss { code } => assert_eq!(code, "delivery_cursor_not_found"),
        other => panic!("expected miss, got {other:?}"),
    }
}

#[test]
fn test_plugin_request_id_params_serialize_camel_case() {
    let update = PluginUpdateParams {
        name: "@deepseek-ai/dsh-base".to_string(),
        version: "0.0.2".to_string(),
        request_id: Some("req-1".to_string()),
        registry: None,
    };
    let value = serde_json::to_value(&update).unwrap();
    assert_eq!(value["requestId"], "req-1");
    assert!(value.get("request_id").is_none());

    let state: PluginInstallStateNotificationParams = serde_json::from_value(serde_json::json!({
        "requestId": "req-2",
        "phase": "applying"
    }))
    .unwrap();
    assert_eq!(state.request_id, "req-2");
    assert_eq!(state.phase, "applying");

    let update_state: PluginUpdateStateNotificationParams = serde_json::from_value(
        serde_json::json!({"requestId": "req-3", "step": "checking"}),
    )
    .unwrap();
    assert_eq!(update_state.request_id, "req-3");
    assert_eq!(update_state.step, "checking");
}
