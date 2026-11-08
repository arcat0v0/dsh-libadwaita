use gtk4::gio;
use gtk4::gio::prelude::*;
use gtk4::glib::object::IsA;
use libadwaita as adw;
use std::path::Path;
use std::process::Command;

pub fn should_notify_turn_completion(window_focused: bool, turn_duration_ms: u64) -> bool {
    if !window_focused {
        return true;
    }
    turn_duration_ms > 3000
}

pub fn truncate_snippet(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max_len).collect();
    format!("{}...", truncated)
}

pub fn terminal_command_candidates() -> Vec<(String, Vec<String>)> {
    let mut candidates = Vec::new();

    candidates.push(("xdg-terminal-exec".to_string(), Vec::new()));
    candidates.push((
        "ptyxis".to_string(),
        vec!["--new-window".to_string()],
    ));
    candidates.push((
        "gnome-terminal".to_string(),
        vec!["--".to_string()],
    ));

    if let Ok(term_env) = std::env::var("TERMINAL") {
        let trimmed = term_env.trim();
        if !trimmed.is_empty() {
            candidates.push((trimmed.to_string(), Vec::new()));
        }
    }

    candidates
}

pub fn open_path_in_default_app(path: &Path) {
    let uri = gio::File::for_path(path).uri();
    let launcher = gtk4::UriLauncher::new(&uri);
    launcher.launch(None::<&gtk4::Window>, None::<&gio::Cancellable>, |_| {});
}

pub fn open_terminal_in_dir(path: &Path) {
    let path_string = path.display().to_string();

    for (bin, extra_args) in terminal_command_candidates() {
        let mut cmd = Command::new(&bin);
        for arg in extra_args {
            if arg == "--" {
                break;
            }
            cmd.arg(arg);
        }

        if bin.contains("xdg-terminal-exec") {
            cmd.current_dir(path);
        } else if bin.contains("ptyxis") {
            cmd.arg("--working-directory").arg(&path_string);
        } else if bin.contains("gnome-terminal") {
            cmd.arg("--working-directory").arg(&path_string);
        } else {
            cmd.current_dir(path);
        }

        if let Ok(mut child) = cmd.spawn() {
            let _ = child.try_wait();
            return;
        }
    }

    open_path_in_default_app(path);
}

pub fn send_turn_completed_notification(
    app: &gtk4::Application,
    session_id: &str,
    title: &str,
    snippet: &str,
) {
    let notification = gio::Notification::new("Turn Completed");
    notification.set_body(Some(&format!("{}: {}", title, snippet)));
    notification.set_default_action_and_target_value(
        "app.open-session",
        Some(&session_id.to_variant()),
    );
    app.send_notification(Some(session_id), &notification);
}

pub fn send_background_notification(app: &gtk4::Application, session_id: &str, title: &str) {
    let notification = gio::Notification::new("dsh is continuing in the background");
    notification.set_body(Some(&format!("{} is still running.", title)));
    notification.set_default_action_and_target_value(
        "app.open-session",
        Some(&session_id.to_variant()),
    );
    app.send_notification(Some(&format!("{}-background", session_id)), &notification);
}

pub fn withdraw_notification(app: &gtk4::Application, id: &str) {
    app.withdraw_notification(id);
}

pub fn begin_background_continuation(
    app: &gtk4::Application,
    session_id: &str,
    title: &str,
    notify: bool,
) -> gio::ApplicationHoldGuard {
    let hold = app.hold();
    if notify {
        send_background_notification(app, session_id, title);
    }
    hold
}

pub fn build_shortcuts_window(parent: &impl IsA<gtk4::Window>) -> gtk4::ShortcutsWindow {
    let window = gtk4::ShortcutsWindow::builder()
        .title("Keyboard Shortcuts")
        .modal(true)
        .transient_for(parent)
        .section_name("shortcuts")
        .view_name("shortcuts")
        .build();

    let section = gtk4::ShortcutsSection::builder()
        .section_name("shortcuts")
        .view_name("shortcuts")
        .title("Shortcuts")
        .build();

    let navigation = gtk4::ShortcutsGroup::builder().title("Navigation").build();
    navigation.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("New Session")
            .accelerator("<Primary>n")
            .build(),
    );
    navigation.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Toggle Sidebar")
            .accelerator("<Primary>b")
            .build(),
    );
    navigation.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Close Window")
            .accelerator("<Primary>w")
            .build(),
    );
    section.add_group(&navigation);

    let actions = gtk4::ShortcutsGroup::builder().title("Actions").build();
    actions.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Send Prompt")
            .accelerator("Return")
            .build(),
    );
    actions.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Stop Turn")
            .accelerator("Escape")
            .subtitle("Press twice within 500 ms")
            .build(),
    );
    actions.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Settings")
            .accelerator("<Primary>comma")
            .build(),
    );
    actions.add_shortcut(
        &gtk4::ShortcutsShortcut::builder()
            .title("Shortcuts")
            .accelerator("<Primary>question")
            .build(),
    );
    section.add_group(&actions);

    window.add_section(&section);
    window
}

pub fn configure_system_style() {
    let style_manager = adw::StyleManager::default();
    let preference = crate::ui_prefs::UiPrefs::load(&crate::ui_prefs::default_ui_prefs_path())
        .theme
        .unwrap_or_else(|| "system".to_string());
    style_manager.set_color_scheme(match preference.as_str() {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    });
    let _ = style_manager.is_high_contrast();

    if let Some(settings) = gtk4::Settings::default() {
        let reduced_motion = !settings.is_gtk_enable_animations();
        settings.set_gtk_enable_animations(!reduced_motion);
    }

    let provider = gtk4::CssProvider::new();
    provider.load_from_string(crate::ui::theme::shell_css());
    if let Some(display) = gtk4::gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
