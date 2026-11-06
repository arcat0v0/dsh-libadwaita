use dsh_libadwaita::supervisor::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn test_profile_generation_package_json_and_cordis_patch() {
    let cfg = ProfileTemplateConfig {
        profile_name: "libadwaita".to_string(),
        bridge_bundle_path_or_url: "file:///usr/lib/dsh/bridge".to_string(),
        disabled_rows: vec!["hmr".to_string(), "credentials".to_string()],
        llm_base_url: Some("http://127.0.0.1:4000/anthropic".to_string()),
        schema_version: 1,
    };

    let pkg_json = generate_profile_package_json(&cfg);
    let parsed: serde_json::Value = serde_json::from_str(&pkg_json).unwrap();
    let bundles = parsed["dsh"]["profile"]["bundles"].as_array().unwrap();
    assert_eq!(bundles.len(), 2);
    assert_eq!(bundles[0], "@deepseek-ai/dsh-base");
    assert_eq!(bundles[1], "@dsh-libadwaita/bridge");
    assert_eq!(
        parsed["dependencies"]["@dsh-libadwaita/bridge"],
        "file:///usr/lib/dsh/bridge"
    );
    assert_eq!(parsed["managedProfile"]["schemaVersion"], 1);

    let patch_yml = generate_cordis_patch_yml(&cfg);
    assert!(patch_yml.contains("- id: hmr"));
    assert!(patch_yml.contains("name: '@deepseek-ai/dsh-hmr'"));
    assert!(patch_yml.contains("root: []"));
    assert!(patch_yml.contains("- id: credentials"));
    assert!(patch_yml.contains("name: '@deepseek-ai/dsh-credentials-local'"));
    assert!(patch_yml.contains("- id: llm-deepseek"));
    assert!(patch_yml.contains("baseURL: http://127.0.0.1:4000/anthropic"));
}

#[test]
fn test_profile_ensure_and_version_upgrade() {
    let temp_dir = std::env::temp_dir().join(format!("dsh_test_profile_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cfg_v1 = ProfileTemplateConfig {
        profile_name: "libadwaita".to_string(),
        bridge_bundle_path_or_url: "file:///opt/bridge-v1".to_string(),
        disabled_rows: vec!["hmr".to_string(), "credentials".to_string()],
        llm_base_url: None,
        schema_version: 1,
    };

    ensure_managed_profile(&temp_dir, &cfg_v1).unwrap();
    let pkg_path = temp_dir.join("profiles/libadwaita/package.json");
    let patch_path = temp_dir.join("profiles/libadwaita/cordis.patch.yml");
    assert!(pkg_path.exists());
    assert!(patch_path.exists());
    let symlink_path = temp_dir.join("profiles/libadwaita/node_modules/@dsh-libadwaita/bridge");
    assert!(symlink_path.is_symlink());

    let content_v1 = std::fs::read_to_string(&pkg_path).unwrap();
    assert!(content_v1.contains("bridge-v1"));
    let cfg_v2 = ProfileTemplateConfig {
        profile_name: "libadwaita".to_string(),
        bridge_bundle_path_or_url: "file:///opt/bridge-v2".to_string(),
        disabled_rows: vec!["hmr".to_string(), "credentials".to_string()],
        llm_base_url: None,
        schema_version: 2,
    };
    ensure_managed_profile(&temp_dir, &cfg_v2).unwrap();
    let content_v2 = std::fs::read_to_string(&pkg_path).unwrap();
    assert!(content_v2.contains("bridge-v2"));
    assert!(content_v2.contains("\"schemaVersion\": 2"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_socket_permission_bits_0600() {
    let temp_dir = std::env::temp_dir().join(format!("dsh_test_sock_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let sock_path = temp_dir.join("test.sock");
    std::fs::write(&sock_path, b"test").unwrap();

    set_socket_permissions_0600(&sock_path).unwrap();

    let metadata = std::fs::metadata(&sock_path).unwrap();
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_supervisor_state_machine_transitions() {
    let mut sm = SupervisorStateMachine::new();
    assert_eq!(sm.state(), SupervisorState::Uninitialized);

    sm.transition(SupervisorState::Starting).unwrap();
    assert_eq!(sm.state(), SupervisorState::Starting);

    sm.transition(SupervisorState::Ready).unwrap();
    assert_eq!(sm.state(), SupervisorState::Ready);

    let err = sm.transition(SupervisorState::Starting).unwrap_err();
    match err {
        SupervisorError::IllegalStateTransition { from, to } => {
            assert_eq!(from, SupervisorState::Ready);
            assert_eq!(to, SupervisorState::Starting);
        }
        _ => panic!("expected IllegalStateTransition"),
    }

    sm.transition(SupervisorState::Stopping).unwrap();
    assert_eq!(sm.state(), SupervisorState::Stopping);

    sm.transition(SupervisorState::Stopped).unwrap();
    assert_eq!(sm.state(), SupervisorState::Stopped);

    sm.transition(SupervisorState::Starting).unwrap();
    assert_eq!(sm.state(), SupervisorState::Starting);

    sm.transition(SupervisorState::Failed).unwrap();
    assert_eq!(sm.state(), SupervisorState::Failed);
}

#[test]
fn test_generation_id_tracking() {
    let mut tracker = GenerationTracker::new();
    assert_eq!(tracker.current_generation(), None);

    let changed1 = tracker.record_generation("gen-1".to_string());
    assert!(changed1);
    assert_eq!(tracker.current_generation(), Some("gen-1"));

    let changed2 = tracker.record_generation("gen-1".to_string());
    assert!(!changed2);

    let changed3 = tracker.record_generation("gen-2".to_string());
    assert!(changed3);
    assert_eq!(tracker.current_generation(), Some("gen-2"));
}

#[test]
fn test_instance_lock_prevents_dual_running() {
    let temp_dir = std::env::temp_dir().join(format!("dsh_test_lock_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let lock_path = temp_dir.join(".instance.lock");
    let lock1 = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();

    unsafe {
        assert_eq!(
            libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&lock1), libc::LOCK_EX | libc::LOCK_NB),
            0
        );
    }

    let lock2 = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .unwrap();

    unsafe {
        assert_ne!(
            libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&lock2), libc::LOCK_EX | libc::LOCK_NB),
            0
        );
        libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&lock1), libc::LOCK_UN);
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}
