use crate::credentials::{CredentialStore, DEFAULT_PROVIDER};
use crate::ui::controller::SessionController;
use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;

pub fn build_preferences_dialog(
    controller: Option<Rc<SessionController>>,
    store: Option<CredentialStore>,
) -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::builder().title("Settings").build();

    let general_page = adw::PreferencesPage::builder()
        .title("General")
        .icon_name("preferences-other-symbolic")
        .build();

    let appearance_group = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Interface theme and color preferences")
        .build();

    let theme_model = gtk4::StringList::new(&["System", "Light", "Dark"]);
    let theme_row = adw::ComboRow::builder()
        .title("Color Scheme")
        .model(&theme_model)
        .selected(0)
        .build();

    theme_row.connect_selected_notify(|row| {
        let sm = adw::StyleManager::default();
        match row.selected() {
            0 => sm.set_color_scheme(adw::ColorScheme::Default),
            1 => sm.set_color_scheme(adw::ColorScheme::ForceLight),
            2 => sm.set_color_scheme(adw::ColorScheme::ForceDark),
            _ => {}
        }
    });
    appearance_group.add(&theme_row);

    let dev_tools_row = adw::SwitchRow::builder()
        .title("Developer Tools")
        .subtitle("Enable trajectory view and execution traces")
        .active(true)
        .build();
    appearance_group.add(&dev_tools_row);

    general_page.add(&appearance_group);

    let general_group = adw::PreferencesGroup::builder()
        .title("System & Version")
        .description("Bridge protocol and dsh runtime status")
        .build();

    let dsh_version_row = adw::ActionRow::builder()
        .title("dsh Version")
        .subtitle("0.2.2-rc.1")
        .build();
    general_group.add(&dsh_version_row);

    let protocol_version_row = adw::ActionRow::builder()
        .title("Protocol Version")
        .subtitle("1")
        .build();
    general_group.add(&protocol_version_row);

    general_page.add(&general_group);
    dialog.add(&general_page);

    let models_page = adw::PreferencesPage::builder()
        .title("Models")
        .icon_name("network-server-symbolic")
        .build();

    let cred_store = store.unwrap_or_default();
    let current_key_opt = cred_store.get_key(DEFAULT_PROVIDER);
    let initial_masked = current_key_opt
        .as_ref()
        .map(|k| CredentialStore::mask_key(k))
        .unwrap_or_else(|| "Not configured".to_string());

    let deepseek_group = adw::PreferencesGroup::builder()
        .title("DeepSeek Provider")
        .description("API key configuration and credentials management")
        .build();

    let status_row = adw::ActionRow::builder()
        .title("Key Status")
        .subtitle(&format!("Masked: {}", initial_masked))
        .build();
    deepseek_group.add(&status_row);

    let key_entry_row = adw::PasswordEntryRow::builder()
        .title("New API Key")
        .build();
    deepseek_group.add(&key_entry_row);

    let update_row = adw::ActionRow::builder().build();
    let update_btn = gtk4::Button::builder()
        .label("Save / Rotate Key")
        .css_classes(["suggested-action"])
        .valign(gtk4::Align::Center)
        .build();

    let store_c = cred_store.clone();
    let ctrl_c = controller.clone();
    let entry_c = key_entry_row.clone();
    let status_c = status_row.clone();

    update_btn.connect_clicked(move |_| {
        let text = entry_c.text().to_string();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            let _ = store_c.set_key(DEFAULT_PROVIDER, trimmed);
            if let Some(ctrl) = &ctrl_c {
                ctrl.rotate_credential(DEFAULT_PROVIDER, trimmed);
            }
            status_c.set_subtitle(&format!(
                "Masked: {}",
                CredentialStore::mask_key(trimmed)
            ));
            entry_c.set_text("");
        }
    });
    update_row.add_suffix(&update_btn);
    deepseek_group.add(&update_row);
    models_page.add(&deepseek_group);

    let catalog_group = adw::PreferencesGroup::builder()
        .title("Model Catalog")
        .description("Supported models")
        .build();

    let chat_row = adw::ActionRow::builder()
        .title("deepseek-chat")
        .subtitle("General conversation and code assistance")
        .build();
    catalog_group.add(&chat_row);

    let coder_row = adw::ActionRow::builder()
        .title("deepseek-coder")
        .subtitle("Specialized code generation and refactoring")
        .build();
    catalog_group.add(&coder_row);

    let reasoner_row = adw::ActionRow::builder()
        .title("deepseek-reasoner")
        .subtitle("Reasoning and chain-of-thought model")
        .build();
    catalog_group.add(&reasoner_row);

    models_page.add(&catalog_group);
    dialog.add(&models_page);

    let account_page = adw::PreferencesPage::builder()
        .title("Account")
        .icon_name("avatar-default-symbolic")
        .build();

    let account_group = adw::PreferencesGroup::builder()
        .title("Account Status")
        .description("Local profile and authentication status")
        .build();

    let account_row = adw::ActionRow::builder()
        .title("Account")
        .subtitle("Local user profile")
        .build();
    account_group.add(&account_row);

    account_page.add(&account_group);
    dialog.add(&account_page);

    let shell_page = adw::PreferencesPage::builder()
        .title("Shell")
        .icon_name("utilities-terminal-symbolic")
        .build();

    let shell_group = adw::PreferencesGroup::builder()
        .title("Terminal Execution")
        .description("Sandbox policy and terminal execution parameters")
        .build();

    let timeout_row = adw::ActionRow::builder()
        .title("Command Timeout")
        .subtitle("300 seconds")
        .build();
    shell_group.add(&timeout_row);

    shell_page.add(&shell_group);
    dialog.add(&shell_page);

    let plugins_page = adw::PreferencesPage::builder()
        .title("Plugins")
        .icon_name("application-x-addon-symbolic")
        .build();

    let banner = adw::Banner::builder()
        .title("Plugin configuration updated. Restart dsh to apply changes.")
        .button_label("Restart Now")
        .revealed(false)
        .build();

    let ctrl_restart = controller.clone();
    let b_restart = banner.clone();
    banner.connect_button_clicked(move |_| {
        b_restart.set_revealed(false);
        if let Some(ctrl) = &ctrl_restart {
            ctrl.restart_supervisor();
        }
    });

    let banner_group = adw::PreferencesGroup::builder().build();
    banner_group.add(&banner);
    plugins_page.add(&banner_group);

    let core_group = adw::PreferencesGroup::builder()
        .title("Core Bundles")
        .description("Mandatory system bundles")
        .build();

    let bridge_row = adw::SwitchRow::builder()
        .title("@dsh-libadwaita/bridge")
        .subtitle("Active (managed bundle)")
        .active(true)
        .sensitive(false)
        .build();
    core_group.add(&bridge_row);

    let base_row = adw::SwitchRow::builder()
        .title("@deepseek-ai/dsh-base")
        .subtitle("Active (core runtime services)")
        .active(true)
        .sensitive(false)
        .build();
    core_group.add(&base_row);
    plugins_page.add(&core_group);

    let optional_group = adw::PreferencesGroup::builder()
        .title("Plugin Management")
        .description("Configurable Cordis bundles in profile")
        .build();

    let title_plugin = adw::SwitchRow::builder()
        .title("session-title-llm")
        .subtitle("Automatic LLM session title generation")
        .active(true)
        .build();

    let ctrl_p1 = controller.clone();
    let b_p1 = banner.clone();
    title_plugin.connect_active_notify(move |row| {
        b_p1.set_revealed(true);
        if let Some(ctrl) = &ctrl_p1 {
            ctrl.toggle_plugin("session-title-llm", row.is_active());
        }
    });
    optional_group.add(&title_plugin);

    let manager_plugin = adw::SwitchRow::builder()
        .title("tool-plugin-manager")
        .subtitle("Agent tool package discovery and installation")
        .active(false)
        .build();

    let ctrl_p2 = controller;
    let b_p2 = banner;
    manager_plugin.connect_active_notify(move |row| {
        b_p2.set_revealed(true);
        if let Some(ctrl) = &ctrl_p2 {
            ctrl.toggle_plugin("tool-plugin-manager", row.is_active());
        }
    });
    optional_group.add(&manager_plugin);

    plugins_page.add(&optional_group);
    dialog.add(&plugins_page);

    dialog
}

pub fn show_settings_dialog(
    parent: &impl IsA<gtk4::Widget>,
    controller: Option<Rc<SessionController>>,
    store: Option<CredentialStore>,
) {
    let dialog = build_preferences_dialog(controller, store);
    dialog.present(Some(parent));
}
