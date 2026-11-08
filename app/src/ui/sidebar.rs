use crate::protocol::{SessionListItem, WorkspaceView};
use crate::ui::brand::{brand_wordmark, fish_logo};
use crate::ui::copy::Copybook;
use crate::ui_prefs::{ArchiveFilter, SessionGrouping, SessionSort};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

const PANEL_PLUGINS: &str = "plugins";
const PANEL_SCHEDULES: &str = "schedules";

pub fn is_registered_panel(panel: &str) -> bool {
    registered_panels().iter().any(|(id, _, _)| *id == panel)
}

fn registered_panels() -> [(&'static str, &'static str, bool); 2] {
    [
        (PANEL_PLUGINS, "application-x-addon-symbolic", true),
        (PANEL_SCHEDULES, "alarm-symbolic", false),
    ]
}

pub struct PanelState {
    active: RefCell<Option<String>>,
    watchers: RefCell<Vec<Box<dyn Fn(Option<&str>)>>>,
}

impl PanelState {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            active: RefCell::new(None),
            watchers: RefCell::new(Vec::new()),
        })
    }

    pub fn active_panel(&self) -> Option<String> {
        self.active.borrow().clone()
    }

    pub fn set_active_panel(&self, panel: Option<&str>) {
        let next = panel.filter(|id| is_registered_panel(id)).map(str::to_string);
        if *self.active.borrow() == next {
            return;
        }
        *self.active.borrow_mut() = next.clone();
        for watcher in self.watchers.borrow().iter() {
            watcher(next.as_deref());
        }
    }

    pub fn connect_changed(&self, watcher: impl Fn(Option<&str>) + 'static) {
        self.watchers.borrow_mut().push(Box::new(watcher));
    }
}

pub struct Sidebar {
    stack: gtk4::Stack,
    wide: gtk4::Box,
    rail: gtk4::Box,
    tree_list: gtk4::ListBox,
    subagent_list: gtk4::ListBox,
    search_entry: gtk4::SearchEntry,
    filter_button: gtk4::MenuButton,
    more_button: gtk4::Button,
    more_menu_box: gtk4::Box,
    more_popover: gtk4::Popover,
    panel_state: Rc<PanelState>,
    wide_panel_rows: gtk4::Box,
    rail_panel_rows: gtk4::Box,
    panel_row_buttons: RefCell<Vec<(String, gtk4::Button)>>,
    unpinned_panels: RefCell<Vec<String>>,
    copy: Copybook,
    workspaces: RefCell<Vec<WorkspaceView>>,
    sessions: RefCell<Vec<SessionListItem>>,
    pinned: RefCell<HashSet<String>>,
    archive_filter: Cell<ArchiveFilter>,
    session_grouping: Cell<SessionGrouping>,
    session_sort: Cell<SessionSort>,
    filter_menu_buttons: RefCell<Vec<(FilterMenuKind, gtk4::Button)>>,
    running: RefCell<HashSet<String>>,
    unread: RefCell<HashSet<String>>,
    active_session: RefCell<Option<String>>,
    collapsed_groups: RefCell<HashSet<String>>,
    group_members: RefCell<HashMap<String, Vec<gtk4::Widget>>>,
    rail_active: Cell<bool>,
    fade_animation: Rc<RefCell<Option<adw::TimedAnimation>>>,
}

impl Sidebar {
    pub fn new(panel_state: Rc<PanelState>) -> Rc<Self> {
        let copy = Copybook::for_code("system");
        let stack = gtk4::Stack::builder().build();
        stack.set_hhomogeneous(false);

        let wide = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-sidebar"])
            .build();

        let brand_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["dsh-brand-row"])
            .build();
        let brand_handle = gtk4::WindowHandle::new();
        brand_handle.set_child(Some(&brand_row));
        let brand_button = gtk4::Button::builder()
            .action_name("win.new-session")
            .tooltip_text(copy.create_session)
            .css_classes(["dsh-brand-button"])
            .build();
        if let Some(wordmark) = brand_wordmark(148) {
            wordmark.add_css_class("dsh-brand-wordmark");
            brand_button.set_child(Some(&wordmark));
        }
        brand_row.append(&brand_button);
        let toggle_btn = gtk4::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(copy.collapse_sidebar)
            .action_name("win.toggle-sidebar")
            .css_classes(["dsh-collapse-button"])
            .build();
        brand_row.append(&toggle_btn);
        wide.append(&brand_handle);

