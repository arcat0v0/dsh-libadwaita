use dsh_libadwaita::ui::sidebar::{reorder_manual_order, shows_schedule_clock};
use std::collections::HashSet;

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn set(values: &[&str]) -> HashSet<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn test_reorder_inserts_before_target() {
    let order = reorder_manual_order(&ids(&["a", "b", "c"]), &ids(&["a", "b", "c"]), "c", "a");
    assert_eq!(order, ids(&["c", "a", "b"]));
    let order = reorder_manual_order(&ids(&["a", "b", "c"]), &ids(&["a", "b", "c"]), "c", "b");
    assert_eq!(order, ids(&["a", "c", "b"]));
}

#[test]
fn test_reorder_appends_known_sessions_missing_from_order() {
    let order = reorder_manual_order(&ids(&["a"]), &ids(&["a", "b", "c"]), "c", "a");
    assert_eq!(order, ids(&["c", "a", "b"]));
}

#[test]
fn test_reorder_unknown_target_appends_dragged() {
    let order = reorder_manual_order(&ids(&["a", "b"]), &ids(&["a", "b"]), "a", "missing");
    assert_eq!(order, ids(&["b", "a"]));
}

#[test]
fn test_reorder_ignores_unknown_saved_ids() {
    let order = reorder_manual_order(&ids(&["gone", "a", "b"]), &ids(&["a", "b"]), "b", "a");
    assert_eq!(order, ids(&["gone", "b", "a"]));
}

#[test]
fn test_schedule_clock_shows_for_idle_scheduled_session() {
    let scheduled = set(&["a"]);
    assert!(shows_schedule_clock(&scheduled, &set(&[]), &set(&[]), "a"));
    assert!(!shows_schedule_clock(&scheduled, &set(&["a"]), &set(&[]), "a"));
    assert!(!shows_schedule_clock(&scheduled, &set(&[]), &set(&["a"]), "a"));
    assert!(!shows_schedule_clock(&scheduled, &set(&[]), &set(&[]), "b"));
}
