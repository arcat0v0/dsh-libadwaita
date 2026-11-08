use dsh_libadwaita::ui::plugins_page::model::*;
use serde_json::json;

fn fixture_bundles() -> Vec<serde_json::Value> {
    vec![
        json!({
            "name": "@deepseek-ai/dsh-base",
            "official": true,
            "availability": "installation",
            "installed": true,
            "optional": false,
            "removable": false,
            "updatable": false,
            "enabled": true,
            "rows": [
                {"rowId": "shell", "moduleName": "@deepseek-ai/dsh-shell", "entryId": "base#shell"}
            ],
            "overrides": []
        }),
        json!({
            "name": "@deepseek-ai/dsh-experimental-auto-review",
            "official": true,
            "availability": "installation",
            "version": "0.4.2",
            "meta": {
                "title": {"en": "Auto review", "zh": "自动审查"},
                "description": {"en": "Review agent changes.", "zh": "自动审查 Agent 的改动。"}
            },
            "installed": true,
            "optional": true,
            "removable": false,
            "updatable": false,
            "enabled": false,
            "listedByDefault": true,
            "rows": [
                {"rowId": "auto-review", "moduleName": "@deepseek-ai/dsh-auto-review/plugin"}
            ],
            "overrides": []
        }),
        json!({
            "name": "@deepseek-ai/dsh-experimental-session-titles-bundle",
            "official": true,
            "availability": "installation",
            "installed": true,
            "optional": true,
            "removable": false,
            "updatable": false,
            "enabled": true,
            "rows": [],
            "overrides": []
        }),
        json!({
            "name": "@deepseek-ai/dsh-subagent-codex",
            "official": true,
            "availability": "missing",
            "installTarget": {"spec": "@deepseek-ai/dsh-subagent-codex", "version": "1.1.0"},
            "installed": false,
            "optional": true,
            "removable": false,
            "updatable": true,
            "enabled": false,
            "rows": [],
            "overrides": []
        }),
        json!({
            "name": "@community/whale-pet",
            "official": false,
            "availability": "profile",
            "version": "1.0.3",
            "installTarget": {"spec": "@community/whale-pet", "version": "1.1.0"},
            "source": "@community/whale-pet@^1.0.0",
            "description": "A whale pet for your harness.",
            "installed": true,
            "optional": false,
            "removable": true,
            "updatable": true,
            "enabled": true,
            "rows": [
                {"rowId": "pet", "moduleName": "@community/whale-pet/plugin", "entryId": "whale-pet#pet"},
                {"rowId": "quiet", "moduleName": "@community/whale-pet/quiet", "entryId": "whale-pet#quiet"}
            ],
            "overrides": []
        })
    ]
}

fn fixture_plugins() -> Vec<serde_json::Value> {
    vec![
        json!({
            "entryId": "whale-pet#pet",
            "moduleName": "@community/whale-pet/plugin",
            "enabled": true,
            "fiberPhase": "active"
        }),
        json!({
            "entryId": "whale-pet#quiet",
            "moduleName": "@community/whale-pet/quiet",
            "enabled": false,
            "fiberPhase": null
        }),
        json!({
            "entryId": "base#shell",
            "moduleName": "@deepseek-ai/dsh-shell",
            "enabled": true,
            "fiberPhase": "failed",
            "diagnostic": "module not found"
        }),
        json!({
            "entryId": "stray#entry",
            "moduleName": "@community/unlisted",
            "enabled": true,
            "fiberPhase": "active"
        })
    ]
}

fn fixture_packages() -> Vec<PackageView> {
    let bundles = fixture_bundles();
    let plugins = fixture_plugins();
    parse_inventory(&bundles, &plugins)
}

#[test]
fn test_short_name_strips_scope_and_prefixes() {
    assert_eq!(
        short_name("@deepseek-ai/dsh-experimental-auto-review"),
        "experimental-auto-review"
    );
    assert_eq!(short_name("@deepseek-ai/dsh-base"), "base");
    assert_eq!(short_name("@community/whale-pet"), "whale-pet");
    assert_eq!(short_name("dsh-host-frontend-static"), "frontend-static");
}

