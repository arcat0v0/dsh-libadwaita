use crate::protocol::{MessageFeedbackItem, TodoItem};
use crate::transcript::{RenderBlock, ToolCallItem};
use crate::ui::controller::SessionController;
use gtk4::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
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

pub fn render_markdown(text: &str) -> gtk4::Box {
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();

    let parser = Parser::new(text);
    let mut current_markup = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut list_index: Option<u64> = None;

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
                _ => {}
            },
            Event::Text(t) => {
                if in_code_block {
                    code_block_content.push_str(&t);
                } else {
                    current_markup.push_str(&gtk4::glib::markup_escape_text(&t));
                }
            }
            Event::Code(c) => {
                if in_code_block {
                    code_block_content.push_str(&c);
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
                } else {
                    current_markup.push(' ');
                }
            }
            Event::HardBreak => {
                if in_code_block {
                    code_block_content.push('\n');
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
        container.append(&label);
    }

    container
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
    container.append(&scroll);

    container
}

pub struct LiveAssistantWidgets {
    pub container: gtk4::Box,
    pub content_box: gtk4::Box,
    pub spinner: gtk4::Spinner,
    pub reasoning_box: gtk4::Box,
    pub reasoning_label: gtk4::Label,
    pub reasoning_expander: gtk4::Expander,
}

#[derive(Clone)]
pub struct AssistantLiveHandle {
    inner: Rc<RefCell<LiveAssistantWidgets>>,
}

impl AssistantLiveHandle {
    pub fn update_text(&self, text: &str) {
        let widgets = self.inner.borrow();
        while let Some(child) = widgets.content_box.first_child() {
            widgets.content_box.remove(&child);
        }
        let rendered = render_markdown(text);
        widgets.content_box.append(&rendered);
    }

    pub fn update_reasoning(&self, reasoning: &str) {
        let widgets = self.inner.borrow();
        widgets.reasoning_label.set_label(reasoning);
        widgets.reasoning_expander.set_visible(!reasoning.is_empty());
    }

    pub fn finalize(&self) {
        let widgets = self.inner.borrow();
        widgets.spinner.stop();
        widgets.spinner.set_visible(false);
    }
}

pub fn build_user_message(text: &str, _time: u64, _seq: i64) -> gtk4::Box {
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
        actions.append(&copy);
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

pub fn build_assistant_message(
    text: &str,
    reasoning: Option<&str>,
    tool_calls: &[ToolCallItem],
    is_live: bool,
    seq: Option<i64>,
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
        .label("Reasoning Process")
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

    if !text.is_empty() {
        let rendered = render_markdown(text);
        content_box.append(&rendered);
    }
    bubble.append(&content_box);

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

    if !is_live {
        if let Some(message_seq) = seq {
            let actions_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .halign(gtk4::Align::End)
                .margin_end(10)
                .margin_bottom(8)
                .build();
            let fork_btn = gtk4::Button::builder()
                .icon_name("view-more-symbolic")
                .tooltip_text("Fork from here")
                .css_classes(["flat", "circular"])
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
                    if down_existing.as_ref().map(|item| item.rating.as_str())
                        == Some("negative")
                    {
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
            bubble.append(&actions_box);
        }
    }

    row.append(&bubble);

    let live_widgets = LiveAssistantWidgets {
        container: row.clone(),
        content_box,
        spinner,
        reasoning_box,
        reasoning_label,
        reasoning_expander,
    };

    let handle = AssistantLiveHandle {
        inner: Rc::new(RefCell::new(live_widgets)),
    };

    (row, handle)
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

pub fn render_block(block: &RenderBlock) -> (gtk4::Box, Option<AssistantLiveHandle>) {
    match block {
        RenderBlock::UserMessage { text, time, seq } => {
            (build_user_message(text, *time, *seq), None)
        }
        RenderBlock::AssistantMessage {
            text,
            reasoning,
            tool_calls,
            is_live,
            seq,
            ..
        } => {
            let (widget, handle) = build_assistant_message(
                text,
                reasoning.as_deref(),
                tool_calls,
                *is_live,
                *seq,
            );
            (widget, Some(handle))
        }
        RenderBlock::ToolResult {
            name,
            arguments,
            result,
            is_error,
            ..
        } => (
            crate::ui::cards::render_tool_result_card(
                name,
                arguments.as_deref(),
                result,
                *is_error,
            ),
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
    }
}
