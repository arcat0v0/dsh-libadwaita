use dsh_libadwaita::ui::presentation::{
    reasoning_header, reasoning_summary, PresentationPolicy, StepGrouping,
};
use dsh_libadwaita::ui_prefs::TranscriptView;

#[test]
fn test_policy_matches_upstream_four_modes() {
    assert_eq!(
        PresentationPolicy::for_mode(TranscriptView::Concise),
        PresentationPolicy {
            fold_completed_turns: true,
            step_grouping: StepGrouping::Collapsed,
            live_process_detail: false,
            settled_reasoning_preview: false,
        }
    );
    assert_eq!(
        PresentationPolicy::for_mode(TranscriptView::Standard),
        PresentationPolicy {
            fold_completed_turns: true,
            step_grouping: StepGrouping::Collapsed,
            live_process_detail: true,
            settled_reasoning_preview: true,
        }
    );
    assert_eq!(
        PresentationPolicy::for_mode(TranscriptView::Detailed),
        PresentationPolicy {
            fold_completed_turns: true,
            step_grouping: StepGrouping::History,
            live_process_detail: true,
            settled_reasoning_preview: true,
        }
    );
    assert_eq!(
        PresentationPolicy::for_mode(TranscriptView::Expanded),
        PresentationPolicy {
            fold_completed_turns: false,
            step_grouping: StepGrouping::None,
            live_process_detail: false,
            settled_reasoning_preview: true,
        }
    );
}

#[test]
fn test_only_expanded_mode_skips_step_grouping() {
    for mode in [
        TranscriptView::Concise,
        TranscriptView::Standard,
        TranscriptView::Detailed,
    ] {
        assert!(PresentationPolicy::for_mode(mode).groups_intermediate());
    }
    assert!(!PresentationPolicy::for_mode(TranscriptView::Expanded).groups_intermediate());
}

#[test]
fn test_reasoning_summary_uses_completed_paragraph_when_running() {
    assert_eq!(
        reasoning_summary("first line\nsecond line", true).as_deref(),
        Some("first line")
    );
    assert_eq!(
        reasoning_summary("first line\nsecond line", false).as_deref(),
        Some("first line")
    );
    assert_eq!(
        reasoning_summary("intro\n\nsecond paragraph\nmore", true).as_deref(),
        Some("second paragraph")
    );
    assert_eq!(
        reasoning_summary("intro\n\nsecond paragraph\nmore", false).as_deref(),
        Some("intro")
    );
    assert_eq!(
        reasoning_summary("**Bold** plan\nrest", false).as_deref(),
        Some("Bold plan")
    );
    assert_eq!(reasoning_summary("   \n\n  ", true), None);
    assert_eq!(reasoning_summary("", false), None);
}

#[test]
fn test_reasoning_header_respects_preview_policy_and_expansion() {
    assert_eq!(reasoning_header(false, true, false, "**Plan** it"), "Reasoning Process · Plan it");
    assert_eq!(reasoning_header(true, true, false, "Plan it"), "Reasoning Process");
    assert_eq!(reasoning_header(false, false, false, "Plan it"), "Reasoning Process");
    assert_eq!(reasoning_header(false, false, true, "Plan it\nmore"), "Reasoning Process · Plan it");
    assert_eq!(reasoning_header(false, false, true, "Plan it"), "Reasoning Process");
    assert_eq!(reasoning_header(false, true, true, ""), "Reasoning Process");
}
