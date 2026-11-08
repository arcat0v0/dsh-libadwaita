use dsh_libadwaita::settings_model::*;
use dsh_libadwaita::ui_prefs::*;
use serde_json::json;

#[test]
fn test_pin_toggle_and_order_persistence() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("prefs/ui-prefs.json");
    let mut prefs = UiPrefs::default();
    prefs.toggle_pin("s2");
    prefs.toggle_pin("s1");
    assert!(prefs.is_pinned("s1"));
    assert!(prefs.is_pinned("s2"));
    prefs.save(&path).unwrap();

    let mut loaded = UiPrefs::load(&path);
    assert_eq!(loaded.pinned_sessions, vec!["s2".to_string(), "s1".to_string()]);
    loaded.toggle_pin("s2");
    assert!(!loaded.is_pinned("s2"));
    assert!(loaded.is_pinned("s1"));
}

#[test]
fn test_archive_filter_and_dock_prefs_persist() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ui-prefs.json");
    let mut prefs = UiPrefs::default();
    prefs.archive_filter = ArchiveFilter::Archived;
    prefs.notifications_enabled = false;
    prefs.set_dock_tab("s1", 2);
    prefs.set_dock_visible("s1", false);
    prefs.save(&path).unwrap();
    let loaded = UiPrefs::load(&path);
    assert_eq!(loaded.archive_filter, ArchiveFilter::Archived);
    assert!(!loaded.notifications_enabled);
    assert_eq!(loaded.dock_tab("s1"), Some(2));
    assert!(!loaded.dock_visible("s1"));
}

#[test]
fn test_session_view_preference_round_trips() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ui-prefs.json");
    let mut prefs = UiPrefs::default();
    assert_eq!(prefs.session_view("s1"), None);
    prefs.set_session_view("s1", "trajectory");
    prefs.save(&path).unwrap();
    let loaded = UiPrefs::load(&path);
    assert_eq!(loaded.session_view("s1"), Some("trajectory"));
    assert_eq!(loaded.session_view("s2"), None);
}

#[test]
fn test_settings_schema_fields_and_find_key() {
    let schema = json!({
        "type": "object",
        "properties": {
            "enabled": {"type": "boolean", "title": "Enabled"},
            "maxParallel": {"type": "integer", "title": "Max parallel"},
            "mode": {"type": "string", "enum": ["fast", "safe"]},
            "apiKey": {"type": "string"}
        }
    });
    let value = json!({"enabled": true, "maxParallel": 4, "mode": "safe"});
    let fields = fields_from_schema(&schema, &value);
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].key, "enabled");
    assert_eq!(fields[0].kind, FieldKind::Bool);
    assert_eq!(fields[1].kind, FieldKind::Number);
    assert_eq!(
        fields[2].kind,
        FieldKind::Enum(vec!["fast".to_string(), "safe".to_string()])
    );
    assert_eq!(find_key(&schema, &["enabled"]), Some("enabled".to_string()));
    assert_eq!(find_key(&schema, &["missing"]), None);
}

#[test]
fn test_typert_schema_fields() {
    let schema = json!({
        "uid": 2972,
        "refs": {
            "2970": {"type": "number", "meta": {"step": 1, "min": 0, "max": 9007199254740991i64, "default": 1}},
            "2971": {"type": "number", "meta": {"step": 1, "min": 1, "default": 8}},
            "2972": {"type": "object", "meta": {"default": {}}, "dict": {"maxDepth": 2970, "maxActiveSubagents": 2971}}
        }
    });
    let value = json!({"maxDepth": 1, "maxActiveSubagents": 8});
    let fields = fields_from_schema(&schema, &value);
    assert_eq!(fields.len(), 2);
    let by_key = |key: &str| {
        fields
            .iter()
            .find(|field| field.key == key)
            .unwrap_or_else(|| panic!("missing field {key}"))
    };
    assert_eq!(by_key("maxDepth").kind, FieldKind::Number);
    assert_eq!(by_key("maxActiveSubagents").kind, FieldKind::Number);
    assert_eq!(find_key(&schema, &["maxDepth"]), Some("maxDepth".to_string()));
    assert_eq!(find_key(&schema, &["missing"]), None);
}

#[test]
fn test_typert_union_enum_and_nested_skip() {
    let schema = json!({
        "uid": 1,
        "refs": {
            "1": {"type": "object", "meta": {}, "dict": {"retryMode": 2, "retryCodes": 3}},
            "2": {"type": "union", "meta": {}, "list": [4, 5]},
            "4": {"type": "const", "meta": {}, "value": "auto"},
            "5": {"type": "const", "meta": {}, "value": "manual"},
            "3": {"type": "union", "meta": {}, "list": [6, 7]},
            "6": {"type": "const", "meta": {}, "value": "a"},
            "7": {"type": "array", "meta": {}, "inner": 4}
        }
    });
    let value = json!({"retryMode": "manual"});
    let fields = fields_from_schema(&schema, &value);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].key, "retryMode");
    assert_eq!(
        fields[0].kind,
        FieldKind::Enum(vec!["auto".to_string(), "manual".to_string()])
    );
    assert_eq!(fields[0].value, json!("manual"));
}

#[test]
fn test_transcript_view_and_registry_persist() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ui-prefs.json");
    let mut prefs = UiPrefs::default();
    prefs.transcript_view = TranscriptView::Expanded;
    prefs.plugin_registry = Some("https://registry.example".to_string());
    prefs.save(&path).unwrap();
    let loaded = UiPrefs::load(&path);
    assert_eq!(loaded.transcript_view, TranscriptView::Expanded);
    assert_eq!(
        loaded.plugin_registry.as_deref(),
        Some("https://registry.example")
    );
}

#[test]
fn test_transcript_view_persists_four_modes() {
    for mode in [
        TranscriptView::Concise,
        TranscriptView::Standard,
        TranscriptView::Detailed,
        TranscriptView::Expanded,
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("ui-prefs.json");
        let mut prefs = UiPrefs::default();
        prefs.transcript_view = mode;
        prefs.save(&path).unwrap();
        assert_eq!(UiPrefs::load(&path).transcript_view, mode);
    }
}