        let new_session_btn = gtk4::Button::builder()
            .css_classes(["dsh-new-session"])
            .action_name("win.new-session")
            .tooltip_text(copy.create_session)
            .build();
        let new_session_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .halign(gtk4::Align::Center)
            .build();
        new_session_content.append(&gtk4::Image::from_icon_name("list-add-symbolic"));
        new_session_content.append(&gtk4::Label::builder().label(copy.new_session).build());
        new_session_btn.set_child(Some(&new_session_content));
        wide.append(&new_session_btn);

        let wide_panel_rows = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .build();

        let more_button = sidebar_row(Some("view-grid-symbolic"), Some(copy.more), None);
        let more_popover = gtk4::Popover::builder().width_request(288).build();
        let more_menu_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        more_popover.set_child(Some(&more_menu_box));
        more_popover.set_parent(&more_button);
        let panel_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_bottom(8)
            .build();
        panel_list.append(&wide_panel_rows);
        panel_list.append(&more_button);
        wide.append(&panel_list);

        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();
        let browser = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        let workspace_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .css_classes(["dsh-section-header"])
            .build();
        let workspace_title = gtk4::Label::builder()
            .label(copy.workspaces)
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .css_classes(["dsh-section-title"])
            .build();
        workspace_header.append(&workspace_title);

        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text(copy.search_sessions)
            .css_classes(["dsh-search-entry"])
            .width_request(28)
            .build();
        workspace_header.append(&search_entry);
        let copy_for_focus = copy;
        search_entry.connect_has_focus_notify(move |entry| {
            entry.set_width_request(if entry.has_focus() { 180 } else { 28 });
            if entry.has_focus() {
                entry.set_placeholder_text(Some(copy_for_focus.search_sessions));
            }
        });

        let filter_button = gtk4::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .tooltip_text(copy.workspaces)
            .css_classes(["dsh-filter-button", "dsh-icon-button"])
            .build();
        let (filter_popover, filter_buttons) = filter_popover();
        filter_button.set_popover(Some(&filter_popover));
        workspace_header.append(&filter_button);
        let new_workspace_btn = gtk4::Button::builder()
            .icon_name("folder-new-symbolic")
            .tooltip_text(copy.workspaces)
            .action_name("win.new-workspace")
            .css_classes(["dsh-icon-button"])
            .build();
        workspace_header.append(&new_workspace_btn);
        browser.append(&workspace_header);

