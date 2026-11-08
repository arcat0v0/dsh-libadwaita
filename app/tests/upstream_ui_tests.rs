use dsh_libadwaita::protocol::{
    PermissionCatalogResult, PermissionGetResult, PresetOptionItem, SessionEvent,
};
use dsh_libadwaita::ui::brand::*;
use dsh_libadwaita::ui::controller::permission_display_label;
use dsh_libadwaita::ui::conversation::{
    compact_token_count, context_breakdown_rows, context_occupancy, token_usage_total,
};
use dsh_libadwaita::ui::copy::*;
use dsh_libadwaita::ui::layout::*;
use dsh_libadwaita::ui::theme::*;
use dsh_libadwaita::ui::trajectory::filter_event_indices;
use dsh_libadwaita::ui_prefs::*;
use serde_json::json;

#[test]
fn test_shell_css_uses_libadwaita_theme_variables() {
    let css = shell_css();
    for token in [
        "--window-bg-color",
        "--window-fg-color",
        "--view-bg-color",
        "--view-fg-color",
        "--card-bg-color",
        "--popover-bg-color",
        "--headerbar-bg-color",
        "--headerbar-fg-color",
        "--sidebar-bg-color",
        "--sidebar-fg-color",
        "--accent-bg-color",
        "--accent-fg-color",
        "--destructive-bg-color",
        "--shade-color",
        "--border-color",
    ] {
        assert!(css.contains(token), "missing libadwaita token {token}");
    }
}

#[test]
fn test_shell_css_has_no_hardcoded_palette_or_global_chrome_overrides() {
    let css = shell_css();
    assert!(!css.contains('#'));
    assert!(!css.contains("rgba("));
    assert!(!css.contains("--dsh-bg-base"));
    assert!(!css.contains("--dsh-sidebar-fill"));
    assert!(!css.contains("--dsh-composer-fill"));
    assert!(!css.contains("font-family:"));
    assert!(!css.contains("popover > contents"));
    assert!(!css.contains("popover > arrow"));
    assert!(!css.contains("headerbar,"));
}

#[test]
fn test_shell_css_uses_color_mix_hover_and_active_fills() {
    let css = shell_css();
    assert!(css.contains("color-mix(in srgb, currentColor 10%, transparent)"));
    assert!(css.contains("color-mix(in srgb, currentColor 14%, transparent)"));
}

#[test]
fn test_upstream_shell_geometry_rules_are_present() {
    let css = shell_css();
    assert!(css.contains("--dsh-sidebar-width: 280px"));
    assert!(css.contains("--dsh-composer-max-width: 952px"));
    assert!(css.contains(".dsh-sidebar {"));
    assert!(!css.contains("min-width: 264px"));
    assert_eq!(SIDEBAR_MAX, 420.0);
    assert!(css.contains(".dsh-sidebar-rail"));
    assert!(css.contains("min-width: 56px"));
    assert!(css.contains(".dsh-new-session"));
    assert!(css.contains("min-height: 38px"));
    assert!(css.contains("margin: 0 4px 10px"));
    assert!(css.contains(".dsh-panel-row {"));
    assert!(css.contains(".dsh-panel-row.dsh-panel-row-active"));
    assert!(css.contains("min-height: 36px"));
    assert!(css.contains("padding: 7px 9px"));
    assert!(css.contains("border-radius: 12px"));
    assert!(css.contains("border-left: 3px solid transparent"));
    assert!(css.contains("border-right: 3px solid transparent"));
    assert!(css.contains("background-clip: padding-box"));
    assert!(css.contains(".dsh-window-chrome"));
    assert!(css.contains("windowcontrols > button"));
    assert!(css.contains(".dsh-chrome-opener"));
    assert!(css.contains(".dsh-composer-card"));
    assert!(css.contains("border-radius: 28px"));
    assert!(css.contains("var(--shade-color)"));
    assert!(css.contains(".dsh-send-button"));
    assert!(css.contains("border-radius: 999px"));
    assert!(css.contains("background: var(--accent-bg-color)"));
    assert!(css.contains("color: var(--accent-fg-color)"));
    assert!(css.contains(".dsh-hero-title"));
    assert!(css.contains("font-size: 26px"));
    assert!(css.contains(".dsh-preview-badge"));
}

#[test]
fn test_menu_button_hover_targets_inner_button_not_outer_widget() {
    let css = shell_css();
    assert!(css.contains(".dsh-hero-chip > button {"));
    assert!(css.contains(".dsh-hero-chip > button:hover"));
    assert!(!css.contains(".dsh-hero-chip:hover"));
    assert!(css.contains("menubutton.dsh-icon-button > button {"));
}

