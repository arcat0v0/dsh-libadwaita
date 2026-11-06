use crate::protocol::SessionListItem;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use libadwaita as adw;

pub struct Sidebar {
    container: gtk4::Box,
    session_list: gtk4::ListBox,
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