        let tree_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["dsh-tree-list"])
            .build();
        browser.append(&tree_list);
        let subagent_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .visible(false)
            .css_classes(["dsh-tree-list"])
            .build();
        browser.append(&subagent_list);
        scroll.set_child(Some(&browser));
        wide.append(&scroll);

        let footer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-sidebar-footer"])
            .build();
        footer.append(&panel_row("emblem-system-symbolic", copy.settings, "app.settings"));
        footer.append(&panel_row("avatar-default-symbolic", copy.account, "win.show-account"));
        wide.append(&footer);

        let rail = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-sidebar", "dsh-sidebar-rail"])
            .build();
        let rail_handle = gtk4::WindowHandle::new();
        rail_handle.set_child(Some(&rail_toggle_button(&copy)));
        rail.append(&rail_handle);
        let rail_new_session = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text(copy.new_session)
            .action_name("win.new-session")
            .css_classes(["dsh-icon-button", "dsh-rail-button"])
            .build();
        rail.append(&rail_new_session);
        let rail_panel_rows = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_bottom(8)
            .build();
        rail.append(&rail_panel_rows);
        for (icon, tooltip, action) in [
            (
                "system-search-symbolic",
                copy.search_sessions,
                "win.focus-search",
            ),
            (
                "folder-new-symbolic",
                copy.workspaces,
                "win.expand-new-workspace",
            ),
        ] {
            let button = gtk4::Button::builder()
                .icon_name(icon)
                .tooltip_text(tooltip)
                .action_name(action)
                .css_classes(["dsh-icon-button", "dsh-rail-button"])
                .build();
            rail.append(&button);
        }
        let rail_spacer = gtk4::Box::builder().vexpand(true).build();
        rail.append(&rail_spacer);
        let rail_settings = gtk4::Button::builder()
            .icon_name("emblem-system-symbolic")
            .tooltip_text(copy.settings)
            .action_name("app.settings")
            .css_classes(["dsh-icon-button", "dsh-rail-button"])
            .build();
        rail.append(&rail_settings);
        let rail_account = gtk4::Button::builder()
            .icon_name("avatar-default-symbolic")
            .tooltip_text(copy.account)
            .action_name("win.show-account")
            .css_classes(["dsh-icon-button", "dsh-rail-button"])
            .build();
        rail.append(&rail_account);

        stack.add_named(&wide, Some("wide"));
        stack.add_named(&rail, Some("rail"));
        stack.set_visible_child_name("wide");

        let this = Rc::new(Self {
            stack,
            wide,
            rail,
            tree_list,
            subagent_list,
            search_entry,
            filter_button,
            more_button,
            more_menu_box,
            more_popover,
            panel_state,
            wide_panel_rows,
            rail_panel_rows,
            panel_row_buttons: RefCell::new(Vec::new()),
            unpinned_panels: RefCell::new(Vec::new()),
            copy,
            workspaces: RefCell::new(Vec::new()),
            sessions: RefCell::new(Vec::new()),
            pinned: RefCell::new(HashSet::new()),
            archive_filter: Cell::new(ArchiveFilter::Active),
            session_grouping: Cell::new(SessionGrouping::Grouped),
            session_sort: Cell::new(SessionSort::Recent),
            filter_menu_buttons: RefCell::new(filter_buttons),
            running: RefCell::new(HashSet::new()),
            unread: RefCell::new(HashSet::new()),
            active_session: RefCell::new(None),
            collapsed_groups: RefCell::new(HashSet::new()),
            group_members: RefCell::new(HashMap::new()),
            rail_active: Cell::new(false),
            fade_animation: Rc::new(RefCell::new(None)),
        });

        let weak_more = Rc::downgrade(&this);
        this.more_button.connect_clicked(move |_| {
            if let Some(this) = weak_more.upgrade() {
                this.render_more_menu();
                this.more_popover.popup();
            }
        });
        let weak = Rc::downgrade(&this);
        this.panel_state.connect_changed(move |panel| {
            if let Some(this) = weak.upgrade() {
                this.apply_panel_fill(panel.is_some());
            }
        });
        this.render_panel_rows();
        this
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn search_entry(&self) -> &gtk4::SearchEntry {
        &self.search_entry
    }

    pub fn set_filter_label(&self, label: &str) {
        self.filter_button.set_tooltip_text(Some(label));
    }

    pub fn set_rail(&self, rail: bool, animate: bool) {
        if self.rail_active.get() == rail {
            return;
        }
        self.rail_active.set(rail);
        if let Some(old) = self.fade_animation.borrow_mut().take() {
            old.skip();
        }
        if !animate || !animations_enabled() {
            self.wide.set_opacity(1.0);
            self.rail.set_opacity(1.0);
            self.stack
                .set_visible_child_name(if rail { "rail" } else { "wide" });
            return;
        }
        if rail {
            let fade_store = self.fade_animation.clone();
            let weak_stack = self.stack.downgrade();
            let weak_rail = self.rail.downgrade();
            play_fade(
                &self.fade_animation,
                self.wide.upcast_ref::<gtk4::Widget>(),
                1.0,
                0.0,
                150,
                Some(Box::new(move || {
                    if let Some(stack) = weak_stack.upgrade() {
                        stack.set_visible_child_name("rail");
                    }
                    if let Some(rail_widget) = weak_rail.upgrade() {
                        play_fade(
                            &fade_store,
                            rail_widget.upcast_ref::<gtk4::Widget>(),
                            0.0,
                            1.0,
                            150,
                            None,
                        );
                    }
                })),
            );
        } else {
            self.stack.set_visible_child_name("wide");
            play_fade(
                &self.fade_animation,
                self.wide.upcast_ref::<gtk4::Widget>(),
                0.0,
                1.0,
                200,
                None,
            );
        }
    }

    pub fn load_subagents(&self, entries: &[serde_json::Value]) {
        while let Some(child) = self.subagent_list.first_child() {
            self.subagent_list.remove(&child);
        }
        for entry in entries {
            let title = entry
                .get("title")
                .and_then(|value| value.as_str())
                .or_else(|| entry.get("name").and_then(|value| value.as_str()))
                .unwrap_or("Subagent");
            let row = sidebar_row(None, Some(title), None);
            if let Some(id) = entry.get("id").and_then(|value| value.as_str()) {
                row.set_action_name(Some("win.open-child"));
                row.set_action_target_value(Some(&id.to_variant()));
            }
            self.subagent_list.append(&row);
        }
        self.subagent_list.set_visible(!entries.is_empty());
    }

    pub fn load_workspaces(self: &Rc<Self>, workspaces: &[WorkspaceView]) {
        *self.workspaces.borrow_mut() = workspaces.to_vec();
        self.rebuild_tree();
    }

    pub fn load_sessions(
        self: &Rc<Self>,
        sessions: &[SessionListItem],
        pinned: &HashSet<String>,
        filter: ArchiveFilter,
        running: &HashSet<String>,
        unread: &HashSet<String>,
    ) {
        *self.sessions.borrow_mut() = sessions.to_vec();
        *self.pinned.borrow_mut() = pinned.clone();
        self.archive_filter.set(filter);
        self.update_filter_menu();
        *self.running.borrow_mut() = running.clone();
        *self.unread.borrow_mut() = unread.clone();
        self.rebuild_tree();
    }

    pub fn set_session_options(self: &Rc<Self>, grouping: SessionGrouping, sort: SessionSort) {
        self.session_grouping.set(grouping);
        self.session_sort.set(sort);
        self.update_filter_menu();
        self.rebuild_tree();
    }

    pub fn update_filter_menu(&self) {
        let filter = self.archive_filter.get();
        let grouping = self.session_grouping.get();
        let sort = self.session_sort.get();
        for (kind, button) in self.filter_menu_buttons.borrow().iter() {
            let checked = match kind {
                FilterMenuKind::Archive(active) => *active == filter,
                FilterMenuKind::Grouping(active) => *active == grouping,
                FilterMenuKind::Sort(active) => *active == sort,
            };
            if let Some(check) = button
                .child()
                .and_downcast::<gtk4::Box>()
                .and_then(|content| content.last_child())
                .and_downcast::<gtk4::Image>()
            {
                check.set_visible(checked);
            }
        }
    }

    pub fn show_search_results(self: &Rc<Self>, results: &[SessionListItem]) {
        *self.sessions.borrow_mut() = results.to_vec();
        self.rebuild_tree();
    }

    pub fn set_active_session(self: &Rc<Self>, session_id: Option<&str>) {
        let next = session_id.map(str::to_string);
        if *self.active_session.borrow() == next {
            return;
        }
        *self.active_session.borrow_mut() = next;
        self.rebuild_tree();
    }

    fn apply_panel_fill(self: &Rc<Self>, panel_active: bool) {
        let active = self.panel_state.active_panel();
        for (id, button) in self.panel_row_buttons.borrow().iter() {
            if Some(id.as_str()) == active.as_deref() {
                button.add_css_class("dsh-panel-row-active");
            } else {
                button.remove_css_class("dsh-panel-row-active");
            }
        }
        if panel_active {
            self.rebuild_tree();
        }
    }

    fn render_panel_rows(self: &Rc<Self>) {
        while let Some(child) = self.wide_panel_rows.first_child() {
            self.wide_panel_rows.remove(&child);
        }
        while let Some(child) = self.rail_panel_rows.first_child() {
            self.rail_panel_rows.remove(&child);
        }
        self.panel_row_buttons.borrow_mut().clear();
        let unpinned = self.unpinned_panels.borrow();
        for (id, icon, fixed) in registered_panels() {
            if unpinned.iter().any(|panel| panel == id) {
                continue;
            }
            let action = format!("win.show-{id}");
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(2)
                .build();
            let button = sidebar_row(Some(icon), Some(self.panel_label(id)), None);
            button.set_action_name(Some(&action));
            button.set_hexpand(true);
            self.panel_row_buttons
                .borrow_mut()
                .push((id.to_string(), button.clone()));
            row.append(&button);
            if !fixed {
                let unpin = gtk4::Button::builder()
                    .icon_name("view-pin-symbolic")
                    .tooltip_text(self.copy.unpin_sidebar)
                    .css_classes(["dsh-icon-button"])
                    .valign(gtk4::Align::Center)
                    .visible(false)
                    .build();
                let weak = Rc::downgrade(self);
                let panel_id = id.to_string();
                unpin.connect_clicked(move |_| {
                    let weak = weak.clone();
                    let panel_id = panel_id.clone();
                    gtk4::glib::idle_add_local_once(move || {
                        if let Some(this) = weak.upgrade() {
                            this.unpin_panel(&panel_id);
                        }
                    });
                });
                row.append(&unpin);
                let hovered = Rc::new(Cell::new(false));
                let unpin_for_enter = unpin.clone();
                let hover = gtk4::EventControllerMotion::new();
                let hovered_for_enter = hovered.clone();
                hover.connect_enter(move |_, _, _| {
                    hovered_for_enter.set(true);
                    unpin_for_enter.set_visible(true);
                });
                let hovered_for_leave = hovered.clone();
                let unpin_for_leave = unpin.clone();
                hover.connect_leave(move |_| {
                    hovered_for_leave.set(false);
                    if !unpin_for_leave.has_focus() {
                        unpin_for_leave.set_visible(false);
                    }
                });
                row.add_controller(hover);
                let hovered_for_focus = hovered.clone();
                let unpin_for_focus = unpin.clone();
                button.connect_has_focus_notify(move |primary| {
                    unpin_for_focus.set_visible(primary.has_focus() || hovered_for_focus.get());
                });
            }
            self.wide_panel_rows.append(&row);

            let rail_button = sidebar_row(Some(icon), None, None);
            rail_button.set_action_name(Some(&action));
            rail_button.set_tooltip_text(Some(self.panel_label(id)));
            rail_button.add_css_class("dsh-rail-button");
            self.panel_row_buttons
                .borrow_mut()
                .push((id.to_string(), rail_button.clone()));
            self.rail_panel_rows.append(&rail_button);
        }
        self.apply_panel_fill(self.panel_state.active_panel().is_some());
    }

    fn panel_label(&self, panel: &str) -> &'static str {
        match panel {
            PANEL_PLUGINS => self.copy.plugins,
            _ => self.copy.schedules,
        }
    }

    fn unpin_panel(self: &Rc<Self>, panel: &str) {
        {
            let mut unpinned = self.unpinned_panels.borrow_mut();
            if !unpinned.iter().any(|id| id == panel) {
                unpinned.push(panel.to_string());
            }
        }
        self.render_panel_rows();
        self.more_button.grab_focus();
    }

    fn pin_panel(self: &Rc<Self>, panel: &str) {
        self.unpinned_panels.borrow_mut().retain(|id| id != panel);
        self.render_panel_rows();
    }

    fn render_more_menu(self: &Rc<Self>) {
        while let Some(child) = self.more_menu_box.first_child() {
            self.more_menu_box.remove(&child);
        }
        let active = self.panel_state.active_panel();
        {
            let unpinned = self.unpinned_panels.borrow();
            for (id, icon, _) in registered_panels() {
                if !unpinned.iter().any(|panel| panel == id) {
                    continue;
                }
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(2)
                    .build();
                let check = gtk4::Image::from_icon_name("object-select-symbolic");
                check.set_visible(Some(id) == active.as_deref());
                let button = sidebar_row(
                    Some(icon),
                    Some(self.panel_label(id)),
                    Some(check.upcast()),
                );
                button.set_hexpand(true);
                let panel_state = self.panel_state.clone();
                let popover = self.more_popover.clone();
                button.connect_clicked(move |_| {
                    panel_state.set_active_panel(Some(id));
                    popover.popdown();
                });
                row.append(&button);
                let pin = gtk4::Button::builder()
                    .icon_name("view-pin-symbolic")
                    .tooltip_text(self.copy.pin_sidebar)
                    .css_classes(["dsh-pin-button"])
                    .valign(gtk4::Align::Center)
                    .build();
                let weak = Rc::downgrade(self);
                let popover = self.more_popover.clone();
                let panel_id = id.to_string();
                pin.connect_clicked(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.pin_panel(&panel_id);
                    }
                    popover.popdown();
                });
                row.append(&pin);
                self.more_menu_box.append(&row);
            }
        }
        self.more_menu_box
            .append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        for (label, action) in [
            (self.copy.browse_plugins, "win.show-plugins"),
            (self.copy.create_plugin, "win.create-plugin"),
        ] {
            let button = sidebar_row(None, Some(label), None);
            button.set_action_name(Some(action));
            let popover = self.more_popover.clone();
            button.connect_clicked(move |_| popover.popdown());
            self.more_menu_box.append(&button);
        }
    }

    fn rebuild_tree(self: &Rc<Self>) {
        while let Some(child) = self.tree_list.first_child() {
            self.tree_list.remove(&child);
        }
        self.group_members.borrow_mut().clear();
        let workspaces = self.workspaces.borrow();
        let sessions = self.sessions.borrow();
        let pinned = self.pinned.borrow();
        let running = self.running.borrow();
        let unread = self.unread.borrow();
        let filter = self.archive_filter.get();
        let grouping = self.session_grouping.get();
        let sort_mode = self.session_sort.get();
        let order = |items: &mut Vec<&SessionListItem>| {
            items.sort_by(|left, right| {
                let left_pinned = pinned.contains(&left.id);
                let right_pinned = pinned.contains(&right.id);
                right_pinned.cmp(&left_pinned).then_with(|| match sort_mode {
                    SessionSort::Recent => right.updated_at.cmp(&left.updated_at),
                    SessionSort::Manual => std::cmp::Ordering::Equal,
                })
            });
        };
        let active = self.active_session.borrow();
        let panel_active = self.panel_state.active_panel().is_some();
        if grouping == SessionGrouping::Flat {
            let mut all: Vec<&SessionListItem> = sessions
                .iter()
                .filter(|item| matches_filter(filter, item))
                .collect();
            order(&mut all);
            for item in all {
                let row = self.session_row(
                    item,
                    &pinned,
                    &running,
                    &unread,
                    active.as_deref(),
                    panel_active,
                );
                self.tree_list.append(&row);
            }
            return;
        }
        let collapsed = self.collapsed_groups.borrow();
        let mut claimed: HashSet<String> = HashSet::new();
        for workspace in workspaces.iter() {
            for id in &workspace.session_ids {
                claimed.insert(id.clone());
            }
            let header = self.workspace_header_row(workspace);
            self.tree_list.append(&header);
            let mut members: Vec<&SessionListItem> = workspace
                .session_ids
                .iter()
                .filter_map(|id| sessions.iter().find(|item| &item.id == id))
                .filter(|item| matches_filter(filter, item))
                .collect();
            order(&mut members);
            let mut member_rows = Vec::new();
            for item in members {
                let row = self.session_row(
                    item,
                    &pinned,
                    &running,
                    &unread,
                    active.as_deref(),
                    panel_active,
                );
                self.tree_list.append(&row);
                if let Some(list_row) = row.parent() {
                    member_rows.push(list_row);
                }
            }
            let hidden = collapsed.contains(&workspace.id);
            for member in &member_rows {
                member.set_visible(!hidden);
            }
            self.group_members
                .borrow_mut()
                .insert(workspace.id.clone(), member_rows);
        }
        let mut orphans: Vec<&SessionListItem> = sessions
            .iter()
            .filter(|item| !claimed.contains(&item.id))
            .filter(|item| matches_filter(filter, item))
            .collect();
        order(&mut orphans);
        if !orphans.is_empty() {
            let header = gtk4::Label::builder()
                .label(self.copy.ungrouped)
                .halign(gtk4::Align::Start)
                .css_classes(["dsh-tree-group-header"])
                .build();
            self.tree_list.append(&header);
            for item in orphans {
                let row = self.session_row(
                    item,
                    &pinned,
                    &running,
                    &unread,
                    active.as_deref(),
                    panel_active,
                );
                self.tree_list.append(&row);
            }
        }
    }

    fn toggle_group(&self, group: &str) {
        {
            let mut collapsed = self.collapsed_groups.borrow_mut();
            if !collapsed.remove(group) {
                collapsed.insert(group.to_string());
            }
        }
        let hidden = self.collapsed_groups.borrow().contains(group);
        if let Some(members) = self.group_members.borrow().get(group) {
            for member in members {
                member.set_visible(!hidden);
            }
        }
    }

    fn workspace_header_row(self: &Rc<Self>, workspace: &WorkspaceView) -> gtk4::Widget {
        let collapsed = self.collapsed_groups.borrow().contains(&workspace.id);
        let title = if workspace.title.trim().is_empty() {
            self.copy.default_workspace
        } else {
            workspace.title.as_str()
        };
        let chevron = gtk4::Image::from_icon_name(if collapsed {
            "pan-end-symbolic"
        } else {
            "pan-down-symbolic"
        });
        chevron.add_css_class("dsh-tree-chevron");
        let button = sidebar_row(
            Some("folder-symbolic"),
            Some(title),
            Some(chevron.upcast()),
        );
        button.set_hexpand(true);
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .build();
        row.append(&button);

        let new_session_btn = gtk4::Button::builder()
            .icon_name("chat-message-new-symbolic")
            .tooltip_text(self.copy.create_session)
            .action_name("win.new-session-in-workspace")
            .css_classes(["dsh-icon-button"])
            .valign(gtk4::Align::Center)
            .build();
        new_session_btn.set_action_target_value(Some(&workspace.id.to_variant()));
        row.append(&new_session_btn);

        let menu_btn = gtk4::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .css_classes(["dsh-icon-button"])
            .valign(gtk4::Align::Center)
            .build();
        let popover = gtk4::Popover::builder().build();
        let pop_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        pop_box.append(&menu_row(
            "重命名",
            "win.rename-workspace",
            &workspace.id,
            &popover,
        ));
        pop_box.append(&menu_row(
            "在文件夹显示",
            "win.reveal-workspace",
            &workspace.id,
            &popover,
        ));
        let remove_btn = menu_row(
            "删除工作区",
            "win.remove-workspace",
            &workspace.id,
            &popover,
        );
        remove_btn.add_css_class("destructive-action");
        pop_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        pop_box.append(&remove_btn);
        popover.set_child(Some(&pop_box));
        menu_btn.set_popover(Some(&popover));
        crate::ui::menu::suppress_initial_selection(&menu_btn);
        row.append(&menu_btn);

        let weak = Rc::downgrade(self);
        let group = workspace.id.clone();
        button.connect_clicked(move |_| {
            let weak = weak.clone();
            let group = group.clone();
            gtk4::glib::idle_add_local_once(move || {
                if let Some(this) = weak.upgrade() {
                    this.toggle_group(&group);
                }
            });
        });
        row.upcast()
    }

    fn session_row(
        &self,
        item: &SessionListItem,
        pinned: &HashSet<String>,
        running: &HashSet<String>,
        unread: &HashSet<String>,
        active_session: Option<&str>,
        panel_active: bool,
    ) -> gtk4::Widget {
        let relative_time = item.relative_time();
        let time_label = relative_time.map(|time| {
            gtk4::Label::builder()
                .label(time)
                .css_classes(["dsh-row-time"])
                .build()
                .upcast::<gtk4::Widget>()
        });
        let button = sidebar_row(None, Some(&item.title), time_label);
        button.set_action_name(Some("win.select-session"));
        button.set_action_target_value(Some(&item.id.to_variant()));
        button.set_hexpand(true);
        if active_session == Some(item.id.as_str()) && !panel_active {
            button.add_css_class("dsh-row-selected");
        }
        if pinned.contains(&item.id) || running.contains(&item.id) {
            button.add_css_class("dsh-row-attention");
        }
        if unread.contains(&item.id) {
            button.add_css_class("dsh-row-unread");
        }
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .build();
        row.append(&button);
        let menu_btn = gtk4::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .css_classes(["dsh-icon-button"])
            .valign(gtk4::Align::Center)
            .build();
        let popover = gtk4::Popover::builder().build();
        let pop_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        pop_box.append(&menu_row("置顶", "win.pin-session", &item.id, &popover));
        pop_box.append(&menu_row(
            "重命名",
            "win.rename-session",
            &item.id,
            &popover,
        ));
        pop_box.append(&menu_row(
            "关闭视图",
            "win.close-session",
            &item.id,
            &popover,
        ));
        pop_box.append(&menu_row(
            if item.is_archived { "取消归档" } else { "归档" },
            if item.is_archived {
                "win.unarchive-session"
            } else {
                "win.archive-session"
            },
            &item.id,
            &popover,
        ));
        popover.set_child(Some(&pop_box));
        menu_btn.set_popover(Some(&popover));
        crate::ui::menu::suppress_initial_selection(&menu_btn);
        row.append(&menu_btn);
        row.upcast()
    }
}