#[test]
fn test_empty_home_uses_upstream_chinese_copy() {
    let zh = Copybook::zh();
    assert_eq!(zh.new_session, "新会话");
    assert_eq!(zh.plugins, "插件");
    assert_eq!(zh.schedules, "自动化任务");
    assert_eq!(zh.more, "更多");
    assert_eq!(zh.workspaces, "工作区");
    assert_eq!(zh.default_workspace, "默认工作区");
    assert_eq!(zh.hero_headline, "探索未至之境");
    assert_eq!(zh.preview, "预览版");
    assert_eq!(zh.choose_workspace, "选择工作区");
    assert_eq!(zh.workspace_write, "工作区内修改");
    assert_eq!(zh.select_model, "请选择模型");
    assert_eq!(zh.composer_placeholder, "描述你想要构建的内容, / 调用指令, @ 文件或对话");
    assert_eq!(zh.search_sessions, "搜索会话");
    assert_eq!(zh.settings, "设置");
    assert_eq!(zh.account, "账号");
    assert_eq!(zh.pin_sidebar, "固定到左侧栏");
    assert_eq!(zh.unpin_sidebar, "从左侧栏取消固定");
    assert_eq!(zh.ungrouped, "未分组");
    assert_eq!(zh.workspace_required, "请先创建一个工作区");
}

#[test]
fn test_copybook_resolves_system_and_supported_locales() {
    assert_eq!(Copybook::for_code("zh").hero_headline, "探索未至之境");
    assert_eq!(Copybook::for_code("zh_CN.UTF-8").hero_headline, "探索未至之境");
    assert_eq!(Copybook::for_code("en").hero_headline, "Into the Unknown");
    let system = Copybook::for_code("system");
    assert!(system == Copybook::zh() || system == Copybook::en());
    assert_eq!(Copybook::for_code("de").hero_headline, "Into the Unknown");
    assert_eq!(UiPrefs::default().locale, None);
    assert!(!UiPrefs::default().dock_visible("new"));
}

#[test]
fn test_brand_artwork_is_embedded_upstream_mit_svg() {
    assert!(FISH_LOGO_SVG.starts_with("<svg"));
    assert!(FISH_LOGO_SVG.contains("viewBox=\"0 0 23.16 17.04\""));
    assert!(FISH_LOGO_SVG.contains("M22.9168 1.43018"));
    assert!(FISH_LOGO_SVG.contains("fill=\"currentColor\""));
    assert!(BRAND_WORDMARK_SVG.starts_with("<svg"));
    assert!(BRAND_WORDMARK_SVG.contains("viewBox=\"0 0 182 24\""));
    assert!(BRAND_WORDMARK_SVG.contains("dsh-wordmark-badge-clip"));
    assert!(!BRAND_WORDMARK_SVG.contains("<text"));
    assert!(BRAND_WORDMARK_SVG.contains("fill=\"currentColor\""));
    assert!(BRAND_NOTICE.contains("MIT"));
    assert!(BRAND_NOTICE.contains("DeepSeek"));
    assert_eq!(brand_colors(true), (BRAND_MARK_DARK_FG, BRAND_MARK_DARK_CUTOUT));
    assert_eq!(brand_colors(false), (BRAND_MARK_LIGHT_FG, BRAND_MARK_LIGHT_CUTOUT));
    assert_eq!(BRAND_MARK_COLORS.len(), 4);
}

#[test]
fn test_layout_solves_sidebar_auto_collapse_and_rightbar_clamp() {
    let wide = solve_columns(1440.0, 280.0, 648.0);
    assert_eq!(wide.sidebar, 280.0);
    assert_eq!(wide.rightbar, 648.0);
    assert_eq!(wide.center, 512.0);

    let auto_collapsed = solve_columns(960.0, 280.0, 432.0);
    assert_eq!(auto_collapsed.sidebar, 56.0);
    assert_eq!(auto_collapsed.center, 472.0);
    assert_eq!(auto_collapsed.rightbar, 432.0);

    let narrow = solve_columns(720.0, 280.0, 324.0);
    assert_eq!(narrow.sidebar, 0.0);
    assert_eq!(narrow.rightbar, 0.0);
    assert_eq!(narrow.center, 720.0);

    let clamped = solve_columns(1200.0, 500.0, 900.0);
    assert_eq!(clamped.sidebar, 420.0);
    assert_eq!(clamped.rightbar, 380.0);
    assert_eq!(clamped.center, 400.0);
}

#[test]
fn test_locale_preference_round_trips() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ui-prefs.json");
    let mut prefs = UiPrefs::default();
    prefs.locale = Some("en".to_string());
    prefs.save(&path).unwrap();
    assert_eq!(UiPrefs::load(&path).locale.as_deref(), Some("en"));
}

