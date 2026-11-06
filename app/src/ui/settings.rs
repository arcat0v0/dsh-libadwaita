use crate::connection::BridgeConnection;
use crate::credentials::{CredentialStore, DEFAULT_PROVIDER};
use crate::protocol::*;
use crate::ui::controller::{tokio_runtime, SessionController};
use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc::unbounded_channel, RwLock};

#[derive(Clone)]
pub struct SettingsDialogHandle {
    refresh: Rc<dyn Fn()>,
    install_state: Rc<dyn Fn(&str, &str)>,
    install_log: Rc<dyn Fn(Option<&str>, &str)>,
    inventory: Rc<dyn Fn(&PluginInventoryResult)>,
    operation: Rc<dyn Fn(&str, &serde_json::Value)>,
    refresh_catalog: Rc<dyn Fn()>,
}

impl SettingsDialogHandle {
    pub fn refresh(&self) {
        (self.refresh)();
    }

    pub fn plugin_install_state(&self, request_id: &str, phase: &str) {
        (self.install_state)(request_id, phase);
    }

    pub fn plugin_install_log(&self, request_id: Option<&str>, text: &str) {
        (self.install_log)(request_id, text);
    }

    pub fn plugin_inventory(&self, inventory: &PluginInventoryResult) {
        (self.inventory)(inventory);
    }

    pub fn plugin_operation(&self, label: &str, value: &serde_json::Value) {
        (self.operation)(label, value);
    }

    pub fn refresh_model_catalog(&self) {
        (self.refresh_catalog)();
    }
}

enum FormWidget {
    Bool(adw::SwitchRow),
    Number(adw::SpinRow),
    Text(adw::EntryRow),
    Enum(adw::ComboRow, Vec<String>),
}

struct FormField {
    key: String,
    widget: FormWidget,
}

struct BridgeViewData {
    settings: Result<SettingsResult, String>,
    descriptors: Result<SettingsDescribeResult, String>,
    goal: Result<GoalGetResult, String>,
}

