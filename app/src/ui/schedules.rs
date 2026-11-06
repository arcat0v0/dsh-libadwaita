use crate::ui::controller::SessionController;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub struct SchedulesPage {
    widget: gtk4::Box,
    list: gtk4::ListBox,
    status: gtk4::Label,
    controller: RefCell<Option<Weak<SessionController>>>,
}

impl SchedulesPage {
    pub fn new() -> Self {
        let widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .build();
        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let title = gtk4::Label::builder()
            .label("Schedules")
            .css_classes(["title-3"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        header.append(&title);
        let refresh = gtk4::Button::builder().label("Refresh").build();
        let create = gtk4::Button::builder()
            .label("New schedule…")
            .css_classes(["suggested-action"])
            .build();
        header.append(&refresh);
        header.append(&create);
        widget.append(&header);
        let status = gtk4::Label::builder()
            .label("Loading…")
            .xalign(0.0)
            .wrap(true)
            .css_classes(["dim-label", "caption"])
            .build();
        widget.append(&status);
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        widget.append(&list);

        let page = Self {
            widget,
            list,
            status,
            controller: RefCell::new(None),
        };
        page.wire_refresh(refresh);
        page.wire_create(create);
        page
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        *self.controller.borrow_mut() = Some(Rc::downgrade(&controller));
        self.load();
    }

    fn with_controller(&self, f: impl FnOnce(&SessionController)) {
        if let Some(controller) = self
            .controller
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
        {
            f(&controller);
        }
    }

    pub fn load(&self) {
        self.status.set_label("Loading schedules…");
        self.with_controller(|ctrl| ctrl.load_schedules());
    }

    pub fn handle_catalog(&self, entries: &[serde_json::Value]) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        if entries.is_empty() {
            self.status.set_label("No schedules");
            return;
        }
        self.status.set_label(&format!("{} schedules", entries.len()));
        for entry in entries {
            let id = entry["id"].as_str().unwrap_or_default().to_string();
            let title = entry["title"]
                .as_str()
                .or_else(|| entry["name"].as_str())
                .unwrap_or("Untitled");
            let next = entry["nextRunAt"]
                .as_str()
                .or_else(|| entry["next"].as_str())
                .unwrap_or("");
            let row = adw::ActionRow::builder()
                .title(title)
                .subtitle(if next.is_empty() {
                    format!("id {}", id)
                } else {
                    format!("next {}", next)
                })
                .build();
            let run = gtk4::Button::builder()
                .label("Run")
                .css_classes(["flat"])
                .valign(gtk4::Align::Center)
                .build();
            let run_id = id.clone();
            let page_weak = self.controller.clone();
            run.connect_clicked(move |_| {
                if let Some(ctrl) = page_weak
                    .borrow()
                    .as_ref()
                    .and_then(|weak| weak.upgrade())
                {
                    ctrl.schedule_run(&run_id);
                }
            });
            row.add_suffix(&run);
            let history = gtk4::Button::builder()
                .label("History")
                .css_classes(["flat"])
                .valign(gtk4::Align::Center)
                .build();
            let history_id = id.clone();
            let page_weak = self.controller.clone();
            history.connect_clicked(move |_| {
                if let Some(ctrl) = page_weak
                    .borrow()
                    .as_ref()
                    .and_then(|weak| weak.upgrade())
                {
                    ctrl.schedule_history(&history_id);
                }
            });
            row.add_suffix(&history);
            let edit = gtk4::Button::builder()
                .label("Edit…")
                .css_classes(["flat"])
                .valign(gtk4::Align::Center)
                .build();
            let edit_id = id.clone();
            let page_weak = self.controller.clone();
            let parent = self.widget.clone();
            edit.connect_clicked(move |_| {
                let dialog = adw::Dialog::builder()
                    .title("Edit Schedule Patch")
                    .content_width(480)
                    .build();
                let content = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Vertical)
                    .spacing(10)
                    .margin_start(16)
                    .margin_end(16)
                    .margin_top(16)
                    .margin_bottom(16)
                    .build();
                let entry = adw::EntryRow::builder()
                    .title("JSON patch")
                    .text("{}")
                    .build();
                content.append(&entry);
                let save = gtk4::Button::builder()
                    .label("Save")
                    .css_classes(["suggested-action"])
                    .build();
                let dialog_for_save = dialog.clone();
                let entry_for_save = entry.clone();
                let id_for_save = edit_id.clone();
                let page_for_save = page_weak.clone();
                save.connect_clicked(move |_| {
                    if let Ok(patch) = serde_json::from_str::<serde_json::Value>(&entry_for_save.text())
                    {
                        if let Some(ctrl) = page_for_save
                            .borrow()
                            .as_ref()
                            .and_then(|weak| weak.upgrade())
                        {
                            ctrl.schedule_edit(&id_for_save, patch);
                        }
                    }
                    dialog_for_save.close();
                });
                content.append(&save);
                dialog.set_child(Some(&content));
                dialog.present(Some(&parent));
            });
            row.add_suffix(&edit);
            let delete = gtk4::Button::builder()
                .label("Delete")
                .css_classes(["flat", "destructive-action"])
                .valign(gtk4::Align::Center)
                .build();
            let delete_id = id.clone();
            let page_weak = self.controller.clone();
            let parent = self.widget.clone();
            delete.connect_clicked(move |_| {
                let dialog = adw::AlertDialog::builder()
                    .heading("Delete schedule?")
                    .body("This removes the schedule from the catalog.")
                    .build();
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("delete", "Delete");
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                dialog.set_close_response("cancel");
                let page_for_delete = page_weak.clone();
                let id_for_delete = delete_id.clone();
                dialog.choose(
                    Some(&parent),
                    None::<&gtk4::gio::Cancellable>,
                    move |response| {
                        if response == "delete" {
                            if let Some(ctrl) = page_for_delete
                                .borrow()
                                .as_ref()
                                .and_then(|weak| weak.upgrade())
                            {
                                ctrl.schedule_delete(&id_for_delete);
                            }
                        }
                    },
                );
            });
            row.add_suffix(&delete);
            self.list.append(&row);
        }
    }