#[test]
fn test_projected_locale_namespace_is_recognized() {
    let settings = json!({"locale": {"language": "en"}});
    assert_eq!(locale_from_settings(&settings).as_deref(), Some("en"));
    assert_eq!(locale_from_settings(&json!({})), None);
}

fn trajectory_event(seq: i64, event_type: &str, data: serde_json::Value) -> SessionEvent {
    SessionEvent {
        event_type: event_type.to_string(),
        seq,
        time: seq as u64,
        data,
        ignorable: None,
        surface_op: None,
        source_event_seqs: None,
    }
}

#[test]
fn test_trajectory_filter_matches_all_events_and_text() {
    let events = vec![
        trajectory_event(1, "session/start", json!({"cwd": "/tmp"})),
        trajectory_event(2, "assistant/message", json!({"text": "hello"})),
        trajectory_event(3, "tool/result", json!({"name": "shell"})),
    ];
    assert_eq!(filter_event_indices(&events, ""), vec![0, 1, 2]);
    assert_eq!(filter_event_indices(&events, "  "), vec![0, 1, 2]);
    assert_eq!(filter_event_indices(&events, "assistant"), vec![1]);
    assert_eq!(filter_event_indices(&events, "SHELL"), vec![2]);
    assert_eq!(filter_event_indices(&events, "seq"), Vec::<usize>::new());
}

#[test]
fn test_plugins_page_css_uses_theme_tokens_only() {
    let css = shell_css();
    for class in [
        ".dsh-plugins-tag",
        ".dsh-plugins-tag-accent",
        ".dsh-plugins-tag-error",
        ".dsh-plugins-dot",
        ".dsh-plugins-dot-success",
        ".dsh-plugins-dot-ongoing",
        ".dsh-plugins-dot-error",
        ".dsh-plugins-skeleton-bar",
        ".dsh-plugins-skeleton-icon",
        "@keyframes dsh-plugins-pulse",
    ] {
        assert!(css.contains(class), "missing plugins css {class}");
    }
    assert!(css.contains("--success-bg-color"));
    assert!(css.contains("--warning-bg-color"));
    assert!(css.contains("--error-bg-color"));
}

#[test]
fn test_permission_display_label_prefers_product_names() {
    let catalog = PermissionCatalogResult {
        options: vec![PresetOptionItem {
            value: "workspace-write".to_string(),
            name: "Workspace Write".to_string(),
            description: Some("Write inside the workspace".to_string()),
        }],
        default_options: Vec::new(),
        default_preset: "workspace-write".to_string(),
    };
    let state = PermissionGetResult {
        current_value: "workspace-write".to_string(),
        catalog,
    };
    assert_eq!(permission_display_label(&state), "工作区内修改");

    let full_access = PermissionGetResult {
        current_value: "danger-full-access".to_string(),
        catalog: PermissionCatalogResult {
            options: Vec::new(),
            default_options: Vec::new(),
            default_preset: "workspace-write".to_string(),
        },
    };
    assert_eq!(permission_display_label(&full_access), "完全权限");
}

#[test]
fn test_token_usage_total_reads_bucket_fields() {
    let usage = json!({
        "uncachedInputTokens": 1200,
        "outputTokens": 345,
        "cacheReadTokens": 2000,
        "cacheWriteTokens": 55
    });
    assert_eq!(token_usage_total(&usage), Some(3600));
    assert_eq!(token_usage_total(&json!({})), None);
}

#[test]
fn test_compact_token_count_formats_magnitudes() {
    assert_eq!(compact_token_count(999), "999");
    assert_eq!(compact_token_count(1500), "1.5K");
    assert_eq!(compact_token_count(1_250_000), "1.2M");
}

#[test]
fn test_context_occupancy_prefers_projection_and_caps_percent() {
    let pressure = json!({
        "pressureTokens": 40000,
        "projectedTokens": 45000,
        "contextWindow": 100000
    });
    assert_eq!(context_occupancy(&pressure), Some((45000, 100000, 45)));

    let over = json!({"projectedTokens": 250000, "contextWindow": 100000});
    assert_eq!(context_occupancy(&over), Some((250000, 100000, 100)));

    let pressure_only = json!({"pressureTokens": 12000, "contextWindow": 64000});
    assert_eq!(context_occupancy(&pressure_only), Some((12000, 64000, 19)));

    assert_eq!(context_occupancy(&json!({"pressureTokens": 12000})), None);
    assert_eq!(context_occupancy(&json!({"contextWindow": 64000})), None);
}

#[test]
fn test_context_breakdown_rows() {
    let breakdown = json!({
        "systemTokens": 100,
        "toolsTokens": 50,
        "messageTokens": 25
    });
    assert_eq!(
        context_breakdown_rows(&breakdown),
        vec![("系统", 100), ("工具", 50), ("消息", 25)]
    );
    assert!(context_breakdown_rows(&json!({})).is_empty());
}
