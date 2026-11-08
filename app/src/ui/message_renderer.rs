use crate::protocol::{MessageFeedbackItem, TodoItem, UserImageAttachment};
use crate::transcript::{MessageAttachment, MessageFileAttachment, RenderBlock, ToolCallItem};
use crate::ui::controller::SessionController;
use crate::ui::presentation::PresentationPolicy;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

thread_local! {
    static UI_CONTROLLER: RefCell<Option<Weak<SessionController>>> = const { RefCell::new(None) };
    static FEEDBACK: RefCell<HashMap<String, MessageFeedbackItem>> = RefCell::new(HashMap::new());
}

pub fn set_controller(controller: &Rc<SessionController>) {
    UI_CONTROLLER.with(|slot| {
        *slot.borrow_mut() = Some(Rc::downgrade(controller));
    });
}

pub fn set_feedback_items(items: &[MessageFeedbackItem]) {
    FEEDBACK.with(|feedback| {
        let mut map = feedback.borrow_mut();
        map.clear();
        for item in items {
            map.insert(item.message_id.clone(), item.clone());
        }
    });
}

fn feedback_for(message_id: &str) -> Option<MessageFeedbackItem> {
    FEEDBACK.with(|feedback| feedback.borrow().get(message_id).cloned())
}

fn with_controller(f: impl FnOnce(&SessionController)) {
    UI_CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref().and_then(|weak| weak.upgrade()) {
            f(&controller);
        }
    });
}

pub fn reopen_question() {
    with_controller(|controller| controller.reopen_question());
}

fn collect_widget_text(widget: &gtk4::Widget) -> String {
    if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
        return label.text().to_string();
    }
    let mut parts = Vec::new();
    let mut child = widget.first_child();
    while let Some(current) = child {
        parts.push(collect_widget_text(&current));
        child = current.next_sibling();
    }
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}


fn detach_popover_on_destroy(anchor: &impl IsA<gtk4::Widget>, popover: &gtk4::Popover) {
    let popover = popover.clone();
    anchor.connect_destroy(move |_| {
        popover.unparent();
    });
}

fn attach_message_context_menu(widget: &impl IsA<gtk4::Widget>, text: String) {
    let popover = gtk4::Popover::builder().build();
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let current = Rc::new(RefCell::new(text));
    let copy = gtk4::Button::builder()
        .label("Copy")
        .css_classes(["flat"])
        .build();
    let current_copy = current.clone();
    let pop_copy = popover.clone();
    copy.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&current_copy.borrow());
        }
        pop_copy.popdown();
    });
    box_widget.append(&copy);
    let quote = gtk4::Button::builder()
        .label("Quote into composer")
        .css_classes(["flat"])
        .build();
    let current_quote = current.clone();
    let pop_quote = popover.clone();
    quote.connect_clicked(move |_| {
        let text = current_quote.borrow().clone();
        if !text.trim().is_empty() {
            with_controller(|controller| {
                controller.insert_composer_text(&format!("> {}\n", text));
            });
        }
        pop_quote.popdown();
    });
    box_widget.append(&quote);
    popover.set_child(Some(&box_widget));
    popover.set_parent(widget);
    detach_popover_on_destroy(widget, &popover);
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    gesture.connect_pressed(move |_, _, _, _| {
        popover.popup();
    });
    widget.add_controller(gesture);
}

fn attach_dynamic_context_menu(
    widget: &impl IsA<gtk4::Widget>,
    provider: Rc<dyn Fn() -> String>,
) {
    let popover = gtk4::Popover::builder().build();
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let current = Rc::new(RefCell::new(String::new()));
    let copy = gtk4::Button::builder()
        .label("Copy")
        .css_classes(["flat"])
        .build();
    let current_copy = current.clone();
    let pop_copy = popover.clone();
    copy.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&current_copy.borrow());
        }
        pop_copy.popdown();
    });
    box_widget.append(&copy);
    let quote = gtk4::Button::builder()
        .label("Quote into composer")
        .css_classes(["flat"])
        .build();
    let current_quote = current.clone();
    let pop_quote = popover.clone();
    quote.connect_clicked(move |_| {
        let text = current_quote.borrow().clone();
        if !text.trim().is_empty() {
            with_controller(|controller| {
                controller.insert_composer_text(&format!("> {}\n", text));
            });
        }
        pop_quote.popdown();
    });
    box_widget.append(&quote);
    popover.set_child(Some(&box_widget));
    popover.set_parent(widget);
    detach_popover_on_destroy(widget, &popover);
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    let current_for_press = current.clone();
    let provider_for_press = provider.clone();
    gesture.connect_pressed(move |_, _, _, _| {
        *current_for_press.borrow_mut() = provider_for_press();
        popover.popup();
    });
    widget.add_controller(gesture);
}

fn attach_code_context_menu(view: &sourceview5::View, code: String) {
    let popover = gtk4::Popover::builder().build();
    let box_widget = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let copy = gtk4::Button::builder()
        .label("Copy Code")
        .css_classes(["flat"])
        .build();
    let pop_copy = popover.clone();
    copy.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&code);
        }
        pop_copy.popdown();
    });
    box_widget.append(&copy);
    popover.set_child(Some(&box_widget));
    popover.set_parent(view);
    detach_popover_on_destroy(view, &popover);
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(3);
    gesture.connect_pressed(move |_, _, _, _| popover.popup());
    view.add_controller(gesture);
}

