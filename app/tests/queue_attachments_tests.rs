use dsh_libadwaita::protocol::{QueueAttachment, QueueItem, QueuePlacement};

fn queue_item(content: serde_json::Value) -> QueueItem {
    QueueItem {
        id: "q1".to_string(),
        placement: QueuePlacement::Queued,
        content: content.as_array().expect("array").clone(),
    }
}

#[test]
fn test_queue_item_text_ignores_attachments() {
    let item = queue_item(serde_json::json!([
        {"type": "text", "text": "first"},
        {"type": "image", "attachment": {"attachmentId": "img1", "mediaType": "image/png", "bytes": 9, "width": 3, "height": 3}},
        {"type": "text", "text": "second"}
    ]));
    assert_eq!(item.text(), "first\nsecond");
}

#[test]
fn test_queue_item_parses_image_and_file_attachments() {
    let item = queue_item(serde_json::json!([
        {"type": "image", "attachment": {"attachmentId": "img1", "mediaType": "image/png", "bytes": 9, "width": 3, "height": 3, "name": "shot.png"}},
        {"type": "file", "attachment": {"attachmentId": "file1", "name": "report.pdf", "bytes": 2048}}
    ]));
    let attachments = item.attachments();
    assert_eq!(attachments.len(), 2);
    match &attachments[0] {
        QueueAttachment::Image(reference) => {
            assert_eq!(reference.attachment_id, "img1");
            assert_eq!(reference.media_type, "image/png");
            assert_eq!(reference.width, 3);
            assert_eq!(reference.name.as_deref(), Some("shot.png"));
        }
        other => panic!("expected image, got {other:?}"),
    }
    match &attachments[1] {
        QueueAttachment::File { name, bytes } => {
            assert_eq!(name, "report.pdf");
            assert_eq!(*bytes, Some(2048));
        }
        other => panic!("expected file, got {other:?}"),
    }
}

#[test]
fn test_queue_item_skips_refs_without_identity() {
    let item = queue_item(serde_json::json!([
        {"type": "image", "attachment": {"mediaType": "image/png"}},
        {"type": "file", "attachment": {"attachmentId": "file1", "bytes": 4}},
        {"type": "file", "name": "loose.txt"}
    ]));
    let attachments = item.attachments();
    assert_eq!(attachments.len(), 1);
    match &attachments[0] {
        QueueAttachment::File { name, bytes } => {
            assert_eq!(name, "loose.txt");
            assert_eq!(*bytes, None);
        }
        other => panic!("expected file, got {other:?}"),
    }
}
