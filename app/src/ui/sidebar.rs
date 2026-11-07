use crate::protocol::{SessionListItem, WorkspaceView};
use crate::ui::brand::brand_wordmark;
use crate::ui::copy::Copybook;
use crate::ui_prefs::ArchiveFilter;
use gtk4::prelude::*;
use std::collections::HashSet;

pub struct Sidebar {
    stack: gtk4::Stack,
    session_list: gtk4::ListBox,
    workspace_list: gtk4::ListBox,
    subagent_list: gtk4::ListBox,
    search_entry: gtk4::SearchEntry,
    filter_button: gtk4::MenuButton,
    new_session_btn: gtk4::Button,
}

impl Sidebar {
    pub fn new() -> Self {
        let copy = Copybook::for_code("system");
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-sidebar"])
            .build();
        let rail = rail_view();
        stack.add_named(&container, Some("wide"));
        stack.add_named(&rail, Some("rail"));

        let brand_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["dsh-brand-row"])
            .build();
        let brand_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(5)
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        if let Some(wordmark) = brand_wordmark(148) {
            wordmark.add_css_class("dsh-brand-wordmark");
            brand_box.append(&wordmark);
        }
        brand_row.append(&brand_box);
        let toggle_btn = gtk4::Button::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(copy.collapse_sidebar)
            .action_name("win.toggle-sidebar")
            .css_classes(["dsh-collapse-button"])
            .build();
        brand_row.append(&toggle_btn);
        container.append(&brand_row);

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
        container.append(&new_session_btn);

        let panel_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_bottom(8)
            .build();
        panel_list.append(&panel_row(
            "application-x-addon-symbolic",
            copy.plugins,
            "win.show-plugins",
        ));
        panel_list.append(&panel_row(
            "alarm-symbolic",
            copy.schedules,
            "win.show-schedules",
        ));
        panel_list.append(&more_row(copy));
        container.append(&panel_list);

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
        let filter_popover = filter_popover();
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

