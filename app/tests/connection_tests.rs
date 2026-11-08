use dsh_libadwaita::connection::*;
use dsh_libadwaita::protocol::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_codec_frame_splitting_and_partial_reads() {
    let mut codec = NdjsonCodec::new(1024);

    let partial1 = b"{\"jsonrpc\":\"2.0\",\"id\":1";
    let frames1 = codec.push_bytes(partial1).unwrap();
    assert_eq!(frames1.len(), 0);

    let partial2 = b",\"result\":true}\n{\"jsonrpc\":\"2.0\",\"method\":\"test\"}\n";
    let frames2 = codec.push_bytes(partial2).unwrap();
    assert_eq!(frames2.len(), 2);
    assert_eq!(frames2[0], "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":true}");
    assert_eq!(frames2[1], "{\"jsonrpc\":\"2.0\",\"method\":\"test\"}");
}

#[test]
fn test_codec_max_frame_guard() {
    let mut codec = NdjsonCodec::new(20);
    let large = b"1234567890123456789012345";
    let err = codec.push_bytes(large).unwrap_err();
    match err {
        ConnectionError::MaxFrameExceeded(limit) => assert_eq!(limit, 20),
        _ => panic!("expected MaxFrameExceeded"),
    }
}

#[tokio::test]
async fn test_request_correlation_out_of_order() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    let server_task = tokio::spawn(async move {
        let mut codec = NdjsonCodec::new(1024);
        let mut buf = vec![0u8; 1024];
        let mut requests = Vec::new();

        while requests.len() < 2 {
            let n = server_io.read(&mut buf).await.unwrap();
            let frames = codec.push_bytes(&buf[..n]).unwrap();
            for f in frames {
                let parsed: JsonRpcRequest = serde_json::from_str(&f).unwrap();
                requests.push(parsed);
            }
        }

        let resp2 = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"val\":2}}}}\n",
            serde_json::to_string(&requests[1].id).unwrap()
        );
        server_io.write_all(resp2.as_bytes()).await.unwrap();

        let resp1 = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"result\":{{\"val\":1}}}}\n",
            serde_json::to_string(&requests[0].id).unwrap()
        );
        server_io.write_all(resp1.as_bytes()).await.unwrap();
    });

    let conn1 = conn.clone();
    let h1 = tokio::spawn(async move {
        conn1
            .call::<serde_json::Value, serde_json::Value>("m1", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap()
    });

    let conn2 = conn.clone();
    let h2 = tokio::spawn(async move {
        conn2
            .call::<serde_json::Value, serde_json::Value>("m2", serde_json::json!({}), Duration::from_secs(2))
            .await
            .unwrap()
    });

    let (res1, res2) = tokio::join!(h1, h2);
    assert_eq!(res1.unwrap()["val"], 1);
    assert_eq!(res2.unwrap()["val"], 2);

    server_task.await.unwrap();
}

#[tokio::test]
async fn test_request_timeout_mapping() {
    let (client_io, _server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    let err = conn
        .call::<serde_json::Value, serde_json::Value>("test", serde_json::json!({}), Duration::from_millis(50))
        .await
        .unwrap_err();

    assert_eq!(err, ConnectionError::Timeout);
}

#[tokio::test]
async fn test_remote_error_mapping() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    tokio::spawn(async move {
        let mut buf = vec![0u8; 1024];
        let n = server_io.read(&mut buf).await.unwrap();
        let req_str = String::from_utf8_lossy(&buf[..n]).to_string();
        let parsed: JsonRpcRequest = serde_json::from_str(req_str.trim()).unwrap();

        let resp = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"error\":{{\"code\":{},\"message\":\"session not found\",\"data\":{{\"sessionId\":\"s1\"}}}}}}\n",
            serde_json::to_string(&parsed.id).unwrap(),
            ERR_SESSION_NOT_FOUND
        );
        server_io.write_all(resp.as_bytes()).await.unwrap();
    });

    let err = conn
        .call::<serde_json::Value, serde_json::Value>("test", serde_json::json!({}), Duration::from_secs(2))
        .await
        .unwrap_err();

    match err {
        ConnectionError::Remote { code, message, data } => {
            assert_eq!(code, ERR_SESSION_NOT_FOUND);
            assert_eq!(message, "session not found");
            assert_eq!(data.unwrap()["sessionId"], "s1");
        }
        _ => panic!("expected Remote error"),
    }
}

