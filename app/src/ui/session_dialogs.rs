use gtk4::glib::object::IsA;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::rc::Rc;

pub fn show_rename_dialog<F: Fn(String) + 'static>(
    parent: &impl IsA<gtk4::Widget>,
    initial_title: &str,
    on_save: F,
) {
    let dialog = adw::Dialog::builder()
        .title("Rename Session")
        .content_width(420)
        .build();

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let entry_row = adw::EntryRow::builder()
        .title("Session title")
        .text(initial_title)
        .build();
    content.append(&entry_row);

    let error_label = gtk4::Label::builder()
        .label("Title cannot be empty")
        .css_classes(["destructive-action", "caption"])
        .visible(false)
        .halign(gtk4::Align::Start)
        .build();
    content.append(&error_label);

    let buttons = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::End)
        .margin_top(8)
        .build();

    let cancel_btn = gtk4::Button::builder().label("Cancel").build();
    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });
    buttons.append(&cancel_btn);

    let save_btn = gtk4::Button::builder()
        .label("Rename")
        .css_classes(["suggested-action"])
        .build();
    let dialog_save = dialog.clone();
    let entry_save = entry_row.clone();
    let error_save = error_label.clone();
    let on_save = Rc::new(on_save);
    save_btn.connect_clicked(move |_| {
        let title = entry_save.text().to_string();
        if title.trim().is_empty() {
            error_save.set_visible(true);
            return;
        }
        on_save(title.trim().to_string());
        dialog_save.close();
    });
    buttons.append(&save_btn);
    content.append(&buttons);

    dialog.set_child(Some(&content));
    dialog.present(Some(parent));
}

pub fn show_fork_dialog<F: Fn() + 'static>(
    parent: &impl IsA<gtk4::Widget>,
    source_title: &str,
    at_seq: i64,
    on_confirm: F,
) {
    let dialog = adw::Dialog::builder()
        .title("Fork Session")
        .content_width(460)
        .build();

    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let title = gtk4::Label::builder()
        .label("Create a child session from this point")
        .css_classes(["title-3"])
        .halign(gtk4::Align::Start)
        .build();
    content.append(&title);

    let source = gtk4::Label::builder()
        .label(&format!("Source: {}", source_title))
        .css_classes(["dim-label"])
        .halign(gtk4::Align::Start)
        .wrap(true)
        .build();
    content.append(&source);

    let boundary = gtk4::Label::builder()
        .label(&format!("atSeq: {}", at_seq))
        .css_classes(["heading"])
        .halign(gtk4::Align::Start)
        .build();
    content.append(&boundary);

    let buttons = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::End)
        .margin_top(8)
        .build();

    let cancel_btn = gtk4::Button::builder().label("Cancel").build();
    let dialog_cancel = dialog.clone();
    cancel_btn.connect_clicked(move |_| {
        dialog_cancel.close();
    });
    buttons.append(&cancel_btn);

    let fork_btn = gtk4::Button::builder()
        .label("Fork")
        .css_classes(["suggested-action"])
        .build();
    let dialog_fork = dialog.clone();
    let on_confirm = Rc::new(on_confirm);
    fork_btn.connect_clicked(move |_| {
        on_confirm();
        dialog_fork.close();
    });
    buttons.append(&fork_btn);
    content.append(&buttons);

    dialog.set_child(Some(&content));
    dialog.present(Some(parent));
}