#[test]
fn test_parse_inventory_joins_rows_with_live_entries() {
    let packages = fixture_packages();
    let whale = packages
        .iter()
        .find(|pkg| pkg.name == "@community/whale-pet")
        .expect("whale-pet parsed");
    assert!(!whale.official);
    assert_eq!(whale.availability, "profile");
    assert_eq!(whale.version.as_deref(), Some("1.0.3"));
    assert_eq!(whale.source.as_deref(), Some("@community/whale-pet@^1.0.0"));
    assert!(whale.installed && whale.enabled && whale.removable && whale.updatable);
    assert_eq!(whale.rows.len(), 2);
    let pet = whale
        .rows
        .iter()
        .find(|row| row.row_id == "pet")
        .expect("pet row");
    assert_eq!(pet.entry_id.as_deref(), Some("whale-pet#pet"));
    assert!(pet.enabled);
    assert_eq!(pet.phase, Some(RowPhase::Active));
    let quiet = whale
        .rows
        .iter()
        .find(|row| row.row_id == "quiet")
        .expect("quiet row");
    assert!(!quiet.enabled);
    assert_eq!(quiet.phase, None);

    let base = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-base")
        .expect("base parsed");
    let shell = base
        .rows
        .iter()
        .find(|row| row.row_id == "shell")
        .expect("shell row");
    assert_eq!(shell.phase, Some(RowPhase::Failed));
    assert_eq!(shell.diagnostic.as_deref(), Some("module not found"));
}

#[test]
fn test_parse_inventory_sorts_by_short_name() {
    let names: Vec<String> = fixture_packages()
        .iter()
        .map(|pkg| short_name(&pkg.name))
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
}

#[test]
fn test_parse_inventory_resolves_localized_meta() {
    let packages = fixture_packages();
    let review = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-experimental-auto-review")
        .expect("review parsed");
    assert_eq!(review.title.as_deref(), Some("自动审查"));
    assert_eq!(review.meta_description.as_deref(), Some("自动审查 Agent 的改动。"));
    assert_eq!(resolve_text(Some(&json!("plain")), "zh"), Some("plain".to_string()));
    assert_eq!(resolve_text(Some(&json!({"en": "Only english"})), "zh"), Some("Only english".to_string()));
    assert_eq!(resolve_text(None, "zh"), None);
}

#[test]
fn test_package_status_running_disabled_problem() {
    let packages = fixture_packages();
    let whale = packages
        .iter()
        .find(|pkg| pkg.name == "@community/whale-pet")
        .unwrap();
    assert_eq!(package_status(whale), PackageStatus::Running);
    let review = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-experimental-auto-review")
        .unwrap();
    assert_eq!(package_status(review), PackageStatus::Disabled);
    let base = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-base")
        .unwrap();
    assert_eq!(package_status(base), PackageStatus::Problem);
    let mut errored = whale.clone();
    errored.error = Some(PackageError {
        code: "unsupported-platform".to_string(),
        diagnostic: None,
    });
    assert_eq!(package_status(&errored), PackageStatus::Problem);
}

#[test]
fn test_package_tags_match_upstream_badges() {
    let packages = fixture_packages();
    let review = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-experimental-auto-review")
        .unwrap();
    assert_eq!(package_tags(review), vec![PackageTag::Experimental]);
    let codex = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-subagent-codex")
        .unwrap();
    assert_eq!(package_tags(codex), vec![PackageTag::NotInstalled]);
    let base = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-base")
        .unwrap();
    assert_eq!(package_tags(base), vec![PackageTag::Problem]);
}

#[test]
fn test_row_state_text_and_dot() {
    let packages = fixture_packages();
    let whale = packages
        .iter()
        .find(|pkg| pkg.name == "@community/whale-pet")
        .unwrap();
    let pet = whale.rows.iter().find(|row| row.row_id == "pet").unwrap();
    let quiet = whale.rows.iter().find(|row| row.row_id == "quiet").unwrap();
    assert_eq!(row_state_text(pet), "运行中");
    assert_eq!(row_dot(pet), DotState::Done);
    assert_eq!(row_state_text(quiet), "已关闭");
    assert_eq!(row_dot(quiet), DotState::Idle);
    let mut loading = pet.clone();
    loading.phase = Some(RowPhase::Loading);
    assert_eq!(row_state_text(&loading), "加载中");
    assert_eq!(row_dot(&loading), DotState::Ongoing);
    let mut idle = pet.clone();
    idle.phase = None;
    assert_eq!(row_state_text(&idle), "未运行");
    assert_eq!(row_dot(&idle), DotState::Idle);
}

#[test]
fn test_update_available_follows_upstream_rule() {
    let packages = fixture_packages();
    let whale = packages
        .iter()
        .find(|pkg| pkg.name == "@community/whale-pet")
        .unwrap();
    assert!(update_available(whale));
    let codex = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-subagent-codex")
        .unwrap();
    assert!(!update_available(codex));
    let base = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-base")
        .unwrap();
    assert!(!update_available(base));
    let mut same_version = whale.clone();
    same_version.version = Some("1.0.0".to_string());
    same_version.install_target = Some(InstallTarget {
        spec: "@community/whale-pet".to_string(),
        version: "1.0.0".to_string(),
    });
    assert!(!update_available(&same_version));
}

