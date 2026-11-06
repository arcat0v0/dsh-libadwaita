use crate::protocol::TodoItem;
use crate::transcript::{
    parse_bash_tool, parse_file_diff, parse_file_read, parse_search_tool, parse_web_tool,
    BashToolInfo, FileDiffInfo, FileReadInfo, SearchToolInfo, WebToolInfo,
};
use crate::ui::message_renderer::{build_code_block, render_markdown};
use gtk4::prelude::*;
use sourceview5::prelude::*;

pub fn build_card_shell(
    icon_name: &str,
    title: &str,
    subtitle: &str,
    status_badge: Option<gtk4::Widget>,
    copy_content: Option<String>,
    body: gtk4::Widget,
    expanded_by_default: bool,
) -> gtk4::Box {
    let container = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(6)
        .build();

    let header_button = gtk4::Button::builder().css_classes(["flat"]).build();

    let header_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(8)
        .margin_end(8)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    let icon = gtk4::Image::from_icon_name(icon_name);
    header_box.append(&icon);

    let title_label = gtk4::Label::builder()
        .label(title)
        .css_classes(["heading"])
        .build();
    header_box.append(&title_label);

    let subtitle_label = gtk4::Label::builder()
        .label(subtitle)
        .css_classes(["dim-label", "caption"])
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .ellipsize(gtk4::pango::EllipsizeMode::End)
        .build();
    header_box.append(&subtitle_label);

    if let Some(badge) = status_badge {
        header_box.append(&badge);
    }

    if let Some(copy_text) = copy_content {
        let copy_btn = gtk4::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy output")
            .css_classes(["flat", "circular"])
            .build();
        copy_btn.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&copy_text);
            }
        });
        header_box.append(&copy_btn);
    }

    let chevron = gtk4::Image::from_icon_name(if expanded_by_default {
        "pan-down-symbolic"
    } else {
        "pan-end-symbolic"
    });
    header_box.append(&chevron);

    header_button.set_child(Some(&header_box));
    container.append(&header_button);

    let revealer = gtk4::Revealer::builder()
        .reveal_child(expanded_by_default)
        .transition_type(gtk4::RevealerTransitionType::SlideDown)
        .child(&body)
        .build();
    container.append(&revealer);

    let rev_clone = revealer.clone();
    let chev_clone = chevron.clone();
    header_button.connect_clicked(move |_| {
        let next_revealed = !rev_clone.reveals_child();
        rev_clone.set_reveal_child(next_revealed);
        chev_clone.set_icon_name(Some(if next_revealed {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        }));
    });

    container
}

pub fn build_bash_card(info: &BashToolInfo) -> gtk4::Box {
    let badge = match info.exit_code {
        Some(0) => {
            let lbl = gtk4::Label::builder()
                .label("0")
                .css_classes(["badge", "success"])
                .build();
            Some(lbl.upcast())
        }
        Some(code) => {
            let lbl = gtk4::Label::builder()
                .label(&format!("exit: {}", code))
                .css_classes(["badge", "destructive-action"])
                .build();
            Some(lbl.upcast())
        }
        None => None,
    };

    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();

    if let Some(ref cwd) = info.cwd {
        let cwd_lbl = gtk4::Label::builder()
            .label(&format!("cwd: {}", cwd))
            .css_classes(["dim-label", "caption", "monospace"])
            .halign(gtk4::Align::Start)
            .build();
        body_box.append(&cwd_lbl);
    }

    let code_view = build_code_block("sh", &info.output);
    body_box.append(&code_view);

    build_card_shell(
        "utilities-terminal-symbolic",
        "Terminal",
        &info.command,
        badge,
        Some(info.output.clone()),
        body_box.upcast(),
        true,
    )
}

pub fn build_diff_card(info: &FileDiffInfo) -> gtk4::Box {
    let title = if info.is_patch {
        "Apply Patch"
    } else {
        "File Edit"
    };

    let lang_manager = sourceview5::LanguageManager::default();
    let diff_lang = lang_manager.language("diff");

    let buffer = sourceview5::Buffer::new(None);
    if let Some(l) = diff_lang {
        buffer.set_language(Some(&l));
    }
    buffer.set_text(&info.diff);

    let view = sourceview5::View::with_buffer(&buffer);
    view.set_show_line_numbers(true);
    view.set_monospace(true);
    view.set_editable(false);
    view.set_left_margin(8);
    view.set_right_margin(8);
    view.set_top_margin(4);
    view.set_bottom_margin(6);

    let scroll = gtk4::ScrolledWindow::builder()
        .min_content_height(50)
        .max_content_height(400)
        .propagate_natural_height(true)
        .build();
    scroll.set_child(Some(&view));

    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();
    body_box.append(&scroll);

    build_card_shell(
        "document-edit-symbolic",
        title,
        &info.path,
        None,
        Some(info.diff.clone()),
        body_box.upcast(),
        true,
    )
}