fn matches_filter(filter: ArchiveFilter, item: &SessionListItem) -> bool {
    match filter {
        ArchiveFilter::Active => !item.is_archived,
        ArchiveFilter::Archived => item.is_archived,
        ArchiveFilter::All => true,
    }
}

pub(crate) fn animations_enabled() -> bool {
    gtk4::Settings::default()
        .map(|settings| settings.is_gtk_enable_animations())
        .unwrap_or(false)
}

fn play_fade(
    store: &Rc<RefCell<Option<adw::TimedAnimation>>>,
    widget: &gtk4::Widget,
    from: f64,
    to: f64,
    duration_ms: u32,
    done: Option<Box<dyn Fn()>>,
) {
    let target = adw::PropertyAnimationTarget::new(widget, "opacity");
    let animation = adw::TimedAnimation::new(widget, from, to, duration_ms, target);
    animation.set_easing(adw::Easing::EaseInOutCubic);
    if let Some(done) = done {
        animation.connect_done(move |_| done());
    }
    *store.borrow_mut() = Some(animation.clone());
    animation.play();
}

fn rail_toggle_button(copy: &Copybook) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .tooltip_text(copy.collapse_sidebar)
        .action_name("win.toggle-sidebar")
        .css_classes(["dsh-collapse-button", "dsh-rail-button"])
        .build();
    if let Some(mark) = fish_logo(24) {
        let faces = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        faces.add_named(&mark, Some("mark"));
        faces.add_named(
            &gtk4::Image::from_icon_name("sidebar-show-symbolic"),
            Some("expand"),
        );
        button.set_child(Some(&faces));
        let hover = gtk4::EventControllerMotion::new();
        let faces_for_enter = faces.clone();
        hover.connect_enter(move |_, _, _| faces_for_enter.set_visible_child_name("expand"));
        let faces_for_leave = faces.clone();
        hover.connect_leave(move |_| faces_for_leave.set_visible_child_name("mark"));
        button.add_controller(hover);
    }
    button
}

