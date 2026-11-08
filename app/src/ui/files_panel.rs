use crate::protocol::{FileEntry, FilesReadBytesResult, FilesReadResult, FilesStatResult};
use crate::ui::controller::SessionController;
use crate::ui::file_tree::{breadcrumb_segments, FileTreeModel, FlattenRow};
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use sourceview5::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct FilesPanel {
    container: gtk4::Box,
    list_box: gtk4::ListBox,
    status_label: gtk4::Label,
    breadcrumb_box: gtk4::Box,
    controller: RefCell<Option<Rc<SessionController>>>,
    preview: RefCell<Option<Rc<PreviewPanel>>>,
    model: RefCell<FileTreeModel>,
    breadcrumb_path: RefCell<Option<String>>,
}

impl FilesPanel {
    pub fn new() -> Rc<Self> {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .margin_start(8)
            .margin_end(8)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();

        let breadcrumb_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(2)
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        header.append(&breadcrumb_box);

        let refresh_btn = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh")
            .css_classes(["flat", "circular"])
            .build();
        header.append(&refresh_btn);
        container.append(&header);

        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let list_box = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::Single)
            .css_classes(["boxed-list"])
            .build();
        scroll.set_child(Some(&list_box));
        container.append(&scroll);

        let status_label = gtk4::Label::builder()
            .label("")
            .css_classes(["dim-label", "caption"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .build();
        container.append(&status_label);

        let panel = Rc::new(Self {
            container,
            list_box,
            status_label,
            breadcrumb_box,
            controller: RefCell::new(None),
            preview: RefCell::new(None),
            model: RefCell::new(FileTreeModel::new("")),
            breadcrumb_path: RefCell::new(None),
        });

        let weak = Rc::downgrade(&panel);
        refresh_btn.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.reload();
            }
        });

        panel.render_breadcrumb();
        panel
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        *self.controller.borrow_mut() = Some(controller);
    }

    pub fn set_preview(&self, preview: Rc<PreviewPanel>) {
        *self.preview.borrow_mut() = Some(preview);
    }

    pub fn reload(self: &Rc<Self>) {
        let Some(controller) = self.controller.borrow().clone() else {
            return;
        };
        self.model.borrow_mut().reset();
        *self.breadcrumb_path.borrow_mut() = None;
        self.render_breadcrumb();
        self.render_rows();
        controller.refresh_files(".");
    }

    pub fn preview_file(self: &Rc<Self>, path: &str) {
        self.locate(path);
        let Some(controller) = self.controller.borrow().clone() else {
            return;
        };
        controller.preview_file(path);
    }

    pub fn locate(self: &Rc<Self>, path: &str) {
        let plan = {
            let mut model = self.model.borrow_mut();
            if model.root().is_empty() {
                return;
            }
            if !breadcrumb_segments(model.root(), path).is_empty() {
                *self.breadcrumb_path.borrow_mut() = Some(path.to_string());
            }
            model.expand_ancestors(path)
        };
        if let Some(controller) = self.controller.borrow().clone() {
            for directory in plan.to_fetch {
                controller.refresh_files(&directory);
            }
        }
        self.render_breadcrumb();
        self.render_rows();
    }

    pub fn apply_listing(self: &Rc<Self>, path: &str, workdir: &str, entries: &[FileEntry]) {
        {
            let mut model = self.model.borrow_mut();
            if !workdir.is_empty() {
                model.set_root(workdir);
            } else if model.root().is_empty() {
                model.set_root(path);
            }
            model.apply_listing(path, entries);
        }
        self.render_breadcrumb();
        self.render_rows();
        self.status_label.set_label(&format!("{} entries", entries.len()));
    }

    pub fn set_status(&self, text: &str) {
        self.status_label.set_label(text);
    }

    pub fn set_stat(&self, stat: &FilesStatResult) {
        self.status_label.set_label(&stat_label(stat));
    }

    pub fn save_file(&self, path: &str) {
        if let Some(controller) = self.controller.borrow().clone() {
            controller.save_file_as(path);
        }
    }

    pub fn preview_panel(&self) -> Option<Rc<PreviewPanel>> {
        self.preview.borrow().clone()
    }

    fn fetch_directory(self: &Rc<Self>, path: &str) {
        {
            let mut model = self.model.borrow_mut();
            if model.level(path).is_some() || model.is_loading(path) {
                return;
            }
            model.mark_loading(path);
        }
        if let Some(controller) = self.controller.borrow().clone() {
            controller.refresh_files(path);
        }
        self.render_rows();
    }

    fn render_rows(self: &Rc<Self>) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        let rows = self.model.borrow().flatten();
        if rows.is_empty() {
            let root_empty = self.model.borrow().level(".").is_none();
            if root_empty {
                let loading = gtk4::Label::builder()
                    .label("Loading workspace tree…")
                    .css_classes(["dim-label", "caption"])
                    .halign(gtk4::Align::Start)
                    .build();
                self.list_box.append(&loading);
            }
            return;
        }
        for row in rows {
            self.list_box.append(&build_tree_row(&row, self));
        }
    }

    fn render_breadcrumb(self: &Rc<Self>) {
        while let Some(child) = self.breadcrumb_box.first_child() {
            self.breadcrumb_box.remove(&child);
        }
        let workdir = self.model.borrow().root().to_string();
        let target = self.breadcrumb_path.borrow().clone();
        let segments = match target {
            Some(path) if !workdir.is_empty() => breadcrumb_segments(&workdir, &path),
            _ if !workdir.is_empty() => breadcrumb_segments(&workdir, "."),
            _ => Vec::new(),
        };
        if segments.is_empty() {
            let placeholder = gtk4::Label::builder()
                .label("No workspace")
                .css_classes(["dim-label", "caption"])
                .halign(gtk4::Align::Start)
                .build();
            self.breadcrumb_box.append(&placeholder);
            return;
        }
        for (index, (name, path)) in segments.iter().enumerate() {
            if index > 0 {
                let separator = gtk4::Label::builder()
                    .label("›")
                    .css_classes(["dim-label", "caption"])
                    .build();
                self.breadcrumb_box.append(&separator);
            }
            let button = gtk4::Button::builder()
                .label(name)
                .css_classes(["flat"])
                .build();
            let weak = Rc::downgrade(self);
            let chain: Vec<String> = segments
                .iter()
                .take(index + 1)
                .map(|(_, segment)| segment.clone())
                .collect();
            let segment_path = path.clone();
            button.connect_clicked(move |_| {
                if let Some(panel) = weak.upgrade() {
                    for ancestor in &chain {
                        panel.model.borrow_mut().expand(ancestor);
                    }
                    panel.fetch_directory(&segment_path);
                    panel.render_rows();
                }
            });
            self.breadcrumb_box.append(&button);
        }
    }
}