#[test]
fn test_optional_bundle_rank_ext_first_more_last() {
    assert_eq!(optional_bundle_rank("@deepseek-ai/dsh-experimental-voice-input-bundle"), 0);
    assert_eq!(optional_bundle_rank("@deepseek-ai/dsh-experimental-plugin-market-bundle"), 1);
    assert_eq!(optional_bundle_rank("@deepseek-ai/dsh-experimental-auto-review"), 2);
    assert_eq!(
        optional_bundle_rank("@deepseek-ai/dsh-personalized-recommendations"),
        EXTENSION_BUNDLES.len() + 1
    );
    assert_eq!(
        optional_bundle_rank("@deepseek-ai/dsh-subagent-claude-code"),
        EXTENSION_BUNDLES.len() + 1 + MORE_BUNDLES.len() - 1
    );
    assert_eq!(optional_bundle_rank("@community/whale-pet"), EXTENSION_BUNDLES.len());
}

#[test]
fn test_group_packages_hides_hidden_and_splits_groups() {
    let packages = fixture_packages();
    let served = served_builtin_items(&["agent-loop".to_string(), "bash-sandbox".to_string()]);
    let groups = group_packages(&packages, &served);
    for group in groups
        .extensions
        .iter()
        .chain(groups.more.iter())
        .chain(groups.installed.iter())
    {
        assert!(!HIDDEN_BUNDLES.contains(&group.name.as_str()));
    }
    let names: Vec<&str> = groups
        .extensions
        .iter()
        .map(|pkg| pkg.name.as_str())
        .collect();
    assert_eq!(names, vec![
        "@deepseek-ai/dsh-experimental-auto-review",
        "@deepseek-ai/dsh-experimental-session-titles-bundle",
    ]);
    let more: Vec<&str> = groups.more.iter().map(|pkg| pkg.name.as_str()).collect();
    assert_eq!(more, vec![
        "@deepseek-ai/dsh-experimental-auto-review",
        "@deepseek-ai/dsh-experimental-session-titles-bundle",
        "@deepseek-ai/dsh-subagent-codex",
    ]);
    let installed: Vec<&str> = groups.installed.iter().map(|pkg| pkg.name.as_str()).collect();
    assert_eq!(installed, vec!["@community/whale-pet"]);
    let item_ids: Vec<&str> = groups.items.iter().map(|item| item.id).collect();
    assert_eq!(item_ids, vec!["agent-loop", "shell"]);
}

#[test]
fn test_served_builtin_items_follow_registration_order() {
    assert!(served_builtin_items(&[]).is_empty());
    assert!(served_builtin_items(&["web".to_string(), "subagent".to_string()])
        .iter()
        .map(|item| item.id)
        .eq(["subagent", "web-search"].iter().copied()));
    let all = served_builtin_items(&[
        "bash-sandbox".to_string(),
        "agent-loop".to_string(),
        "subagent".to_string(),
        "web".to_string(),
    ]);
    let ids: Vec<&str> = all.iter().map(|item| item.id).collect();
    assert_eq!(ids, vec!["subagent", "web-search", "agent-loop", "shell"]);
}

#[test]
fn test_hidden_bundles_match_upstream_sets() {
    assert_eq!(HIDDEN_BUNDLES.len(), 8);
    assert!(HIDDEN_BUNDLES.contains(&"@deepseek-ai/dsh-base"));
    assert!(HIDDEN_BUNDLES.contains(&"@deepseek-ai/dsh-experimental-tool-worktree"));
    assert_eq!(MORE_BUNDLES.len(), 10);
    assert_eq!(MORE_BUNDLES[0], "@deepseek-ai/dsh-personalized-recommendations");
    assert_eq!(MORE_BUNDLES[9], "@deepseek-ai/dsh-subagent-claude-code");
    assert_eq!(EXTENSION_BUNDLES[0], "@deepseek-ai/dsh-experimental-voice-input-bundle");
    assert_eq!(EXPERIMENTAL_BUNDLES.len(), 14);
    assert!(!EXPERIMENTAL_BUNDLES.contains(&"@deepseek-ai/dsh-subagent-codex"));
    assert!(EXPERIMENTAL_BUNDLES.contains(&"@deepseek-ai/dsh-experimental-liveshare"));
}

#[test]
fn test_unavailable_bundle_is_not_installed_and_flags_error() {
    let packages = fixture_packages();
    let codex = packages
        .iter()
        .find(|pkg| pkg.name == "@deepseek-ai/dsh-subagent-codex")
        .unwrap();
    assert!(!codex.installed);
    assert_eq!(codex.availability, "missing");
    assert_eq!(
        codex.install_target.as_ref().map(|target| target.version.as_str()),
        Some("1.1.0")
    );
}