fn attach_link_handler(label: &gtk4::Label) {
    label.connect_activate_link(|_, uri| {
        let uri = uri.to_string();
        with_controller(|controller| controller.open_link_target(&uri));
        gtk4::glib::Propagation::Stop
    });
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownOptions {
    pub image_placeholders: bool,
}

pub fn render_markdown(text: &str) -> gtk4::Box {
    render_markdown_with(text, MarkdownOptions::default())
}

pub fn render_markdown_with(text: &str, options: MarkdownOptions) -> gtk4::Box {
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let parser = Parser::new_ext(
        text,
        Options::ENABLE_TABLES
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS,
    );
    let mut current_markup = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut list_index: Option<u64> = None;
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_cell = String::new();
    let mut in_image = false;
    let mut image_alt = String::new();
    let mut image_dest = String::new();
    let mut image_title = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    current_markup.clear();
                }
                Tag::Heading { level, .. } => {
                    current_markup.clear();
                    let css_class = match level {
                        HeadingLevel::H1 => "title-1",
                        HeadingLevel::H2 => "title-2",
                        HeadingLevel::H3 => "title-3",
                        HeadingLevel::H4 => "title-4",
                        _ => "heading",
                    };
                    current_markup.push_str(&format!("<span class=\"{}\"><b>", css_class));
                }
                Tag::BlockQuote(_) => {
                    current_markup.clear();
                    current_markup.push_str("<i>");
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_content.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => "text".to_string(),
                    };
                }
                Tag::List(first_num) => {
                    list_index = first_num;
                }
                Tag::Table(_) => {
                    in_table = true;
                    table_rows.clear();
                }
                Tag::TableHead | Tag::TableRow => {
                    table_rows.push(Vec::new());
                }
                Tag::TableCell => {
                    current_cell.clear();
                }
                Tag::Item => {
                    current_markup.clear();
                    if let Some(idx) = list_index {
                        current_markup.push_str(&format!("{}. ", idx));
                        list_index = Some(idx + 1);
                    } else {
                        current_markup.push_str("• ");
                    }
                }
                Tag::Strong => {
                    current_markup.push_str("<b>");
                }
                Tag::Emphasis => {
                    current_markup.push_str("<i>");
                }
                Tag::Strikethrough => {
                    current_markup.push_str("<s>");
                }
                Tag::Link { dest_url, .. } => {
                    current_markup.push_str(&format!(
                        "<a href=\"{}\">",
                        gtk4::glib::markup_escape_text(&dest_url)
                    ));
                }
                Tag::FootnoteDefinition(name) => {
                    current_markup.push_str(&format!("[{}] ", name));
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    if options.image_placeholders {
                        in_image = true;
                        image_alt.clear();
                        image_dest = dest_url.to_string();
                        image_title = title.to_string();
                    }
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    if !current_markup.is_empty() {
                        let label = gtk4::Label::builder()
                            .use_markup(true)
                            .label(&current_markup)
                            .wrap(true)
                            .wrap_mode(gtk4::pango::WrapMode::WordChar)
                            .halign(gtk4::Align::Start)
                            .xalign(0.0)
                            .selectable(true)
                            .build();
                        attach_link_handler(&label);
                        container.append(&label);
                        current_markup.clear();
                    }
                }
                TagEnd::Heading(_) => {
                    current_markup.push_str("</b></span>");
                    let label = gtk4::Label::builder()
                        .use_markup(true)
                        .label(&current_markup)
                        .wrap(true)
                        .wrap_mode(gtk4::pango::WrapMode::WordChar)
                        .halign(gtk4::Align::Start)
                        .xalign(0.0)
                        .selectable(true)
                        .build();
                    attach_link_handler(&label);
                    container.append(&label);
                    current_markup.clear();
                }
                TagEnd::BlockQuote(_) => {
                    current_markup.push_str("</i>");
                    let bq_box = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Vertical)
                        .margin_start(12)
                        .css_classes(["card"])
                        .build();
                    let label = gtk4::Label::builder()
                        .use_markup(true)
                        .label(&current_markup)
                        .wrap(true)
                        .wrap_mode(gtk4::pango::WrapMode::WordChar)
                        .halign(gtk4::Align::Start)
                        .xalign(0.0)
                        .margin_start(8)
                        .margin_end(8)
                        .margin_top(6)
                        .margin_bottom(6)
                        .selectable(true)
                        .build();
                    attach_link_handler(&label);
                    bq_box.append(&label);
                    container.append(&bq_box);
                    current_markup.clear();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let code_widget = build_code_block(&code_block_lang, &code_block_content);
                    container.append(&code_widget);
                }
                TagEnd::List(_) => {
                    list_index = None;
                }
                TagEnd::Item => {
                    if !current_markup.is_empty() {
                        let label = gtk4::Label::builder()
                            .use_markup(true)
                            .label(&current_markup)
                            .wrap(true)
                            .wrap_mode(gtk4::pango::WrapMode::WordChar)
                            .halign(gtk4::Align::Start)
                            .xalign(0.0)
                            .margin_start(12)
                            .selectable(true)
                            .build();
                        attach_link_handler(&label);
                        container.append(&label);
                        current_markup.clear();
                    }
                }
                TagEnd::Strong => {
                    current_markup.push_str("</b>");
                }
                TagEnd::Emphasis => {
                    current_markup.push_str("</i>");
                }
                TagEnd::Strikethrough => {
                    current_markup.push_str("</s>");
                }
                TagEnd::Link => {
                    current_markup.push_str("</a>");
                }
                TagEnd::TableCell => {
                    if let Some(row) = table_rows.last_mut() {
                        row.push(current_cell.clone());
                    }
                    current_cell.clear();
                }
                TagEnd::Table => {
                    in_table = false;
                    let grid = build_table(&table_rows);
                    container.append(&grid);
                    table_rows.clear();
                }
                TagEnd::Image => {
                    if in_image {
                        in_image = false;
                        append_markup_label(&container, &mut current_markup);
                        let label = image_placeholder_label(&image_alt, &image_title, &image_dest);
                        container.append(&build_readme_image_placeholder(&label, &image_dest));
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_code_block {
                    code_block_content.push_str(&t);
                } else if in_table {
                    current_cell.push_str(&t);
                } else if in_image {
                    image_alt.push_str(&t);
                } else {
                    current_markup.push_str(&gtk4::glib::markup_escape_text(&t));
                }
            }
            Event::Code(c) => {
                if in_code_block {
                    code_block_content.push_str(&c);
                } else if in_table {
                    current_cell.push_str(&c);
                } else if in_image {
                    image_alt.push_str(&c);
                } else {
                    current_markup.push_str(&format!(
                        "<tt>{}</tt>",
                        gtk4::glib::markup_escape_text(&c)
                    ));
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_table {
                    current_cell.push(' ');
                } else if in_image {
                    image_alt.push(' ');
                } else {
                    current_markup.push(' ');
                }
            }
            Event::FootnoteReference(name) => {
                current_markup.push_str(&format!("[{}]", name));
            }
            Event::TaskListMarker(checked) => {
                current_markup.push_str(if checked { "☑ " } else { "☐ " });
            }
            Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
                } else if in_image {
                    image_alt.push(' ');
                } else {
                    current_markup.push_str("\n");
                }
            }
            Event::Rule => {
                let sep = gtk4::Separator::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .margin_top(6)
                    .margin_bottom(6)
                    .build();
                container.append(&sep);
            }
            _ => {}
        }
    }

    if !current_markup.is_empty() {
        let label = gtk4::Label::builder()
            .use_markup(true)
            .label(&current_markup)
            .wrap(true)
            .wrap_mode(gtk4::pango::WrapMode::WordChar)
            .halign(gtk4::Align::Start)
            .xalign(0.0)
            .selectable(true)
            .build();
        attach_link_handler(&label);
        container.append(&label);
    }

    container
}

