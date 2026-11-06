use dsh_libadwaita::protocol::SessionEvent;
use dsh_libadwaita::session::{SessionModel, MAX_RAW_EVENTS};

fn event(seq: i64) -> SessionEvent {
    SessionEvent {
        event_type: "test".to_string(),
        seq,
        time: seq as u64,
        data: serde_json::json!({"seq": seq}),
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    }
}

#[test]
fn test_raw_event_buffer_caps_and_orders() {
    let mut session = SessionModel::new("s1", "/tmp");
    for seq in 0..(MAX_RAW_EVENTS as i64 + 5) {
        let _ = session.handle_event(event(seq));
    }
    assert_eq!(session.raw_events().len(), MAX_RAW_EVENTS);
    assert_eq!(session.raw_events().front().unwrap().seq, 5);
    assert_eq!(
        session.raw_events().back().unwrap().seq,
        MAX_RAW_EVENTS as i64 + 4
    );
}

#[test]
fn test_raw_event_older_prepend() {
    let mut session = SessionModel::new("s1", "/tmp");
    let _ = session.handle_event(event(10));
    session.record_older_events(vec![event(3), event(4)]);
    let seqs: Vec<i64> = session.raw_events().iter().map(|e| e.seq).collect();
    assert_eq!(seqs, vec![3, 4, 10]);
}