#[tokio::test]
async fn test_fail_closed_approval_when_no_responder() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let _conn = BridgeConnection::from_stream(client_io);

    let req = r#"{"jsonrpc":"2.0","id":"srv-1","method":"approval/request","params":{"sessionId":"s1","toolName":"bash","callId":"c1"}}"#;
    server_io.write_all(format!("{}\n", req).as_bytes()).await.unwrap();

    let mut buf = vec![0u8; 1024];
    let n = server_io.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]).to_string();
    let resp: JsonRpcResponse<ApprovalRequestResult> = serde_json::from_str(resp_str.trim()).unwrap();

    assert_eq!(resp.id, RequestId::from("srv-1"));
    assert_eq!(resp.result.unwrap().outcome, ApprovalOutcome::Rejected);
}

#[tokio::test]
async fn test_slow_approval_responder_is_not_cancelled() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    conn.set_approval_responder(|_req: ApprovalRequestParams| async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        ApprovalOutcome::AllowedOnce
    });

    let req = r#"{"jsonrpc":"2.0","id":"srv-2","method":"approval/request","params":{"sessionId":"s1","toolName":"bash","callId":"c2"}}"#;
    server_io.write_all(format!("{}\n", req).as_bytes()).await.unwrap();

    let mut buf = vec![0u8; 1024];
    let n = server_io.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]).to_string();
    let resp: JsonRpcResponse<ApprovalRequestResult> = serde_json::from_str(resp_str.trim()).unwrap();

    assert_eq!(resp.id, RequestId::from("srv-2"));
    assert_eq!(resp.result.unwrap().outcome, ApprovalOutcome::AllowedOnce);
}

#[tokio::test]
async fn test_slow_question_responder_is_not_cancelled() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    conn.set_user_question_responder(|_req: SdkUserQuestionRequest| async {
        tokio::time::sleep(Duration::from_millis(200)).await;
        SdkUserQuestionResult::answers(vec![UserQuestionAnswer {
            id: "q1".to_string(),
            selected: vec!["A".to_string()],
        }])
    });

    let req = r#"{"jsonrpc":"2.0","id":"srv-3","method":"user-question/ask","params":{"requestId":"r1","sessionId":"s1","questions":[{"id":"q1","question":"Pick","options":[{"label":"A"}]}]}}"#;
    server_io.write_all(format!("{}\n", req).as_bytes()).await.unwrap();

    let mut buf = vec![0u8; 1024];
    let n = server_io.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]).to_string();
    let resp: JsonRpcResponse<SdkUserQuestionResult> =
        serde_json::from_str(resp_str.trim()).unwrap();

    assert_eq!(resp.id, RequestId::from("srv-3"));
    let result = resp.result.unwrap();
    assert_eq!(result.answers.unwrap()[0].selected, vec!["A".to_string()]);
}

#[tokio::test]
async fn test_unknown_notification_tolerated() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);
    let mut notif_rx = conn.subscribe_notifications();

    let unknown_line = "{\"jsonrpc\":\"2.0\",\"method\":\"future.extension\",\"params\":{\"magic\":123}}\n";
    server_io.write_all(unknown_line.as_bytes()).await.unwrap();

    let received = tokio::time::timeout(Duration::from_secs(1), notif_rx.recv())
        .await
        .unwrap()
        .unwrap();

    match received {
        IncomingNotification::Unknown { method, params } => {
            assert_eq!(method, "future.extension");
            assert_eq!(params["magic"], 123);
        }
        _ => panic!("expected unknown notification"),
    }
}

#[tokio::test]
async fn test_fail_closed_user_question_when_no_responder() {
    let (client_io, mut server_io) = tokio::io::duplex(4096);
    let _conn = BridgeConnection::from_stream(client_io);

    let req = r#"{"jsonrpc":"2.0","id":"srv-q","method":"user-question/ask","params":{"requestId":"q1","sessionId":"s1","questions":[]}}"#;
    server_io.write_all(format!("{}\n", req).as_bytes()).await.unwrap();

    let mut buf = vec![0u8; 1024];
    let n = server_io.read(&mut buf).await.unwrap();
    let resp_str = String::from_utf8_lossy(&buf[..n]).to_string();
    let resp: JsonRpcResponse<SdkUserQuestionResult> = serde_json::from_str(resp_str.trim()).unwrap();

    assert_eq!(resp.id, RequestId::from("srv-q"));
    assert_eq!(resp.result.unwrap().cancelled, Some(true));
}

#[tokio::test]
async fn test_clean_shutdown_cancels_pending_requests() {
    let (client_io, _server_io) = tokio::io::duplex(4096);
    let conn = BridgeConnection::from_stream(client_io);

    let conn_clone = conn.clone();
    let call_fut = tokio::spawn(async move {
        conn_clone
            .call::<serde_json::Value, serde_json::Value>("hanging_method", serde_json::json!({}), Duration::from_secs(10))
            .await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    conn.close().await.unwrap();
    assert_eq!(conn.state(), ConnectionState::Closed);

    let call_res = call_fut.await.unwrap();
    assert_eq!(call_res.unwrap_err(), ConnectionError::Closed);
}