fn append_markup_label(container: &gtk4::Box, markup: &mut String) {
    if markup.is_empty() {
        return;
    }
    let label = gtk4::Label::builder()
        .use_markup(true)
        .label(&*markup)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .halign(gtk4::Align::Start)
        .xalign(0.0)
        .selectable(true)
        .build();
    attach_link_handler(&label);
    container.append(&label);
    markup.clear();
}

pub fn image_placeholder_label(alt: &str, title: &str, destination: &str) -> String {
    let candidate = [alt, title]
        .into_iter()
        .map(str::trim)
        .find(|text| !text.is_empty());
    let text = match candidate {
        Some(text) => text.to_string(),
        None => destination
            .rsplit(|character| character == '/' || character == '\\')
            .next()
            .unwrap_or(destination)
            .trim()
            .to_string(),
    };
    let label = if text.is_empty() {
        "图片".to_string()
    } else {
        format!("图片：{text}")
    };
    if label.chars().count() > 60 {
        let mut truncated: String = label.chars().take(59).collect();
        truncated.push('…');
        truncated
    } else {
        label
    }
}

fn build_readme_image_placeholder(label: &str, destination: &str) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["dsh-readme-image"])
        .build();
    row.append(&gtk4::Image::from_icon_name("image-x-generic-symbolic"));
    row.append(
        &gtk4::Label::builder()
            .label(label)
            .wrap(true)
            .xalign(0.0)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build(),
    );
    if !destination.is_empty() {
        row.set_tooltip_text(Some(destination));
    }
    row
}

fn build_table(rows: &[Vec<String>]) -> gtk4::ScrolledWindow {
    let grid = gtk4::Grid::builder()
        .column_spacing(12)
        .row_spacing(6)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, cell) in row.iter().enumerate() {
            let label = gtk4::Label::builder()
                .label(cell)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .xalign(0.0)
                .selectable(true)
                .build();
            if row_index == 0 {
                label.add_css_class("heading");
            }
            grid.attach(&label, column_index as i32, row_index as i32, 1, 1);
        }
    }
    gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&grid)
        .build()
}

pub fn build_code_block(lang: &str, code: &str) -> gtk4::Box {
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_top(4)
        .margin_bottom(4)
        .build();

    let header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();

    let display_lang = if lang.is_empty() { "text" } else { lang };
    let lang_label = gtk4::Label::builder()
        .label(display_lang)
        .css_classes(["dim-label", "caption"])
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .build();
    header.append(&lang_label);

    let copy_btn = gtk4::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy code")
        .css_classes(["flat", "circular"])
        .build();

    let code_to_copy = code.to_string();
    copy_btn.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&code_to_copy);
        }
    });
    header.append(&copy_btn);
    container.append(&header);

    let lang_manager = sourceview5::LanguageManager::default();
    let source_lang = if !lang.is_empty() {
        lang_manager.language(lang)
    } else {
        None
    };

    let buffer = sourceview5::Buffer::new(None);
    if let Some(l) = source_lang {
        buffer.set_language(Some(&l));
    }
    buffer.set_text(code);

    let view = sourceview5::View::with_buffer(&buffer);
    view.set_show_line_numbers(true);
    view.set_monospace(true);
    view.set_editable(false);
    view.set_left_margin(8);
    view.set_right_margin(8);
    view.set_top_margin(4);
    view.set_bottom_margin(6);

    let scroll = gtk4::ScrolledWindow::builder()
        .min_content_height(40)
        .max_content_height(350)
        .propagate_natural_height(true)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    scroll.set_child(Some(&view));
    attach_code_context_menu(&view, code.to_string());
    container.append(&scroll);

    container
}

pub struct LiveAssistantWidgets {
    pub container: gtk4::Box,
    pub bubble: gtk4::Box,
    pub content_box: gtk4::Box,
    pub stable_box: gtk4::Box,
    pub tail_box: gtk4::Box,
    pub spinner: gtk4::Spinner,
    pub reasoning_box: gtk4::Box,
    pub reasoning_label: gtk4::Label,
    pub reasoning_expander: gtk4::Expander,
    pub policy: PresentationPolicy,
    pub stable_end: std::cell::Cell<usize>,
    pub text: RefCell<String>,
    pub reasoning_text: RefCell<String>,
    pub running: std::cell::Cell<bool>,
    pub fork_button: RefCell<Option<gtk4::Button>>,
}

