use crate::ui::composer::Composer;
use gtk4::prelude::*;
use libadwaita as adw;

pub struct ConversationView {
    stack: gtk4::Stack,
    composer: Composer,
    transcript_box: gtk4::Box,
    status_label: gtk4::Label,
    title_label: gtk4::Label,
}

impl ConversationView {
    pub fn new() -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let conversation_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let session_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let title_label = gtk4::Label::builder()
            .label("New Session")
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .css_classes(["title-4"])
            .build();
        session_header.append(&title_label);

        let status_label = gtk4::Label::builder()
            .label("Idle")
            .css_classes(["dim-label", "caption"])
            .build();
        session_header.append(&status_label);

        conversation_box.append(&session_header);

        let view_stack = adw::ViewStack::new();

        let chat_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let transcript_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let transcript_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();

        let welcome_bubble = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .css_classes(["card"])
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let welcome_title = gtk4::Label::builder()
            .label("Welcome to dsh")
            .css_classes(["title-3"])
            .halign(gtk4::Align::Start)
            .build();
        welcome_bubble.append(&welcome_title);

        let welcome_text = gtk4::Label::builder()
            .label("Ask questions, inspect workspace diffs, or run developer agent commands.")
            .wrap(true)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build();
        welcome_bubble.append(&welcome_text);
        transcript_box.append(&welcome_bubble);

        transcript_scroll.set_child(Some(&transcript_box));
        chat_box.append(&transcript_scroll);

        let chat_page = view_stack.add_titled(&chat_box, Some("chat"), "Chat");
        chat_page.set_icon_name(Some("chat-bubbles-symbolic"));

        let trajectory_page = adw::StatusPage::builder()
            .title("Trajectory")
            .description("Agent tool calls and execution trace")
            .icon_name("view-list-tree-symbolic")
            .build();
        let traj_page = view_stack.add_titled(&trajectory_page, Some("trajectory"), "Trajectory");
        traj_page.set_icon_name(Some("view-list-tree-symbolic"));

        let view_switcher = adw::ViewSwitcher::builder()
            .stack(&view_stack)
            .policy(adw::ViewSwitcherPolicy::Wide)
            .halign(gtk4::Align::Center)
            .build();

        let switcher_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::Center)
            .margin_bottom(4)
            .build();
        switcher_bar.append(&view_switcher);
        conversation_box.append(&switcher_bar);

        conversation_box.append(&view_stack);

        let composer = Composer::new();
        conversation_box.append(composer.widget());

        stack.add_named(&conversation_box, Some("conversation"));

        let plugins_page = adw::StatusPage::builder()
            .title("Plugin Manager")
            .description("Discover and manage installed Cordis plugin bundles")
            .icon_name("application-x-addon-symbolic")
            .build();
        stack.add_named(&plugins_page, Some("plugins"));

        let schedules_page = adw::StatusPage::builder()
            .title("Schedules")
            .description("Automated recurring agent tasks")
            .icon_name("alarm-symbolic")
            .build();
        stack.add_named(&schedules_page, Some("schedules"));

        Self {
            stack,
            composer,
            transcript_box,
            status_label,
            title_label,
        }
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    pub fn transcript_box(&self) -> &gtk4::Box {
        &self.transcript_box
    }

    pub fn set_title(&self, title: &str) {
        self.title_label.set_label(title);
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_label(status);
        self.composer.set_is_running(status == "Running");
    }

    pub fn select_panel(&self, panel_name: &str) {
        self.stack.set_visible_child_name(panel_name);
    }
}

impl Default for ConversationView {
    fn default() -> Self {
        Self::new()
    }
}