fn panel_row(icon: &str, label: &str, action: &str) -> gtk4::Button {
    let button = sidebar_row(Some(icon), Some(label), None);
    button.set_action_name(Some(action));
    button
}

fn sidebar_row(
    icon: Option<&str>,
    label: Option<&str>,
    trailing: Option<gtk4::Widget>,
) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .css_classes(["dsh-panel-row"])
        .halign(gtk4::Align::Fill)
        .build();
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    if let Some(icon) = icon {
        let image = gtk4::Image::from_icon_name(icon);
        image.add_css_class("dsh-panel-glyph");
        content.append(&image);
    }
    if let Some(label) = label {
        content.append(
            &gtk4::Label::builder()
                .label(label)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
    }
    if let Some(trailing) = trailing {
        content.append(&trailing);
    }
    button.set_child(Some(&content));
    button
}

fn menu_row(label: &str, action: &str, id: &str, popover: &gtk4::Popover) -> gtk4::Button {
    let button = sidebar_row(None, Some(label), None);
    button.set_action_name(Some(action));
    button.set_action_target_value(Some(&id.to_variant()));
    let popover_clone = popover.clone();
    button.connect_clicked(move |_| popover_clone.popdown());
    button
}

#[derive(Clone, Copy, PartialEq)]
enum FilterMenuKind {
    Archive(ArchiveFilter),
    Grouping(SessionGrouping),
    Sort(SessionSort),
}

