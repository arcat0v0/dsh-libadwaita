use dsh_libadwaita::protocol::*;

#[test]
fn test_question_answer_collection_prefers_options() {
    let answer = build_question_answer(
        "q1",
        vec!["A".to_string(), "B".to_string()],
        "ignored free text",
    );
    assert_eq!(answer.id, "q1");
    assert_eq!(answer.selected, vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn test_question_answer_collection_uses_free_text_when_no_options() {
    let answer = build_question_answer("q2", Vec::new(), "  custom answer  ");
    assert_eq!(answer.selected, vec!["custom answer".to_string()]);
}

#[test]
fn test_question_answer_collection_allows_skip() {
    let answer = build_question_answer("q3", Vec::new(), "   ");
    assert!(answer.selected.is_empty());
}
