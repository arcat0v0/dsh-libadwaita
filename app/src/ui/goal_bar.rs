use crate::protocol::GoalView;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;

pub struct GoalBar {
    container: gtk4::Box,
    label: gtk4::Label,
    edit: gtk4::Button,
    pause: gtk4::Button,
    resume: gtk4::Button,
    clear: gtk4::Button,
    goal: RefCell<Option<GoalView>>,
}

impl GoalBar {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(4)
            .css_classes(["card"])
            .visible(false)
            .build();
        let label = gtk4::Label::builder()
            .label("")
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .wrap(true)
            .css_classes(["caption"])
            .build();
        container.append(&label);
        let edit = gtk4::Button::builder()
            .label("Edit")
            .css_classes(["flat"])
            .action_name("win.goal-edit")
            .build();
        let pause = gtk4::Button::builder()
            .label("Pause")
            .css_classes(["flat"])
            .action_name("win.goal-pause")
            .build();
        let resume = gtk4::Button::builder()
            .label("Resume")
            .css_classes(["flat"])
            .action_name("win.goal-resume")
            .build();
        let clear = gtk4::Button::builder()
            .label("Clear")
            .css_classes(["flat", "destructive-action"])
            .action_name("win.goal-clear")
            .build();
        container.append(&edit);
        container.append(&pause);
        container.append(&resume);
        container.append(&clear);
        Self {
            container,
            label,
            edit,
            pause,
            resume,
            clear,
            goal: RefCell::new(None),
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn goal(&self) -> Option<GoalView> {
        self.goal.borrow().clone()
    }

    pub fn set_goal(&self, goal: Option<GoalView>) {
        let Some(goal) = goal else {
            self.container.set_visible(false);
            *self.goal.borrow_mut() = None;
            return;
        };
        self.label.set_label(&format!(
            "Goal · {} · {} · rev {}",
            goal.phase, goal.objective, goal.revision
        ));
        self.pause.set_visible(goal.phase == "active");
        self.resume.set_visible(goal.phase == "paused");
        self.edit.set_visible(true);
        self.clear.set_visible(true);
        self.container.set_visible(true);
        *self.goal.borrow_mut() = Some(goal);
    }
}

impl Default for GoalBar {
    fn default() -> Self {
        Self::new()
    }
}

pub fn goal_edit_dialog(parent: &impl IsA<gtk4::Widget>, current: &GoalView, on_save: impl Fn(String) + 'static) {
    let dialog = adw::Dialog::builder()
        .title("Edit Goal")
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
        .title("Objective")
        .text(&current.objective)
        .build();
    let entry_list = gtk4::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk4::SelectionMode::None)
        .build();
    entry_list.append(&entry);
    content.append(&entry_list);
    let save = gtk4::Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    let dialog_for_save = dialog.clone();
    let entry_for_save = entry.clone();
    save.connect_clicked(move |_| {
        let text = entry_for_save.text().to_string();
        if !text.trim().is_empty() {
            on_save(text.trim().to_string());
        }
        dialog_for_save.close();
    });
    content.append(&save);
    dialog.set_child(Some(&content));
    dialog.present(Some(parent));
}
