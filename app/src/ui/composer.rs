use crate::ui::copy::Copybook;
use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ComposerAttachmentView {
    pub id: String,
    pub name: String,
    pub image: bool,
    pub data: String,
    pub bytes: usize,
}

pub struct Composer {
    container: gtk4::Box,
    attachment_rail: gtk4::FlowBox,
    on_remove_attachment: Rc<RefCell<Box<dyn Fn(String)>>>,
    text_view: gtk4::TextView,
    send_button: gtk4::Button,
    attach_button: gtk4::Button,
    model_button: gtk4::Button,
    model_label: gtk4::Label,
    permission_button: gtk4::Button,
    permission_label: gtk4::Label,
    continue_button: gtk4::Button,
    plan_button: gtk4::Button,
    status_label: gtk4::Label,
}

impl Composer {
    pub fn new() -> Self {
        let copy = Copybook::for_code("system");
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .css_classes(["dsh-composer-shell"])
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .build();

        let card = gtk4::Frame::builder()
            .css_classes(["dsh-composer-card"])
            .hexpand(true)
            .halign(gtk4::Align::Fill)
            .build();
        let card_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();

        let editor_overlay = gtk4::Overlay::builder()
            .css_classes(["dsh-composer-entry"])
            .build();
        let scrolled = gtk4::ScrolledWindow::builder()
            .min_content_height(52)
            .max_content_height(336)
            .propagate_natural_height(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();
        let text_view = gtk4::TextView::builder()
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(14)
            .right_margin(8)
            .top_margin(4)
            .accepts_tab(false)
            .css_classes(["dsh-composer-entry"])
            .build();
        scrolled.set_child(Some(&text_view));
        editor_overlay.set_child(Some(&scrolled));
        let placeholder = gtk4::Label::builder()
            .label(copy.composer_placeholder)
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .css_classes(["dsh-composer-placeholder"])
            .margin_start(14)
            .margin_end(8)
            .margin_top(4)
            .build();
        placeholder.set_sensitive(false);
        editor_overlay.add_overlay(&placeholder);
        let placeholder_weak = placeholder.downgrade();
        text_view.buffer().connect_changed(move |buffer| {
            if let Some(placeholder) = placeholder_weak.upgrade() {
                let (start, end) = buffer.bounds();
                placeholder.set_visible(buffer.text(&start, &end, false).is_empty());
            }
        });
        let attachment_rail = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .max_children_per_line(6)
            .column_spacing(6)
            .row_spacing(6)
            .css_classes(["dsh-composer-attachments"])
            .margin_start(10)
            .margin_end(10)
            .margin_top(8)
            .visible(false)
            .build();
        card_content.append(&attachment_rail);
        card_content.append(&editor_overlay);

        let action_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .css_classes(["dsh-composer-toolbar"])
            .build();
        let attach_btn = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(copy.add_files_or_commands)
            .css_classes(["dsh-attach-button"])
            .build();
        action_bar.append(&attach_btn);

        let permission_button = gtk4::Button::builder()
            .css_classes(["dsh-composer-chip"])
            .build();
        let permission_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        let permission_label = gtk4::Label::builder()
            .label(copy.workspace_write)
            .build();
        permission_content.append(&permission_label);
        permission_content.append(&gtk4::Image::from_icon_name("pan-down-symbolic"));
        permission_button.set_child(Some(&permission_content));
        action_bar.append(&permission_button);
        let plan_button = gtk4::Button::builder()
            .label("计划模式")
            .tooltip_text("退出计划模式")
            .css_classes(["dsh-composer-chip", "dsh-plan-chip"])
            .visible(false)
            .build();
        action_bar.append(&plan_button);

        let spacer = gtk4::Box::builder().hexpand(true).build();
        action_bar.append(&spacer);
        let status_label = gtk4::Label::builder()
            .label("Ready")
            .css_classes(["dsh-composer-status"])
            .visible(false)
            .build();
        action_bar.append(&status_label);
        let continue_button = gtk4::Button::builder()
            .label("继续生成")
            .css_classes(["dsh-composer-chip", "dsh-continue-button"])
            .visible(false)
            .build();
        action_bar.append(&continue_button);

        let model_button = gtk4::Button::builder()
            .css_classes(["dsh-composer-chip"])
            .build();
        let model_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        let model_label = gtk4::Label::builder()
            .label(copy.select_model)
            .build();
        model_content.append(&model_label);
        model_content.append(&gtk4::Image::from_icon_name("pan-down-symbolic"));
        model_button.set_child(Some(&model_content));
        action_bar.append(&model_button);
        let send_button = gtk4::Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text(copy.send_message)
            .css_classes(["dsh-send-button"])
            .build();
        action_bar.append(&send_button);
        card_content.append(&action_bar);
        card.set_child(Some(&card_content));
        container.append(&card);

        Self {
            container,
            attachment_rail,
            on_remove_attachment: Rc::new(RefCell::new(Box::new(|_| {}))),
            text_view,
            send_button,
            attach_button: attach_btn,
            model_button,
            model_label,
            permission_button,
            permission_label,
            continue_button,
            plan_button,
            status_label,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn text_view(&self) -> &gtk4::TextView {
        &self.text_view
    }

    pub fn send_button(&self) -> &gtk4::Button {
        &self.send_button
    }

    pub fn attach_button(&self) -> &gtk4::Button {
        &self.attach_button
    }

    pub fn set_on_remove_attachment<F: Fn(String) + 'static>(&self, callback: F) {
        *self.on_remove_attachment.borrow_mut() = Box::new(callback);
    }

    pub fn set_attachments(&self, attachments: &[ComposerAttachmentView]) {
        while let Some(child) = self.attachment_rail.first_child() {
            self.attachment_rail.remove(&child);
        }
        self.attachment_rail.set_visible(!attachments.is_empty());
        for attachment in attachments {
            let item = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .css_classes(["card"])
                .margin_start(2)
                .margin_end(2)
                .margin_top(2)
                .margin_bottom(2)
                .build();
            if attachment.image {
                let picture = gtk4::Picture::builder()
                    .content_fit(gtk4::ContentFit::Cover)
                    .can_shrink(true)
                    .width_request(72)
                    .height_request(52)
                    .build();
                if let Some(bytes) = crate::attachments::decode_base64(&attachment.data) {
                    if let Ok(texture) = gtk4::gdk::Texture::from_bytes(
                        &gtk4::glib::Bytes::from_owned(bytes),
                    ) {
                        picture.set_paintable(Some(&texture));
                    }
                }
                let frame = gtk4::ScrolledWindow::builder()
                    .min_content_width(72)
                    .min_content_height(52)
                    .max_content_width(72)
                    .max_content_height(52)
                    .hscrollbar_policy(gtk4::PolicyType::Never)
                    .vscrollbar_policy(gtk4::PolicyType::Never)
                    .child(&picture)
                    .build();
                frame.set_size_request(72, 52);
                item.append(&frame);
            } else {
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(4)
                    .margin_start(6)
                    .margin_end(6)
                    .margin_top(4)
                    .margin_bottom(2)
                    .build();
                row.append(&gtk4::Image::from_icon_name("text-x-generic-symbolic"));
                let size =
                    crate::ui::conversation::compact_token_count(attachment.bytes as u64);
                let label = if size.is_empty() {
                    attachment.name.clone()
                } else {
                    format!("{} · {}", attachment.name, size)
                };
                row.append(
                    &gtk4::Label::builder()
                        .label(&label)
                        .css_classes(["caption"])
                        .build(),
                );
                item.append(&row);
            }
            let footer = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(2)
                .margin_start(4)
                .margin_end(4)
                .margin_bottom(3)
                .build();
            footer.append(
                &gtk4::Label::builder()
                    .label(&attachment.name)
                    .ellipsize(gtk4::pango::EllipsizeMode::Middle)
                    .max_width_chars(14)
                    .css_classes(["dim-label", "caption"])
                    .hexpand(true)
                    .build(),
            );
            let remove = gtk4::Button::builder()
                .icon_name("window-close-symbolic")
                .tooltip_text("Remove attachment")
                .css_classes(["flat", "circular"])
                .build();
            let on_remove = self.on_remove_attachment.clone();
            let id = attachment.id.clone();
            remove.connect_clicked(move |_| (on_remove.borrow())(id.clone()));
            footer.append(&remove);
            item.append(&footer);
            self.attachment_rail.append(&item);
        }
    }

    pub fn model_button(&self) -> &gtk4::Button {
        &self.model_button
    }

    pub fn permission_button(&self) -> &gtk4::Button {
        &self.permission_button
    }

    pub fn plan_button(&self) -> &gtk4::Button {
        &self.plan_button
    }

    pub fn set_plan_mode(&self, active: bool) {
        self.plan_button.set_visible(active);
    }

    pub fn set_permission_label(&self, label: &str) {
        let copy = Copybook::for_code("system");
        let label = label.trim();
        self.permission_label
            .set_label(if label.is_empty() { copy.workspace_write } else { label });
    }

    pub fn set_model_label(&self, label: &str) {
        let copy = Copybook::for_code("system");
        let label = label.trim();
        self.model_label
            .set_label(if label.is_empty() { copy.select_model } else { label });
    }

    pub fn set_is_running(&self, running: bool) {
        let copy = Copybook::for_code("system");
        if running {
            self.send_button.set_icon_name("media-playback-stop-symbolic");
            self.send_button.set_tooltip_text(Some(copy.stop_generating));
            self.send_button.set_css_classes(&["dsh-send-button", "dsh-running"]);
            self.status_label.set_label("Running");
        } else {
            self.send_button.set_icon_name("go-up-symbolic");
            self.send_button.set_tooltip_text(Some(copy.send_message));
            self.send_button.set_css_classes(&["dsh-send-button"]);
            self.status_label.set_label("Ready");
        }
    }

    pub fn continue_button(&self) -> &gtk4::Button {
        &self.continue_button
    }

    pub fn set_continue_visible(&self, visible: bool) {
        self.continue_button.set_visible(visible);
    }
}

impl Default for Composer {
    fn default() -> Self {
        Self::new()
    }
}
