use dsh_libadwaita::ui::controller::{DraftDisposition, PendingDraft, resolve_pending_draft};

fn draft(cwd: Option<&str>, session_id: Option<&str>) -> PendingDraft {
    PendingDraft {
        prompt: "写一个插件".to_string(),
        cwd: cwd.map(str::to_string),
        session_id: session_id.map(str::to_string),
    }
}

#[test]
fn test_known_session_id_consumes_only_on_id_match() {
    let pending = draft(Some("/a"), Some("sess-1"));
    assert_eq!(
        resolve_pending_draft(&pending, "sess-1", Some("/a")),
        DraftDisposition::Consume
    );
    assert_eq!(
        resolve_pending_draft(&pending, "sess-1", None),
        DraftDisposition::Consume
    );
}

#[test]
fn test_known_session_id_never_consumes_on_same_cwd_other_session() {
    let pending = draft(Some("/a"), Some("sess-1"));
    assert_eq!(
        resolve_pending_draft(&pending, "sess-other", Some("/a")),
        DraftDisposition::Clear
    );
}

#[test]
fn test_unknown_session_id_consumes_on_cwd_match() {
    let pending = draft(Some("/a"), None);
    assert_eq!(
        resolve_pending_draft(&pending, "sess-new", Some("/a")),
        DraftDisposition::Consume
    );
    assert_eq!(
        resolve_pending_draft(&pending, "sess-new", None),
        DraftDisposition::Clear
    );
}

#[test]
fn test_other_workspace_follow_clears_draft() {
    let pending = draft(Some("/a"), None);
    assert_eq!(
        resolve_pending_draft(&pending, "sess-2", Some("/b")),
        DraftDisposition::Clear
    );
}
