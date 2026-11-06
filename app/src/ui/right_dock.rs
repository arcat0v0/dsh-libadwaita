use gtk4::prelude::*;
use libadwaita as adw;

pub struct RightDock {
    container: gtk4::Box,
    tab_view: adw::TabView,
    tab_bar: adw::TabBar,
}

impl RightDock {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let tab_view = adw::TabView::new();
        let tab_bar = adw::TabBar::builder()
            .view(&tab_view)
            .autohide(false)
            .build();

        container.append(&tab_bar);

        let terminal_page = adw::StatusPage::builder()
            .title("Terminal")
            .description("Embedded terminal for workspace commands")
            .icon_name("utilities-terminal-symbolic")
            .build();
        let page1 = tab_view.append(&terminal_page);
        page1.set_title("Terminal");
        page1.set_icon(Some(&gtk4::gio::ThemedIcon::new(
            "utilities-terminal-symbolic",
        )));

        let files_page = adw::StatusPage::builder()
            .title("Files")
            .description("Workspace changes and git diff viewer")
            .icon_name("folder-symbolic")
            .build();
        let page2 = tab_view.append(&files_page);
        page2.set_title("Files");
        page2.set_icon(Some(&gtk4::gio::ThemedIcon::new("folder-symbolic")));

        let preview_page = adw::StatusPage::builder()
            .title("Preview")
            .description("Artifact and rendered document preview")
            .icon_name("view-reveal-symbolic")
            .build();
        let page3 = tab_view.append(&preview_page);
        page3.set_title("Preview");
        page3.set_icon(Some(&gtk4::gio::ThemedIcon::new("view-reveal-symbolic")));

        container.append(&tab_view);

        Self {
            container,
            tab_view,
            tab_bar,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn tab_view(&self) -> &adw::TabView {
        &self.tab_view
    }

    pub fn tab_bar(&self) -> &adw::TabBar {
        &self.tab_bar
    }
}

impl Default for RightDock {
    fn default() -> Self {
        Self::new()
    }
}
