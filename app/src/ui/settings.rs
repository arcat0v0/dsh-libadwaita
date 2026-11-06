use crate::credentials::{CredentialStore, DEFAULT_PROVIDER};
use crate::protocol::{GoalGetResult, SettingsDescribeResult, SettingsResult};
use crate::ui::controller::{tokio_runtime, SessionController};
use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;

#[derive(Clone)]
pub struct SettingsDialogHandle {
    refresh: Rc<dyn Fn()>,
}

impl SettingsDialogHandle {
    pub fn refresh(&self) {
        (self.refresh)();
    }
}

struct BridgeViewData {
    settings: Result<SettingsResult, String>,
    descriptors: Result<SettingsDescribeResult, String>,
    goal: Result<GoalGetResult, String>,
}
pub fn build_preferences_dialog(
    controller: Option<Rc<SessionController>>,
    store: Option<CredentialStore>,
) -> (adw::PreferencesDialog, SettingsDialogHandle) {
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

    let ctrl_p2 = controller.clone();
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

    let bridge_page = adw::PreferencesPage::builder()
        .title("Bridge")
        .icon_name("network-workgroup-symbolic")
        .build();

    let settings_group = adw::PreferencesGroup::builder()
        .title("Bridge Settings")
        .description("Read-only values and schema descriptors from dsh")
        .build();
    let settings_values_label = gtk4::Label::builder()
        .label("Loading settings…")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    settings_group.add(&settings_values_label);
    let descriptors_label = gtk4::Label::builder()
        .label("Loading descriptors…")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    settings_group.add(&descriptors_label);
    bridge_page.add(&settings_group);

    let goal_group = adw::PreferencesGroup::builder()
        .title("Goal Activity")
        .description("Read-only active goal for the current session")
        .build();
    let goal_title_label = gtk4::Label::builder()
        .label("Loading goal…")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["heading"])
        .build();
    goal_group.add(&goal_title_label);
    let goal_detail_label = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    goal_group.add(&goal_detail_label);
    bridge_page.add(&goal_group);

    let refresh_row = adw::ActionRow::builder().title("Live bridge views").build();
    let refresh_button = gtk4::Button::builder()
        .label("Refresh")
        .valign(gtk4::Align::Center)
        .build();
    refresh_row.add_suffix(&refresh_button);
    settings_group.add(&refresh_row);
    dialog.add(&bridge_page);

    let refresh_controller = controller.clone();
    let values_for_refresh = settings_values_label.clone();
    let descriptors_for_refresh = descriptors_label.clone();
    let goal_title_for_refresh = goal_title_label.clone();
    let goal_detail_for_refresh = goal_detail_label.clone();
    let refresh: Rc<dyn Fn()> = Rc::new(move || {
        values_for_refresh.set_label("Loading settings…");
        descriptors_for_refresh.set_label("Loading descriptors…");
        goal_title_for_refresh.set_label("Loading goal…");
        goal_detail_for_refresh.set_label("");

        let Some(ctrl) = refresh_controller.as_ref() else {
            values_for_refresh.set_label("Not connected");
            descriptors_for_refresh.set_label("Not connected");
            goal_title_for_refresh.set_label("Not connected");
            return;
        };
        let conn_arc = ctrl.connection_handle();
        let session_id = ctrl.active_session_id();
        let values_label = values_for_refresh.clone();
        let descriptors_label = descriptors_for_refresh.clone();
        let goal_title = goal_title_for_refresh.clone();
        let goal_detail = goal_detail_for_refresh.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<BridgeViewData>();

        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(data) = receiver.recv().await {
                match data.settings {
                    Ok(result) => {
                        let text = serde_json::to_string_pretty(&result.settings)
                            .unwrap_or_else(|_| "{}".to_string());
                        values_label.set_label(&text);
                    }
                    Err(err) => values_label.set_label(&format!("settings/get failed: {}", err)),
                }
                match data.descriptors {
                    Ok(result) => {
                        let text = result
                            .namespaces
                            .iter()
                            .map(|ns| format!("{} · revision {}", ns.ns, ns.revision))
                            .collect::<Vec<_>>()
                            .join("\n");
                        descriptors_label.set_label(if text.is_empty() {
                            "No descriptors"
                        } else {
                            &text
                        });
                    }
                    Err(err) => {
                        descriptors_label.set_label(&format!("settings/describe failed: {}", err))
                    }
                }
                match data.goal {
                    Ok(result) => match result.goal {
                        Some(goal) => {
                            goal_title.set_label(&format!(
                                "{} · {} · rev {}",
                                goal.id, goal.phase, goal.revision
                            ));
                            goal_detail.set_label(&format!(
                                "Objective: {}\nRounds: {}/{}\nActivation: {}\nCreated: {}\nUpdated: {}",
                                goal.objective,
                                goal.rounds_started,
                                goal.max_goal_rounds,
                                goal.activation,
                                goal.created_at,
                                goal.updated_at
                            ));
                        }
                        None => {
                            goal_title.set_label("No active goal");
                            goal_detail.set_label("The current session has no goal.");
                        }
                    },
                    Err(err) => {
                        goal_title.set_label("Goal unavailable");
                        goal_detail.set_label(&format!("goal/get failed: {}", err));
                    }
                }
            }
        });

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                let _ = sender.send(BridgeViewData {
                    settings: Err("Not connected".to_string()),
                    descriptors: Err("Not connected".to_string()),
                    goal: Err("Not connected".to_string()),
                });
                return;
            };
            let settings = conn
                .call::<crate::protocol::SettingsGetParams, SettingsResult>(
                    crate::protocol::METHOD_SETTINGS_GET,
                    crate::protocol::SettingsGetParams { keys: None },
                    std::time::Duration::from_secs(5),
                )
                .await
                .map_err(|err| err.to_string());
            let descriptors = conn
                .call::<serde_json::Value, SettingsDescribeResult>(
                    crate::protocol::METHOD_SETTINGS_DESCRIBE,
                    serde_json::json!({}),
                    std::time::Duration::from_secs(5),
                )
                .await
                .map_err(|err| err.to_string());
            let goal = match session_id {
                Some(session_id) => conn
                    .call::<crate::protocol::GoalGetParams, GoalGetResult>(
                        crate::protocol::METHOD_GOAL_GET,
                        crate::protocol::GoalGetParams { session_id },
                        std::time::Duration::from_secs(5),
                    )
                    .await
                    .map_err(|err| err.to_string()),
                None => Ok(GoalGetResult { goal: None }),
            };
            let _ = sender.send(BridgeViewData {
                settings,
                descriptors,
                goal,
            });
        });
    });

    let refresh_for_button = refresh.clone();
    refresh_button.connect_clicked(move |_| refresh_for_button());
    refresh();

    (
        dialog,
        SettingsDialogHandle { refresh },
    )
}

pub fn show_settings_dialog(
    parent: &impl IsA<gtk4::Widget>,
    controller: Option<Rc<SessionController>>,
    store: Option<CredentialStore>,
) -> SettingsDialogHandle {
    let (dialog, handle) = build_preferences_dialog(controller, store);
    dialog.present(Some(parent));
    handle
}