    pub fn show_history(&self, value: &serde_json::Value) {
        let dialog = adw::Dialog::builder()
            .title("Schedule History")
            .content_width(560)
            .build();
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
        let label = gtk4::Label::builder()
            .label(text)
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .css_classes(["monospace", "caption"])
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .min_content_height(240)
            .child(&label)
            .build();
        content.append(&scroll);
        dialog.set_child(Some(&content));
        dialog.present(Some(&self.widget));
    }

    fn wire_refresh(&self, button: gtk4::Button) {
        let weak = self.controller.clone();
        button.connect_clicked(move |_| {
            if let Some(ctrl) = weak.borrow().as_ref().and_then(|weak| weak.upgrade()) {
                ctrl.load_schedules();
            }
        });
    }

    fn wire_create(&self, button: gtk4::Button) {
        let weak = self.controller.clone();
        let parent = self.widget.clone();
        button.connect_clicked(move |_| {
            let dialog = adw::Dialog::builder()
                .title("New Schedule")
                .content_width(520)
                .build();
            let content = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(10)
                .margin_start(16)
                .margin_end(16)
                .margin_top(16)
                .margin_bottom(16)
                .build();
            let title = adw::EntryRow::builder().title("Title").build();
            let prompt = adw::EntryRow::builder().title("Prompt").build();
            let timing = adw::EntryRow::builder()
                .title("Timing JSON")
                .text(r#"{"kind":"after","after":"5m"}"#)
                .build();
            content.append(&title);
            content.append(&prompt);
            content.append(&timing);
            let create = gtk4::Button::builder()
                .label("Create")
                .css_classes(["suggested-action"])
                .build();
            let dialog_for_create = dialog.clone();
            let weak_for_create = weak.clone();
            let title_for_create = title.clone();
            let prompt_for_create = prompt.clone();
            let timing_for_create = timing.clone();
            create.connect_clicked(move |_| {
                if let Ok(timing_value) =
                    serde_json::from_str::<serde_json::Value>(&timing_for_create.text())
                {
                    if let Some(ctrl) = weak_for_create
                        .borrow()
                        .as_ref()
                        .and_then(|weak| weak.upgrade())
                    {
                        ctrl.schedule_create(
                            title_for_create.text().as_str(),
                            prompt_for_create.text().as_str(),
                            timing_value,
                        );
                    }
                }
                dialog_for_create.close();
            });
            content.append(&create);
            dialog.set_child(Some(&content));
            dialog.present(Some(&parent));
        });
    }
}

impl Default for SchedulesPage {
    fn default() -> Self {
        Self::new()
    }
}
