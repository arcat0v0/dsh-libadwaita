use crate::protocol::TodoItem;
use gtk4::prelude::*;

pub struct TodoDock {
    container: gtk4::Box,
    title: gtk4::Label,
    list: gtk4::Box,
}

impl TodoDock {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(4)
            .css_classes(["card"])
            .visible(false)
            .build();
        let title = gtk4::Label::builder()
            .label("任务")
            .css_classes(["heading"])
            .halign(gtk4::Align::Start)
            .build();
        container.append(&title);
        let list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .build();
        container.append(&list);
        Self {
            container,
            title,
            list,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self, items: &[TodoItem]) {
        self.container.set_visible(!items.is_empty());
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let total = items.len();
        let done = items.iter().filter(|item| item.status == "completed").count();
        self.title.set_label(&format!("Todo ({}/{})", done, total));
        if items.is_empty() {
            let empty = gtk4::Label::builder()
                .label("No todos")
                .css_classes(["dim-label", "caption"])
                .halign(gtk4::Align::Start)
                .build();
            self.list.append(&empty);
            return;
        }
        for item in items {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .build();
            let check = gtk4::CheckButton::builder()
                .active(item.status == "completed")
                .sensitive(false)
                .build();
            row.append(&check);
            let label = gtk4::Label::builder()
                .label(&item.content)
                .wrap(true)
                .halign(gtk4::Align::Start)
                .xalign(0.0)
                .build();
            row.append(&label);
            self.list.append(&row);
        }
    }
}

impl Default for TodoDock {
    fn default() -> Self {
        Self::new()
    }
}
pub type TodoPanel = TodoDock;
