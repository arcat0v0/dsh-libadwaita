use crate::ui::conversation::ConversationView;
use crate::ui::right_dock::RightDock;
use crate::ui::settings::show_settings_dialog;
use crate::ui::sidebar::Sidebar;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    split_view: adw::OverlaySplitView,
    sidebar: Sidebar,
    conversation_view: Rc<ConversationView>,
    right_dock: RightDock,
    window_title: adw::WindowTitle,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let sidebar = Sidebar::new();
        let conversation_view = Rc::new(ConversationView::new());
        let right_dock = RightDock::new();

        let split_view = adw::OverlaySplitView::new();
        split_view.set_sidebar(Some(sidebar.widget()));

        let paned = gtk4::Paned::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .position(720)
            .build();
        paned.set_start_child(Some(conversation_view.widget()));
        paned.set_end_child(Some(right_dock.widget()));

        split_view.set_content(Some(&paned));
        split_view.set_show_sidebar(true);

        let header_bar = adw::HeaderBar::new();

        let toggle_sidebar_btn = gtk4::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text("Toggle Sidebar (Ctrl+B)")
            .action_name("win.toggle-sidebar")
            .build();
        header_bar.pack_start(&toggle_sidebar_btn);

        let window_title = adw::WindowTitle::builder()
            .title("dsh")
            .subtitle("Implementing Bridge Protocol")
            .build();
        header_bar.set_title_widget(Some(&window_title));

        let settings_btn = gtk4::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text("Settings (Ctrl+,)")
            .action_name("app.settings")
            .build();
        header_bar.pack_end(&settings_btn);

        let new_session_btn = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Session (Ctrl+N)")
            .action_name("win.new-session")
            .build();
        header_bar.pack_end(&new_session_btn);

        let model_btn = gtk4::Button::builder()
            .label("deepseek-chat")
            .css_classes(["flat"])
            .tooltip_text("Active Model")
            .build();
        header_bar.pack_end(&model_btn);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&split_view));

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("dsh")
            .content(&toast_overlay)
            .default_width(1280)
            .default_height(820)
            .build();

        let split_view_weak = split_view.downgrade();
        let toggle_sidebar_action = gtk4::gio::SimpleAction::new("toggle-sidebar", None);
        toggle_sidebar_action.connect_activate(move |_, _| {
            if let Some(sv) = split_view_weak.upgrade() {
                sv.set_show_sidebar(!sv.shows_sidebar());
            }
        });
        window.add_action(&toggle_sidebar_action);

        let toast_overlay_weak = toast_overlay.downgrade();
        let conv_weak = Rc::downgrade(&conversation_view);
        let title_weak = window_title.downgrade();
        let new_session_action = gtk4::gio::SimpleAction::new("new-session", None);
        new_session_action.connect_activate(move |_, _| {
            if let Some(to) = toast_overlay_weak.upgrade() {
                to.add_toast(adw::Toast::new("Started new session"));
            }
            if let Some(cv) = conv_weak.upgrade() {
                cv.set_title("New Session");
                cv.select_panel("conversation");
            }
            if let Some(wt) = title_weak.upgrade() {
                wt.set_subtitle("New Session");
            }
        });
        window.add_action(&new_session_action);

        let conv_plugins = Rc::downgrade(&conversation_view);
        let show_plugins_action = gtk4::gio::SimpleAction::new("show-plugins", None);
        show_plugins_action.connect_activate(move |_, _| {
            if let Some(cv) = conv_plugins.upgrade() {
                cv.select_panel("plugins");
            }
        });
        window.add_action(&show_plugins_action);

        let conv_schedules = Rc::downgrade(&conversation_view);
        let show_schedules_action = gtk4::gio::SimpleAction::new("show-schedules", None);
        show_schedules_action.connect_activate(move |_, _| {
            if let Some(cv) = conv_schedules.upgrade() {
                cv.select_panel("schedules");
            }
        });
        window.add_action(&show_schedules_action);

        let window_weak = window.downgrade();
        let show_account_action = gtk4::gio::SimpleAction::new("show-account", None);
        show_account_action.connect_activate(move |_, _| {
            if let Some(w) = window_weak.upgrade() {
                show_settings_dialog(&w);
            }
        });
        window.add_action(&show_account_action);

        Self {
            window,
            toast_overlay,
            split_view,
            sidebar,
            conversation_view,
            right_dock,
            window_title,
        }
    }

    pub fn window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn toast(&self, message: &str) {
        self.toast_overlay.add_toast(adw::Toast::new(message));
    }

    pub fn sidebar(&self) -> &Sidebar {
        &self.sidebar
    }

    pub fn conversation_view(&self) -> &ConversationView {
        &self.conversation_view
    }

    pub fn right_dock(&self) -> &RightDock {
        &self.right_dock
    }

    pub fn split_view(&self) -> &adw::OverlaySplitView {
        &self.split_view
    }

    pub fn window_title(&self) -> &adw::WindowTitle {
        &self.window_title
    }
}
