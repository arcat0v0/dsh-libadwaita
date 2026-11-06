use crate::protocol::{QueueItem, QueuePlacement};
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type TextCallback = Rc<RefCell<Box<dyn Fn(String, String)>>>;
type IdCallback = Rc<RefCell<Box<dyn Fn(String)>>>;
type ListCallback = Rc<RefCell<Box<dyn Fn(Vec<String>)>>>;

pub struct QueueDock {
    container: gtk4::Box,
    list_box: gtk4::Box,
    queued_ids: Rc<RefCell<Vec<String>>>,
    on_edit: TextCallback,
    on_remove: IdCallback,
    on_steer_item: IdCallback,
    on_steer_all: ListCallback,
}

impl QueueDock {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["card"])
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(0)
            .spacing(6)
            .visible(false)
            .build();

        let title = gtk4::Label::builder()
            .label("Queued messages")
            .css_classes(["title-4"])
            .halign(gtk4::Align::Start)
            .build();
        container.append(&title);

        let list_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        container.append(&list_box);

        let footer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();

        let queued_ids = Rc::new(RefCell::new(Vec::<String>::new()));
        let on_steer_all: ListCallback = Rc::new(RefCell::new(Box::new(|_| {})));
        let steer_all_btn = gtk4::Button::builder()
            .label("Steer All")
            .css_classes(["flat"])
            .build();
        let queued_ids_for_button = queued_ids.clone();
        let on_steer_all_for_button = on_steer_all.clone();
        steer_all_btn.connect_clicked(move |_| {
            let ids = queued_ids_for_button.borrow().clone();
            (on_steer_all_for_button.borrow())(ids);
        });
        footer.append(&steer_all_btn);
        container.append(&footer);

        Self {
            container,
            list_box,
            queued_ids,
            on_edit: Rc::new(RefCell::new(Box::new(|_, _| {}))),
            on_remove: Rc::new(RefCell::new(Box::new(|_| {}))),
            on_steer_item: Rc::new(RefCell::new(Box::new(|_| {}))),
            on_steer_all,
        }
    }

    pub fn set_on_edit<F: Fn(String, String) + 'static>(&self, callback: F) {
        *self.on_edit.borrow_mut() = Box::new(callback);
    }

    pub fn set_on_remove<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_remove.borrow_mut() = Box::new(callback);
    }

    pub fn set_on_steer_item<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_steer_item.borrow_mut() = Box::new(callback);
    }

    pub fn set_on_steer_all<F: Fn(Vec<String>) + 'static>(&self, callback: F) {
        *self.on_steer_all.borrow_mut() = Box::new(callback);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self, items: &[QueueItem]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        self.container.set_visible(!items.is_empty());
        let mut queued_ids = Vec::new();

        for item in items {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();

            let badge_text = match item.placement {
                QueuePlacement::Queued => "queued",
                QueuePlacement::Steering => "steering",
                QueuePlacement::Context => "context",
            };
            let badge = gtk4::Label::builder()
                .label(badge_text)
                .css_classes(["dim-label", "caption", "accent-badge"])
                .build();
            row.append(&badge);

            let entry = gtk4::Entry::builder()
                .text(&item.text())
                .hexpand(true)
                .build();
            row.append(&entry);

            let save_btn = gtk4::Button::builder()
                .icon_name("document-save-symbolic")
                .tooltip_text("Save queued text")
                .css_classes(["flat", "circular"])
                .build();
            let on_edit = self.on_edit.clone();
            let item_id = item.id.clone();
            save_btn.connect_clicked(move |_| {
                (on_edit.borrow())(item_id.clone(), entry.text().to_string());
            });
            row.append(&save_btn);

            if item.placement == QueuePlacement::Queued {
                queued_ids.push(item.id.clone());
                let steer_btn = gtk4::Button::builder()
                    .icon_name("media-seek-forward-symbolic")
                    .tooltip_text("Steer this queued message")
                    .css_classes(["flat", "circular"])
                    .build();
                let on_steer_item = self.on_steer_item.clone();
                let item_id = item.id.clone();
                steer_btn.connect_clicked(move |_| (on_steer_item.borrow())(item_id.clone()));
                row.append(&steer_btn);
            }

            let remove_btn = gtk4::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text("Remove queued message")
                .css_classes(["flat", "circular"])
                .build();
            let on_remove = self.on_remove.clone();
            let item_id = item.id.clone();
            remove_btn.connect_clicked(move |_| (on_remove.borrow())(item_id.clone()));
            row.append(&remove_btn);

            self.list_box.append(&row);
        }

        *self.queued_ids.borrow_mut() = queued_ids;
    }
}

impl Default for QueueDock {
    fn default() -> Self {
        Self::new()
    }
}
