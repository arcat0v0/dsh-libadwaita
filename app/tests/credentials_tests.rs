use dsh_libadwaita::credentials::*;
use dsh_libadwaita::protocol::*;

#[test]
fn test_credential_store_set_get_rotate_delete() {
    let store = CredentialStore::in_memory();
    assert_eq!(store.get_key("DEEPSEEK_API_KEY"), None);
    assert!(!store.is_configured("DEEPSEEK_API_KEY"));

    store.set_key("DEEPSEEK_API_KEY", "sk-first-key-123456").unwrap();
    assert_eq!(
        store.get_key("DEEPSEEK_API_KEY"),
        Some("sk-first-key-123456".to_string())
    );
    assert!(store.is_configured("DEEPSEEK_API_KEY"));

    store.set_key("DEEPSEEK_API_KEY", "sk-rotated-key-789012").unwrap();
    assert_eq!(
        store.get_key("DEEPSEEK_API_KEY"),
        Some("sk-rotated-key-789012".to_string())
    );

    store.delete_key("DEEPSEEK_API_KEY").unwrap();
    assert_eq!(store.get_key("DEEPSEEK_API_KEY"), None);
    assert!(!store.is_configured("DEEPSEEK_API_KEY"));

    let err = store.set_key("DEEPSEEK_API_KEY", "").unwrap_err();
    assert_eq!(err, CredentialError::EmptyKey);
}

#[test]
fn test_mask_key_formatting() {
    assert_eq!(
        CredentialStore::mask_key("sk-1234567890abcdef"),
        "sk-...cdef"
    );
    assert_eq!(CredentialStore::mask_key("sk-short"), "sk-...hort");
    assert_eq!(CredentialStore::mask_key("tiny"), "****");
}

#[test]
fn test_should_show_onboarding_logic() {
    let unconfigured_items = vec![CredentialStatusItem {
        provider: "DEEPSEEK_API_KEY".to_string(),
        configured: false,
        masked: None,
    }];
    assert!(should_show_onboarding(&unconfigured_items));

    let configured_items = vec![CredentialStatusItem {
        provider: "DEEPSEEK_API_KEY".to_string(),
        configured: true,
        masked: Some("sk-...xxxx".to_string()),
    }];
    assert!(!should_show_onboarding(&configured_items));

    let empty_items: Vec<CredentialStatusItem> = vec![];
    assert!(should_show_onboarding(&empty_items));
}

#[test]
fn test_plugin_toggle_restart_required_semantics() {
    let toggle_res: PluginChangeResult = serde_json::from_str(
        r#"{
            "changed": true,
            "application": "restart-required",
            "restartReason": "no-hmr",
            "stage": "enable",
            "target": "tool-plugin-manager",
            "enabled": true
        }"#,
    )
    .unwrap();

    assert_eq!(toggle_res.application, "restart-required");
    assert_eq!(toggle_res.restart_reason.as_deref(), Some("no-hmr"));
    assert_eq!(toggle_res.enabled, Some(true));
}

#[test]
fn test_credentials_rotate_roundtrip() {
    let rotate_req = CredentialsRotateParams {
        provider: "DEEPSEEK_API_KEY".to_string(),
        key: "sk-mock-deepseek-test-key-1234".to_string(),
    };

    let json_str = serde_json::to_string(&rotate_req).unwrap();
    let parsed: CredentialsRotateParams = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.provider, "DEEPSEEK_API_KEY");
    assert_eq!(parsed.key, "sk-mock-deepseek-test-key-1234");
}

#[test]
fn test_plugin_list_parsing() {
    let raw = r#"{
        "plugins": [
            {
                "id": "bridge",
                "name": "@dsh-libadwaita/bridge",
                "enabled": true,
                "restartRequired": false,
                "description": "Bridge bundle"
            },
            {
                "id": "tool-plugin-manager",
                "name": "@deepseek-ai/dsh-tool-plugin-manager",
                "enabled": false,
                "restartRequired": true,
                "description": null
            }
        ]
    }"#;

    let res: PluginListResult = serde_json::from_str(raw).unwrap();
    assert_eq!(res.plugins.len(), 2);
    assert!(res.plugins[0].enabled);
    assert!(!res.plugins[0].restart_required);
    assert!(!res.plugins[1].enabled);
    assert!(res.plugins[1].restart_required);
}
