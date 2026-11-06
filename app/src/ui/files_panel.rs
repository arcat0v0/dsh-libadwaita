use crate::protocol::{FileEntry, FilesReadBytesResult, FilesReadResult, FilesStatResult};
use crate::ui::controller::SessionController;
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
    path_label: gtk4::Label,
    controller: RefCell<Option<Rc<SessionController>>>,
    preview: RefCell<Option<Rc<PreviewPanel>>>,
    current_path: RefCell<String>,
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

        let path_label = gtk4::Label::builder()
            .label(".")
            .css_classes(["dim-label", "caption"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .ellipsize(gtk4::pango::EllipsizeMode::Middle)
            .build();
        header.append(&path_label);

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
            path_label,
            controller: RefCell::new(None),
            preview: RefCell::new(None),
            current_path: RefCell::new(".".to_string()),
        });

        let weak = Rc::downgrade(&panel);
        refresh_btn.connect_clicked(move |_| {
            if let Some(panel) = weak.upgrade() {
                panel.reload();
            }
        });

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

    pub fn reload(&self) {
        let Some(controller) = self.controller.borrow().clone() else {
            return;
        };
        let path = self.current_path.borrow().clone();
        controller.refresh_files(&path);
    }

    pub fn navigate_to(&self, path: &str) {
        *self.current_path.borrow_mut() = path.to_string();
        self.reload();
    }

    pub fn preview_file(&self, path: &str) {
        let Some(controller) = self.controller.borrow().clone() else {
            return;
        };
        controller.preview_file(path);
    }

    pub fn apply_listing(self: &Rc<Self>, path: &str, entries: &[FileEntry]) {
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.path_label.set_label(path);
        self.status_label.set_label(&format!("{} entries", entries.len()));
        for entry in entries {
            self.list_box.append(&build_entry_row(entry, self));
        }
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
}

fn build_entry_row(entry: &FileEntry, panel: &Rc<FilesPanel>) -> gtk4::Widget {
    let icon = match entry.kind.as_str() {
        "directory" => "folder-symbolic",
        "symlink" => "emblem-symbolic-link-symbolic",
        _ => "text-x-generic-symbolic",
    };
    let row = adw::ActionRow::builder()
        .title(&entry.name)
        .subtitle(&format!("{} — {} bytes", entry.kind, entry.size))
        .activatable(true)
        .build();
    row.add_prefix(&gtk4::Image::from_icon_name(icon));

    let save = gtk4::Button::builder()
        .icon_name("document-save-symbolic")
        .tooltip_text("Save As")
        .css_classes(["flat", "circular"])
        .valign(gtk4::Align::Center)
        .build();
    let save_panel = panel.clone();
    let save_path = entry.path.clone();
    save.connect_clicked(move |_| save_panel.save_file(&save_path));
    row.add_suffix(&save);

    let panel = panel.clone();
    let path = entry.path.clone();
    let kind = entry.kind.clone();
    row.connect_activated(move |_| {
        if kind == "directory" {
            panel.navigate_to(&path);
        } else {
            panel.preview_file(&path);
        }
    });

    row.upcast()
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