fn build_tree_row(row: &FlattenRow, panel: &Rc<FilesPanel>) -> gtk4::Widget {
    let entry = &row.entry;
    let is_directory = entry.kind == "directory";
    let icon = match entry.kind.as_str() {
        "directory" => "folder-symbolic",
        "symlink" => "emblem-symbolic-link-symbolic",
        _ => "text-x-generic-symbolic",
    };
    let (expanded, loading, failure) = {
        let model = panel.model.borrow();
        (
            model.is_expanded(&entry.path),
            model.is_loading(&entry.path),
            model.failure(&entry.path),
        )
    };
    let subtitle = if is_directory {
        if loading {
            "Loading…".to_string()
        } else if failure {
            "Failed to load".to_string()
        } else if expanded {
            "directory · expanded".to_string()
        } else {
            "directory".to_string()
        }
    } else {
        format!("{} — {} bytes", entry.kind, entry.size)
    };
    let action_row = adw::ActionRow::builder()
        .title(&entry.name)
        .subtitle(&subtitle)
        .activatable(true)
        .margin_start(row.depth as i32 * 18)
        .build();

    if is_directory {
        let chevron = gtk4::Image::from_icon_name(if expanded {
            "pan-down-symbolic"
        } else {
            "pan-right-symbolic"
        });
        let toggle = gtk4::Button::builder()
            .child(&chevron)
            .css_classes(["flat", "circular"])
            .valign(gtk4::Align::Center)
            .build();
        let weak = Rc::downgrade(panel);
        let path = entry.path.clone();
        toggle.connect_clicked(move |_| {
            let Some(panel) = weak.upgrade() else {
                return;
            };
            let now_expanded = panel.model.borrow_mut().toggle(&path);
            if now_expanded {
                panel.fetch_directory(&path);
            } else {
                panel.render_rows();
            }
        });
        action_row.add_prefix(&toggle);
    }
    action_row.add_prefix(&gtk4::Image::from_icon_name(icon));

    if !is_directory {
        let save = gtk4::Button::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Save As")
            .css_classes(["flat", "circular"])
            .valign(gtk4::Align::Center)
            .build();
        let save_panel = panel.clone();
        let save_path = entry.path.clone();
        save.connect_clicked(move |_| save_panel.save_file(&save_path));
        action_row.add_suffix(&save);
    }

    let panel = panel.clone();
    let path = entry.path.clone();
    let kind = entry.kind.clone();
    action_row.connect_activated(move |_| {
        if kind == "directory" {
            let now_expanded = panel.model.borrow_mut().toggle(&path);
            if now_expanded {
                panel.fetch_directory(&path);
            } else {
                panel.render_rows();
            }
        } else {
            panel.preview_file(&path);
        }
    });

    action_row.upcast()
}

