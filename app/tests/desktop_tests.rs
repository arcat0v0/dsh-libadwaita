use dsh_libadwaita::ui::desktop::*;

#[test]
fn test_should_notify_when_unfocused() {
    assert!(should_notify_turn_completion(false, 1000));
    assert!(should_notify_turn_completion(false, 0));
}

#[test]
fn test_should_notify_when_long_turn_focused() {
    assert!(should_notify_turn_completion(true, 3500));
}

#[test]
fn test_no_notify_when_focused_short_turn() {
    assert!(!should_notify_turn_completion(true, 500));
    assert!(!should_notify_turn_completion(true, 2999));
}

#[test]
fn test_truncate_snippet() {
    let full = "a".repeat(300);
    let snip = truncate_snippet(&full, 100);
    assert!(snip.len() <= 104);
    assert!(snip.ends_with("..."));

    let short = "hello world";
    assert_eq!(truncate_snippet(short, 100), "hello world");
}

#[test]
fn test_terminal_candidates_include_expected() {
    let candidates = terminal_command_candidates();
    let flat: Vec<&str> = candidates.iter().map(|c| c.0.as_str()).collect();
    assert!(flat.contains(&"xdg-terminal-exec"));
    assert!(flat.contains(&"ptyxis"));
    assert!(flat.contains(&"gnome-terminal"));
}
