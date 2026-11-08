use dsh_libadwaita::connection::ConnectionError;
use dsh_libadwaita::protocol::SettingsDescriptor;
use dsh_libadwaita::settings_cas::{api_key_failure, conflict_revision, SettingsRevisionCache};

fn descriptor(ns: &str, revision: u64) -> SettingsDescriptor {
    SettingsDescriptor {
        ns: ns.to_string(),
        auto_generate: false,
        schema: serde_json::json!({}),
        value: serde_json::json!({}),
        base: None,
        user: None,
        revision,
        secrets: None,
    }
}

#[test]
fn test_revision_cache_seed_read_and_update() {
    let mut cache = SettingsRevisionCache::new();
    assert_eq!(cache.revision("agent-loop"), None);
    cache.seed("agent-loop", 3);
    assert_eq!(cache.revision("agent-loop"), Some(3));
    cache.apply_updated("agent-loop", 9);
    assert_eq!(cache.revision("agent-loop"), Some(9));
    cache.apply_conflict_actual("agent-loop", 12);
    assert_eq!(cache.revision("agent-loop"), Some(12));
}

#[test]
fn test_revision_cache_seed_from_descriptors() {
    let mut cache = SettingsRevisionCache::new();
    cache.seed_from_descriptors(&[descriptor("agent-loop", 4), descriptor("ui-theme", 8)]);
    assert_eq!(cache.revision("agent-loop"), Some(4));
    assert_eq!(cache.revision("ui-theme"), Some(8));
    assert_eq!(cache.revision("other"), None);
}

#[test]
fn test_conflict_revision_parses_remote_error_data() {
    let err = ConnectionError::Remote {
        code: -32020,
        message: "settings conflict".to_string(),
        data: Some(serde_json::json!({"expected": 3, "actual": 7})),
    };
    assert_eq!(conflict_revision(&err), Some(7));

    let wrong_code = ConnectionError::Remote {
        code: -32001,
        message: "other".to_string(),
        data: Some(serde_json::json!({"actual": 9})),
    };
    assert_eq!(conflict_revision(&wrong_code), None);

    let no_data = ConnectionError::Remote {
        code: -32020,
        message: "conflict".to_string(),
        data: None,
    };
    assert_eq!(conflict_revision(&no_data), None);

    let bad_data = ConnectionError::Remote {
        code: -32020,
        message: "conflict".to_string(),
        data: Some(serde_json::json!({"actual": "seven"})),
    };
    assert_eq!(conflict_revision(&bad_data), None);
}

#[test]
fn test_api_key_failure_rules() {
    assert_eq!(api_key_failure(""), None);
    assert_eq!(api_key_failure("   "), Some("keyBlank"));
    assert_eq!(api_key_failure("sk-abc123XYZ"), None);
    assert_eq!(api_key_failure("sk-abc 123"), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("DEEPSEEK_API_KEY=sk-x"), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("\"sk-abc\""), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("'sk-abc'"), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("`sk-abc`"), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("sk-é123"), Some("keyIllegalCharacters"));
    assert_eq!(api_key_failure("ABCD=="), None);
}
