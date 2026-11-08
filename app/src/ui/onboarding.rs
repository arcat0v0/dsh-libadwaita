use crate::credentials::{CredentialStore, DEFAULT_PROVIDER};
use crate::ui::controller::SessionController;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::rc::Rc;

pub fn show_onboarding_dialog(
    parent: &impl IsA<gtk4::Widget>,
    controller: Rc<SessionController>,
    store: CredentialStore,
) {
    let dialog = adw::Dialog::builder()
        .title("Welcome to dsh")
        .content_width(480)
        .build();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let status_page = adw::StatusPage::builder()
        .icon_name("applications-science-symbolic")
        .title("Welcome to dsh")
        .description("Enter your DeepSeek API key to activate developer agent capabilities.")
        .build();
    content_box.append(&status_page);

    let pref_group = adw::PreferencesGroup::builder().build();

    let key_row = adw::PasswordEntryRow::builder()
        .title("DeepSeek API Key")
        .build();
    pref_group.add(&key_row);
    content_box.append(&pref_group);

    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(["destructive-action", "caption"])
        .visible(false)
        .build();
    content_box.append(&error_label);

    let button_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::End)
        .margin_top(8)
        .build();

    let later_btn = gtk4::Button::builder()
        .label("Configure Later")
        .build();
    let d_later = dialog.clone();
    later_btn.connect_clicked(move |_| {
        d_later.close();
    });
    button_box.append(&later_btn);

    let save_btn = gtk4::Button::builder()
        .label("Save & Continue")
        .css_classes(["suggested-action"])
        .build();

    let d_save = dialog.clone();
    let row_save = key_row.clone();
    let err_save = error_label.clone();
    let store_clone = store.clone();
    let ctrl_clone = controller.clone();

    save_btn.connect_clicked(move |_| {
        let key = row_save.text().to_string();

        if key.trim().is_empty() {
            err_save.set_label("API key cannot be empty");
            err_save.set_visible(true);
            return;
        }
        if let Some(failure) = crate::settings_cas::api_key_failure(&key) {
            err_save.set_label(match failure {
                "keyBlank" => "API key cannot be empty",
                _ => "API key must be printable ASCII without spaces, quote wrapping, or NAME=value paste",
            });
            err_save.set_visible(true);
            return;
        }
        let trimmed = key.trim();

        let _ = store_clone.set_key(DEFAULT_PROVIDER, trimmed);
        err_save.set_visible(false);

        ctrl_clone.inject_credential(DEFAULT_PROVIDER, trimmed);
        d_save.close();
    });

    button_box.append(&save_btn);
    content_box.append(&button_box);

    dialog.set_child(Some(&content_box));
    dialog.present(Some(parent));
}