#[derive(Clone)]
pub struct AssistantLiveHandle {
    inner: Rc<RefCell<LiveAssistantWidgets>>,
}

impl AssistantLiveHandle {
    pub fn update_text(&self, text: &str) {
        let widgets = self.inner.borrow();
        *widgets.text.borrow_mut() = text.to_string();
        let previous = widgets.stable_end.get();
        let end = crate::ui::stream_markdown::stable_end(text, previous);
        if end > previous {
            while let Some(child) = widgets.stable_box.first_child() {
                widgets.stable_box.remove(&child);
            }
            if end > 0 {
                widgets
                    .stable_box
                    .append(&render_markdown(&text[..end]));
            }
            widgets.stable_end.set(end);
        }
        while let Some(child) = widgets.tail_box.first_child() {
            widgets.tail_box.remove(&child);
        }
        if end < text.len() {
            widgets.tail_box.append(&render_markdown(&text[end..]));
        }
    }

    pub fn update_reasoning(&self, reasoning: &str) {
        let widgets = self.inner.borrow();
        *widgets.reasoning_text.borrow_mut() = reasoning.to_string();
        widgets.reasoning_label.set_label(reasoning);
        widgets
            .reasoning_expander
            .set_visible(!reasoning.is_empty());
        drop(widgets);
        self.refresh_reasoning_header();
    }

    pub fn finalize(&self, seq: Option<i64>, time: u64) {
        let widgets = self.inner.borrow();
        widgets.running.set(false);
        widgets.spinner.stop();
        widgets.spinner.set_visible(false);
        if widgets.fork_button.borrow().is_none() {
            if let Some(message_seq) = seq {
                let text = widgets.text.borrow().clone();
                let (actions, fork) = build_assistant_actions(&text, message_seq, time);
                widgets.bubble.append(&actions);
                *widgets.fork_button.borrow_mut() = Some(fork);
            }
        }
        drop(widgets);
        self.refresh_reasoning_header();
    }

    pub fn set_fork_visible(&self, visible: bool) {
        let widgets = self.inner.borrow();
        if let Some(button) = widgets.fork_button.borrow().as_ref() {
            button.set_visible(visible);
        }
    }

    fn refresh_reasoning_header(&self) {
        let widgets = self.inner.borrow();
        let header = crate::ui::presentation::reasoning_header(
            widgets.reasoning_expander.is_expanded(),
            widgets.policy.settled_reasoning_preview,
            widgets.running.get(),
            &widgets.reasoning_text.borrow(),
        );
        widgets.reasoning_expander.set_label(Some(&header));
    }
}

pub fn build_user_message(
    text: &str,
    _time: u64,
    _seq: i64,
    images: &[UserImageAttachment],
) -> gtk4::Box {
    let masked = crate::drafts::mask_secret_patterns(text);
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::End)
        .margin_start(48)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let bubble = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    let label = gtk4::Label::builder()
        .label(&masked)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .halign(gtk4::Align::Start)
        .xalign(0.0)
        .margin_start(10)
        .margin_end(10)
        .margin_top(8)
        .margin_bottom(8)
        .selectable(true)
        .build();
    bubble.append(&label);
    attach_message_context_menu(&label, text.to_string());
    if !images.is_empty() {
        let gallery = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .max_children_per_line(3)
            .margin_start(10)
            .margin_end(10)
            .margin_top(8)
            .build();
        for image in images {
            gallery.append(&build_user_image(image));
        }
        bubble.append(&gallery);
    }
    if let Some(chips) = build_file_chips(text) {
        bubble.append(&chips);
    }
    let footer = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .halign(gtk4::Align::End)
        .margin_end(8)
        .margin_bottom(6)
        .build();
    let copy = gtk4::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy original text")
        .css_classes(["flat", "circular"])
        .build();
    let copy_text = text.to_string();
    copy.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&copy_text);
        }
    });
    footer.append(&copy);
    if _time > 0 {
        let stamp = gtk4::glib::DateTime::from_unix_local((_time / 1000) as i64)
            .ok()
            .and_then(|date| date.format("%H:%M").ok())
            .map(|value| value.to_string())
            .unwrap_or_default();
        if !stamp.is_empty() {
            let time_label = gtk4::Label::builder()
                .label(&stamp)
                .css_classes(["dim-label", "caption"])
                .build();
            footer.append(&time_label);
        }
    }
    bubble.append(&footer);
    if masked != text {
        let actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .halign(gtk4::Align::End)
            .margin_end(8)
            .margin_bottom(6)
            .build();
        let reveal = gtk4::Button::builder()
            .icon_name("view-reveal-symbolic")
            .tooltip_text("Reveal secrets")
            .css_classes(["flat", "circular"])
            .build();
        let label_toggle = label.clone();
        let original = text.to_string();
        let masked_text = masked.clone();
        let revealed = Rc::new(RefCell::new(false));
        let revealed_toggle = revealed.clone();
        reveal.connect_clicked(move |button| {
            let mut shown = revealed_toggle.borrow_mut();
            *shown = !*shown;
            if *shown {
                label_toggle.set_label(&original);
                button.set_icon_name("view-conceal-symbolic");
                button.set_tooltip_text(Some("Hide secrets"));
            } else {
                label_toggle.set_label(&masked_text);
                button.set_icon_name("view-reveal-symbolic");
                button.set_tooltip_text(Some("Reveal secrets"));
            }
        });
        actions.append(&reveal);
        bubble.append(&actions);
    }
    row.append(&bubble);
    let avatar = gtk4::Image::builder()
        .icon_name("avatar-default-symbolic")
        .pixel_size(24)
        .valign(gtk4::Align::Start)
        .margin_top(8)
        .build();
    row.append(&avatar);
    row
}

pub fn build_attachment_thumbnail(image: &UserImageAttachment) -> gtk4::Button {
    build_user_image(image)
}

