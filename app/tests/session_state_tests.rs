use dsh_libadwaita::session_state::*;
use std::path::PathBuf;

#[test]
fn test_session_state_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("state/session-state.json");
    let state = SessionState {
        last_session_id: Some("sess-42".to_string()),
        last_cwd: Some("/tmp/workspace".to_string()),
    };
    state.save(&path).unwrap();
    let loaded = SessionState::load(&path);
    assert_eq!(loaded, state);
}

#[test]
fn test_session_state_clear_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("session-state.json");
    let state = SessionState::default();
    state.save(&path).unwrap();
    let loaded = SessionState::load(&path);
    assert_eq!(loaded.last_session_id, None);
    assert_eq!(loaded.last_cwd, None);
}

#[test]
fn test_session_state_load_missing_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing/session-state.json");
    let state = SessionState::load(&path);
    assert_eq!(state, SessionState::default());
}

#[test]
fn test_default_state_path_layout() {
    let tmp = tempfile::tempdir().unwrap();
    let old = std::env::var("XDG_CONFIG_HOME").ok();
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    }
    let path = default_state_path();
    assert_eq!(path, PathBuf::from(tmp.path()).join("dsh-libadwaita/session-state.json"));
    match old {
        Some(value) => unsafe { std::env::set_var("XDG_CONFIG_HOME", value) },
        None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
    }
}
