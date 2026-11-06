use crate::ui::controller::SessionController;
use crate::ui::conversation::ConversationView;
use crate::ui::desktop::{begin_background_continuation, build_shortcuts_window};
use crate::ui::right_dock::RightDock;
use crate::ui::settings::show_settings_dialog;
use crate::ui::sidebar::Sidebar;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    split_view: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    right_dock: RightDock,
    window_title: adw::WindowTitle,
    model_btn: gtk4::Button,
    controller: RefCell<Option<Rc<SessionController>>>,
    shortcuts_window: Rc<RefCell<Option<gtk4::ShortcutsWindow>>>,
    startup_banner: adw::Banner,
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let sidebar = Rc::new(Sidebar::new());
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
            .subtitle("New Session")
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

        let startup_banner = adw::Banner::builder()
            .title("dsh failed to start")
            .button_label("Retry")
            .revealed(false)
            .build();

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.add_top_bar(&startup_banner);
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

        let shortcuts_window = Rc::new(RefCell::new(None));
        let shortcuts_store = shortcuts_window.clone();
        let help_window_weak = window.downgrade();
        let show_help_action = gtk4::gio::SimpleAction::new("show-help-overlay", None);
        show_help_action.connect_activate(move |_, _| {
            let mut guard = shortcuts_store.borrow_mut();
            if let Some(w) = help_window_weak.upgrade() {
                if guard.is_none() {
                    *guard = Some(build_shortcuts_window(&w));
                }
                if let Some(shortcuts) = guard.as_ref() {
                    shortcuts.present();
                }
            }
        });
        window.add_action(&show_help_action);

        let window_weak = window.downgrade();
        let show_account_action = gtk4::gio::SimpleAction::new("show-account", None);
        show_account_action.connect_activate(move |_, _| {
            if let Some(w) = window_weak.upgrade() {
                show_settings_dialog(&w, None, None);
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
            model_btn,
            controller: RefCell::new(None),
            shortcuts_window,
            startup_banner,
        }
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        let ctrl_new = Rc::downgrade(&controller);
        let new_session_action = gtk4::gio::SimpleAction::new("new-session", None);
        new_session_action.connect_activate(move |_, _| {
            if let Some(c) = ctrl_new.upgrade() {
                c.prompt_new_session_portal();
            }
        });
        self.window.add_action(&new_session_action);

        let ctrl_select = Rc::downgrade(&controller);
        let select_session_action = gtk4::gio::SimpleAction::new(
            "select-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        select_session_action.connect_activate(move |_, param| {
            if let Some(id) = param.and_then(|p| p.get::<String>()) {
                if let Some(c) = ctrl_select.upgrade() {
                    c.follow_session(&id);
                }
            }
        });
        self.window.add_action(&select_session_action);

        let ctrl_archive = Rc::downgrade(&controller);
        let archive_session_action = gtk4::gio::SimpleAction::new(
            "archive-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        archive_session_action.connect_activate(move |_, param| {
            if let Some(id) = param.and_then(|p| p.get::<String>()) {
                if let Some(c) = ctrl_archive.upgrade() {
                    c.archive_session(&id);
                }
            }
        });
        self.window.add_action(&archive_session_action);

        let ctrl_model = Rc::downgrade(&controller);
        self.model_btn.connect_clicked(move |btn| {
            if let Some(c) = ctrl_model.upgrade() {
                c.open_model_picker(btn);
            }
        });


        let ctrl_close = Rc::downgrade(&controller);
        let window_for_close = self.window.clone();
        self.window.connect_close_request(move |_| {
            if let Some(c) = ctrl_close.upgrade() {
                if c.any_session_running() {
                    if let Some((session_id, title)) = c.running_session_info() {
                        if let Some(app) = window_for_close.application() {
                            let hold =
                                begin_background_continuation(&app, &session_id, &title);
                            c.set_background_hold(hold, &session_id);
                        }
                    }
                    window_for_close.set_visible(false);
                    return gtk4::glib::Propagation::Stop;
                }
            }
            gtk4::glib::Propagation::Proceed
        });
        *self.controller.borrow_mut() = Some(controller);
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

    pub fn sidebar(&self) -> &Rc<Sidebar> {
        &self.sidebar
    }

    pub fn conversation_view_rc(&self) -> &Rc<ConversationView> {
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

    pub fn shortcuts_window(&self) -> &Rc<RefCell<Option<gtk4::ShortcutsWindow>>> {
        &self.shortcuts_window
    }

    pub fn startup_banner(&self) -> &adw::Banner {
        &self.startup_banner
    }
}
