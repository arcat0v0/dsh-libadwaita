use crate::ui::controller::SessionController;
use crate::ui::files_panel::{FilesPanel, PreviewPanel};
use crate::ui::terminal_panel::TerminalPanel;
use gtk4::prelude::*;
use libadwaita as adw;
use std::rc::Rc;

pub struct RightDock {
    container: gtk4::Box,
    tab_view: adw::TabView,
    tab_bar: adw::TabBar,
    files_panel: Rc<FilesPanel>,
    preview_panel: Rc<PreviewPanel>,
    terminal_panel: Rc<TerminalPanel>,
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

        let terminal_panel = Rc::new(TerminalPanel::new());
        let page1 = tab_view.append(terminal_panel.widget());
        page1.set_title("Terminal");
        page1.set_icon(Some(&gtk4::gio::ThemedIcon::new(
            "utilities-terminal-symbolic",
        )));

        let files_panel = FilesPanel::new();
        let preview_panel = Rc::new(PreviewPanel::new());
        files_panel.set_preview(preview_panel.clone());

        let page2 = tab_view.append(files_panel.widget());
        page2.set_title("Files");
        page2.set_icon(Some(&gtk4::gio::ThemedIcon::new("folder-symbolic")));

        let page3 = tab_view.append(preview_panel.widget());
        page3.set_title("Preview");
        page3.set_icon(Some(&gtk4::gio::ThemedIcon::new("view-reveal-symbolic")));

        container.append(&tab_view);

        Self {
            container,
            tab_view,
            tab_bar,
            files_panel,
            preview_panel,
            terminal_panel,
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

    pub fn files_panel(&self) -> &Rc<FilesPanel> {
        &self.files_panel
    }

    pub fn preview_panel(&self) -> &Rc<PreviewPanel> {
        &self.preview_panel
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        self.files_panel.set_controller(controller.clone());
        self.terminal_panel.set_controller(controller.clone());
    }

    pub fn terminal_panel(&self) -> &Rc<TerminalPanel> {
        &self.terminal_panel
    }

    pub fn on_session_followed(&self, session_id: &str, cwd: &str) {
        self.terminal_panel.restore(session_id, cwd);
    }

    pub fn selected_index(&self) -> usize {
        self.tab_view
            .selected_page()
            .map(|page| self.tab_view.page_position(&page).max(0) as usize)
            .unwrap_or(0)
    }

    pub fn select_tab(&self, index: usize) {
        let index = index.min(self.tab_view.n_pages().saturating_sub(1) as usize) as i32;
        let page = self.tab_view.nth_page(index);
        self.tab_view.set_selected_page(&page);
    }

    pub fn show_files(&self) {
        let index = self.tab_view.n_pages().saturating_sub(2);
        if index >= 0 {
            let page = self.tab_view.nth_page(index);
            self.tab_view.set_selected_page(&page);
        }
    }

    pub fn show_preview(&self) {
        let index = self.tab_view.n_pages().saturating_sub(1);
        if index >= 0 {
            let page = self.tab_view.nth_page(index);
            self.tab_view.set_selected_page(&page);
        }
    }
}

impl Default for RightDock {
    fn default() -> Self {
        Self::new()
    }
}