fn build_message_file_chip(file: &MessageFileAttachment) -> gtk4::Box {
    let chip = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .css_classes(["card"])
        .margin_start(4)
        .margin_end(4)
        .margin_top(2)
        .margin_bottom(2)
        .build();
    let icon = gtk4::Image::from_icon_name("text-x-generic-symbolic");
    chip.append(&icon);
    let size = file
        .bytes
        .map(crate::ui::conversation::compact_token_count)
        .unwrap_or_default();
    let label = if size.is_empty() {
        file.name.clone()
    } else {
        format!("{} · {}", file.name, size)
    };
    let label = gtk4::Label::builder()
        .label(&label)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .max_width_chars(32)
        .css_classes(["caption"])
        .build();
    chip.append(&label);
    chip
}

fn build_question_reply(
    summary: &str,
    selected: &[String],
    details: &serde_json::Value,
) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::End)
        .margin_start(48)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let bubble = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .css_classes(["card"])
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    let header = gtk4::Label::builder()
        .label("Question reply")
        .css_classes(["dim-label", "caption"])
        .halign(gtk4::Align::Start)
        .build();
    bubble.append(&header);
    let question = gtk4::Label::builder()
        .label(summary)
        .wrap(true)
        .selectable(true)
        .xalign(0.0)
        .margin_start(10)
        .margin_end(10)
        .build();
    bubble.append(&question);
    if selected.is_empty() {
        let empty = gtk4::Label::builder()
            .label("No selected answers")
            .css_classes(["dim-label", "caption"])
            .halign(gtk4::Align::Start)
            .build();
        bubble.append(&empty);
    } else {
        for answer in selected {
            let label = gtk4::Label::builder()
                .label(&format!("• {}", answer))
                .wrap(true)
                .selectable(true)
                .xalign(0.0)
                .margin_start(10)
                .margin_end(10)
                .build();
            bubble.append(&label);
        }
    }
    let raw = gtk4::Label::builder()
        .label(serde_json::to_string_pretty(details).unwrap_or_default())
        .selectable(true)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["monospace", "caption"])
        .build();
    let expander = gtk4::Expander::builder()
        .label("View details")
        .child(&raw)
        .build();
    bubble.append(&expander);
    let avatar = gtk4::Image::builder()
        .icon_name("avatar-default-symbolic")
        .pixel_size(24)
        .valign(gtk4::Align::Start)
        .margin_top(8)
        .build();
    row.append(&bubble);
    row.append(&avatar);
    row
}

fn build_file_chips(text: &str) -> Option<gtk4::FlowBox> {
    let chips = gtk4::FlowBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .max_children_per_line(3)
        .margin_start(10)
        .margin_end(10)
        .margin_top(6)
        .build();
    let mut found = false;
    for token in text.split_whitespace() {
        let Some(raw) = token.strip_prefix('@') else {
            continue;
        };
        let path = raw.trim_end_matches(['.', ',', ';', ')', ']']);
        if path.is_empty() {
            continue;
        }
        let path = std::path::Path::new(path);
        let label = match std::fs::metadata(path) {
            Ok(meta) => format!(
                "{} · {} bytes",
                crate::attachments::display_name(path),
                meta.len()
            ),
            Err(_) => crate::attachments::display_name(path),
        };
        let chip = gtk4::Label::builder()
            .label(&label)
            .css_classes(["card", "caption"])
            .margin_start(4)
            .margin_end(4)
            .margin_top(2)
            .margin_bottom(2)
            .selectable(true)
            .build();
        chips.append(&chip);
        found = true;
    }
    if found {
        Some(chips)
    } else {
        None
    }
}

fn build_user_image(image: &UserImageAttachment) -> gtk4::Button {
    let picture = gtk4::Picture::builder()
        .content_fit(gtk4::ContentFit::Cover)
        .can_shrink(true)
        .width_request(140)
        .height_request(100)
        .build();
    let button = gtk4::Button::builder()
        .child(&picture)
        .css_classes(["flat"])
        .tooltip_text(image.name.as_deref().unwrap_or("Image attachment"))
        .build();
    let picture_for_dialog = picture.clone();
    button.connect_clicked(move |_| {
        let dialog = libadwaita::Dialog::builder()
            .title("Image")
            .content_width(720)
            .build();
        let preview = gtk4::Picture::builder()
            .content_fit(gtk4::ContentFit::Contain)
            .can_shrink(true)
            .build();
        preview.set_paintable(picture_for_dialog.paintable().as_ref());
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .child(&preview)
            .build();
        dialog.set_child(Some(&scroll));
        dialog.present(Some(&picture_for_dialog));
    });
    if let Some(data) = &image.data {
        if let Some(bytes) = crate::attachments::decode_base64(data) {
            set_picture_bytes(&picture, &bytes);
        }
    } else if let Some(attachment_id) = &image.attachment_id {
        let Some(controller) = controller_ref() else {
            return button;
        };
        let Some(session_id) = controller.active_session_id() else {
            return button;
        };
        let conn = controller.connection_handle();
        let attachment_id = attachment_id.clone();
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<crate::protocol::AttachmentReadResult, String>>();
        crate::ui::controller::tokio_runtime().spawn(async move {
            let conn_opt = conn.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let result = conn
                .call::<crate::protocol::AttachmentReadParams, crate::protocol::AttachmentReadResult>(
                    crate::protocol::METHOD_ATTACHMENT_READ,
                    crate::protocol::AttachmentReadParams {
                        session_id,
                        attachment_id,
                        encoding: Some("base64".to_string()),
                    },
                    std::time::Duration::from_secs(15),
                )
                .await
                .map_err(|err| err.to_string());
            let _ = sender.send(result);
        });
        let picture_for_load = picture.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Some(Ok(read)) = receiver.recv().await {
                if let Some(bytes) = crate::attachments::decode_base64(&read.data) {
                    set_picture_bytes(&picture_for_load, &bytes);
                }
            }
        });
    }
    button
}

