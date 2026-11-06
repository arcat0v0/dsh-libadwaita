use dsh_libadwaita::protocol::*;
use dsh_libadwaita::supervisor::*;
use std::path::PathBuf;
use std::time::Duration;

struct MockLlmServer {
    port: u16,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockLlmServer {
    async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => {
                        break;
                    }
                    accept_res = listener.accept() => {
                        if let Ok((mut stream, _)) = accept_res {
                            tokio::spawn(async move {
                                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                                let mut buf = [0u8; 4096];
                                let _ = stream.read(&mut buf).await;
                                let sse_body = concat!(
                                    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_mock_1\",\"type\":\"message\",\"role\":\"assistant\",\"content\":[],\"model\":\"deepseek-chat\",\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n\n",
                                    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
                                    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello from mock endpoint!\"}}\n\n",
                                    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
                                    "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":5}}\n\n",
                                    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
                                );
                                let resp = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                                    sse_body.len(),
                                    sse_body
                                );
                                let _ = stream.write_all(resp.as_bytes()).await;
                                let _ = stream.flush().await;
                            });
                        }
                    }
                }
            }
        });

        Self {
            port,
            shutdown_tx: Some(shutdown_tx),
        }
    }

    fn close(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for MockLlmServer {
    fn drop(&mut self) {
        self.close();
    }
}

#[tokio::test]
async fn test_real_dsh_lifecycle_and_session_create() {
    let mut mock_server = MockLlmServer::start().await;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap();
    let bridge_dir = repo_root.join("bridge");
    let dsh_bin = bridge_dir.join("node_modules/@deepseek-ai/dsh/lib/bin.js");
    assert!(dsh_bin.exists());

    let temp = tempfile::tempdir().unwrap();
    let dsh_home = temp.path().join("dsh-home");
    let runtime_dir = temp.path().join("runtime");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    let profile_config = ProfileTemplateConfig {
        profile_name: "libadwaita".to_string(),
        bridge_bundle_path_or_url: format!("file:{}", bridge_dir.display()),
        disabled_rows: vec!["hmr".to_string(), "credentials".to_string()],
        llm_base_url: Some(format!("http://127.0.0.1:{}/anthropic", mock_server.port)),
        schema_version: 1,
    };

    let config = SupervisorConfig {
        dsh_home: dsh_home.clone(),
        runtime_dir: runtime_dir.clone(),
        dsh_bin: dsh_bin.display().to_string(),
        client_version: "0.1.0".to_string(),
        profile_config,
        handshake_timeout: Duration::from_secs(20),
        shutdown_timeout: Duration::from_secs(5),
    };

    let supervisor = HarnessSupervisor::new(config);

    let init_result = supervisor.start().await.unwrap();
    assert_eq!(init_result.protocol_version, 1);
    assert_eq!(init_result.profile_name, "libadwaita");
    assert_eq!(init_result.server_info.name, "dsh-libadwaita-bridge");
    assert!(!init_result.generation_id.is_empty());
    assert_eq!(supervisor.state().await, SupervisorState::Ready);
    assert_eq!(
        supervisor.generation_id().await,
        Some(init_result.generation_id.clone())
    );
    assert_eq!(supervisor.stdout_purity_violations(), 0);

    let conn = supervisor.connection().await.unwrap();

    let create_res: SessionCreateResult = conn
        .call(
            METHOD_SESSION_CREATE,
            SessionCreateParams {
                session_id: None,
                cwd: workspace.display().to_string(),
                provider: None,
                model: None,
                agent_preset: None,
            },
            Duration::from_secs(10),
        )
        .await
        .unwrap();

    assert!(!create_res.session_id.is_empty());
    assert_eq!(create_res.header.cwd, workspace.display().to_string());

    let lock_path = dsh_home.join(".instance.lock");

    supervisor.stop().await.unwrap();
    assert_eq!(supervisor.state().await, SupervisorState::Stopped);
    assert!(!lock_path.exists());

    let init_result2 = supervisor.start().await.unwrap();
    assert_eq!(init_result2.protocol_version, 1);
    assert_ne!(init_result2.generation_id, init_result.generation_id);
    assert_eq!(
        supervisor.generation_id().await,
        Some(init_result2.generation_id.clone())
    );
    assert_eq!(supervisor.state().await, SupervisorState::Ready);

    supervisor.stop().await.unwrap();
    assert_eq!(supervisor.state().await, SupervisorState::Stopped);
    assert!(!lock_path.exists());

    mock_server.close();
}