pub fn build_read_card(info: &FileReadInfo) -> gtk4::Box {
    let lang_manager = sourceview5::LanguageManager::default();
    let guessed_lang = if info.path.ends_with(".rs") {
        lang_manager.language("rust")
    } else if info.path.ends_with(".ts") || info.path.ends_with(".js") {
        lang_manager.language("typescript")
    } else if info.path.ends_with(".json") {
        lang_manager.language("json")
    } else if info.path.ends_with(".toml") {
        lang_manager.language("toml")
    } else if info.path.ends_with(".yaml") || info.path.ends_with(".yml") {
        lang_manager.language("yaml")
    } else if info.path.ends_with(".sh") || info.path.ends_with(".bash") {
        lang_manager.language("sh")
    } else {
        None
    };

    let buffer = sourceview5::Buffer::new(None);
    if let Some(l) = guessed_lang {
        buffer.set_language(Some(&l));
    }
    buffer.set_text(&info.content);

    let view = sourceview5::View::with_buffer(&buffer);
    view.set_show_line_numbers(true);
    view.set_monospace(true);
    view.set_editable(false);
    view.set_left_margin(8);
    view.set_right_margin(8);
    view.set_top_margin(4);
    view.set_bottom_margin(6);

    let scroll = gtk4::ScrolledWindow::builder()
        .min_content_height(50)
        .max_content_height(400)
        .propagate_natural_height(true)
        .build();
    scroll.set_child(Some(&view));

    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();
    body_box.append(&scroll);

    let subtitle = format!("{} (lines: {})", info.path, info.range);
    build_card_shell(
        "document-open-symbolic",
        "Read File",
        &subtitle,
        None,
        Some(info.content.clone()),
        body_box.upcast(),
        false,
    )
}

pub fn build_search_card(info: &SearchToolInfo) -> gtk4::Box {
    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();

    let output_view = build_code_block("text", &info.results);
    body_box.append(&output_view);

    let subtitle = format!("'{}' in {}", info.pattern, info.path);
    build_card_shell(
        "system-search-symbolic",
        &info.tool,
        &subtitle,
        None,
        Some(info.results.clone()),
        body_box.upcast(),
        true,
    )
}

pub fn build_web_card(info: &WebToolInfo) -> gtk4::Box {
    let badge = info.status_code.map(|code| {
        let lbl = gtk4::Label::builder()
            .label(&code.to_string())
            .css_classes(["badge", "caption"])
            .build();
        lbl.upcast::<gtk4::Widget>()
    });

    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    let md = render_markdown(&info.content);
    body_box.append(&md);

    build_card_shell(
        "web-browser-symbolic",
        &info.tool,
        &info.query_or_url,
        badge,
        Some(info.content.clone()),
        body_box.upcast(),
        true,
    )
}

pub fn build_todo_diff_card(items: &[TodoItem], summary: &str) -> gtk4::Box {
    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    for item in items {
        let is_completed = item.status == "completed";
        let check = gtk4::CheckButton::builder()
            .label(&item.content)
            .active(is_completed)
            .sensitive(false)
            .margin_start(4)
            .margin_end(4)
            .build();
        body_box.append(&check);
    }

    build_card_shell(
        "view-list-bullet-symbolic",
        "Todo List",
        summary,
        None,
        None,
        body_box.upcast(),
        true,
    )
}

pub fn build_plan_card(plan_text: &str, status: &str) -> gtk4::Box {
    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    let md = render_markdown(plan_text);
    body_box.append(&md);

    build_card_shell(
        "view-list-tree-symbolic",
        "Execution Plan",
        status,
        None,
        Some(plan_text.to_string()),
        body_box.upcast(),
        true,
    )
}

pub fn build_deliverables_card(
    summary: &serde_json::Value,
    diff: Option<&serde_json::Value>,
) -> gtk4::Box {
    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_bottom(8)
        .build();

    let files_count = summary["files"].as_array().map(|a| a.len()).unwrap_or(0);
    let subtitle = format!("{} changed files", files_count);

    if let Some(files) = summary["files"].as_array() {
        for file in files {
            let path_str = file["path"].as_str().unwrap_or_default();
            let added = file["added"].as_u64().unwrap_or(0);
            let removed = file["removed"].as_u64().unwrap_or(0);

            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();

            let file_icon = gtk4::Image::from_icon_name("text-x-generic-symbolic");
            row.append(&file_icon);

            let name_label = gtk4::Label::builder()
                .label(path_str)
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .build();
            row.append(&name_label);

            let stats_label = gtk4::Label::builder()
                .label(&format!("+{} -{}", added, removed))
                .css_classes(["caption", "dim-label"])
                .build();
            row.append(&stats_label);

            body_box.append(&row);
        }
    }

    if let Some(diff_val) = diff {
        let diff_str = diff_val["patch"]
            .as_str()
            .or_else(|| diff_val.as_str())
            .unwrap_or_default();
        if !diff_str.is_empty() {
            let diff_code = build_code_block("diff", diff_str);
            body_box.append(&diff_code);
        }
    }

    build_card_shell(
        "package-x-generic-symbolic",
        "Deliverables",
        &subtitle,
        None,
        None,
        body_box.upcast(),
        true,
    )
}