fn set_picture_bytes(picture: &gtk4::Picture, bytes: &[u8]) {
    if let Ok(texture) =
        gtk4::gdk::Texture::from_bytes(&gtk4::glib::Bytes::from_owned(bytes.to_vec()))
    {
        picture.set_paintable(Some(&texture));
    }
}

fn controller_ref() -> Option<Rc<SessionController>> {
    UI_CONTROLLER.with(|slot| slot.borrow().as_ref().and_then(|weak| weak.upgrade()))
}

pub fn build_assistant_message(
    text: &str,
    reasoning: Option<&str>,
    tool_calls: &[ToolCallItem],
    attachments: &[MessageAttachment],
    is_live: bool,
    seq: Option<i64>,
    time: u64,
    policy: PresentationPolicy,
) -> (gtk4::Box, AssistantLiveHandle) {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Start)
        .margin_start(8)
        .margin_end(48)
        .margin_top(4)
        .margin_bottom(4)
        .spacing(8)
        .build();

    let avatar = gtk4::Image::builder()
        .icon_name("applications-science-symbolic")
        .pixel_size(24)
        .valign(gtk4::Align::Start)
        .margin_top(8)
        .build();
    row.append(&avatar);

    let bubble = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_start(6)
        .margin_end(6)
        .margin_top(6)
        .margin_bottom(6)
        .spacing(6)
        .build();

    let reasoning_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(8)
        .margin_end(8)
        .margin_top(4)
        .build();

    let reasoning_text = reasoning.unwrap_or_default();
    let reasoning_label = gtk4::Label::builder()
        .label(reasoning_text)
        .wrap(true)
        .wrap_mode(gtk4::pango::WrapMode::WordChar)
        .halign(gtk4::Align::Start)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .selectable(true)
        .build();

    let reasoning_expander = gtk4::Expander::builder()
        .label(crate::ui::presentation::reasoning_header(
            false,
            policy.settled_reasoning_preview,
            is_live,
            reasoning_text,
        ))
        .child(&reasoning_label)
        .visible(!reasoning_text.is_empty())
        .build();
    reasoning_box.append(&reasoning_expander);
    bubble.append(&reasoning_box);

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(10)
        .margin_end(10)
        .margin_top(4)
        .margin_bottom(8)
        .build();
    let stable_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    let tail_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    content_box.append(&stable_box);
    content_box.append(&tail_box);

    let stable_end = if is_live {
        crate::ui::stream_markdown::stable_end(text, 0)
    } else {
        text.len()
    };
    if stable_end > 0 {
        stable_box.append(&render_markdown(&text[..stable_end]));
    }
    if stable_end < text.len() {
        tail_box.append(&render_markdown(&text[stable_end..]));
    }
    bubble.append(&content_box);
    if !attachments.is_empty() {
        let gallery = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .max_children_per_line(3)
            .margin_start(10)
            .margin_end(10)
            .margin_top(6)
            .build();
        for attachment in attachments {
            match attachment {
                MessageAttachment::Image(image) => {
                    gallery.append(&build_attachment_thumbnail(image));
                }
                MessageAttachment::File(file) => {
                    gallery.append(&build_message_file_chip(file));
                }
            }
        }
        bubble.append(&gallery);
    }
    let content_for_menu = content_box.clone();
    attach_dynamic_context_menu(
        &content_box,
        Rc::new(move || collect_widget_text(content_for_menu.upcast_ref())),
    );

    for tc in tool_calls {
        let tool_card = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["card"])
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(4)
            .build();
        let tool_icon = gtk4::Image::from_icon_name("utilities-terminal-symbolic");
        tool_card.append(&tool_icon);
        let tool_info = gtk4::Label::builder()
            .label(&format!("{}({})", tc.name, tc.arguments))
            .css_classes(["caption", "monospace"])
            .halign(gtk4::Align::Start)
            .build();
        tool_card.append(&tool_info);
        bubble.append(&tool_card);
    }

    let spinner = gtk4::Spinner::builder()
        .spinning(is_live)
        .visible(is_live)
        .halign(gtk4::Align::Start)
        .margin_start(10)
        .margin_bottom(6)
        .build();
    bubble.append(&spinner);

    let mut fork_button = None;
    if !is_live {
        if let Some(message_seq) = seq {
            let (actions_box, fork) = build_assistant_actions(text, message_seq, time);
            bubble.append(&actions_box);
            fork_button = Some(fork);
        }
    }

    row.append(&bubble);

    let expander_for_notify = reasoning_expander.clone();
    let live_widgets = LiveAssistantWidgets {
        container: row.clone(),
        bubble,
        content_box,
        stable_box,
        tail_box,
        spinner,
        reasoning_box,
        reasoning_label,
        reasoning_expander,
        policy,
        stable_end: std::cell::Cell::new(stable_end),
        text: RefCell::new(text.to_string()),
        reasoning_text: RefCell::new(reasoning_text.to_string()),
        running: std::cell::Cell::new(is_live),
        fork_button: RefCell::new(fork_button),
    };

    let handle = AssistantLiveHandle {
        inner: Rc::new(RefCell::new(live_widgets)),
    };
    let handle_for_notify = handle.clone();
    expander_for_notify.connect_expanded_notify(move |_| {
        handle_for_notify.refresh_reasoning_header();
    });

    (row, handle)
}

