use dsh_libadwaita::ui::stream_markdown::stable_end;

#[test]
fn test_no_paragraph_boundary_keeps_whole_tail_unstable() {
    assert_eq!(stable_end("hello", 0), 0);
    assert_eq!(stable_end("one line\nstill same paragraph", 0), 0);
}

#[test]
fn test_stable_prefix_ends_after_last_blank_line() {
    assert_eq!(stable_end("a\n\nb\n\nc", 0), 6);
    assert_eq!(stable_end("a\n\nb\n\n", 0), 6);
    let text = "paragraph one\n\nparagraph two";
    assert_eq!(stable_end(text, 0), "paragraph one\n\n".len());
}

#[test]
fn test_stable_end_never_moves_backwards() {
    assert_eq!(stable_end("a\n\nb\n\nc", 6), 6);
    assert_eq!(stable_end("a\n\nb", 6), 4);
    assert_eq!(stable_end("short", 100), 5);
}

#[test]
fn test_crlf_paragraph_boundary_is_stable() {
    assert_eq!(stable_end("a\r\n\r\nb", 0), 5);
}

#[test]
fn test_open_code_fence_blocks_paragraph_boundaries() {
    assert_eq!(stable_end("```rust\ncode\n\nmore", 0), 0);
    assert_eq!(stable_end("intro\n\n```\ncode\n\n", 0), 7);
}

#[test]
fn test_closed_fence_allows_following_paragraph_boundary() {
    let text = "```\nx\n```\n\npara";
    assert_eq!(stable_end(text, 0), "```\nx\n```\n\n".len());
}

#[test]
fn test_boundary_advances_with_previous_end() {
    let text = "a\n\nb\n\nc\n\nd";
    assert_eq!(stable_end(text, 0), 9);
    assert_eq!(stable_end(text, 9), 9);
    let earlier = "a\n\nb\n\nc";
    assert_eq!(stable_end(earlier, 6), 6);
    assert_eq!(stable_end(text, 6), 9);
}