        let workspace_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["dsh-tree-list"])
            .build();
        browser.append(&workspace_list);
        let subagent_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .visible(false)
            .css_classes(["dsh-tree-list"])
            .build();
        browser.append(&subagent_list);
        let session_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["dsh-tree-list"])
            .build();
        browser.append(&session_list);
        scroll.set_child(Some(&browser));
        container.append(&scroll);

        let footer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-sidebar-footer"])
            .build();
        footer.append(&panel_row(
            "emblem-system-symbolic",
            copy.settings,
            "app.settings",
        ));
        footer.append(&panel_row(
            "avatar-default-symbolic",
            copy.account,
            "win.show-account",
        ));
        container.append(&footer);

        Self {
            stack,
            session_list,
            workspace_list,
            subagent_list,
            search_entry,
            filter_button,
            new_session_btn,
        }
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn set_rail_mode(&self, rail: bool) {
        self.stack.set_visible_child_name(if rail { "rail" } else { "wide" });
    }

    pub fn session_list(&self) -> &gtk4::ListBox {
        &self.session_list
    }

    pub fn search_entry(&self) -> &gtk4::SearchEntry {
        &self.search_entry
    }

    pub fn new_session_button(&self) -> &gtk4::Button {
        &self.new_session_btn
    }

    pub fn workspace_list(&self) -> &gtk4::ListBox {
        &self.workspace_list
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
            let row = tree_button(title, None, None);
            if let Some(id) = entry.get("id").and_then(|value| value.as_str()) {
                row.set_action_name(Some("win.open-child"));
                row.set_action_target_value(Some(&id.to_variant()));
            }
            self.subagent_list.append(&row);
        }
        self.subagent_list.set_visible(!entries.is_empty());
    }

    pub fn load_workspaces(&self, workspaces: &[WorkspaceView], _sessions: &[SessionListItem]) {
        while let Some(child) = self.workspace_list.first_child() {
            self.workspace_list.remove(&child);
        }
        for workspace in workspaces {
            let label = if workspace.title.trim().is_empty() {
                Copybook::for_code("system").default_workspace
            } else {
                &workspace.title
            };
            let row = tree_button(label, None, Some("folder-symbolic"));
            row.set_action_name(Some("win.select-workspace"));
            row.set_action_target_value(Some(&workspace.id.to_variant()));
            self.workspace_list.append(&row);

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
            let rename_btn = popover_button("重命名", "win.rename-workspace", &workspace.id);
            let reveal_btn = popover_button("在文件夹显示", "win.reveal-workspace", &workspace.id);
            let remove_btn = popover_button("删除工作区", "win.remove-workspace", &workspace.id);
            remove_btn.add_css_class("destructive-action");
            pop_box.append(&rename_btn);
            pop_box.append(&reveal_btn);
            pop_box.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            pop_box.append(&remove_btn);
            popover.set_child(Some(&pop_box));
            menu_btn.set_popover(Some(&popover));
            let row_box = row.child().and_downcast::<gtk4::Box>();
            if let Some(row_box) = row_box {
                let new_session_btn = gtk4::Button::builder()
                    .icon_name("chat-message-new-symbolic")
                    .tooltip_text(Copybook::for_code("system").create_session)
                    .action_name("win.new-session-in-workspace")
                    .css_classes(["dsh-icon-button"])
                    .valign(gtk4::Align::Center)
                    .build();
                new_session_btn.set_action_target_value(Some(&workspace.id.to_variant()));
                row_box.append(&new_session_btn);
                crate::ui::menu::suppress_initial_selection(&menu_btn);
                row_box.append(&menu_btn);
            }
        }
    }

    pub fn set_filter_label(&self, label: &str) {
        self.filter_button.set_tooltip_text(Some(label));
    }

    pub fn load_sessions(
        &self,
        sessions: &[SessionListItem],
        pinned: &HashSet<String>,
        filter: ArchiveFilter,
        running: &HashSet<String>,
        unread: &HashSet<String>,
    ) {
        while let Some(child) = self.session_list.first_child() {
            self.session_list.remove(&child);
        }
        let mut ordered: Vec<&SessionListItem> = sessions
            .iter()
            .filter(|item| match filter {
                ArchiveFilter::Active => !item.is_archived,
                ArchiveFilter::Archived => item.is_archived,
                ArchiveFilter::All => true,
            })
            .collect();
        ordered.sort_by_key(|item| !pinned.contains(&item.id));
        for item in ordered {
            let row = session_row(item, pinned, running, unread);
            self.session_list.append(&row);
        }
    }
}

fn panel_row(icon: &str, label: &str, action: &str) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .action_name(action)
        .css_classes(["dsh-panel-row"])
        .halign(gtk4::Align::Fill)
        .build();
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    let image = gtk4::Image::from_icon_name(icon);
    image.add_css_class("dsh-panel-glyph");
    content.append(&image);
    content.append(
        &gtk4::Label::builder()
            .label(label)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build(),
    );
    button.set_child(Some(&content));
    button
}

fn more_row(copy: Copybook) -> gtk4::MenuButton {
    let button = gtk4::MenuButton::builder()
        .css_classes(["dsh-panel-row"])
        .build();
    let content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();
    let image = gtk4::Image::from_icon_name("view-grid-symbolic");
    image.add_css_class("dsh-panel-glyph");
    content.append(&image);
    content.append(
        &gtk4::Label::builder()
            .label(copy.more)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .build(),
    );
    content.append(&gtk4::Image::from_icon_name("pan-down-symbolic"));
    button.set_child(Some(&content));
    let popover = gtk4::Popover::builder().build();
    let menu_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let browse = gtk4::Button::builder()
        .label(copy.browse_plugins)
        .action_name("win.show-plugins")
        .css_classes(["dsh-panel-row"])
        .build();
    let create = gtk4::Button::builder()
        .label(copy.create_plugin)
        .action_name("win.show-plugins")
        .css_classes(["dsh-panel-row"])
        .build();
    for item in [browse, create] {
        let popover_clone = popover.clone();
        item.connect_clicked(move |_| popover_clone.popdown());
        menu_box.append(&item);
    }
    popover.set_child(Some(&menu_box));
    button.set_popover(Some(&popover));
    crate::ui::menu::suppress_initial_selection(&button);
    button
}

