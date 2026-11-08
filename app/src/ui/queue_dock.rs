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

    pub fn update(&self, session_id: &str, items: &[QueueItem]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        self.container.set_visible(!items.is_empty());
        let mut queued_ids = Vec::new();

        for item in items {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(4)
                .build();
            let top = gtk4::Box::builder()
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
                .valign(gtk4::Align::Center)
                .build();
            top.append(&badge);

            let entry = gtk4::Entry::builder()
                .text(&item.text())
                .hexpand(true)
                .build();
            top.append(&entry);

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
            top.append(&save_btn);

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
                top.append(&steer_btn);
            }

            let remove_btn = gtk4::Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text("Remove queued message")
                .css_classes(["flat", "circular"])
                .build();
            let on_remove = self.on_remove.clone();
            let item_id = item.id.clone();
            remove_btn.connect_clicked(move |_| (on_remove.borrow())(item_id.clone()));
            top.append(&remove_btn);
            row.append(&top);

            let attachments = item.attachments();
            if !attachments.is_empty() {
                let strip = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(6)
                    .margin_start(8)
                    .build();
                for attachment in attachments {
                    match attachment {
                        crate::protocol::QueueAttachment::Image(reference) => {
                            let picture = gtk4::Picture::builder()
                                .content_fit(gtk4::ContentFit::Cover)
                                .can_shrink(true)
                                .width_request(56)
                                .height_request(44)
                                .build();
                            if !session_id.is_empty() {
                                let image = crate::protocol::UserImageAttachment {
                                    name: reference.name.clone(),
                                    mime_type: reference.media_type.clone(),
                                    data: None,
                                    attachment_id: Some(reference.attachment_id.clone()),
                                    reference: Some(reference),
                                };
                                crate::ui::message_renderer::request_image(
                                    session_id,
                                    &image,
                                    &picture,
                                );
                            }
                            let frame = gtk4::ScrolledWindow::builder()
                                .min_content_width(56)
                                .min_content_height(44)
                                .max_content_width(56)
                                .max_content_height(44)
                                .hscrollbar_policy(gtk4::PolicyType::Never)
                                .vscrollbar_policy(gtk4::PolicyType::Never)
                                .child(&picture)
                                .build();
                            frame.set_size_request(56, 44);
                            strip.append(&frame);
                        }
                        crate::protocol::QueueAttachment::File { name, bytes } => {
                            let chip = gtk4::Box::builder()
                                .orientation(gtk4::Orientation::Horizontal)
                                .spacing(4)
                                .css_classes(["card"])
                                .margin_top(2)
                                .margin_bottom(2)
                                .build();
                            chip.append(&gtk4::Image::from_icon_name("text-x-generic-symbolic"));
                            let size = bytes
                                .map(crate::ui::conversation::compact_token_count)
                                .unwrap_or_default();
                            let label = if size.is_empty() {
                                name
                            } else {
                                format!("{} · {}", name, size)
                            };
                            chip.append(
                                &gtk4::Label::builder()
                                    .label(&label)
                                    .css_classes(["caption"])
                                    .build(),
                            );
                            strip.append(&chip);
                        }
                    }
                }
                row.append(&strip);
            }

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
