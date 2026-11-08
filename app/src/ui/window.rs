use crate::ui::controller::SessionController;
use crate::ui::conversation::ConversationView;
use crate::ui::desktop::{begin_background_continuation, build_shortcuts_window};
use crate::ui::layout::{
    SIDEBAR_AUTO_COLLAPSE, SIDEBAR_COLLAPSED, SIDEBAR_DEFAULT, SIDEBAR_DRAWER_BREAKPOINT,
};
use crate::ui::right_dock::RightDock;
use crate::ui::sidebar::{animations_enabled, PanelState, Sidebar};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct MainWindow {
    window: adw::ApplicationWindow,
    toast_overlay: adw::ToastOverlay,
    split_view: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    right_dock: Rc<RightDock>,
    controller: RefCell<Option<Rc<SessionController>>>,
    startup_banner: adw::Banner,
    content_stack: gtk4::Stack,
    startup_page: adw::StatusPage,
    startup_spinner: gtk4::Spinner,
    panel_state: Rc<PanelState>,
    chrome: Rc<SidebarChrome>,
}

struct SidebarLayout {
    collapsed: Cell<bool>,
    narrow_expanded: Cell<bool>,
    last_width: Cell<i32>,
}

impl SidebarLayout {
    fn new() -> Self {
        Self {
            collapsed: Cell::new(false),
            narrow_expanded: Cell::new(false),
            last_width: Cell::new(0),
        }
    }
}

struct SidebarChrome {
    window: adw::ApplicationWindow,
    split_view: adw::OverlaySplitView,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    layout: Rc<SidebarLayout>,
    width_animation: Rc<RefCell<Option<adw::TimedAnimation>>>,
    opener: gtk4::Button,
}

impl SidebarChrome {
    fn window_width(&self) -> i32 {
        if self.window.width() > 0 {
            self.window.width()
        } else {
            self.window.default_width()
        }
    }

    fn apply(&self, force: bool) {
        let width = self.window_width();
        if !force && width == self.layout.last_width.get() {
            return;
        }
        let animate = self.layout.last_width.get() != 0 && animations_enabled();
        self.layout.last_width.set(width);
        let drawer = f64::from(width) < SIDEBAR_DRAWER_BREAKPOINT;
        self.split_view.set_collapsed(drawer);
        if drawer {
            self.sidebar.set_rail(false, animate);
            self.animate_width(SIDEBAR_DEFAULT, animate);
            self.update_opener(!self.split_view.shows_sidebar());
            return;
        }
        self.update_opener(false);
        let rail = if f64::from(width) < SIDEBAR_AUTO_COLLAPSE {
            !self.layout.narrow_expanded.get()
        } else {
            self.layout.collapsed.get()
        };
        self.sidebar.set_rail(rail, animate);
        let target = if rail {
            SIDEBAR_COLLAPSED
        } else {
            SIDEBAR_DEFAULT
        };
        self.animate_width(target, animate);
    }

    fn toggle(&self) {
        let width = self.window_width();
        if f64::from(width) < SIDEBAR_DRAWER_BREAKPOINT {
            self.split_view.set_show_sidebar(false);
            return;
        }
        if f64::from(width) < SIDEBAR_AUTO_COLLAPSE {
            self.layout
                .narrow_expanded
                .set(!self.layout.narrow_expanded.get());
        } else {
            self.layout.collapsed.set(!self.layout.collapsed.get());
        }
        self.apply(true);
    }

    fn expand(&self) {
        let width = self.window_width();
        if f64::from(width) < SIDEBAR_DRAWER_BREAKPOINT {
            self.split_view.set_show_sidebar(true);
            return;
        }
        if f64::from(width) < SIDEBAR_AUTO_COLLAPSE {
            self.layout.narrow_expanded.set(true);
        } else {
            self.layout.collapsed.set(false);
        }
        self.apply(true);
    }

    fn update_opener(&self, visible: bool) {
        self.opener.set_visible(visible);
        self.conversation_view.set_drawer_opener_visible(visible);
    }

    fn animate_width(&self, target: f64, animate: bool) {        let split_view = &self.split_view;
        if !animate {
            split_view.set_min_sidebar_width(target);
            split_view.set_max_sidebar_width(target);
            return;
        }
        let from = split_view.min_sidebar_width();
        if (from - target).abs() < f64::EPSILON {
            return;
        }
        let split_for_target = split_view.clone();
        let step = adw::CallbackAnimationTarget::new(move |value| {
            let width = from + (target - from) * value;
            split_for_target.set_min_sidebar_width(width);
            split_for_target.set_max_sidebar_width(width);
        });
        let animation = adw::TimedAnimation::new(split_view, 0.0, 1.0, 300, step);
        animation.set_easing(adw::Easing::EaseInOutCubic);
        *self.width_animation.borrow_mut() = Some(animation.clone());
        animation.play();
    }
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        let panel_state = PanelState::new();
        let sidebar = Sidebar::new(panel_state.clone());
        let conversation_view = Rc::new(ConversationView::new());
        let right_dock = Rc::new(RightDock::new());