fn filter_popover() -> (gtk4::Popover, Vec<(FilterMenuKind, gtk4::Button)>) {
    let popover = gtk4::Popover::builder().build();
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let entries = [
        (
            "隐藏已归档",
            FilterMenuKind::Archive(ArchiveFilter::Active),
            "win.filter-active",
        ),
        (
            "全部对话（显示已归档）",
            FilterMenuKind::Archive(ArchiveFilter::All),
            "win.filter-all",
        ),
        (
            "仅显示已归档",
            FilterMenuKind::Archive(ArchiveFilter::Archived),
            "win.filter-archived",
        ),
        (
            "按工作区",
            FilterMenuKind::Grouping(SessionGrouping::Grouped),
            "win.group-by-workspace",
        ),
        (
            "平铺列表",
            FilterMenuKind::Grouping(SessionGrouping::Flat),
            "win.flatten-sessions",
        ),
        (
            "最近更新",
            FilterMenuKind::Sort(SessionSort::Recent),
            "win.sort-recent",
        ),
        (
            "手动排序",
            FilterMenuKind::Sort(SessionSort::Manual),
            "win.sort-manual",
        ),
    ];
    let mut buttons = Vec::new();
    for (label, kind, action) in entries {
        let check = gtk4::Image::from_icon_name("object-select-symbolic");
        check.set_visible(false);
        let button = sidebar_row(None, Some(label), Some(check.upcast()));
        button.set_action_name(Some(action));
        let popover_clone = popover.clone();
        button.connect_clicked(move |_| popover_clone.popdown());
        box_widget.append(&button);
        buttons.push((kind, button));
    }
    popover.set_child(Some(&box_widget));
    (popover, buttons)
}

trait RelativeTime {
    fn relative_time(&self) -> Option<String>;
}

impl RelativeTime for SessionListItem {
    fn relative_time(&self) -> Option<String> {
        let elapsed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs()
            .saturating_sub(self.updated_at);
        Some(match elapsed {
            0..=59 => "刚刚".to_string(),
            60..=3599 => format!("{}分钟", elapsed / 60),
            3600..=86_399 => format!("{}小时", elapsed / 3600),
            86_400..=2_591_999 => format!("{}天", elapsed / 86_400),
            2_592_000..=31_535_999 => format!("{}个月", elapsed / 2_592_000),
            _ => format!("{}年", elapsed / 31_536_000),
        })
    }
}