pub struct PreviewPanel {
    container: gtk4::Box,
    content: RefCell<Option<gtk4::Widget>>,
    status_label: gtk4::Label,
    pending_line: Cell<Option<u32>>,
    copy_png: gtk4::Button,
    open_external: gtk4::Button,
    current_path: RefCell<String>,
    texture: RefCell<Option<gtk4::gdk::Texture>>,
}

impl PreviewPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .margin_start(8)
            .margin_end(8)
            .margin_top(6)
            .margin_bottom(6)
            .build();

        let status_label = gtk4::Label::builder()
            .label("Select a file to preview")
            .css_classes(["dim-label", "caption"])
            .halign(gtk4::Align::Start)
            .wrap(true)
            .build();
        let copy_png = gtk4::Button::builder()
            .label("Copy PNG")
            .css_classes(["flat"])
            .visible(false)
            .build();
        let open_external = gtk4::Button::builder()
            .label("Open Externally")
            .css_classes(["flat"])
            .visible(false)
            .build();
        container.append(&status_label);
        container.append(&copy_png);
        container.append(&open_external);

        let panel = Self {
            container,
            content: RefCell::new(None),
            status_label,
            pending_line: Cell::new(None),
            copy_png,
            open_external,
            current_path: RefCell::new(String::new()),
            texture: RefCell::new(None),
        };
        let texture_slot = panel.texture.clone();
        panel.copy_png.connect_clicked(move |_| {
            if let Some(texture) = texture_slot.borrow().as_ref() {
                if let Some(display) = gtk4::gdk::Display::default() {
                    display.clipboard().set_texture(texture);
                }
            }
        });
        let path_slot = panel.current_path.clone();
        panel.open_external.connect_clicked(move |_| {
            let path = path_slot.borrow().clone();
            if !path.is_empty() {
                crate::ui::desktop::open_path_in_default_app(std::path::Path::new(&path));
            }
        });
        panel
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn clear(&self) {
        if let Some(widget) = self.content.borrow_mut().take() {
            self.container.remove(&widget);
        }
        self.status_label.set_label("Select a file to preview");
        self.copy_png.set_visible(false);
        self.open_external.set_visible(false);
        self.current_path.borrow_mut().clear();
        *self.texture.borrow_mut() = None;
    }

    pub fn show_unavailable(&self, reason: &str) {
        self.clear();
        self.status_label.set_label(reason);
    }

    pub fn set_pending_line(&self, line: Option<u32>) {
        self.pending_line.set(line);
    }
    pub fn show_text(&self, path: &str, text: &str, truncated: bool) {
        self.clear();
        self.copy_png.set_visible(false);
        let buffer = sourceview5::Buffer::new(None);
        if let Some(lang) = sourceview5::LanguageManager::default().language(language_for(path)) {
            buffer.set_language(Some(&lang));
        }
        buffer.set_text(text);

        let view = sourceview5::View::with_buffer(&buffer);
        view.set_show_line_numbers(true);
        view.set_monospace(true);
        view.set_editable(false);
        view.set_left_margin(8);
        view.set_right_margin(8);
        view.set_top_margin(4);
        view.set_bottom_margin(6);

        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();
        scroll.set_child(Some(&view));

        *self.current_path.borrow_mut() = path.to_string();
        self.open_external.set_visible(true);
        self.status_label.set_label(&format!(
            "{}{}",
            path,
            if truncated { " (truncated)" } else { "" }
        ));
        self.container.append(&scroll);
        if let Some(line) = self.pending_line.take() {
            if let Some(mut iter) = buffer.iter_at_line(line.saturating_sub(1) as i32) {
                view.scroll_to_iter(&mut iter, 0.0, false, 0.0, 0.0);
            }
        }
        *self.content.borrow_mut() = Some(scroll.upcast());
    }

    pub fn show_image(&self, path: &str, bytes: &[u8]) {
        self.clear();
        let texture = match gtk4::gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes.to_vec())) {
            Ok(texture) => texture,
            Err(_) => {
                self.status_label
                    .set_label(&format!("{}: unsupported image data", path));
                return;
            }
        };
        let picture = gtk4::Picture::builder()
            .content_fit(gtk4::ContentFit::ScaleDown)
            .can_shrink(true)
            .build();
        picture.set_paintable(Some(&texture));

        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();
        scroll.set_child(Some(&picture));

        self.status_label.set_label(path);
        *self.current_path.borrow_mut() = path.to_string();
        self.open_external.set_visible(true);
        self.container.append(&scroll);
        *self.content.borrow_mut() = Some(scroll.upcast());
        *self.texture.borrow_mut() = Some(texture);
        self.copy_png.set_visible(true);
    }
}

