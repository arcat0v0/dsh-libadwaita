use dsh_libadwaita::drafts::*;

#[test]
fn test_draft_store_set_get_remove() {
    let mut store = DraftStore::default();
    assert_eq!(store.get("s1"), None);

    store.set("s1", "hello draft");
    assert_eq!(store.get("s1"), Some("hello draft"));

    store.set("s2", "another");
    assert_eq!(store.get("s2"), Some("another"));

    store.remove("s1");
    assert_eq!(store.get("s1"), None);
    assert_eq!(store.get("s2"), Some("another"));
}

#[test]
fn test_draft_store_composer_text_is_session_scoped() {
    let mut store = DraftStore::default();
    store.set("s1", "hello");
    assert_eq!(store.text_for("s1"), "hello");
    assert_eq!(store.text_for("s2"), "");
}

#[test]
fn test_draft_store_blank_text_is_removed() {
    let mut store = DraftStore::default();
    store.set("s1", "text");
    store.set("s1", "   ");
    assert_eq!(store.get("s1"), None);
}

#[test]
fn test_draft_store_persistence_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("nested/drafts.json");
    let mut store = DraftStore::default();
    store.set("sess-1", "kept draft");
    store.save(&path).unwrap();

    let loaded = DraftStore::load(&path);
    assert_eq!(loaded.get("sess-1"), Some("kept draft"));
}

#[test]
fn test_draft_store_load_missing_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing/drafts.json");
    assert_eq!(DraftStore::load(&path), DraftStore::default());
}

#[test]
fn test_pending_echo_submit_confirm_cycle() {
    let mut tracker = PendingEchoTracker::default();
    assert_eq!(tracker.pending_text(), None);

    tracker.submit("hello");
    assert_eq!(tracker.pending_text(), Some("hello"));
    assert!(matches!(
        tracker.state(),
        PendingEchoState::Awaiting { .. }
    ));

    let confirmed = tracker.confirm();
    assert_eq!(confirmed.as_deref(), Some("hello"));
    assert_eq!(tracker.pending_text(), None);
    assert_eq!(tracker.state(), &PendingEchoState::Confirmed);
}

#[test]
fn test_pending_echo_failure_restores_text() {
    let mut tracker = PendingEchoTracker::default();
    tracker.submit("draft to restore");

    let restored = tracker.fail("transport closed");
    assert_eq!(restored.as_deref(), Some("draft to restore"));
    assert_eq!(tracker.pending_text(), None);
    match tracker.state() {
        PendingEchoState::Failed { text, error } => {
            assert_eq!(text, "draft to restore");
            assert_eq!(error, "transport closed");
        }
        _ => panic!("expected Failed state"),
    }
}

#[test]
fn test_continuation_eligibility_rules() {
    assert_eq!(
        continuation_eligibility(Some("RATE_LIMIT"), true, false),
        ContinuationEligibility::Eligible
    );
    assert_eq!(
        continuation_eligibility(Some("STREAM_CLOSED"), true, false),
        ContinuationEligibility::Eligible
    );
    assert_eq!(
        continuation_eligibility(Some("RATE_LIMIT"), false, false),
        ContinuationEligibility::NotEligible
    );
    assert_eq!(
        continuation_eligibility(Some("RATE_LIMIT"), true, true),
        ContinuationEligibility::NotEligible
    );
    assert_eq!(
        continuation_eligibility(Some("UNKNOWN"), true, false),
        ContinuationEligibility::NotEligible
    );
    assert_eq!(
        continuation_eligibility(None, true, false),
        ContinuationEligibility::NotEligible
    );
}

#[test]
fn test_secret_masking_of_known_patterns() {
    let prefix = format!("{}k-", "s");
    let body = format!("{}{}", "abcdefgh", "1234");
    let key = format!("{}{}", prefix, body);
    let text = format!("key {} done", key);
    let masked = mask_secret_patterns(&text);
    let expected = format!("{}abcde...1234", prefix);
    assert!(masked.contains(&expected));
    assert!(!masked.contains(&body));
    assert!(masked.contains("done"));

    let short = mask_secret_patterns("token short-token");
    assert_eq!(short, "token short-token");

    let plain = mask_secret_patterns("nothing to hide");
    assert_eq!(plain, "nothing to hide");
}
