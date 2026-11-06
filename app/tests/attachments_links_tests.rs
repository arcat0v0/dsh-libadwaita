use dsh_libadwaita::attachments::*;
use dsh_libadwaita::links::*;
use std::path::Path;

#[test]
fn test_attachment_classification() {
    assert_eq!(
        classify_path(Path::new("/tmp/shot.PNG")),
        AttachmentKind::Image("image/png")
    );
    assert_eq!(
        classify_path(Path::new("photo.jpeg")),
        AttachmentKind::Image("image/jpeg")
    );
    assert_eq!(classify_path(Path::new("notes.txt")), AttachmentKind::File);
    assert_eq!(image_mime_for_path(Path::new("anim.gif")), Some("image/gif"));
    let encoded = encode_base64(b"hello");
    assert_eq!(decode_base64(&encoded), Some(b"hello".to_vec()));
}

#[test]
fn test_link_classification_and_lines() {
    assert_eq!(
        classify_link("https://example.com/a"),
        LinkTarget::External("https://example.com/a".to_string())
    );
    assert_eq!(
        classify_link("mailto:dev@example.com"),
        LinkTarget::External("mailto:dev@example.com".to_string())
    );
    assert_eq!(classify_link("javascript:alert(1)"), LinkTarget::Blocked);
    assert_eq!(classify_link("data:text/html,hi"), LinkTarget::Blocked);
    assert_eq!(
        classify_link("src/main.rs#L24-L30"),
        LinkTarget::File {
            path: "src/main.rs".to_string(),
            line: Some(24),
            end_line: Some(30),
        }
    );
    assert_eq!(
        classify_link("src/lib.rs:42"),
        LinkTarget::File {
            path: "src/lib.rs".to_string(),
            line: Some(42),
            end_line: None,
        }
    );
}
