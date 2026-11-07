use dsh_libadwaita::protocol::SessionEvent;
use dsh_libadwaita::ui::brand::*;
use dsh_libadwaita::ui::copy::*;
use dsh_libadwaita::ui::layout::*;
use dsh_libadwaita::ui::theme::*;
use dsh_libadwaita::ui::trajectory::filter_event_indices;
use dsh_libadwaita::ui_prefs::*;
use serde_json::json;

#[test]
fn test_upstream_dark_shell_tokens_are_preserved() {
    let dark = shell_css(true);
    assert!(dark.contains("--dsh-bg-base: #151517"));
    assert!(dark.contains("--dsh-sidebar-fill: #1b1b1c"));
    assert!(dark.contains("--dsh-composer-fill: #2c2c2e"));
    assert!(dark.contains("--dsh-label-primary: #f9fafb"));
    assert!(dark.contains("--dsh-accent-blue: #7aaaff"));
    assert!(dark.contains("--dsh-sidebar-width: 280px"));
    assert!(dark.contains("--dsh-composer-max-width: 952px"));
}

#[test]
fn test_light_theme_uses_upstream_light_palette() {
    let light = shell_css(false);
    assert!(light.contains("--dsh-bg-base: #ffffff"));
    assert!(light.contains("--dsh-sidebar-fill: #f5f6f7"));
    assert!(light.contains("--dsh-composer-fill: #ffffff"));
    assert!(light.contains("--dsh-label-primary: #0f1115"));
    assert!(light.contains("--dsh-accent-blue: #4176e6"));
}

#[test]
fn test_upstream_shell_geometry_rules_are_present() {
    let css = shell_css(true);
    assert!(css.contains(".dsh-sidebar {"));
    assert!(css.contains("min-width: 264px"));
    assert_eq!(SIDEBAR_MAX, 420.0);
    assert!(css.contains(".dsh-sidebar-rail"));
    assert!(css.contains("min-width: 56px"));
    assert!(css.contains(".dsh-composer-card"));
    assert!(css.contains("border-radius: 28px"));
    assert!(css.contains(".dsh-send-button"));
    assert!(css.contains("border-radius: 999px"));
    assert!(css.contains("background: var(--dsh-accent-blue)"));
    assert!(css.contains(".dsh-hero-title"));
    assert!(css.contains("font-size: 26px"));
    assert!(css.contains(".dsh-preview-badge"));
}

#[test]
fn test_menu_button_hover_targets_inner_button_not_outer_widget() {
    let css = shell_css(true);
    assert!(css.contains(".dsh-hero-chip > button {"));
    assert!(css.contains(".dsh-hero-chip > button:hover"));
    assert!(!css.contains(".dsh-hero-chip:hover"));
}

#[test]
fn test_popover_contents_and_arrow_share_menu_fill() {
    let css = shell_css(true);
    assert!(css.contains("popover > contents"));
    assert!(css.contains("popover > arrow"));
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
