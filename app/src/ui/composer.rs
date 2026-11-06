use gtk4::prelude::*;

pub struct Composer {
    container: gtk4::Box,
    text_view: gtk4::TextView,
    send_button: gtk4::Button,
    attach_button: gtk4::Button,
    model_button: gtk4::Button,
    permission_button: gtk4::Button,
    continue_button: gtk4::Button,
    plan_button: gtk4::Button,
    status_label: gtk4::Label,
}

impl Composer {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(12)
            .build();

        let frame = gtk4::Frame::builder()
            .css_classes(["view", "card"])
            .build();

        let input_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let scrolled = gtk4::ScrolledWindow::builder()
            .min_content_height(60)
            .max_content_height(180)
            .propagate_natural_height(true)
            .build();

        let text_view = gtk4::TextView::builder()
            .wrap_mode(gtk4::WrapMode::WordChar)
            .left_margin(4)
            .right_margin(4)
            .top_margin(4)
            .bottom_margin(4)
            .accepts_tab(false)
            .build();
        scrolled.set_child(Some(&text_view));
        input_box.append(&scrolled);

        let action_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(4)
            .build();

        let attach_btn = gtk4::Button::builder()
            .icon_name("mail-attachment-symbolic")
            .tooltip_text("Attach file or image")
            .css_classes(["flat", "circular"])
            .build();
        action_bar.append(&attach_btn);

        let model_button = gtk4::Button::builder()
            .label("deepseek-chat")
            .icon_name("pan-down-symbolic")
            .css_classes(["flat"])
            .build();
        action_bar.append(&model_button);

        let plan_button = gtk4::Button::builder()
            .label("Plan mode")
            .tooltip_text("Exit plan mode")
            .css_classes(["flat", "accent-badge"])
            .visible(false)
            .build();
        action_bar.append(&plan_button);

        let permission_button = gtk4::Button::builder()
            .label("default")
            .tooltip_text("Permission preset")
            .css_classes(["flat"])
            .build();
        action_bar.append(&permission_button);

        let status_label = gtk4::Label::builder()
            .label("Ready")
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build();
        action_bar.append(&status_label);

        let continue_button = gtk4::Button::builder()
            .label("Continue")
            .tooltip_text("Continue the failed generation")
            .css_classes(["flat", "suggested-action"])
            .visible(false)
            .build();
        action_bar.append(&continue_button);

        let send_button = gtk4::Button::builder()
            .icon_name("send-to-symbolic")
            .tooltip_text("Send prompt (Enter)")
            .css_classes(["suggested-action", "circular"])
            .build();
        action_bar.append(&send_button);

        input_box.append(&action_bar);
        frame.set_child(Some(&input_box));
        container.append(&frame);

        Self {
            container,
            text_view,
            send_button,
            attach_button: attach_btn.clone(),
            model_button,
            permission_button,
            continue_button,
            plan_button: plan_button.clone(),
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
        self.permission_button.set_label(label);
    }
    pub fn set_is_running(&self, running: bool) {
        if running {
            self.send_button.set_icon_name("media-playback-stop-symbolic");
            self.send_button.set_tooltip_text(Some("Cancel generation"));
            self.send_button.set_css_classes(&["destructive-action", "circular"]);
            self.status_label.set_label("Running");
        } else {
            self.send_button.set_icon_name("send-to-symbolic");
            self.send_button.set_tooltip_text(Some("Send prompt (Enter)"));
            self.send_button.set_css_classes(&["suggested-action", "circular"]);
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