fn filter_popover() -> gtk4::Popover {
    let popover = gtk4::Popover::builder().build();
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    for (label, action) in [
        ("隐藏已归档", "win.filter-active"),
        ("全部对话（显示已归档）", "win.filter-all"),
        ("仅显示已归档", "win.filter-archived"),
        ("按工作区", "win.filter-all"),
        ("手动排序", "win.filter-all"),
        ("最近更新", "win.filter-all"),
    ] {
        let button = gtk4::Button::builder()
            .label(label)
            .action_name(action)
            .css_classes(["dsh-panel-row"])
            .build();
        let popover_clone = popover.clone();
        button.connect_clicked(move |_| popover_clone.popdown());
        box_widget.append(&button);
    }
    popover.set_child(Some(&box_widget));
    popover
}

fn tree_button(label: &str, time: Option<&str>, icon: Option<&str>) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .css_classes(["dsh-tree-row"])
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
    content.append(
        &gtk4::Label::builder()
            .label(label)
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .halign(gtk4::Align::Start)
            .hexpand(true)
            .css_classes(["dsh-row-label"])
            .build(),
    );
    if let Some(time) = time {
        content.append(
            &gtk4::Label::builder()
                .label(time)
                .css_classes(["dsh-row-time"])
                .build(),
        );
    }
    button.set_child(Some(&content));
    button
}

fn session_row(
    item: &SessionListItem,
    pinned: &HashSet<String>,
    running: &HashSet<String>,
    unread: &HashSet<String>,
) -> gtk4::Widget {
    let relative_time = item.relative_time();
    let button = tree_button(&item.title, relative_time.as_deref(), None);
    button.set_action_name(Some("win.select-session"));
    button.set_action_target_value(Some(&item.id.to_variant()));
    if item.turn_count == 0 || item.title == "New Session" || item.title == "新会话" {
        button.add_css_class("dsh-row-selected");
    }
    if pinned.contains(&item.id) || running.contains(&item.id) {
        button.add_css_class("dsh-row-attention");
    }
    if unread.contains(&item.id) {
        button.add_css_class("dsh-row-unread");
    }
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(2)
        .build();
    let content = button.child().and_downcast::<gtk4::Box>().unwrap();
    button.set_child(None::<&gtk4::Widget>);
    box_widget.append(&content);
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
    pop_box.append(&popover_button("置顶", "win.pin-session", &item.id));
    pop_box.append(&popover_button("重命名", "win.rename-session", &item.id));
    pop_box.append(&popover_button("关闭视图", "win.close-session", &item.id));
    pop_box.append(&popover_button(
        if item.is_archived { "取消归档" } else { "归档" },
        if item.is_archived {
            "win.unarchive-session"
        } else {
            "win.archive-session"
        },
        &item.id,
    ));
    popover.set_child(Some(&pop_box));
    menu_btn.set_popover(Some(&popover));
    box_widget.append(&menu_btn);
    button.set_child(Some(&box_widget));
    button.upcast()
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

fn popover_button(label: &str, action: &str, id: &str) -> gtk4::Button {
    let button = gtk4::Button::builder()
        .label(label)
        .action_name(action)
        .css_classes(["dsh-panel-row"])
        .build();
    button.set_action_target_value(Some(&id.to_variant()));
    button
}

fn rail_view() -> gtk4::Box {
    let rail = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["dsh-sidebar", "dsh-sidebar-rail"])
        .build();
    for (icon, action) in [
        ("sidebar-show-symbolic", "win.toggle-sidebar"),
        ("list-add-symbolic", "win.new-session"),
        ("application-x-addon-symbolic", "win.show-plugins"),
        ("alarm-symbolic", "win.show-schedules"),
        ("system-search-symbolic", "win.toggle-sidebar"),
    ] {
        let button = gtk4::Button::builder()
            .icon_name(icon)
            .action_name(action)
            .css_classes(["dsh-icon-button", "dsh-rail-button"])
            .build();
        rail.append(&button);
    }
    let spacer = gtk4::Box::builder().vexpand(true).build();
    rail.append(&spacer);
    let settings = gtk4::Button::builder()
        .icon_name("emblem-system-symbolic")
        .action_name("app.settings")
        .css_classes(["dsh-icon-button", "dsh-rail-button"])
        .build();
    rail.append(&settings);
    rail
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