        let split_view = adw::OverlaySplitView::builder()
            .min_sidebar_width(SIDEBAR_DEFAULT)
            .max_sidebar_width(SIDEBAR_DEFAULT)
            .build();
        split_view.set_sidebar(Some(sidebar.widget()));

        let paned = gtk4::Paned::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .position(720)
            .shrink_start_child(false)
            .shrink_end_child(true)
            .build();
        paned.set_start_child(Some(conversation_view.widget()));
        paned.set_end_child(Some(right_dock.widget()));
        right_dock.widget().set_visible(false);

        split_view.set_content(Some(&paned));
        split_view.set_show_sidebar(true);

        let opener_btn = gtk4::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(crate::ui::copy::Copybook::for_code("system").open_sidebar)
            .css_classes(["dsh-icon-button", "dsh-chrome-opener"])
            .halign(gtk4::Align::Start)
            .valign(gtk4::Align::Start)
            .visible(false)
            .build();

        let window_controls = gtk4::WindowControls::new(gtk4::PackType::End);
        let controls_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::Start)
            .margin_top(6)
            .margin_end(8)
            .css_classes(["dsh-window-chrome"])
            .build();
        controls_box.append(&window_controls);

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
        let startup_handle = gtk4::WindowHandle::new();
        startup_handle.set_child(Some(&startup_page));
        let content_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        content_stack.add_named(&startup_handle, Some("startup"));
        content_stack.add_named(&split_view, Some("main"));
        content_stack.set_visible_child_name("startup");

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&content_stack));
        overlay.add_overlay(&opener_btn);
        overlay.add_overlay(&controls_box);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&startup_banner);
        toolbar_view.set_content(Some(&overlay));

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&toolbar_view));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("dsh")
            .content(&toast_overlay)
            .default_width(1280)
            .default_height(820)
            .css_classes(["dsh-window"])
            .build();

        let chrome = Rc::new(SidebarChrome {
            window: window.clone(),
            split_view: split_view.clone(),
            sidebar: sidebar.clone(),
            conversation_view: conversation_view.clone(),
            layout: Rc::new(SidebarLayout::new()),
            width_animation: Rc::new(RefCell::new(None)),
            opener: opener_btn.clone(),
        });
        {
            let chrome_for_resize = chrome.clone();
            let layout_hooked = Rc::new(Cell::new(false));
            window.connect_realize(move |window| {
                if layout_hooked.get() {
                    return;
                }
                let Some(surface) = window.surface() else {
                    return;
                };
                layout_hooked.set(true);
                let chrome_for_layout = chrome_for_resize.clone();
                surface.connect_layout(move |_, width, _| {
                    if width > 0 {
                        chrome_for_layout.apply(false);
                    }
                });
            });
        }        {
            let chrome_for_show = chrome.clone();
            split_view.connect_show_sidebar_notify(move |split_view| {
                let width = chrome_for_show.window_width();
                chrome_for_show.update_opener(
                    f64::from(width) < SIDEBAR_DRAWER_BREAKPOINT && !split_view.shows_sidebar(),
                );
            });
        }
        {
            let chrome_for_opener = chrome.clone();
            opener_btn.connect_clicked(move |_| {
                chrome_for_opener.split_view.set_show_sidebar(true);
            });
        }

        let toggle_sidebar_action = gtk4::gio::SimpleAction::new("toggle-sidebar", None);
        {
            let chrome_for_toggle = chrome.clone();
            toggle_sidebar_action.connect_activate(move |_, _| {
                chrome_for_toggle.toggle();
            });
        }
        window.add_action(&toggle_sidebar_action);

        let focus_search_action = gtk4::gio::SimpleAction::new("focus-search", None);
        {
            let chrome_for_search = chrome.clone();
            let search_entry = sidebar.search_entry().clone();
            focus_search_action.connect_activate(move |_, _| {
                chrome_for_search.expand();
                search_entry.grab_focus();
            });
        }
        window.add_action(&focus_search_action);

        let panel_plugins = panel_state.clone();
        let show_plugins_action = gtk4::gio::SimpleAction::new("show-plugins", None);
        show_plugins_action.connect_activate(move |_, _| {
            panel_plugins.set_active_panel(Some("plugins"));
        });
        window.add_action(&show_plugins_action);

        let panel_schedules = panel_state.clone();
        let show_schedules_action = gtk4::gio::SimpleAction::new("show-schedules", None);
        show_schedules_action.connect_activate(move |_, _| {
            panel_schedules.set_active_panel(Some("schedules"));
        });
        window.add_action(&show_schedules_action);

        let open_plugin_target = gtk4::gio::SimpleAction::new(
            "open-plugin-target",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        {
            let conv_for_plugin = conversation_view.clone();
            open_plugin_target.connect_activate(move |_, param| {
                if let Some(target) = param.and_then(|value| value.get::<String>()) {
                    conv_for_plugin.plugins_page().open_plugin_target(&target);
                }
            });
        }
        window.add_action(&open_plugin_target);

        {
            let conv_for_panel = conversation_view.clone();
            let chrome_for_panel = chrome.clone();
            panel_state.connect_changed(move |panel| {
                conv_for_panel.select_panel(panel.unwrap_or("conversation"));
                let width = f64::from(chrome_for_panel.window_width());
                if width < SIDEBAR_DRAWER_BREAKPOINT {
                    chrome_for_panel.split_view.set_show_sidebar(false);
                } else if panel.is_some() && width < SIDEBAR_AUTO_COLLAPSE {
                    chrome_for_panel.layout.narrow_expanded.set(false);
                    chrome_for_panel.apply(true);
                }
            });
        }


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

        Self {
            window,
            toast_overlay,
            split_view,
            sidebar,
            conversation_view,
            right_dock,
            controller: RefCell::new(None),
            startup_banner,
            content_stack,
            startup_page,
            startup_spinner,
            panel_state,
            chrome,
        }
    }

    pub fn controller(&self) -> Option<Rc<SessionController>> {
        self.controller.borrow().clone()
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
                c.start_new_session();
            }
        });
        self.window.add_action(&new_session_action);

        let ctrl_new_in_workspace = Rc::downgrade(&controller);
        let new_session_in_workspace = gtk4::gio::SimpleAction::new(
            "new-session-in-workspace",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        new_session_in_workspace.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_new_in_workspace.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.start_session_in_workspace(&id);
            }
        });
        self.window.add_action(&new_session_in_workspace);

        let ctrl_create_plugin = Rc::downgrade(&controller);
        let create_plugin_action = gtk4::gio::SimpleAction::new("create-plugin", None);
        create_plugin_action.connect_activate(move |_, _| {
            if let Some(c) = ctrl_create_plugin.upgrade() {
                c.start_plugin_creator_session();
            }
        });
        self.window.add_action(&create_plugin_action);

        let chrome_for_expand = self.chrome.clone();
        let ctrl_expand_add = Rc::downgrade(&controller);
        let expand_new_workspace = gtk4::gio::SimpleAction::new("expand-new-workspace", None);
        expand_new_workspace.connect_activate(move |_, _| {
            chrome_for_expand.expand();
            if let Some(c) = ctrl_expand_add.upgrade() {
                c.prompt_new_workspace();
            }
        });
        self.window.add_action(&expand_new_workspace);

        let ctrl_reveal = Rc::downgrade(&controller);
        let reveal_workspace = gtk4::gio::SimpleAction::new(
            "reveal-workspace",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        reveal_workspace.connect_activate(move |_, param| {
            if let (Some(c), Some(id)) = (
                ctrl_reveal.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.reveal_workspace(&id);
            }
        });
        self.window.add_action(&reveal_workspace);

        let ctrl_preset = Rc::downgrade(&controller);
        let preset_action = gtk4::gio::SimpleAction::new(
            "agent-preset",
            Some(&gtk4::glib::VariantTy::STRING),
        );
        preset_action.connect_activate(move |_, param| {
            if let (Some(c), Some(preset)) = (
                ctrl_preset.upgrade(),
                param.and_then(|value| value.get::<String>()),
            ) {
                c.select_agent_preset(&preset);
            }
        });
        self.window.add_action(&preset_action);

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
        self.window.connect_close_request(move |_| {            if let Some(c) = ctrl_close.upgrade() {
                c.remember_composer_draft();
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
        let dock_for_panel = self.conversation_view.header_dock_button().clone();
        let panel_for_dock = self.panel_state.clone();
        let ctrl_for_dock = Rc::downgrade(&controller);
        self.panel_state.connect_changed(move |_| {
            if let Some(c) = ctrl_for_dock.upgrade() {
                dock_for_panel.set_visible(
                    c.active_session_id().is_some() && panel_for_dock.active_panel().is_none(),
                );
            }
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

    pub fn panel_state(&self) -> &Rc<PanelState> {
        &self.panel_state
    }

    pub fn dock_button(&self) -> &gtk4::Button {
        self.conversation_view.header_dock_button()
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
