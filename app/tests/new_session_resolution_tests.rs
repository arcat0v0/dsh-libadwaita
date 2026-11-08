use dsh_libadwaita::protocol::{SessionListItem, WorkspaceView};
use dsh_libadwaita::ui::controller::{find_reusable_blank, resolve_new_session_target};

fn workspace(id: &str, path: &str, session_ids: &[&str]) -> WorkspaceView {
    WorkspaceView {
        id: id.to_string(),
        path: path.to_string(),
        title: format!("ws-{id}"),
        created_at: "0".to_string(),
        updated_at: "0".to_string(),
        session_ids: session_ids.iter().map(|id| id.to_string()).collect(),
        status: None,
    }
}

fn session(
    id: &str,
    cwd: &str,
    updated_at: u64,
    turn_count: u32,
    is_archived: bool,
) -> SessionListItem {
    SessionListItem {
        id: id.to_string(),
        created_at: 0,
        updated_at,
        cwd: cwd.to_string(),
        title: format!("session-{id}"),
        turn_count,
        is_archived,
    }
}

#[test]
fn test_explicit_workspace_arg_wins() {
    let workspaces = vec![
        workspace("ws-a", "/a", &["s-1"]),
        workspace("ws-b", "/b", &[]),
    ];
    let sessions = vec![session("s-1", "/a", 100, 2, false)];
    let target = resolve_new_session_target(Some("ws-b"), Some("s-1"), &workspaces, &sessions);
    assert_eq!(target.map(|ws| ws.id.as_str()), Some("ws-b"));
}

#[test]
fn test_current_session_workspace_wins_over_recency() {
    let workspaces = vec![
        workspace("ws-a", "/a", &["s-1"]),
        workspace("ws-b", "/b", &["s-2"]),
    ];
    let sessions = vec![
        session("s-1", "/a", 10, 2, false),
        session("s-2", "/b", 999, 2, false),
    ];
    let target = resolve_new_session_target(None, Some("s-1"), &workspaces, &sessions);
    assert_eq!(target.map(|ws| ws.id.as_str()), Some("ws-a"));
}

#[test]
fn test_most_recent_workspace_by_member_session_updated_at() {
    let workspaces = vec![
        workspace("ws-a", "/a", &["s-1", "s-2"]),
        workspace("ws-b", "/b", &["s-3"]),
    ];
    let sessions = vec![
        session("s-1", "/a", 5, 1, false),
        session("s-2", "/a", 50, 1, false),
        session("s-3", "/b", 40, 1, false),
    ];
    let target = resolve_new_session_target(None, None, &workspaces, &sessions);
    assert_eq!(target.map(|ws| ws.id.as_str()), Some("ws-a"));
}

#[test]
fn test_recency_tie_resolves_in_host_order() {
    let workspaces = vec![
        workspace("ws-first", "/first", &["s-1"]),
        workspace("ws-second", "/second", &["s-2"]),
    ];
    let sessions = vec![
        session("s-1", "/first", 30, 1, false),
        session("s-2", "/second", 30, 1, false),
    ];
    let target = resolve_new_session_target(None, None, &workspaces, &sessions);
    assert_eq!(target.map(|ws| ws.id.as_str()), Some("ws-first"));
}

#[test]
fn test_no_target_without_workspaces() {
    let sessions = vec![session("s-1", "/a", 30, 1, false)];
    assert!(resolve_new_session_target(None, Some("s-1"), &[], &sessions).is_none());
}

#[test]
fn test_unknown_explicit_workspace_falls_back_to_no_target() {
    let workspaces = vec![workspace("ws-a", "/a", &["s-1"])];
    let sessions = vec![session("s-1", "/a", 30, 1, false)];
    assert!(
        resolve_new_session_target(Some("missing"), None, &workspaces, &sessions).is_none()
    );
}

#[test]
fn test_reusable_blank_is_found_in_workspace() {
    let ws = workspace("ws-a", "/a", &["blank", "s-2"]);
    let sessions = vec![
        session("s-2", "/a", 100, 3, false),
        session("blank", "/a", 10, 0, false),
    ];
    let blank = find_reusable_blank(&ws, &sessions, |_| true);
    assert_eq!(blank.map(|item| item.id.as_str()), Some("blank"));
}

#[test]
fn test_reusable_blank_requires_blank_member_cwd_and_archive_state() {
    let ws = workspace("ws-a", "/a", &["archived", "foreign", "moved", "blank"]);
    let sessions = vec![
        session("archived", "/a", 1, 0, true),
        session("foreign", "/b", 2, 0, false),
        session("moved", "/a", 3, 1, false),
        session("blank", "/a", 4, 0, false),
    ];
    let blank = find_reusable_blank(&ws, &sessions, |_| true);
    assert_eq!(blank.map(|item| item.id.as_str()), Some("blank"));
}

#[test]
fn test_reusable_blank_honors_preset_filter() {
    let ws = workspace("ws-a", "/a", &["blank"]);
    let sessions = vec![session("blank", "/a", 4, 0, false)];
    let blank = find_reusable_blank(&ws, &sessions, |_| false);
    assert!(blank.is_none());
}

#[test]
fn test_reusable_blank_returns_first_match_in_list_order() {
    let ws = workspace("ws-a", "/a", &["first", "second"]);
    let sessions = vec![
        session("second", "/a", 9, 0, false),
        session("first", "/a", 8, 0, false),
    ];
    let blank = find_reusable_blank(&ws, &sessions, |_| true);
    assert_eq!(blank.map(|item| item.id.as_str()), Some("second"));
}