pub fn language_for(path: &str) -> &'static str {
    let lowered = path.to_ascii_lowercase();
    if lowered.ends_with(".rs") {
        "rust"
    } else if lowered.ends_with(".ts") || lowered.ends_with(".tsx") {
        "typescript"
    } else if lowered.ends_with(".js") || lowered.ends_with(".jsx") {
        "javascript"
    } else if lowered.ends_with(".json") {
        "json"
    } else if lowered.ends_with(".toml") {
        "toml"
    } else if lowered.ends_with(".yaml") || lowered.ends_with(".yml") {
        "yaml"
    } else if lowered.ends_with(".sh") || lowered.ends_with(".bash") {
        "sh"
    } else if lowered.ends_with(".py") {
        "python"
    } else if lowered.ends_with(".c") || lowered.ends_with(".h") {
        "c"
    } else if lowered.ends_with(".cpp") || lowered.ends_with(".hpp") {
        "cpp"
    } else if lowered.ends_with(".md") {
        "markdown"
    } else {
        "text"
    }
}

pub fn is_image_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.ends_with(".png")
        || lowered.ends_with(".jpg")
        || lowered.ends_with(".jpeg")
        || lowered.ends_with(".webp")
        || lowered.ends_with(".gif")
        || lowered.ends_with(".svg")
}

pub fn stat_label(result: &FilesStatResult) -> String {
    format!(
        "{} — {} bytes — modified {}",
        result.path, result.size, result.mtime
    )
}

pub fn read_text_result(result: &FilesReadResult) -> Option<(String, bool)> {
    if result.kind != "text" {
        return None;
    }
    let text = result.text.clone().unwrap_or_default();
    Some((text, result.truncated.unwrap_or(false)))
}

pub fn read_bytes_result(result: &FilesReadBytesResult) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(result.data.as_bytes())
        .ok()
}