fn rpc_receiver<P, R>(
    conn: Arc<RwLock<Option<BridgeConnection>>>,
    method: &'static str,
    params: P,
    timeout: Duration,
) -> tokio::sync::mpsc::UnboundedReceiver<Result<R, String>>
where
    P: serde::Serialize + Send + 'static,
    R: serde::de::DeserializeOwned + Send + 'static,
{
    let (sender, receiver) = unbounded_channel();
    tokio_runtime().spawn(async move {
        let conn_opt = conn.read().await.clone();
        let result = match conn_opt {
            Some(conn) => conn
                .call::<P, R>(method, params, timeout)
                .await
                .map_err(|err| err.to_string()),
            None => Err("Not connected".to_string()),
        };
        let _ = sender.send(result);
    });
    receiver
}
fn persist_setting(
    conn: Arc<RwLock<Option<BridgeConnection>>>,
    namespace: String,
    key: String,
    value: serde_json::Value,
    revision: u64,
    status: gtk4::Label,
) {
    let mut patch = serde_json::Map::new();
    patch.insert(key, value);
    let params = SettingsMutateParams {
        namespace: Some(namespace),
        patch: serde_json::Value::Object(patch),
        expected_revision: Some(revision),
    };
    let rx = rpc_receiver::<SettingsMutateParams, serde_json::Value>(
        conn,
        METHOD_SETTINGS_MUTATE,
        params,
        Duration::from_secs(15),
    );
    gtk4::glib::MainContext::default().spawn_local(async move {
        let mut rx = rx;
        match rx.recv().await {
            Some(Ok(_)) => status.set_label("Preference saved to Host settings"),
            Some(Err(err)) if err.contains("-32020") => {
                status.set_label("Settings conflict; reload and retry")
            }
            Some(Err(err)) => status.set_label(&format!("Host setting unavailable: {}", err)),
            None => status.set_label("Host setting unavailable"),
        }
    });
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

    let prefs = controller
        .as_ref()
        .map(|ctrl| ctrl.prefs_snapshot())
        .unwrap_or_default();
    let appearance_group = adw::PreferencesGroup::builder()
        .title("Appearance")
        .description("Theme, language, font size, and desktop notifications")
        .build();

    let locale_model = gtk4::StringList::new(&[
        "System",
        "English",
        "中文",
        "Deutsch",
        "Español",
        "Français",
        "日本語",
        "Português",
        "Русский",
    ]);
    let locale_row = adw::ComboRow::builder()
        .title("Language")
        .model(&locale_model)
        .selected(0)
        .sensitive(false)
        .build();
    let locale_config: Rc<RefCell<Option<(String, String, u64)>>> =
        Rc::new(RefCell::new(None));
    let locale_config_for_change = locale_config.clone();
    let locale_status = gtk4::Label::builder()
        .label("Checking locale support…")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let locale_status_for_change = locale_status.clone();
    let locale_controller = controller.clone();
    locale_row.connect_selected_notify(move |row| {
        let code = match row.selected() {
            1 => "en",
            2 => "zh",
            3 => "de",
            4 => "es",
            5 => "fr",
            6 => "ja",
            7 => "pt",
            8 => "ru",
            _ => "system",
        };
        if let (Some(config), Some(ctrl)) = (
            locale_config_for_change.borrow().clone(),
            locale_controller.as_ref(),
        ) {
            persist_setting(
                ctrl.connection_handle(),
                config.0,
                config.1,
                serde_json::json!(code),
                config.2,
                locale_status_for_change.clone(),
            );
        }
    });
    appearance_group.add(&locale_row);
    appearance_group.add(&locale_status);

    let theme_model = gtk4::StringList::new(&["System", "Light", "Dark"]);
    let theme_row = adw::ComboRow::builder()
        .title("Color Scheme")
        .model(&theme_model)
        .selected(0)
        .build();
    let theme_status = gtk4::Label::builder()
        .label("Theme and font preferences are stored locally.")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let theme_config: Rc<RefCell<Option<(String, String, u64)>>> =
        Rc::new(RefCell::new(None));
    let theme_config_for_change = theme_config.clone();
    let ctrl_theme = controller.clone();
    let theme_status_for_change = theme_status.clone();
    theme_row.connect_selected_notify(move |row| {
        let value = match row.selected() {
            1 => "light",
            2 => "dark",
            _ => "system",
        };
        if let Some(ctrl) = &ctrl_theme {
            ctrl.set_theme_preference(value);
            if let Some(config) = theme_config_for_change.borrow().clone() {
                persist_setting(
                    ctrl.connection_handle(),
                    config.0,
                    config.1,
                    serde_json::json!(value),
                    config.2,
                    theme_status_for_change.clone(),
                );
            }
        }
    });
    appearance_group.add(&theme_row);

    let font_adjustment = gtk4::Adjustment::new(
        prefs.font_size.unwrap_or(100) as f64,
        80.0,
        160.0,
        1.0,
        10.0,
        0.0,
    );
    let font_row = adw::SpinRow::builder()
        .title("Font Size")
        .subtitle("Applied to the current process")
        .adjustment(&font_adjustment)
        .digits(0)
        .build();
    let ctrl_font = controller.clone();
    font_row.connect_changed(move |row| {
        if let Some(ctrl) = &ctrl_font {
            ctrl.set_font_size(row.value() as u32);
        }
    });
    appearance_group.add(&font_row);

    let dev_tools_row = adw::SwitchRow::builder()
        .title("Developer Tools")
        .subtitle("Show trajectory view and execution traces")
        .active(prefs.developer_tools)
        .build();
    let ctrl_dev = controller.clone();
    dev_tools_row.connect_active_notify(move |row| {
        if let Some(ctrl) = &ctrl_dev {
            ctrl.set_developer_tools(row.is_active());
        }
    });
    appearance_group.add(&dev_tools_row);

    let notifications_row = adw::SwitchRow::builder()
        .title("Desktop Notifications")
        .subtitle("Notify when a background turn completes")
        .active(prefs.notifications_enabled)
        .build();
    let ctrl_notifications = controller.clone();
    notifications_row.connect_active_notify(move |row| {
        if let Some(ctrl) = &ctrl_notifications {
            ctrl.set_notifications_enabled(row.is_active());
        }
    });
    appearance_group.add(&notifications_row);

    appearance_group.add(&theme_status);

    general_page.add(&appearance_group);

    let host_page = adw::PreferencesPage::builder()
        .title("Host Settings")
        .icon_name("preferences-system-symbolic")
        .build();
    let host_status = gtk4::Label::builder()
        .label("Loading Host settings…")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    let host_status_group = adw::PreferencesGroup::builder().build();
    host_status_group.add(&host_status);
    host_page.add(&host_status_group);

    let describe_controller = controller.clone();
    let locale_row_for_schema = locale_row.clone();
    let locale_status_for_schema = locale_status.clone();
    let locale_config_for_schema = locale_config.clone();
    let theme_config_for_schema = theme_config.clone();
    let theme_row_for_schema = theme_row.clone();
    let theme_status_for_schema = theme_status.clone();
    let host_page_for_schema = host_page.clone();
    let host_status_for_schema = host_status.clone();
    let host_controller = controller.clone();
    if let Some(ctrl) = describe_controller {
        let rx = rpc_receiver::<serde_json::Value, SettingsDescribeResult>(
            ctrl.connection_handle(),
            METHOD_SETTINGS_DESCRIBE,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            let Some(Ok(result)) = rx.recv().await else {
                locale_status_for_schema.set_label("Locale settings unavailable");
                theme_status_for_schema.set_label("Host settings unavailable; preferences stored locally");
                host_status_for_schema.set_label("Host settings unavailable");
                return;
            };
            if let Some(descriptor) = result
                .namespaces
                .iter()
                .find(|descriptor| descriptor.ns == "locale")
            {
                if let Some(key) = crate::settings_model::find_key(
                    &descriptor.schema,
                    &["language", "locale", "lang"],
                ) {
                    if let Some(current) = descriptor.value.get(&key).and_then(|value| value.as_str()) {
                        let index = match current {
                            "en" => 1,
                            "zh" => 2,
                            "de" => 3,
                            "es" => 4,
                            "fr" => 5,
                            "ja" => 6,
                            "pt" => 7,
                            "ru" => 8,
                            _ => 0,
                        };
                        locale_row_for_schema.set_selected(index);
                    }
                    *locale_config_for_schema.borrow_mut() =
                        Some((descriptor.ns.clone(), key, descriptor.revision));
                    locale_row_for_schema.set_sensitive(true);
                    locale_status_for_schema.set_label("Locale is stored in Host settings");
                } else {
                    locale_status_for_schema.set_label("Locale namespace has no language field");
                }
            } else {
                locale_status_for_schema.set_label("Locale settings unavailable in this profile");
            }
            if let Some(descriptor) = result
                .namespaces
                .iter()
                .find(|descriptor| descriptor.ns == "ui-theme")
            {
                if let Some(key) = crate::settings_model::find_key(
                    &descriptor.schema,
                    &["colorScheme", "theme", "mode"],
                ) {
                    if let Some(current) = descriptor.value.get(&key).and_then(|value| value.as_str()) {
                        let index = match current {
                            "light" => 1,
                            "dark" => 2,
                            _ => 0,
                        };
                        theme_row_for_schema.set_selected(index);
                    }
                    *theme_config_for_schema.borrow_mut() =
                        Some((descriptor.ns.clone(), key, descriptor.revision));
                    theme_status_for_schema.set_label("Theme is stored in Host settings");
                }
            }

            let targets = [
                "agent-loop",
                "subagent",
                "subagent-model-selection",
                "web",
                "web-search-deepseek",
                "web-deepseek-api",
                "bash-sandbox",
                "pwsh-sandbox",
                "shell",
                "session-log-deepseek",
                "ui-settings",
            ];
            let mut rendered = 0;
            for target in targets {
                let Some(descriptor) = result
                    .namespaces
                    .iter()
                    .find(|descriptor| descriptor.ns == target)
                else {
                    continue;
                };
                let fields = crate::settings_model::fields_from_schema(
                    &descriptor.schema,
                    &descriptor.value,
                );
                if fields.is_empty() {
                    continue;
                }
                let group = adw::PreferencesGroup::builder()
                    .title(&descriptor.ns)
                    .description(format!("revision {}", descriptor.revision))
                    .build();
                let mut form_fields = Vec::new();
                for field in fields {
                    let widget = match field.kind {
                        crate::settings_model::FieldKind::Bool => {
                            let row = adw::SwitchRow::builder()
                                .title(&field.label)
                                .active(field.value.as_bool().unwrap_or(false))
                                .build();
                            group.add(&row);
                            FormWidget::Bool(row)
                        }
                        crate::settings_model::FieldKind::Number => {
                            let value = field.value.as_f64().unwrap_or(0.0);
                            let adjustment = gtk4::Adjustment::new(
                                value,
                                0.0,
                                1_000_000.0,
                                1.0,
                                10.0,
                                0.0,
                            );
                            let row = adw::SpinRow::builder()
                                .title(&field.label)
                                .adjustment(&adjustment)
                                .digits(0)
                                .build();
                            group.add(&row);
                            FormWidget::Number(row)
                        }
                        crate::settings_model::FieldKind::Text => {
                            let row = adw::EntryRow::builder()
                                .title(&field.label)
                                .text(
                                    field
                                        .value
                                        .as_str()
                                        .unwrap_or_default(),
                                )
                                .build();
                            group.add(&row);
                            FormWidget::Text(row)
                        }
                        crate::settings_model::FieldKind::Enum(options) => {
                            let model = gtk4::StringList::new(
                                &options
                                    .iter()
                                    .map(String::as_str)
                                    .collect::<Vec<_>>(),
                            );
                            let selected = field
                                .value
                                .as_str()
                                .and_then(|value| {
                                    options.iter().position(|option| option == value)
                                })
                                .unwrap_or(0) as u32;
                            let row = adw::ComboRow::builder()
                                .title(&field.label)
                                .model(&model)
                                .selected(selected)
                                .build();
                            group.add(&row);
                            FormWidget::Enum(row, options)
                        }
                        crate::settings_model::FieldKind::Unavailable => continue,
                    };
                    form_fields.push(FormField {
                        key: field.key,
                        widget,
                    });
                }
                let save_row = adw::ActionRow::builder().build();
                let save_button = gtk4::Button::builder()
                    .label("Save")
                    .css_classes(["suggested-action"])
                    .valign(gtk4::Align::Center)
                    .build();
                let status_label = gtk4::Label::builder()
                    .label("")
                    .xalign(0.0)
                    .css_classes(["dimension", "caption"])
                    .build();
                group.add(&status_label);
                let namespace = descriptor.ns.clone();
                let revision = descriptor.revision;
                let conn = host_controller
                    .as_ref()
                    .map(|ctrl| ctrl.connection_handle());
                let form_fields = Rc::new(RefCell::new(form_fields));
                let status_for_save = status_label.clone();
                save_button.connect_clicked(move |_| {
                    let Some(conn) = conn.clone() else {
                        return;
                    };
                    let mut patch = serde_json::Map::new();
                    for field in form_fields.borrow().iter() {
                        let value = match &field.widget {
                            FormWidget::Bool(row) => serde_json::json!(row.is_active()),
                            FormWidget::Number(row) => {
                                let value = row.value();
                                if value.fract() == 0.0 {
                                    serde_json::json!(value as i64)
                                } else {
                                    serde_json::json!(value)
                                }
                            }
                            FormWidget::Text(row) => serde_json::json!(row.text().to_string()),
                            FormWidget::Enum(row, options) => {
                                let index = row.selected() as usize;
                                serde_json::json!(options.get(index).cloned().unwrap_or_default())
                            }
                        };
                        patch.insert(field.key.clone(), value);
                    }
                    let params = SettingsMutateParams {
                        namespace: Some(namespace.clone()),
                        patch: serde_json::Value::Object(patch),
                        expected_revision: Some(revision),
                    };
                    let rx = rpc_receiver::<SettingsMutateParams, serde_json::Value>(
                        conn.clone(),
                        METHOD_SETTINGS_MUTATE,
                        params,
                        Duration::from_secs(15),
                    );
                    let status = status_for_save.clone();
                    gtk4::glib::MainContext::default().spawn_local(async move {
                        let mut rx = rx;
                        match rx.recv().await {
                            Some(Ok(_)) => status.set_label("Saved"),
                            Some(Err(err)) if err.contains("-32020") => {
                                status.set_label("Conflict; reload settings")
                            }
                            Some(Err(err)) => status.set_label(&format!("Save failed: {}", err)),
                            None => status.set_label("Save failed"),
                        }
                    });
                });
                save_row.add_suffix(&save_button);
                group.add(&save_row);
                host_page_for_schema.add(&group);
                rendered += 1;
            }
            if rendered == 0 {
                host_status_for_schema.set_label("No typed Host settings namespaces available");
            } else {
                host_status_for_schema.set_label(&format!("{} namespaces available", rendered));
            }
        });
    }
    dialog.add(&host_page);



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

    let records_group = adw::PreferencesGroup::builder()
        .title("Credential Records")
        .description("Metadata only; secret values are never rendered")
        .build();
    let records_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    records_group.add(&records_list);
    let records_result = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    records_group.add(&records_result);
    let records_refresh = gtk4::Button::builder().label("Refresh records").build();
    records_group.add(&records_refresh);
    models_page.add(&records_group);

    let records_conn = controller.as_ref().map(|ctrl| ctrl.connection_handle());
    let load_records = {
        let conn = records_conn.clone();
        let list = records_list.clone();
        let result_label = records_result.clone();
        Rc::new(move || {
            let Some(conn) = conn.clone() else {
                result_label.set_label("Not connected");
                return;
            };
            let rx = rpc_receiver::<serde_json::Value, CredentialRecordsListResult>(
                conn.clone(),
                METHOD_CREDENTIALS_RECORDS_LIST,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let list_for_ui = list.clone();
            let label_for_ui = result_label.clone();
            let conn_for_actions = conn.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = rx;
                let Some(Ok(result)) = rx.recv().await else {
                    label_for_ui.set_label("Credential records unavailable");
                    return;
                };
                while let Some(child) = list_for_ui.first_child() {
                    list_for_ui.remove(&child);
                }
                if result.records.is_empty() {
                    let row = adw::ActionRow::builder()
                        .title("No credential records")
                        .build();
                    list_for_ui.append(&row);
                    return;
                }
                for record in result.records {
                    let row = adw::ActionRow::builder()
                        .title(&record.key)
                        .subtitle(format!(
                            "{} · configured={} · writable={}",
                            record.kind, record.configured, record.writable
                        ))
                        .build();
                    let read = gtk4::Button::builder()
                        .label("Read")
                        .css_classes(["flat"])
                        .valign(gtk4::Align::Center)
                        .build();
                    let read_key = record.key.clone();
                    let read_conn = conn_for_actions.clone();
                    let read_label = label_for_ui.clone();
                    read.connect_clicked(move |_| {
                        let rx = rpc_receiver::<CredentialRecordReadParams, CredentialRecordReadResult>(
                            read_conn.clone(),
                            METHOD_CREDENTIALS_RECORDS_READ,
                            CredentialRecordReadParams {
                                key: read_key.clone(),
                            },
                            Duration::from_secs(10),
                        );
                        let label = read_label.clone();
                        gtk4::glib::MainContext::default().spawn_local(async move {
                            let mut rx = rx;
                            match rx.recv().await {
                                Some(Ok(read)) => label.set_label(&format!(
                                    "{} · kind={:?} · hasKey={:?} · env={:?}",
                                    read.key, read.kind, read.has_key, read.env_names
                                )),
                                _ => label.set_label("Record read failed"),
                            }
                        });
                    });
                    row.add_suffix(&read);
                    let edit = gtk4::Button::builder()
                        .label("Edit")
                        .css_classes(["flat"])
                        .valign(gtk4::Align::Center)
                        .build();
                    let edit_key = record.key.clone();
                    let edit_conn = conn_for_actions.clone();
                    let edit_label = label_for_ui.clone();
                    let edit_parent = list_for_ui.clone();
                    edit.connect_clicked(move |_| {
                        let dialog = adw::Dialog::builder()
                            .title("Edit Credential Record")
                            .content_width(480)
                            .build();
                        let content = gtk4::Box::builder()
                            .orientation(gtk4::Orientation::Vertical)
                            .spacing(10)
                            .margin_start(16)
                            .margin_end(16)
                            .margin_top(16)
                            .margin_bottom(16)
                            .build();
                        let entry = adw::EntryRow::builder()
                            .title("Record JSON")
                            .text(r#"{"kind":"api-key"}"#)
                            .build();
                        content.append(&entry);
                        let save = gtk4::Button::builder()
                            .label("Save")
                            .css_classes(["suggested-action"])
                            .build();
                        let dialog_for_save = dialog.clone();
                        let entry_for_save = entry.clone();
                        let key_for_save = edit_key.clone();
                        let conn_for_save = edit_conn.clone();
                        let label_for_save = edit_label.clone();
                        save.connect_clicked(move |_| {
                            let Ok(record) =
                                serde_json::from_str::<serde_json::Value>(&entry_for_save.text())
                            else {
                                label_for_save.set_label("Invalid record JSON");
                                return;
                            };
                            let rx = rpc_receiver::<
                                CredentialRecordModifyParams,
                                CredentialRecordMutationResult,
                            >(
                                conn_for_save.clone(),
                                METHOD_CREDENTIALS_RECORDS_MODIFY,
                                CredentialRecordModifyParams {
                                    key: key_for_save.clone(),
                                    record,
                                },
                                Duration::from_secs(10),
                            );
                            let label = label_for_save.clone();
                            gtk4::glib::MainContext::default().spawn_local(async move {
                                let mut rx = rx;
                                match rx.recv().await {
                                    Some(Ok(meta)) => label.set_label(&format!(
                                        "Updated {} · configured={}",
                                        meta.metadata.key, meta.metadata.configured
                                    )),
                                    _ => label.set_label("Record modify failed"),
                                }
                            });
                            dialog_for_save.close();
                        });
                        content.append(&save);
                        dialog.set_child(Some(&content));
                        dialog.present(Some(&edit_parent));
                    });
                    row.add_suffix(&edit);
                    let delete = gtk4::Button::builder()
                        .label("Delete")
                        .css_classes(["flat", "destructive-action"])
                        .valign(gtk4::Align::Center)
                        .build();
                    let delete_key = record.key.clone();
                    let delete_conn = conn_for_actions.clone();
                    let delete_label = label_for_ui.clone();
                    delete.connect_clicked(move |_| {
                        let rx = rpc_receiver::<CredentialRecordDeleteParams, CredentialRecordMutationResult>(
                            delete_conn.clone(),
                            METHOD_CREDENTIALS_RECORDS_DELETE,
                            CredentialRecordDeleteParams {
                                key: delete_key.clone(),
                            },
                            Duration::from_secs(10),
                        );
                        let label = delete_label.clone();
                        gtk4::glib::MainContext::default().spawn_local(async move {
                            let mut rx = rx;
                            match rx.recv().await {
                                Some(Ok(meta)) => label.set_label(&format!(
                                    "Deleted {} · configured={}",
                                    meta.metadata.key, meta.metadata.configured
                                )),
                                _ => label.set_label("Delete failed"),
                            }
                        });
                    });
                    row.add_suffix(&delete);
                    list_for_ui.append(&row);
                }
            });
        })
    };
    let load_records_for_button = load_records.clone();
    records_refresh.connect_clicked(move |_| load_records_for_button());
    load_records();

    let catalog_group = adw::PreferencesGroup::builder()
        .title("Model Catalog")
        .description("Providers and models reported by the bridge")
        .build();
    let catalog_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    catalog_group.add(&catalog_list);
    let catalog_status = gtk4::Label::builder()
        .label("Loading model catalog…")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    catalog_group.add(&catalog_status);
    let catalog_refresh = gtk4::Button::builder().label("Refresh catalog").build();
    catalog_group.add(&catalog_refresh);
    models_page.add(&catalog_group);

    let catalog_conn = controller.as_ref().map(|ctrl| ctrl.connection_handle());
    let load_catalog = {
        let conn = catalog_conn.clone();
        let list = catalog_list.clone();
        let status = catalog_status.clone();
        Rc::new(move || {
            let Some(conn) = conn.clone() else {
                status.set_label("Not connected");
                return;
            };
            let rx = rpc_receiver::<serde_json::Value, ModelCatalogResult>(
                conn,
                METHOD_MODEL_CATALOG,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let list_for_ui = list.clone();
            let status_for_ui = status.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = rx;
                let Some(Ok(catalog)) = rx.recv().await else {
                    status_for_ui.set_label("Model catalog unavailable");
                    return;
                };
                while let Some(child) = list_for_ui.first_child() {
                    list_for_ui.remove(&child);
                }
                if catalog.providers.is_empty() {
                    status_for_ui.set_label("No model providers configured");
                    return;
                }
                status_for_ui.set_label(&format!(
                    "default: {}/{}",
                    catalog.default_provider, catalog.default_model
                ));
                for provider in catalog.providers {
                    let row = adw::ActionRow::builder()
                        .title(&provider.name)
                        .subtitle(format!(
                            "{} · {}",
                            provider.id,
                            if provider.configured {
                                "configured"
                            } else {
                                "not configured"
                            }
                        ))
                        .build();
                    list_for_ui.append(&row);
                    for model in provider.models {
                        let model_row = adw::ActionRow::builder()
                            .title(&model.display_name)
                            .subtitle(format!(
                                "{} · context {}",
                                model.id,
                                model
                                    .context_window
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "?".to_string())
                            ))
                            .build();
                        list_for_ui.append(&model_row);
                    }
                }
            });
        })
    };
    let load_for_button = load_catalog.clone();
    catalog_refresh.connect_clicked(move |_| load_for_button());
    load_catalog();


    if let Some(ctrl) = controller.as_ref() {
        let rx = rpc_receiver::<serde_json::Value, ModelCatalogResult>(
            ctrl.connection_handle(),
            METHOD_MODEL_CATALOG,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let list_for_ui = catalog_list.clone();
        let status_for_ui = catalog_status.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            let Some(Ok(catalog)) = rx.recv().await else {
                status_for_ui.set_label("Model catalog unavailable");
                return;
            };
            while let Some(child) = list_for_ui.first_child() {
                list_for_ui.remove(&child);
            }
            if catalog.providers.is_empty() {
                status_for_ui.set_label("No model providers configured");
                return;
            }
            status_for_ui.set_label(&format!(
                "default: {}/{}",
                catalog.default_provider, catalog.default_model
            ));
            for provider in catalog.providers {
                let row = adw::ActionRow::builder()
                    .title(&provider.name)
                    .subtitle(format!(
                        "{} · {}",
                        provider.id,
                        if provider.configured {
                            "configured"
                        } else {
                            "not configured"
                        }
                    ))
                    .build();
                list_for_ui.append(&row);
                for model in provider.models {
                    let model_row = adw::ActionRow::builder()
                        .title(&model.display_name)
                        .subtitle(format!(
                            "{} · context {}",
                            model.id,
                            model
                                .context_window
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "?".to_string())
                        ))
                        .build();
                    list_for_ui.append(&model_row);
                }
            }
        });
    }

    dialog.add(&models_page);

    let account_page = adw::PreferencesPage::builder()
        .title("Account")
        .icon_name("avatar-default-symbolic")
        .build();

    let account_group = adw::PreferencesGroup::builder()
        .title("Account Status")
        .description("DeepSeek account state from the bridge")
        .build();
    let account_state_label = gtk4::Label::builder()
        .label("Not loaded")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    account_group.add(&account_state_label);
    let account_profile_label = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    account_group.add(&account_profile_label);
    let account_balance_label = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    account_group.add(&account_balance_label);
    let account_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    let account_refresh = gtk4::Button::builder().label("Refresh").build();
    let account_sign_out = gtk4::Button::builder()
        .label("Sign out")
        .css_classes(["destructive-action"])
        .build();
    account_actions.append(&account_refresh);
    account_actions.append(&account_sign_out);
    account_group.add(&account_actions);
    let browser_note = gtk4::Label::builder()
        .label("Browser sign-in is unavailable: it requires the Web Host callback, which this native client does not provide.")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    account_group.add(&browser_note);
    let account_conn = controller
        .as_ref()
        .map(|ctrl| ctrl.connection_handle());
    let load_account = {
        let conn = account_conn.clone();
        let state_label = account_state_label.clone();
        let profile_label = account_profile_label.clone();
        let balance_label = account_balance_label.clone();
        Rc::new(move || {
            let Some(conn) = conn.clone() else {
                state_label.set_label("Not connected");
                return;
            };
            state_label.set_label("Loading…");
            let status_rx = rpc_receiver::<serde_json::Value, AccountStatusResult>(
                conn.clone(),
                METHOD_ACCOUNT_STATUS,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let state_for_status = state_label.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = status_rx;
                if let Some(Ok(result)) = rx.recv().await {
                    let text = serde_json::to_string_pretty(&result.state)
                        .unwrap_or_else(|_| result.state.to_string());
                    state_for_status.set_label(&text);
                } else {
                    state_for_status.set_label("Account status unavailable");
                }
            });
            let profile_rx = rpc_receiver::<serde_json::Value, AccountProfileResult>(
                conn.clone(),
                METHOD_ACCOUNT_PROFILE,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let profile_for_ui = profile_label.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = profile_rx;
                match rx.recv().await {
                    Some(Ok(result)) => match result.profile {
                        Some(profile) => profile_for_ui.set_label(
                            &serde_json::to_string_pretty(&profile)
                                .unwrap_or_else(|_| profile.to_string()),
                        ),
                        None => profile_for_ui.set_label("No profile"),
                    },
                    _ => profile_for_ui.set_label("Profile unavailable"),
                }
            });
            let balance_rx = rpc_receiver::<serde_json::Value, AccountBalanceResult>(
                conn.clone(),
                METHOD_ACCOUNT_BALANCE,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let balance_for_ui = balance_label.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = balance_rx;
                match rx.recv().await {
                    Some(Ok(result)) => match result.balance {
                        Some(balance) => balance_for_ui.set_label(
                            &serde_json::to_string_pretty(&balance)
                                .unwrap_or_else(|_| balance.to_string()),
                        ),
                        None => balance_for_ui.set_label("Balance unavailable"),
                    },
                    _ => balance_for_ui.set_label("Balance unavailable"),
                }
            });
        })
    };
    let load_for_button = load_account.clone();
    account_refresh.connect_clicked(move |_| load_for_button());
    let sign_out_conn = account_conn.clone();
    let state_for_sign_out = account_state_label.clone();
    account_sign_out.connect_clicked(move |_| {
        let Some(conn) = sign_out_conn.clone() else {
            return;
        };
        let rx = rpc_receiver::<serde_json::Value, AccountSignOutResult>(
            conn,
            METHOD_ACCOUNT_SIGN_OUT,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let label = state_for_sign_out.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            match rx.recv().await {
                Some(Ok(result)) => label.set_label(&format!(
                    "Signed out: {}",
                    serde_json::to_string(&result.state).unwrap_or_default()
                )),
                _ => label.set_label("Sign out failed"),
            }
        });
    });
    load_account();

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

    let plugin_group = adw::PreferencesGroup::builder()
        .title("Plugin Inventory")
        .description("Installed bundles and components from the bridge")
        .build();
    let plugin_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    plugin_group.add(&plugin_list);
    let plugin_refresh = gtk4::Button::builder().label("Refresh").build();
    let plugin_group_actions = adw::ActionRow::builder().title("Inventory actions").build();
    plugin_group_actions.add_suffix(&plugin_refresh);
    plugin_group.add(&plugin_group_actions);
    let plugin_spec = adw::EntryRow::builder()
        .title("npm name / local path / Git URL")
        .build();
    plugin_group.add(&plugin_spec);
    let plugin_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    let inspect_btn = gtk4::Button::builder().label("Inspect").build();
    let install_btn = gtk4::Button::builder()
        .label("Install")
        .css_classes(["suggested-action"])
        .build();
    plugin_actions.append(&inspect_btn);
    plugin_actions.append(&install_btn);
    let cancel_install_btn = gtk4::Button::builder()
        .label("Cancel install")
        .css_classes(["destructive-action"])
        .sensitive(false)
        .build();
    plugin_actions.append(&cancel_install_btn);
    plugin_group.add(&plugin_actions);
    let plugin_result = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    plugin_group.add(&plugin_result);
    let install_state = gtk4::Label::builder()
        .label("")
        .xalign(0.0)
        .css_classes(["caption"])
        .build();
    plugin_group.add(&install_state);
    let install_log_view = gtk4::TextView::builder()
        .editable(false)
        .monospace(true)
        .build();
    let install_log_scroll = gtk4::ScrolledWindow::builder()
        .min_content_height(80)
        .max_content_height(180)
        .child(&install_log_view)
        .build();
    plugin_group.add(&install_log_scroll);
    let market_label = gtk4::Label::builder()
        .label("插件市场依赖 Web Host，本客户端暂不支持")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    plugin_group.add(&market_label);
    plugins_page.add(&plugin_group);

    let plugin_list_for_render = plugin_list.clone();
    let plugin_controller = controller.clone();
    let render_inventory: Rc<dyn Fn(&PluginInventoryResult)> =
        Rc::new(move |inventory: &PluginInventoryResult| {
            while let Some(child) = plugin_list_for_render.first_child() {
                plugin_list_for_render.remove(&child);
            }
            let mut entries: Vec<(&str, &serde_json::Value)> = Vec::new();
            for bundle in &inventory.bundles {
                entries.push(("bundle", bundle));
            }
            for plugin in &inventory.plugins {
                entries.push(("plugin", plugin));
            }
            if entries.is_empty() {
                let row = adw::ActionRow::builder()
                    .title("No installed plugins")
                    .subtitle("Use the spec field to install a bundle")
                    .build();
                plugin_list_for_render.append(&row);
                return;
            }
            for (kind, value) in entries {
                let name = value["name"]
                    .as_str()
                    .or_else(|| value["id"].as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let version = value["version"].as_str().unwrap_or("");
                let row = adw::ActionRow::builder()
                    .title(&name)
                    .subtitle(format!("{} · {}", kind, version))
                    .build();
                let remove = gtk4::Button::builder()
                    .label("Remove")
                    .css_classes(["flat", "destructive-action"])
                    .valign(gtk4::Align::Center)
                    .build();
                let name_for_remove = name.clone();
                let ctrl_remove = plugin_controller.clone();
                let parent_for_remove = plugin_list_for_render.clone();
                remove.connect_clicked(move |_| {
                    let dialog = adw::AlertDialog::builder()
                        .heading("Remove plugin?")
                        .body(format!("Remove {} from the profile?", name_for_remove))
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("remove", "Remove");
                    dialog.set_response_appearance(
                        "remove",
                        adw::ResponseAppearance::Destructive,
                    );
                    dialog.set_close_response("cancel");
                    let ctrl = ctrl_remove.clone();
                    let name = name_for_remove.clone();
                    dialog.choose(
                        Some(&parent_for_remove),
                        None::<&gtk4::gio::Cancellable>,
                        move |response| {
                            if response == "remove" {
                                if let Some(ctrl) = &ctrl {
                                    ctrl.plugin_remove(&name);
                                }
                            }
                        },
                    );
                });
                row.add_suffix(&remove);
                let versions = gtk4::Button::builder()
                    .label("Versions")
                    .css_classes(["flat"])
                    .valign(gtk4::Align::Center)
                    .build();
                let name_for_versions = name.clone();
                let ctrl_versions = plugin_controller.clone();
                versions.connect_clicked(move |_| {
                    if let Some(ctrl) = &ctrl_versions {
                        ctrl.plugin_versions(&name_for_versions);
                    }
                });
                row.add_suffix(&versions);
                let update = gtk4::Button::builder()
                    .label("Update…")
                    .css_classes(["flat"])
                    .valign(gtk4::Align::Center)
                    .build();
                let name_for_update = name.clone();
                let ctrl_for_update = plugin_controller.clone();
                let parent_for_update = plugin_list_for_render.clone();
                update.connect_clicked(move |_| {
                    let update_dialog = adw::Dialog::builder()
                        .title("Update Plugin")
                        .content_width(420)
                        .build();
                    let content = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Vertical)
                        .spacing(10)
                        .margin_start(16)
                        .margin_end(16)
                        .margin_top(16)
                        .margin_bottom(16)
                        .build();
                    let entry = adw::EntryRow::builder().title("Version").build();
                    content.append(&entry);
                    let run = gtk4::Button::builder()
                        .label("Update")
                        .css_classes(["suggested-action"])
                        .build();
                    let dialog_for_click = update_dialog.clone();
                    let entry_for_click = entry.clone();
                    let name_for_click = name_for_update.clone();
                    let ctrl_for_click = ctrl_for_update.clone();
                    run.connect_clicked(move |_| {
                        let version = entry_for_click.text().to_string();
                        if !version.trim().is_empty() {
                            if let Some(ctrl) = &ctrl_for_click {
                                ctrl.plugin_update(&name_for_click, version.trim());
                            }
                        }
                        dialog_for_click.close();
                    });
                    content.append(&run);
                    update_dialog.set_child(Some(&content));
                    update_dialog.present(Some(&parent_for_update));
                });
                row.add_suffix(&update);
                plugin_list_for_render.append(&row);
            }
        });

    let ctrl_refresh = controller.clone();
    plugin_refresh.connect_clicked(move |_| {
        if let Some(ctrl) = &ctrl_refresh {
            ctrl.refresh_plugins();
        }
    });
    let ctrl_inspect = controller.clone();
    let spec_for_inspect = plugin_spec.clone();
    inspect_btn.connect_clicked(move |_| {
        if let Some(ctrl) = &ctrl_inspect {
            ctrl.plugin_inspect(&spec_for_inspect.text());
        }
    });
    let ctrl_install = controller.clone();
    let spec_for_install = plugin_spec.clone();
    let install_request: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let install_request_for_click = install_request.clone();
    let cancel_for_install = cancel_install_btn.clone();
    install_btn.connect_clicked(move |_| {
        let spec = spec_for_install.text().to_string();
        if spec.trim().is_empty() {
            return;
        }
        let request_id = format!(
            "gui-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        *install_request_for_click.borrow_mut() = Some(request_id.clone());
        cancel_for_install.set_sensitive(true);
        if let Some(ctrl) = &ctrl_install {
            ctrl.plugin_install(spec.trim(), Some(request_id));
        }
    });
    let ctrl_cancel = controller.clone();
    let install_request_for_cancel = install_request.clone();
    let cancel_for_click = cancel_install_btn.clone();
    cancel_install_btn.connect_clicked(move |_| {
        if let Some(request_id) = install_request_for_cancel.borrow().clone() {
            if let Some(ctrl) = &ctrl_cancel {
                ctrl.plugin_cancel_install(&request_id);
            }
            cancel_for_click.set_sensitive(false);
        }
    });

    let inventory_cb = render_inventory.clone();
    let cancel_for_state = cancel_install_btn.clone();
    let state_cb = Rc::new({
        let state_label = install_state.clone();
        move |request_id: &str, phase: &str| {
            state_label.set_label(&format!("{} · {}", request_id, phase));
            cancel_for_state.set_sensitive(phase != "finished");
        }
    });
    let log_cb = Rc::new({
        let log_view = install_log_view.clone();
        move |_request_id: Option<&str>, text: &str| {
            let buffer = log_view.buffer();
            let mut end = buffer.end_iter();
            buffer.insert(&mut end, text);
            buffer.insert(&mut buffer.end_iter(), "\n");
            let mut end = buffer.end_iter();
            log_view.scroll_to_iter(&mut end, 0.0, false, 0.0, 0.0);
        }
    });
    let result_for_op = plugin_result.clone();
    let operation_cb = Rc::new(move |label: &str, value: &serde_json::Value| {
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        result_for_op.set_label(&format!("{}: {}", label, text));
    });

    let ctrl_initial = controller.clone();
    if let Some(ctrl) = &ctrl_initial {
        ctrl.refresh_plugins();
    }
    dialog.add(&plugins_page);

    let bridge_page = adw::PreferencesPage::builder()
        .title("Bridge")
        .icon_name("network-workgroup-symbolic")
        .build();

    let editor_group = adw::PreferencesGroup::builder()
        .title("Settings Editor")
        .description("Editable namespace values with expectedRevision CAS")
        .build();
    let editor_namespace = adw::EntryRow::builder().title("Namespace").build();
    editor_group.add(&editor_namespace);
    let editor_revision = adw::EntryRow::builder().title("Expected revision").build();
    editor_group.add(&editor_revision);
    let editor_patch = gtk4::TextView::builder()
        .editable(true)
        .monospace(true)
        .top_margin(4)
        .bottom_margin(4)
        .left_margin(4)
        .right_margin(4)
        .build();
    let editor_scroll = gtk4::ScrolledWindow::builder()
        .min_content_height(120)
        .max_content_height(240)
        .child(&editor_patch)
        .build();
    editor_group.add(&editor_scroll);
    let editor_banner = adw::Banner::builder()
        .title("Settings changed elsewhere. Draft kept; refresh to reconcile revision.")
        .revealed(false)
        .build();
    editor_group.add(&editor_banner);
    let editor_status = gtk4::Label::builder()
        .label("")
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build();
    editor_group.add(&editor_status);
    let editor_actions = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    let editor_reload = gtk4::Button::builder().label("Reload").build();
    let editor_save = gtk4::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    editor_actions.append(&editor_reload);
    editor_actions.append(&editor_save);
    editor_group.add(&editor_actions);
    bridge_page.add(&editor_group);

    let editor_conn = controller.as_ref().map(|ctrl| ctrl.connection_handle());
    let load_editor = {
        let conn = editor_conn.clone();
        let namespace_row = editor_namespace.clone();
        let revision_row = editor_revision.clone();
        let patch_view = editor_patch.clone();
        let status = editor_status.clone();
        Rc::new(move || {
            let Some(conn) = conn.clone() else {
                status.set_label("Not connected");
                return;
            };
            let rx = rpc_receiver::<serde_json::Value, SettingsDescribeResult>(
                conn.clone(),
                METHOD_SETTINGS_DESCRIBE,
                serde_json::json!({}),
                Duration::from_secs(10),
            );
            let namespace_row = namespace_row.clone();
            let revision_row = revision_row.clone();
            let patch_view = patch_view.clone();
            let status = status.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = rx;
                match rx.recv().await {
                    Some(Ok(result)) => {
                        if let Some(first) = result.namespaces.first() {
                            namespace_row.set_text(&first.ns);
                            revision_row.set_text(&first.revision.to_string());
                            let text = serde_json::to_string_pretty(&first.value)
                                .unwrap_or_else(|_| "{}".to_string());
                            patch_view.buffer().set_text(&text);
                            status.set_label("Loaded namespace");
                        } else {
                            status.set_label("No settings namespaces");
                        }
                    }
                    _ => status.set_label("settings/describe failed"),
                }
            });
        })
    };
    let reload_for_button = load_editor.clone();
    editor_reload.connect_clicked(move |_| reload_for_button());

    let save_conn = editor_conn.clone();
    let namespace_for_save = editor_namespace.clone();
    let revision_for_save = editor_revision.clone();
    let patch_for_save = editor_patch.clone();
    let status_for_save = editor_status.clone();
    let banner_for_save = editor_banner.clone();
    let load_for_save = load_editor.clone();
    editor_save.connect_clicked(move |_| {
        let Some(conn) = save_conn.clone() else {
            return;
        };
        let buffer = patch_for_save.buffer();
        let (start, end) = buffer.bounds();
        let patch_text = buffer.text(&start, &end, false).to_string();
        let patch: serde_json::Value = match serde_json::from_str(&patch_text) {
            Ok(value) => value,
            Err(err) => {
                status_for_save.set_label(&format!("Invalid JSON: {}", err));
                return;
            }
        };
        let namespace = namespace_for_save.text().to_string();
        let revision = revision_for_save.text().parse::<u64>().ok();
        let params = SettingsMutateParams {
            namespace: if namespace.trim().is_empty() {
                None
            } else {
                Some(namespace)
            },
            patch,
            expected_revision: revision,
        };
        let rx = rpc_receiver::<SettingsMutateParams, serde_json::Value>(
            conn,
            METHOD_SETTINGS_MUTATE,
            params,
            Duration::from_secs(15),
        );
        let status = status_for_save.clone();
        let banner = banner_for_save.clone();
        let load_for_save = load_for_save.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            match rx.recv().await {
                Some(Ok(value)) => {
                    banner.set_revealed(false);
                    status.set_label(&format!(
                        "Saved: {}",
                        serde_json::to_string(&value).unwrap_or_default()
                    ));
                    load_for_save();
                }
                Some(Err(err)) => {
                    if err.contains("-32020") || err.to_lowercase().contains("conflict") {
                        banner.set_revealed(true);
                        status.set_label("Conflict: draft retained");
                    } else {
                        status.set_label(&format!("Save failed: {}", err));
                    }
                }
                None => status.set_label("Save failed"),
            }
        });
    });
    load_editor();

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
        SettingsDialogHandle {
            refresh,
            install_state: state_cb,
            install_log: log_cb,
            inventory: inventory_cb,
            operation: operation_cb,
            refresh_catalog: load_catalog.clone(),
        },
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