pub fn build_control_details_card(name: &str, arguments: &str, result: &str) -> gtk4::Box {
    let value: serde_json::Value = serde_json::from_str(result).unwrap_or_default();
    let body = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(4)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();
    if let Some(object) = value.as_object() {
        let grid = gtk4::Grid::builder()
            .column_spacing(10)
            .row_spacing(4)
            .build();
        for (row_index, (key, value)) in object.iter().take(24).enumerate() {
            let key_label = gtk4::Label::builder()
                .label(key)
                .css_classes(["dim-label", "caption"])
                .xalign(0.0)
                .build();
            let value_label = gtk4::Label::builder()
                .label(match value {
                    serde_json::Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .wrap(true)
                .selectable(true)
                .xalign(0.0)
                .build();
            grid.attach(&key_label, 0, row_index as i32, 1, 1);
            grid.attach(&value_label, 1, row_index as i32, 1, 1);
        }
        body.append(&grid);
    } else {
        body.append(&gtk4::Label::builder()
            .label(result)
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .build());
    }
    build_card_shell(
        "view-list-symbolic",
        name,
        arguments,
        None,
        Some(result.to_string()),
        body.upcast(),
        false,
    )
}

pub fn build_generic_fallback_card(name: &str, arguments: &str, result: &str) -> gtk4::Box {
    let body_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_start(10)
        .margin_end(10)
        .margin_bottom(8)
        .build();

    if !arguments.is_empty() && arguments != "{}" {
        let args_view = build_code_block("json", arguments);
        body_box.append(&args_view);
    }

    let output_view = build_code_block("text", result);
    body_box.append(&output_view);

    build_card_shell(
        "application-x-executable-symbolic",
        name,
        arguments,
        None,
        Some(result.to_string()),
        body_box.upcast(),
        false,
    )
}

pub fn render_tool_result_card(
    name: &str,
    arguments: Option<&str>,
    result: &str,
    _is_error: bool,
) -> gtk4::Box {
    let args_str = arguments.unwrap_or("{}");

    match name {
        "bash" | "pwsh" | "terminal_send" => {
            let info = parse_bash_tool(args_str, result);
            build_bash_card(&info)
        }
        "write" | "edit" | "apply_patch" => {
            let info = parse_file_diff(args_str, result);
            build_diff_card(&info)
        }
        "read" => {
            let info = parse_file_read(args_str, result);
            build_read_card(&info)
        }
        "grep" | "glob" => {
            let info = parse_search_tool(name, args_str, result);
            build_search_card(&info)
        }
        "web_search" | "web_fetch" => {
            let info = parse_web_tool(name, args_str, result);
            build_web_card(&info)
        }
        "read_image" => {
            let card = build_generic_fallback_card(name, args_str, result);
            let value: serde_json::Value = serde_json::from_str(result).unwrap_or_default();
            let attachment_id = value["attachmentId"]
                .as_str()
                .or_else(|| value["attachment"]["id"].as_str())
                .or_else(|| value["meta"]["attachmentId"].as_str())
                .map(str::to_string);
            let data = value["data"].as_str().map(str::to_string);
            if attachment_id.is_some() || data.is_some() {
                let thumbnail = crate::ui::message_renderer::build_attachment_thumbnail(
                    &crate::protocol::UserImageAttachment {
                        name: value["name"].as_str().map(str::to_string),
                        mime_type: value["mimeType"].as_str().unwrap_or("image/png").to_string(),
                        data,
                        attachment_id,
                    },
                );
                card.append(&thumbnail);
            }
            card
        }
        "subagent" | "job" | "schedule" | "goal" | "workflow" | "lsp" => {
            build_control_details_card(name, args_str, result)
        }
        "exit_plan_mode" => build_plan_card(result, "Plan completed"),
        "ask_user_question" => {
            let card = build_generic_fallback_card(name, args_str, result);
            let actions = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .halign(gtk4::Align::End)
                .margin_end(8)
                .margin_bottom(6)
                .build();
            let reopen = gtk4::Button::builder()
                .label("Reopen")
                .css_classes(["flat"])
                .build();
            reopen.connect_clicked(|_| {
                crate::ui::message_renderer::reopen_question();
            });
            actions.append(&reopen);
            card.append(&actions);
            card
        }
        _ => build_generic_fallback_card(name, args_str, result),
    }
}