fn build_assistant_actions(
    text: &str,
    message_seq: i64,
    time: u64,
) -> (gtk4::Box, gtk4::Button) {
    let actions_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .halign(gtk4::Align::End)
        .margin_end(10)
        .margin_bottom(8)
        .build();
    let copy_btn = gtk4::Button::builder()
        .icon_name("edit-copy-symbolic")
        .tooltip_text("Copy full message")
        .css_classes(["flat", "circular"])
        .build();
    let copy_text = text.to_string();
    copy_btn.connect_clicked(move |_| {
        if let Some(display) = gtk4::gdk::Display::default() {
            display.clipboard().set_text(&copy_text);
        }
    });
    actions_box.append(&copy_btn);
    if time > 0 {
        let stamp = gtk4::glib::DateTime::from_unix_local((time / 1000) as i64)
            .ok()
            .and_then(|date| date.format("%H:%M").ok())
            .map(|value| value.to_string())
            .unwrap_or_default();
        if !stamp.is_empty() {
            let time_label = gtk4::Label::builder()
                .label(&stamp)
                .css_classes(["dim-label", "caption"])
                .build();
            actions_box.append(&time_label);
        }
    }
    let fork_btn = gtk4::Button::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text("Fork from here")
        .css_classes(["flat", "circular"])
        .visible(false)
        .build();
    fork_btn.set_action_name(Some("win.fork-session"));
    fork_btn.set_action_target_value(Some(&message_seq.to_variant()));
    actions_box.append(&fork_btn);
    let message_id = message_seq.to_string();
    let existing = feedback_for(&message_id);
    let up = gtk4::Button::builder()
        .icon_name("thumbs-up-symbolic")
        .tooltip_text("Good response")
        .css_classes(["flat", "circular"])
        .build();
    if existing.as_ref().map(|item| item.rating.as_str()) == Some("positive") {
        up.add_css_class("accent");
    }
    let down = gtk4::Button::builder()
        .icon_name("thumbs-down-symbolic")
        .tooltip_text("Bad response")
        .css_classes(["flat", "circular"])
        .build();
    if existing.as_ref().map(|item| item.rating.as_str()) == Some("negative") {
        down.add_css_class("accent");
    }
    let up_id = message_id.clone();
    let up_version = existing.as_ref().map(|item| item.version.clone());
    let up_existing = existing.clone();
    up.connect_clicked(move |_| {
        with_controller(|ctrl| {
            if up_existing.as_ref().map(|item| item.rating.as_str()) == Some("positive") {
                if let Some(item) = &up_existing {
                    ctrl.delete_feedback(&up_id, &item.version);
                }
            } else {
                ctrl.put_feedback(&up_id, "positive", up_version.clone(), None, None);
            }
        });
    });
    let down_id = message_id.clone();
    let down_version = existing.as_ref().map(|item| item.version.clone());
    let down_existing = existing;
    down.connect_clicked(move |_| {
        with_controller(|ctrl| {
            if down_existing.as_ref().map(|item| item.rating.as_str()) == Some("negative") {
                if let Some(item) = &down_existing {
                    ctrl.delete_feedback(&down_id, &item.version);
                }
            } else {
                ctrl.put_feedback(&down_id, "negative", down_version.clone(), None, None);
            }
        });
    });
    actions_box.append(&up);
    actions_box.append(&down);
    let note = gtk4::Button::builder()
        .label("Feedback")
        .css_classes(["flat"])
        .build();
    let note_parent = actions_box.clone();
    note.connect_clicked(move |_| {
        let dialog = libadwaita::Dialog::builder()
            .title("Session Feedback")
            .content_width(420)
            .build();
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();
        let entry = libadwaita::EntryRow::builder()
            .title("What went wrong or right?")
            .build();
        let entry_list = gtk4::ListBox::builder()
            .css_classes(["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        entry_list.append(&entry);
        content.append(&entry_list);
        let send = gtk4::Button::builder()
            .label("Record")
            .css_classes(["suggested-action"])
            .build();
        let dialog_for_send = dialog.clone();
        let entry_for_send = entry.clone();
        send.connect_clicked(move |_| {
            let text = entry_for_send.text().to_string();
            if !text.trim().is_empty() {
                with_controller(|ctrl| ctrl.record_session_feedback(&text));
            }
            dialog_for_send.close();
        });
        content.append(&send);
        dialog.set_child(Some(&content));
        dialog.present(Some(&note_parent));
    });
    actions_box.append(&note);
    (actions_box, fork_btn)
}

pub fn build_tool_result(call_id: &str, name: &str, result: &str, is_error: bool, _seq: i64) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Start)
        .margin_start(40)
        .margin_end(48)
        .margin_top(2)
        .margin_bottom(4)
        .build();

    let card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_start(6)
        .margin_end(6)
        .margin_top(4)
        .margin_bottom(4)
        .spacing(4)
        .build();

    let header = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(6)
        .build();

    let icon_name = if is_error {
        "dialog-error-symbolic"
    } else {
        "emblem-ok-symbolic"
    };
    let icon = gtk4::Image::from_icon_name(icon_name);
    header.append(&icon);

    let title = format!("{} ({})", if name.is_empty() { "Tool Result" } else { name }, call_id);
    let title_label = gtk4::Label::builder()
        .label(&title)
        .css_classes(["heading", "caption"])
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .build();
    header.append(&title_label);
    card.append(&header);

    let output_view = build_code_block("text", result);
    let expander = gtk4::Expander::builder()
        .label("Output")
        .child(&output_view)
        .margin_start(8)
        .margin_end(8)
        .margin_bottom(6)
        .build();
    card.append(&expander);

    row.append(&card);
    row
}

pub fn build_todo_block(items: &[TodoItem], _seq: i64) -> gtk4::Box {
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_start(40)
        .margin_end(48)
        .margin_top(4)
        .margin_bottom(4)
        .spacing(4)
        .build();

    let header = gtk4::Label::builder()
        .label("Task List")
        .css_classes(["title-4"])
        .halign(gtk4::Align::Start)
        .margin_start(10)
        .margin_top(6)
        .build();
    container.append(&header);

    for item in items {
        let is_completed = item.status == "completed";
        let check = gtk4::CheckButton::builder()
            .label(&item.content)
            .active(is_completed)
            .sensitive(false)
            .margin_start(12)
            .margin_end(12)
            .margin_top(2)
            .margin_bottom(2)
            .build();
        container.append(&check);
    }

    container
}

