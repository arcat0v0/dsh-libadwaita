use dsh_libadwaita::ui::scroll_follow::{near_bottom, ScrollFollow, FOLLOW_THRESHOLD};

#[test]
fn test_follow_starts_active_when_requested() {
    assert!(ScrollFollow::new(true).following());
    assert!(!ScrollFollow::new(false).following());
}

#[test]
fn test_near_bottom_uses_threshold_and_clamps_short_content() {
    assert!(near_bottom(500.0, 1000.0, 500.0));
    assert!(near_bottom(500.0 - FOLLOW_THRESHOLD, 1000.0, 500.0));
    assert!(!near_bottom(500.0 - FOLLOW_THRESHOLD - 1.0, 1000.0, 500.0));
    assert!(near_bottom(0.0, 200.0, 500.0));
    assert!(near_bottom(0.0, 0.0, 0.0));
}

#[test]
fn test_reader_scroll_pauses_and_returning_to_floor_resumes() {
    let mut follow = ScrollFollow::new(true);
    assert!(!follow.sample(400.0, 2000.0, 600.0));
    assert!(follow.sample(1408.0, 2000.0, 600.0));
    assert!(follow.sample(1400.0, 2000.0, 600.0));
}

#[test]
fn test_jump_and_pause_override_intent_without_moving() {
    let mut follow = ScrollFollow::new(false);
    follow.jump();
    assert!(follow.following());
    follow.pause();
    assert!(!follow.following());
}

#[test]
fn test_sample_at_old_offset_after_growth_records_reader_position() {
    let mut follow = ScrollFollow::new(true);
    assert!(!follow.sample(500.0, 3000.0, 600.0));
    assert!(follow.sample(2400.0, 3000.0, 600.0));
}
