use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;

pub fn build_preferences_dialog() -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::builder().title("Settings").build();

    let general_page = adw::PreferencesPage::builder()
        .title("General")
        .icon_name("preferences-other-symbolic")
        .build();

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

    let models_group = adw::PreferencesGroup::builder()
        .title("Model Providers")
        .description("Configured providers and model catalog")
        .build();

    let deepseek_row = adw::ActionRow::builder()
        .title("DeepSeek")
        .subtitle("Configured (deepseek-chat, deepseek-coder)")
        .build();
    models_group.add(&deepseek_row);

    models_page.add(&models_group);
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

    let plugins_group = adw::PreferencesGroup::builder()
        .title("Installed Plugins")
        .description("Cordis bundles mounted in profile")
        .build();

    let bridge_plugin_row = adw::ActionRow::builder()
        .title("@dsh-libadwaita/bridge")
        .subtitle("Active (managed bundle)")
        .build();
    plugins_group.add(&bridge_plugin_row);

    let base_plugin_row = adw::ActionRow::builder()
        .title("@deepseek-ai/dsh-base")
        .subtitle("Active (core services)")
        .build();
    plugins_group.add(&base_plugin_row);

    plugins_page.add(&plugins_group);
    dialog.add(&plugins_page);

    dialog
}

pub fn show_settings_dialog(parent: &impl IsA<gtk4::Widget>) {
    let dialog = build_preferences_dialog();
    dialog.present(Some(parent));
}
