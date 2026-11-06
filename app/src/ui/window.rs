use crate::ui::controller::SessionController;
use crate::ui::conversation::ConversationView;
use crate::ui::desktop::{begin_background_continuation, build_shortcuts_window};
use crate::ui::right_dock::RightDock;
use crate::ui::settings::show_settings_dialog;
use crate::ui::sidebar::Sidebar;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    split_view: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    right_dock: Rc<RightDock>,
    window_title: adw::WindowTitle,
    model_btn: gtk4::Button,
    controller: RefCell<Option<Rc<SessionController>>>,
    shortcuts_window: Rc<RefCell<Option<gtk4::ShortcutsWindow>>>,
    startup_banner: adw::Banner,
    content_stack: gtk4::Stack,
    startup_page: adw::StatusPage,
    startup_spinner: gtk4::Spinner,
}
impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let sidebar = Rc::new(Sidebar::new());
        let conversation_view = Rc::new(ConversationView::new());
        let right_dock = Rc::new(RightDock::new());

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

        let dock_btn = gtk4::Button::builder()
            .icon_name("sidebar-show-right-symbolic")
            .tooltip_text("Toggle Right Dock")
            .action_name("win.toggle-dock")
            .build();
        header_bar.pack_end(&dock_btn);

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

        let startup_spinner = gtk4::Spinner::builder()
            .spinning(true)
            .width_request(36)
            .height_request(36)
            .build();
        let startup_page = adw::StatusPage::builder()
            .title("Starting dsh")
            .description("Connecting to the local harness")
            .child(&startup_spinner)
            .build();
        let content_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        content_stack.add_named(&startup_page, Some("startup"));
        content_stack.add_named(&split_view, Some("main"));
        content_stack.set_visible_child_name("startup");

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.add_top_bar(&startup_banner);
        toolbar_view.set_content(Some(&content_stack));

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
                let _ = show_settings_dialog(&w, None, None);
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
            content_stack,
            startup_page,
            startup_spinner,
        }
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        let ctrl_new_workspace = Rc::downgrade(&controller);
        let new_workspace_action = gtk4::gio::SimpleAction::new("new-workspace", None);
        new_workspace_action.connect_activate(move |_, _| {
            if let Some(c) = ctrl_new_workspace.upgrade() {
                c.prompt_new_workspace();
            }
        });
        self.window.add_action(&new_workspace_action);

        let ctrl_select_workspace = Rc::downgrade(&controller);
        let select_workspace_action = gtk4::gio::SimpleAction::new(
            "select-workspace",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        select_workspace_action.connect_activate(move |_, param| {
            let id = param.and_then(|p| p.get::<String>());
            if let (Some(c), Some(id)) = (ctrl_select_workspace.upgrade(), id) {
                c.switch_workspace(&id);
            }
        });
        self.window.add_action(&select_workspace_action);

        let ctrl_rename_workspace = Rc::downgrade(&controller);
        let rename_workspace_action = gtk4::gio::SimpleAction::new(
            "rename-workspace",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        rename_workspace_action.connect_activate(move |_, param| {
            let id = param.and_then(|p| p.get::<String>());
            if let (Some(c), Some(id)) = (ctrl_rename_workspace.upgrade(), id) {
                c.rename_workspace(&id);
            }
        });
        self.window.add_action(&rename_workspace_action);

        let ctrl_remove_workspace = Rc::downgrade(&controller);
        let remove_workspace_action = gtk4::gio::SimpleAction::new(
            "remove-workspace",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        remove_workspace_action.connect_activate(move |_, param| {
            let id = param.and_then(|p| p.get::<String>());
            if let (Some(c), Some(id)) = (ctrl_remove_workspace.upgrade(), id) {
                c.remove_workspace(&id);
            }
        });
        self.window.add_action(&remove_workspace_action);

        let ctrl_goal_edit = Rc::downgrade(&controller);
        let goal_edit = gtk4::gio::SimpleAction::new("goal-edit", None);
        goal_edit.connect_activate(move |_, _| {
            if let Some(c) = ctrl_goal_edit.upgrade() {
                c.goal_edit_dialog();
            }
        });
        self.window.add_action(&goal_edit);

        let ctrl_goal_pause = Rc::downgrade(&controller);
        let goal_pause = gtk4::gio::SimpleAction::new("goal-pause", None);
        goal_pause.connect_activate(move |_, _| {
            if let Some(c) = ctrl_goal_pause.upgrade() {
                c.goal_pause();
            }
        });
        self.window.add_action(&goal_pause);

        let ctrl_goal_resume = Rc::downgrade(&controller);
        let goal_resume = gtk4::gio::SimpleAction::new("goal-resume", None);
        goal_resume.connect_activate(move |_, _| {
            if let Some(c) = ctrl_goal_resume.upgrade() {
                c.goal_resume();
            }
        });
        self.window.add_action(&goal_resume);

        let ctrl_goal_clear = Rc::downgrade(&controller);
        let goal_clear = gtk4::gio::SimpleAction::new("goal-clear", None);
        goal_clear.connect_activate(move |_, _| {
            if let Some(c) = ctrl_goal_clear.upgrade() {
                c.goal_clear();
            }
        });
        self.window.add_action(&goal_clear);

        let ctrl_pin = Rc::downgrade(&controller);
        let pin_action = gtk4::gio::SimpleAction::new(
            "pin-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        pin_action.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_pin.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.toggle_pin_session(&id);
            }
        });
        self.window.add_action(&pin_action);

        let ctrl_unarchive = Rc::downgrade(&controller);
        let unarchive_action = gtk4::gio::SimpleAction::new(
            "unarchive-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        unarchive_action.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_unarchive.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.unarchive_session(&id);
            }
        });
        self.window.add_action(&unarchive_action);

        let ctrl_filter = Rc::downgrade(&controller);
        let filter_active = gtk4::gio::SimpleAction::new("filter-active", None);
        filter_active.connect_activate(move |_, _| {
            if let Some(c) = ctrl_filter.upgrade() {
                c.set_archive_filter(crate::ui_prefs::ArchiveFilter::Active);
            }
        });
        self.window.add_action(&filter_active);

        let ctrl_filter = Rc::downgrade(&controller);
        let filter_archived = gtk4::gio::SimpleAction::new("filter-archived", None);
        filter_archived.connect_activate(move |_, _| {
            if let Some(c) = ctrl_filter.upgrade() {
                c.set_archive_filter(crate::ui_prefs::ArchiveFilter::Archived);
            }
        });
        self.window.add_action(&filter_archived);

        let ctrl_filter = Rc::downgrade(&controller);
        let filter_all = gtk4::gio::SimpleAction::new("filter-all", None);
        filter_all.connect_activate(move |_, _| {
            if let Some(c) = ctrl_filter.upgrade() {
                c.set_archive_filter(crate::ui_prefs::ArchiveFilter::All);
            }
        });
        self.window.add_action(&filter_all);

        let ctrl_dock = Rc::downgrade(&controller);
        let toggle_dock = gtk4::gio::SimpleAction::new("toggle-dock", None);
        toggle_dock.connect_activate(move |_, _| {
            if let Some(c) = ctrl_dock.upgrade() {
                c.toggle_dock();
            }
        });
        self.window.add_action(&toggle_dock);

        let ctrl_open_child = Rc::downgrade(&controller);
        let open_child = gtk4::gio::SimpleAction::new(
            "open-child",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        open_child.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_open_child.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.open_child_session(&id);
            }
        });
        self.window.add_action(&open_child);

        let ctrl_interrupt_child = Rc::downgrade(&controller);
        let interrupt_child = gtk4::gio::SimpleAction::new(
            "interrupt-subagent",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        interrupt_child.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_interrupt_child.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.interrupt_subagent(&id);
            }
        });
        self.window.add_action(&interrupt_child);

        let ctrl_follow_job = Rc::downgrade(&controller);
        let follow_job = gtk4::gio::SimpleAction::new(
            "follow-job",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        follow_job.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_follow_job.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.follow_job(&id);
            }
        });
        self.window.add_action(&follow_job);

        let ctrl_kill_job = Rc::downgrade(&controller);
        let window_for_kill = self.window.clone();
        let kill_job = gtk4::gio::SimpleAction::new(
            "kill-job",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        kill_job.connect_activate(move |_, param| {
            let Some(id) = param.and_then(|value| value.get::<String>()) else {
                return;
            };
            let dialog = adw::AlertDialog::builder()
                .heading("Kill background job?")
                .body(format!("Confirm stopping job {}.", id))
                .build();
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("kill", "Kill");
            dialog.set_response_appearance("kill", adw::ResponseAppearance::Destructive);
            dialog.set_close_response("cancel");
            let ctrl = ctrl_kill_job.clone();
            dialog.choose(
                Some(&window_for_kill),
                None::<&gtk4::gio::Cancellable>,
                move |response| {
                    if response == "kill" {
                        if let Some(c) = ctrl.upgrade() {
                            c.kill_job(&id);
                        }
                    }
                },
            );
        });
        self.window.add_action(&kill_job);

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

        let ctrl_rename = Rc::downgrade(&controller);
        let rename_session_action = gtk4::gio::SimpleAction::new(
            "rename-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        rename_session_action.connect_activate(move |_, param| {
            if let Some(id) = param.and_then(|p| p.get::<String>()) {
                if let Some(c) = ctrl_rename.upgrade() {
                    c.open_rename_for_id(&id);
                }
            }
        });
        self.window.add_action(&rename_session_action);

        let ctrl_close_view = Rc::downgrade(&controller);
        let close_session_action = gtk4::gio::SimpleAction::new(
            "close-session",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        close_session_action.connect_activate(move |_, param| {
            if let Some(id) = param.and_then(|p| p.get::<String>()) {
                if let Some(c) = ctrl_close_view.upgrade() {
                    c.close_session(&id);
                }
            }
        });
        self.window.add_action(&close_session_action);

        let ctrl_fork = Rc::downgrade(&controller);
        let fork_session_action = gtk4::gio::SimpleAction::new(
            "fork-session",
            Some(&gtk4::glib::VariantTy::INT64),
        );
        fork_session_action.connect_activate(move |_, param| {
            if let Some(at_seq) = param.and_then(|p| p.get::<i64>()) {
                if let Some(c) = ctrl_fork.upgrade() {
                    c.open_fork_dialog(at_seq);
                }
            }
        });
        self.window.add_action(&fork_session_action);

        let ctrl_model = Rc::downgrade(&controller);
        self.model_btn.connect_clicked(move |btn| {
            if let Some(c) = ctrl_model.upgrade() {
                c.open_model_picker(btn);
            }
        });

        let ctrl_settings = Rc::downgrade(&controller);
        let show_settings_action = gtk4::gio::SimpleAction::new("show-account", None);
        show_settings_action.connect_activate(move |_, _| {
            if let Some(c) = ctrl_settings.upgrade() {
                c.open_settings();
            }
        });
        self.window.add_action(&show_settings_action);

        let ctrl_close = Rc::downgrade(&controller);
        let window_for_close = self.window.clone();
        self.window.connect_close_request(move |_| {
            if let Some(c) = ctrl_close.upgrade() {
                if c.any_session_running() {
                    if let Some((session_id, title)) = c.running_session_info() {
                        if let Some(app) = window_for_close.application() {
                            let hold = begin_background_continuation(
                                &app,
                                &session_id,
                                &title,
                                c.prefs_snapshot().notifications_enabled,
                            );
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

    pub fn right_dock(&self) -> &Rc<RightDock> {
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

    pub fn content_stack(&self) -> &gtk4::Stack {
        &self.content_stack
    }

    pub fn startup_page(&self) -> &adw::StatusPage {
        &self.startup_page
    }

    pub fn startup_spinner(&self) -> &gtk4::Spinner {
        &self.startup_spinner
    }
}
