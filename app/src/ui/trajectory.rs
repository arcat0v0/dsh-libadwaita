use crate::protocol::SessionEvent;
use gtk4::prelude::*;
use libadwaita as adw;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct TrajectoryView {
    widget: gtk4::Box,
    store: gtk4::gio::ListStore,
    search: gtk4::SearchEntry,
    inspector: sourceview5::View,
    events: Rc<RefCell<Vec<SessionEvent>>>,
    filtered: Rc<RefCell<Vec<usize>>>,
}

impl TrajectoryView {
    pub fn new() -> Self {
        let widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        let search = gtk4::SearchEntry::builder()
            .placeholder_text("Filter trajectory by type or text")
            .margin_start(8)
            .margin_end(8)
            .margin_top(6)
            .build();
        widget.append(&search);
        let store = gtk4::gio::ListStore::new::<gtk4::glib::BoxedAnyObject>();
        let selection = gtk4::SingleSelection::new(Some(store.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let column_view = gtk4::ColumnView::new(Some(selection.clone()));
        column_view.set_vexpand(true);
        let events: Rc<RefCell<Vec<SessionEvent>>> = Rc::new(RefCell::new(Vec::new()));
        let filtered: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));

        for (title, field) in [
            ("Seq", "seq"),
            ("Time", "time"),
            ("Type", "type"),
            ("Summary", "summary"),
        ] {
            let factory = gtk4::SignalListItemFactory::new();
            let events_for_bind = events.clone();
            let filtered_for_bind = filtered.clone();
            factory.connect_setup(|_, item| {
                let label = gtk4::Label::builder()
                    .xalign(0.0)
                    .selectable(true)
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .build();
                if let Some(item) = item.downcast_ref::<gtk4::ListItem>() {
                    item.set_child(Some(&label));
                }
            });
            factory.connect_bind(move |_, item| {
                let Some(item) = item.downcast_ref::<gtk4::ListItem>() else {
                    return;
                };
                let Some(label) = item.child().and_downcast::<gtk4::Label>() else {
                    return;
                };
                let Some(boxed) = item
                    .item()
                    .and_downcast::<gtk4::glib::BoxedAnyObject>()
                else {
                    return;
                };
                let row_index = *boxed.borrow::<usize>();
                let Some(event_index) = filtered_for_bind.borrow().get(row_index).copied() else {
                    return;
                };
                let events = events_for_bind.borrow();
                let Some(event) = events.get(event_index) else {
                    return;
                };
                let text = match field {
                    "seq" => event.seq.to_string(),
                    "time" => event.time.to_string(),
                    "type" => event.event_type.clone(),
                    _ => summary(&event.data),
                };
                label.set_label(&text);
                label.set_tooltip_text(Some(&text));
            });
            let column = gtk4::ColumnViewColumn::new(Some(title), Some(factory));
            column.set_resizable(true);
            column.set_expand(title == "Summary");
            column_view.append_column(&column);
        }

        let inspector = sourceview5::View::new();
        inspector.set_editable(false);
        inspector.set_monospace(true);
        inspector.set_show_line_numbers(true);
        inspector.set_left_margin(8);
        inspector.set_right_margin(8);
        inspector.set_top_margin(6);
        inspector.set_bottom_margin(6);
        let inspector_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&inspector)
            .build();
        let event_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .child(&column_view)
            .build();
        let paned = gtk4::Paned::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .position(520)
            .start_child(&event_scroll)
            .end_child(&inspector_scroll)
            .vexpand(true)
            .build();
        widget.append(&paned);

        let view = Self {
            widget,
            store,
            search,
            inspector,
            events,
            filtered,
        };
        let selection_inspector = view.inspector.clone();
        let selection_events = view.events.clone();
        let selection_filtered = view.filtered.clone();
        selection.connect_selected_notify(move |selection| {
            let Some(item) = selection.selected_item() else {
                return;
            };
            let Some(boxed) = item.downcast::<gtk4::glib::BoxedAnyObject>().ok() else {
                return;
            };
            let index = *boxed.borrow::<usize>();
            let Some(event_index) = selection_filtered.borrow().get(index).copied() else {
                return;
            };
            let events = selection_events.borrow();
            if let Some(event) = events.get(event_index) {
                let text = serde_json::to_string_pretty(event).unwrap_or_default();
                selection_inspector.buffer().set_text(&text);
            }
        });
        let view_for_search = view.clone();
        view.search.connect_search_changed(move |_| {
            view_for_search.rebuild();
        });
        view
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.widget
    }

    pub fn set_events(&self, events: &[SessionEvent]) {
        *self.events.borrow_mut() = events.to_vec();
        self.rebuild();
    }

    pub fn append_event(&self, event: SessionEvent) {
        let mut events = self.events.borrow_mut();
        if events
            .last()
            .map(|existing| existing.seq == event.seq)
            .unwrap_or(false)
        {
            return;
        }
        events.push(event);
        while events.len() > crate::session::MAX_RAW_EVENTS {
            events.remove(0);
        }
        drop(events);
        self.rebuild();
    }

    fn rebuild(&self) {
        while self.store.n_items() > 0 {
            self.store.remove(0);
        }
        let query = self.search.text().to_string().to_ascii_lowercase();
        let rows = {
            let events = self.events.borrow();
            let indices = filter_event_indices(&events, &query);
            *self.filtered.borrow_mut() = indices.clone();
            indices
        };
        for row in 0..rows.len() {
            self.store
                .append(&gtk4::glib::BoxedAnyObject::new(row));
        }
        if rows.is_empty() {
            self.inspector.buffer().set_text("");
        }
    }
}

impl Clone for TrajectoryView {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            store: self.store.clone(),
            search: self.search.clone(),
            inspector: self.inspector.clone(),
            events: self.events.clone(),
            filtered: self.filtered.clone(),
        }
    }
}

impl Default for TrajectoryView {
    fn default() -> Self {
        Self::new()
    }
}

pub fn filter_event_indices(events: &[SessionEvent], query: &str) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            if query.is_empty() {
                return true;
            }
            let haystack = format!("{} {} {}", event.event_type, event.seq, event.data);
            haystack.to_ascii_lowercase().contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}

fn summary(value: &serde_json::Value) -> String {    let text = value.to_string();
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(160).collect::<String>())
    } else {
        text
    }
}

pub fn placeholder() -> adw::StatusPage {
    adw::StatusPage::builder()
        .title("Trajectory")
        .description("No trajectory available")
        .icon_name("view-list-tree-symbolic")
        .build()
}