pub fn build_plan_mode_badge(active: bool, _seq: i64) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Center)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let label_text = if active {
        "Plan Mode Activated"
    } else {
        "Plan Mode Completed"
    };

    let badge = gtk4::Label::builder()
        .label(label_text)
        .css_classes(["card", "heading", "caption"])
        .margin_start(8)
        .margin_end(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    row.append(&badge);
    row
}

pub fn build_generic_event(event_type: &str, summary: &str, _seq: i64) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .halign(gtk4::Align::Center)
        .margin_top(2)
        .margin_bottom(2)
        .build();

    let label = gtk4::Label::builder()
        .label(&format!("{}: {}", event_type, summary))
        .css_classes(["dim-label", "caption"])
        .wrap(true)
        .halign(gtk4::Align::Center)
        .build();
    row.append(&label);
    row
}

pub fn render_block(
    block: &RenderBlock,
    policy: PresentationPolicy,
) -> (gtk4::Box, Option<AssistantLiveHandle>) {
    match block {
        RenderBlock::UserMessage {
            text,
            time,
            seq,
            images,
        } => (build_user_message(text, *time, *seq, images), None),
        RenderBlock::QuestionReply {
            summary,
            selected,
            details,
            ..
        } => (build_question_reply(summary, selected, details), None),
        RenderBlock::AssistantMessage {
            text,
            reasoning,
            tool_calls,
            attachments,
            is_live,
            seq,
            time,
            ..
        } => {
            let (widget, handle) = build_assistant_message(
                text,
                reasoning.as_deref(),
                tool_calls,
                attachments,
                *is_live,
                *seq,
                *time,
                policy,
            );
            (widget, Some(handle))
        }
        RenderBlock::ToolResult { info, .. } => (
            crate::ui::cards::render_tool_result_card(info),
            None,
        ),
        RenderBlock::PlanCard { plan, status, .. } => (
            crate::ui::cards::build_plan_card(plan, status),
            None,
        ),
        RenderBlock::DeliverablesBlock { summary, diff, .. } => (
            crate::ui::cards::build_deliverables_card(summary, diff.as_ref()),
            None,
        ),
        RenderBlock::PlanModeBadge { active, seq } => {
            (build_plan_mode_badge(*active, *seq), None)
        }
        RenderBlock::TodoBlock { items, .. } => (
            crate::ui::cards::build_todo_diff_card(items, "Tasks updated"),
            None,
        ),
        RenderBlock::GenericEvent {
            event_type,
            summary,
            seq,
        } => (build_generic_event(event_type, summary, *seq), None),
        RenderBlock::CompactionCard { summary, .. } => (build_compaction_card(summary), None),
        RenderBlock::TurnError { message, code, .. } => {
            (build_turn_error(message, code.as_deref()), None)
        }
        RenderBlock::MaxTokensHint { .. } => (build_max_tokens_hint(), None),
        RenderBlock::RetryHint { message, .. } => (build_retry_hint(message), None),
        RenderBlock::ForkBoundary { .. } => (build_fork_boundary(), None),
    }
}

fn build_fork_boundary() -> gtk4::Box {
    let zh = crate::ui::copy::system_locale().starts_with("zh");
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(8)
        .margin_bottom(8)
        .css_classes(["dsh-fork-boundary"])
        .build();
    let start = gtk4::Separator::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();
    let label = gtk4::Label::builder()
        .label(if zh {
            "从原会话分叉 · 以上为继承历史"
        } else {
            "Forked from parent session · history above is inherited"
        })
        .css_classes(["dim-label", "caption"])
        .build();
    let end = gtk4::Separator::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .hexpand(true)
        .valign(gtk4::Align::Center)
        .build();
    row.append(&start);
    row.append(&label);
    row.append(&end);
    row
}

fn build_compaction_card(summary: &str) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["card", "dsh-compaction-card"])
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let icon = gtk4::Image::from_icon_name("view-refresh-symbolic");
    row.append(&icon);
    let label = gtk4::Label::builder()
        .label(&format!("上下文压缩 · {}", summary))
        .xalign(0.0)
        .wrap(true)
        .hexpand(true)
        .css_classes(["dim-label", "caption"])
        .build();
    row.append(&label);
    row
}

fn build_turn_error(message: &str, code: Option<&str>) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let dot = gtk4::Label::builder().label("●").css_classes(["error"]).build();
    row.append(&dot);
    let copy = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .hexpand(true)
        .build();
    let title = gtk4::Label::builder()
        .label("轮次出错")
        .xalign(0.0)
        .css_classes(["error", "caption"])
        .build();
    copy.append(&title);
    let message_label = gtk4::Label::builder()
        .label(message)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    copy.append(&message_label);
    row.append(&copy);
    if let Some(code) = code {
        let code_label = gtk4::Label::builder()
            .label(code)
            .css_classes(["monospace", "caption", "dim-label"])
            .build();
        row.append(&code_label);
    }
    row
}

fn build_max_tokens_hint() -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let title = gtk4::Label::builder()
        .label("已达到输出 token 上限")
        .xalign(0.0)
        .css_classes(["warning", "caption"])
        .build();
    row.append(&title);
    let hint = gtk4::Label::builder()
        .label("回答被截断，已有输出保留在对话中。发送“继续”可让模型接着输出。")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["dim-label", "caption"])
        .build();
    row.append(&hint);
    row
}

fn build_retry_hint(message: &str) -> gtk4::Box {
    let row = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_top(4)
        .margin_bottom(4)
        .build();
    let icon = gtk4::Image::from_icon_name("view-refresh-symbolic");
    row.append(&icon);
    let label = gtk4::Label::builder()
        .label(&format!("正在重试 · {}", message))
        .xalign(0.0)
        .wrap(true)
        .hexpand(true)
        .css_classes(["dim-label", "caption"])
        .build();
    row.append(&label);
    row
}
