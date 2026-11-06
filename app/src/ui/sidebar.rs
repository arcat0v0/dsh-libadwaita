use crate::protocol::{SessionListItem, WorkspaceView};
use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita as adw;

pub struct Sidebar {
    container: gtk4::Box,
    session_list: gtk4::ListBox,
    workspace_list: gtk4::ListBox,
    subagent_list: gtk4::ListBox,
    search_entry: gtk4::SearchEntry,
    new_session_btn: gtk4::Button,
}

impl Sidebar {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["navigation-sidebar"])
            .width_request(240)
            .build();

        let brand_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(8)
            .build();

        let brand_label = gtk4::Label::builder()
            .label("dsh")
            .css_classes(["title-2"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        brand_box.append(&brand_label);

        let new_session_btn = gtk4::Button::builder()
            .icon_name("list-add-symbolic")
            .tooltip_text("New Session (Ctrl+N)")
            .action_name("win.new-session")
            .css_classes(["flat"])
            .build();
        brand_box.append(&new_session_btn);
        container.append(&brand_box);

        let subagent_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        let subagent_title = gtk4::Label::builder()
            .label("Subagents")
            .css_classes(["dim-label", "caption"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        subagent_header.append(&subagent_title);
        container.append(&subagent_header);
        let subagent_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();
        container.append(&subagent_list);

        let workspace_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        let workspace_title = gtk4::Label::builder()
            .label("Workspaces")
            .css_classes(["dim-label", "caption"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        workspace_header.append(&workspace_title);
        let new_workspace_btn = gtk4::Button::builder()
            .icon_name("folder-new-symbolic")
            .tooltip_text("Create workspace")
            .action_name("win.new-workspace")
            .css_classes(["flat", "circular"])
            .build();
        workspace_header.append(&new_workspace_btn);
        container.append(&workspace_header);

        let workspace_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();
        container.append(&workspace_list);

        let search_box = gtk4::Box::builder()
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text("Search sessions...")
            .hexpand(true)
            .build();
        search_box.append(&search_entry);
        container.append(&search_box);

        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let session_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["navigation-sidebar"])
            .build();

        let session1 = adw::ActionRow::builder()
            .title("Implementing Bridge Protocol")
            .subtitle("Just now")
            .activatable(true)
            .build();
        session_list.append(&session1);

        let session2 = adw::ActionRow::builder()
            .title("Code Refactoring")
            .subtitle("Yesterday")
            .activatable(true)
            .build();
        session_list.append(&session2);

        let session3 = adw::ActionRow::builder()
            .title("Linux libadwaita UI")
            .subtitle("2 days ago")
            .activatable(true)
            .build();
        session_list.append(&session3);

        scroll.set_child(Some(&session_list));
        container.append(&scroll);

        let bottom_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .spacing(2)
            .build();

        let bottom_list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();

        let plugins_row = adw::ActionRow::builder()
            .title("Plugins")
            .activatable(true)
            .action_name("win.show-plugins")
            .build();
        plugins_row.add_prefix(&gtk4::Image::from_icon_name("application-x-addon-symbolic"));
        bottom_list.append(&plugins_row);

        let schedules_row = adw::ActionRow::builder()
            .title("Schedules")
            .activatable(true)
            .action_name("win.show-schedules")
            .build();
        schedules_row.add_prefix(&gtk4::Image::from_icon_name("alarm-symbolic"));
        bottom_list.append(&schedules_row);

        let settings_row = adw::ActionRow::builder()
            .title("Settings")
            .subtitle("Ctrl+,")
            .activatable(true)
            .action_name("app.settings")
            .build();
        settings_row.add_prefix(&gtk4::Image::from_icon_name("emblem-system-symbolic"));
        bottom_list.append(&settings_row);

        let account_row = adw::ActionRow::builder()
            .title("Account")
            .activatable(true)
            .action_name("win.show-account")
            .build();
        account_row.add_prefix(&gtk4::Image::from_icon_name("avatar-default-symbolic"));
        bottom_list.append(&account_row);

        bottom_box.append(&bottom_list);
        container.append(&bottom_box);

        Self {
            container,
            session_list,
            workspace_list,
            subagent_list,
            search_entry,
            new_session_btn,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
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
        if entries.is_empty() {
            let row = adw::ActionRow::builder()
                .title("No child agents")
                .subtitle("Subagents appear here when spawned")
                .sensitive(false)
                .build();
            self.subagent_list.append(&row);
            return;
        }
        for entry in entries {
            let child_id = entry["sessionId"]
                .as_str()
                .or_else(|| entry["childSessionId"].as_str())
                .or_else(|| entry["id"].as_str())
                .unwrap_or_default()
                .to_string();
            let title = entry["title"]
                .as_str()
                .or_else(|| entry["label"].as_str())
                .or_else(|| entry["name"].as_str())
                .unwrap_or("Subagent")
                .to_string();
            let status = entry["status"].as_str().unwrap_or("unknown");
            let row = adw::ActionRow::builder()
                .title(&title)
                .subtitle(status)
                .activatable(true)
                .action_name("win.open-child")
                .build();
            row.set_action_target_value(Some(&child_id.to_variant()));
            let interrupt = gtk4::Button::builder()
                .icon_name("media-playback-stop-symbolic")
                .tooltip_text("Interrupt subagent")
                .action_name("win.interrupt-subagent")
                .css_classes(["flat", "circular"])
                .valign(gtk4::Align::Center)
                .build();
            interrupt.set_action_target_value(Some(&child_id.to_variant()));
            row.add_suffix(&interrupt);
            self.subagent_list.append(&row);
        }
    }

    pub fn load_workspaces(&self, workspaces: &[WorkspaceView], sessions: &[SessionListItem]) {
        while let Some(child) = self.workspace_list.first_child() {
            self.workspace_list.remove(&child);
        }

        for workspace in workspaces {
            let header = adw::ActionRow::builder()
                .title(&workspace.title)
                .subtitle(&workspace.path)
                .activatable(true)
                .action_name("win.select-workspace")
                .build();
            header.set_action_target_value(Some(&workspace.id.to_variant()));

            let menu_btn = gtk4::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .css_classes(["flat", "circular"])
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

            let rename_btn = gtk4::Button::builder()
                .label("Rename")
                .css_classes(["flat"])
                .action_name("win.rename-workspace")
                .build();
            rename_btn.set_action_target_value(Some(&workspace.id.to_variant()));
            let pop_rename = popover.clone();
            rename_btn.connect_clicked(move |_| pop_rename.popdown());
            pop_box.append(&rename_btn);

            let remove_btn = gtk4::Button::builder()
                .label("Remove")
                .css_classes(["flat", "destructive-action"])
                .action_name("win.remove-workspace")
                .build();
            remove_btn.set_action_target_value(Some(&workspace.id.to_variant()));
            let pop_remove = popover.clone();
            remove_btn.connect_clicked(move |_| pop_remove.popdown());
            pop_box.append(&remove_btn);

            popover.set_child(Some(&pop_box));
            menu_btn.set_popover(Some(&popover));
            header.add_suffix(&menu_btn);

            let expander = gtk4::Expander::builder().child(&header).build();
            let sessions_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .margin_start(16)
                .build();

            for session_id in &workspace.session_ids {
                if let Some(session) = sessions.iter().find(|s| &s.id == session_id) {
                    let row = adw::ActionRow::builder()
                        .title(&session.title)
                        .subtitle(&session.cwd)
                        .activatable(true)
                        .action_name("win.select-session")
                        .build();
                    row.set_action_target_value(Some(&session.id.to_variant()));
                    sessions_box.append(&row);
                }
            }
            if sessions_box.first_child().is_some() {
                expander.set_child(Some(&sessions_box));
            }
            self.workspace_list.append(&expander);
        }
    }

    pub fn load_sessions(&self, sessions: &[SessionListItem]) {
        while let Some(child) = self.session_list.first_child() {
            self.session_list.remove(&child);
        }

        for item in sessions {
            let row = adw::ActionRow::builder()
                .title(&item.title)
                .subtitle(&item.cwd)
                .activatable(true)
                .action_name("win.select-session")
                .build();
            row.set_action_target_value(Some(&item.id.to_variant()));

            let menu_btn = gtk4::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .css_classes(["flat", "circular"])
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

            let rename_btn = gtk4::Button::builder()
                .label("Rename")
                .css_classes(["flat"])
                .action_name("win.rename-session")
                .build();
            rename_btn.set_action_target_value(Some(&item.id.to_variant()));
            let pop_clone_rename = popover.clone();
            rename_btn.connect_clicked(move |_| {
                pop_clone_rename.popdown();
            });
            pop_box.append(&rename_btn);

            let close_btn = gtk4::Button::builder()
                .label("Close View")
                .css_classes(["flat"])
                .action_name("win.close-session")
                .build();
            close_btn.set_action_target_value(Some(&item.id.to_variant()));
            let pop_clone_close = popover.clone();
            close_btn.connect_clicked(move |_| {
                pop_clone_close.popdown();
            });
            pop_box.append(&close_btn);

            let archive_btn = gtk4::Button::builder()
                .label("Archive")
                .css_classes(["flat"])
                .action_name("win.archive-session")
                .build();
            archive_btn.set_action_target_value(Some(&item.id.to_variant()));

            let pop_clone = popover.clone();
            archive_btn.connect_clicked(move |_| {
                pop_clone.popdown();
            });

            pop_box.append(&archive_btn);
            popover.set_child(Some(&pop_box));
            menu_btn.set_popover(Some(&popover));

            row.add_suffix(&menu_btn);
            self.session_list.append(&row);
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}
