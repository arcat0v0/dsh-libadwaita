use dsh_libadwaita::protocol::FileEntry;
use dsh_libadwaita::ui::file_tree::{breadcrumb_segments, FileTreeModel};

fn dir(name: &str, path: &str) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        path: path.to_string(),
        kind: "directory".to_string(),
        size: 0,
    }
}

fn file(name: &str, path: &str, size: u64) -> FileEntry {
    FileEntry {
        name: name.to_string(),
        path: path.to_string(),
        kind: "file".to_string(),
        size,
    }
}

#[test]
fn test_apply_listing_sorts_directories_first_then_name() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(
        ".",
        &[
            file("z.txt", "z.txt", 1),
            dir("Beta", "Beta"),
            file("a.txt", "a.txt", 2),
            dir("alpha", "alpha"),
        ],
    );
    let names: Vec<&str> = model
        .level(".")
        .unwrap()
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, vec!["alpha", "Beta", "a.txt", "z.txt"]);
}

#[test]
fn test_toggle_expands_and_keeps_loaded_level_on_collapse() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(".", &[dir("src", "src")]);
    assert!(model.toggle("src"));
    assert!(model.is_expanded("src"));
    model.apply_listing("src", &[file("main.rs", "src/main.rs", 10)]);
    assert_eq!(model.level("src").unwrap().len(), 1);
    assert!(!model.toggle("src"));
    assert!(!model.is_expanded("src"));
    assert_eq!(model.level("src").unwrap().len(), 1);
    assert!(model.toggle("src"));
    assert_eq!(model.level("src").unwrap().len(), 1);
}

#[test]
fn test_apply_listing_prunes_vanished_subdirectories() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(".", &[dir("gone", "gone"), dir("keep", "keep")]);
    model.toggle("gone");
    model.toggle("keep");
    model.apply_listing("gone", &[file("x", "gone/x", 1)]);
    model.apply_listing("keep", &[file("y", "keep/y", 1)]);
    model.apply_listing(".", &[dir("keep", "keep")]);
    assert!(!model.is_expanded("gone"));
    assert!(model.level("gone").is_none());
    assert!(model.is_expanded("keep"));
    assert!(model.level("keep").is_some());
}

#[test]
fn test_apply_failure_keeps_previous_level() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(".", &[dir("src", "src")]);
    model.toggle("src");
    model.apply_listing("src", &[file("main.rs", "src/main.rs", 3)]);
    model.apply_failure("src");
    assert_eq!(model.level("src").unwrap().len(), 1);
    assert!(model.failure("src"));
    model.apply_failure("other");
    assert!(model.level("other").is_none());
    assert!(model.failure("other"));
}

#[test]
fn test_flatten_depth_order() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(
        ".",
        &[
            file("root.txt", "root.txt", 1),
            dir("src", "src"),
            dir("docs", "docs"),
        ],
    );
    model.toggle("src");
    model.apply_listing(
        "src",
        &[file("main.rs", "src/main.rs", 2), dir("ui", "src/ui")],
    );
    model.toggle("src/ui");
    model.apply_listing("src/ui", &[file("app.rs", "src/ui/app.rs", 3)]);

    let rows = model.flatten();
    let rendered: Vec<(String, usize)> = rows
        .iter()
        .map(|row| (row.entry.name.clone(), row.depth))
        .collect();
    assert_eq!(
        rendered,
        vec![
            ("docs".to_string(), 0),
            ("src".to_string(), 0),
            ("ui".to_string(), 1),
            ("app.rs".to_string(), 2),
            ("main.rs".to_string(), 1),
            ("root.txt".to_string(), 0),
        ]
    );
}

#[test]
fn test_expand_ancestors_returns_missing_chain() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(".", &[dir("src", "src")]);
    model.toggle("src");
    let plan = model.expand_ancestors("src/ui/cards.rs");
    assert_eq!(plan.to_expand, vec!["src".to_string(), "src/ui".to_string()]);
    assert_eq!(plan.to_fetch, vec!["src/ui".to_string()]);
    assert!(model.is_expanded("src"));

    let loaded = model.expand_ancestors("/ws/src/main.rs");
    assert_eq!(loaded.to_expand, vec!["src".to_string()]);
    assert!(loaded.to_fetch.is_empty());
}

#[test]
fn test_set_root_resets_state() {
    let mut model = FileTreeModel::new("/ws");
    model.apply_listing(".", &[dir("src", "src")]);
    model.toggle("src");
    assert!(model.set_root("/other"));
    assert!(model.level(".").is_none());
    assert!(model.level("src").is_none());
    assert!(!model.is_expanded("src"));
    assert_eq!(model.root(), "/other");
    assert!(!model.set_root("/other"));
}

#[test]
fn test_breadcrumb_segments_split_relative_path() {
    let segments = breadcrumb_segments("/tmp/dsh-gui/ws", "src/ui/cards.rs");
    assert_eq!(
        segments,
        vec![
            ("ws".to_string(), ".".to_string()),
            ("src".to_string(), "src".to_string()),
            ("ui".to_string(), "src/ui".to_string()),
        ]
    );
    let absolute = breadcrumb_segments("/tmp/dsh-gui/ws", "/tmp/dsh-gui/ws/src/main.rs");
    assert_eq!(
        absolute,
        vec![
            ("ws".to_string(), ".".to_string()),
            ("src".to_string(), "src".to_string()),
        ]
    );
    let outside = breadcrumb_segments("/tmp/dsh-gui/ws", "/etc/hosts");
    assert!(outside.is_empty());
    let root_only = breadcrumb_segments("/tmp/dsh-gui/ws", ".");
    assert_eq!(root_only, vec![("ws".to_string(), ".".to_string())]);
}
