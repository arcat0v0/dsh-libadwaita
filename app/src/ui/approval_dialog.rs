use crate::protocol::*;
use crate::ui::message_renderer::render_markdown;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

struct QuestionPageState {
    id: String,
    checks: Vec<(String, gtk4::CheckButton)>,
    entry: gtk4::Entry,
}
use tokio::sync::oneshot;

pub fn show_approval_dialog(
    parent: &impl IsA<gtk4::Widget>,
    req: ApprovalRequestParams,
    sender: oneshot::Sender<ApprovalOutcome>,
) {
    let dialog = adw::Dialog::builder()
        .title("Operation Approval")
        .content_width(460)
        .build();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let header_label = gtk4::Label::builder()
        .label("Tool Execution Authorization")
        .css_classes(["title-3"])
        .halign(gtk4::Align::Start)
        .build();
    content_box.append(&header_label);

    let desc_label = gtk4::Label::builder()
        .label("An agent tool requires your authorization to proceed.")
        .css_classes(["dim-label"])
        .halign(gtk4::Align::Start)
        .wrap(true)
        .build();
    content_box.append(&desc_label);

    let details_card = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card"])
        .margin_top(4)
        .margin_bottom(4)
        .spacing(6)
        .build();

    let tool_row = adw::ActionRow::builder()
        .title("Tool")
        .subtitle(&req.tool_name)
        .build();
    tool_row.add_prefix(&gtk4::Image::from_icon_name("utilities-terminal-symbolic"));
    details_card.append(&tool_row);

    if let Some(ref call_id) = req.call_id {
        let call_row = adw::ActionRow::builder()
            .title("Call ID")
            .subtitle(call_id)
            .build();
        details_card.append(&call_row);
    }

    if let Some(ref reason) = req.reason {
        let reason_row = adw::ActionRow::builder()
            .title("Reason")
            .subtitle(reason)
            .build();
        details_card.append(&reason_row);
    }
    if let Some(ref text) = req.display_reason {
        let row = adw::ActionRow::builder()
            .title("Display Reason")
            .subtitle(text)
            .build();
        details_card.append(&row);
    }
    if let Some(ref text) = req.warning {
        let row = adw::ActionRow::builder()
            .title("Warning")
            .subtitle(text)
            .build();
        details_card.append(&row);
    }
    if let Some(ref detail) = req.detail {
        let row = adw::ActionRow::builder()
            .title("Detail")
            .subtitle(
                serde_json::to_string_pretty(detail).unwrap_or_else(|_| detail.to_string()),
            )
            .build();
        details_card.append(&row);
    }
    let raw = serde_json::to_string_pretty(&req).unwrap_or_default();
    let raw_label = gtk4::Label::builder()
        .label(&raw)
        .selectable(true)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["monospace", "caption"])
        .build();
    let expander = gtk4::Expander::builder()
        .label("Details")
        .child(&raw_label)
        .build();
    details_card.append(&expander);
    content_box.append(&details_card);

    let button_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(10)
        .halign(gtk4::Align::End)
        .margin_top(8)
        .build();

    let sender_cell = Rc::new(RefCell::new(Some(sender)));

    let reject_btn = gtk4::Button::builder()
        .label("Reject")
        .css_classes(["destructive-action"])
        .build();

    let s_reject = sender_cell.clone();
    let d_reject = dialog.clone();
    reject_btn.connect_clicked(move |_| {
        if let Some(tx) = s_reject.borrow_mut().take() {
            let _ = tx.send(ApprovalOutcome::Rejected);
        }
        d_reject.close();
    });
    button_box.append(&reject_btn);

    let allow_btn = gtk4::Button::builder()
        .label("Allow Once")
        .css_classes(["suggested-action"])
        .build();

    let s_allow = sender_cell.clone();
    let d_allow = dialog.clone();
    allow_btn.connect_clicked(move |_| {
        if let Some(tx) = s_allow.borrow_mut().take() {
            let _ = tx.send(ApprovalOutcome::AllowedOnce);
        }
        d_allow.close();
    });
    button_box.append(&allow_btn);

    content_box.append(&button_box);

    let s_key = sender_cell.clone();
    let d_key = dialog.clone();
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        let outcome = if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
            Some(ApprovalOutcome::AllowedOnce)
        } else if key == gtk4::gdk::Key::Escape {
            Some(ApprovalOutcome::Rejected)
        } else {
            None
        };
        if let Some(outcome) = outcome {
            if let Some(tx) = s_key.borrow_mut().take() {
                let _ = tx.send(outcome);
            }
            d_key.close();
            return gtk4::glib::Propagation::Stop;
        }
        gtk4::glib::Propagation::Proceed
    });
    dialog.add_controller(key_controller);
    dialog.set_child(Some(&content_box));


    let s_close = sender_cell.clone();
    dialog.connect_closed(move |_| {
        if let Some(tx) = s_close.borrow_mut().take() {
            let _ = tx.send(ApprovalOutcome::Rejected);
        }
    });

    dialog.present(Some(parent));
}

pub fn show_user_question_dialog(
    parent: &impl IsA<gtk4::Widget>,
    req: SdkUserQuestionRequest,
    sender: oneshot::Sender<SdkUserQuestionResult>,
) {
    let is_plan_review = req.questions.iter().any(|q| {
        matches!(q.intent, Some(UserQuestionIntent::PlanReview { .. }))
    });

    let title = if is_plan_review {
        "Plan Review"
    } else {
        "User Question"
    };

    let dialog = adw::Dialog::builder()
        .title(title)
        .content_width(520)
        .build();

    let content_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(16)
        .margin_bottom(16)
        .build();

    let sender_cell = Rc::new(RefCell::new(Some(sender)));

    if is_plan_review {
        let q = req.questions.first().unwrap();
        let approve_label = match &q.intent {
            Some(UserQuestionIntent::PlanReview { approve, .. }) => approve.clone(),
            _ => "Agree & Execute".to_string(),
        };

        let header = gtk4::Label::builder()
            .label(&q.question)
            .css_classes(["title-3"])
            .halign(gtk4::Align::Start)
            .build();
        content_box.append(&header);

        if let Some(ref detail) = q.detail {
            let scroll = gtk4::ScrolledWindow::builder()
                .min_content_height(120)
                .max_content_height(350)
                .propagate_natural_height(true)
                .css_classes(["card"])
                .build();
            let md = render_markdown(detail);
            md.set_margin_start(12);
            md.set_margin_end(12);
            md.set_margin_top(8);
            md.set_margin_bottom(8);
            scroll.set_child(Some(&md));
            content_box.append(&scroll);
        }

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk4::Align::End)
            .margin_top(8)
            .build();

        let cancel_btn = gtk4::Button::builder()
            .label("Request Changes")
            .css_classes(["destructive-action"])
            .build();

        let s_cancel = sender_cell.clone();
        let d_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            if let Some(tx) = s_cancel.borrow_mut().take() {
                let _ = tx.send(SdkUserQuestionResult::cancelled());
            }
            d_cancel.close();
        });
        button_box.append(&cancel_btn);

        let approve_btn = gtk4::Button::builder()
            .label(&approve_label)
            .css_classes(["suggested-action"])
            .build();

        let s_approve = sender_cell.clone();
        let d_approve = dialog.clone();
        let q_id = q.id.clone();
        approve_btn.connect_clicked(move |_| {
            if let Some(tx) = s_approve.borrow_mut().take() {
                let res = SdkUserQuestionResult::answers(vec![UserQuestionAnswer {
                    id: q_id.clone(),
                    selected: vec![approve_label.clone()],
                }]);
                let _ = tx.send(res);
            }
            d_approve.close();
        });
        button_box.append(&approve_btn);

        content_box.append(&button_box);
    } else {
        let pages_state: Rc<RefCell<Vec<QuestionPageState>>> = Rc::new(RefCell::new(Vec::new()));
        let page_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::SlideLeftRight)
            .build();
        let mut first_groups: std::collections::HashMap<String, gtk4::CheckButton> =
            std::collections::HashMap::new();

        for (index, q) in req.questions.iter().enumerate() {
            let q_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(6)
                .build();
            let prompt = gtk4::Label::builder()
                .label(&q.question)
                .css_classes(["heading"])
                .halign(gtk4::Align::Start)
                .wrap(true)
                .build();
            q_box.append(&prompt);
            if let Some(ref detail) = q.detail {
                let detail_label = gtk4::Label::builder()
                    .label(detail)
                    .css_classes(["dim-label"])
                    .halign(gtk4::Align::Start)
                    .wrap(true)
                    .build();
                q_box.append(&detail_label);
            }

            let mut checks = Vec::new();
            if let Some(options) = &q.options {
                let multi = q.multi_select.unwrap_or(false);
                let group_key = format!("q-{}", index);
                for option in options {
                    let label = match &option.description {
                        Some(description) if !description.is_empty() => {
                            format!("{} — {}", option.label, description)
                        }
                        _ => option.label.clone(),
                    };
                    let check = gtk4::CheckButton::builder()
                        .label(&label)
                        .css_classes(["flat"])
                        .build();
                    if !multi {
                        if let Some(first) = first_groups.get(&group_key) {
                            check.set_group(Some(first));
                        } else {
                            first_groups.insert(group_key.clone(), check.clone());
                        }
                    }
                    checks.push((option.label.clone(), check.clone()));
                    q_box.append(&check);
                }
            }

            let entry = gtk4::Entry::builder()
                .placeholder_text("Custom answer...")
                .build();
            q_box.append(&entry);
            page_stack.add_named(&q_box, Some(&index.to_string()));
            pages_state.borrow_mut().push(QuestionPageState {
                id: q.id.clone(),
                checks,
                entry,
            });
        }

        content_box.append(&page_stack);

        let index_cell = Rc::new(RefCell::new(0usize));
        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk4::Align::End)
            .margin_top(8)
            .build();

        let back_btn = gtk4::Button::builder().label("Back").build();
        let back_stack = page_stack.clone();
        let back_index = index_cell.clone();
        back_btn.connect_clicked(move |_| {
            let mut index = back_index.borrow_mut();
            if *index > 0 {
                *index -= 1;
                back_stack.set_visible_child_name(&index.to_string());
            }
        });
        button_box.append(&back_btn);

        let skip_btn = gtk4::Button::builder().label("Skip").build();
        let skip_stack = page_stack.clone();
        let skip_index = index_cell.clone();
        let skip_pages = pages_state.clone();
        skip_btn.connect_clicked(move |_| {
            let mut index = skip_index.borrow_mut();
            if let Some(page) = skip_pages.borrow().get(*index) {
                for (_, check) in &page.checks {
                    check.set_active(false);
                }
                page.entry.set_text("");
            }
            if *index + 1 < skip_pages.borrow().len() {
                *index += 1;
                skip_stack.set_visible_child_name(&index.to_string());
            }
        });
        button_box.append(&skip_btn);

        let next_btn = gtk4::Button::builder()
            .label("Next")
            .css_classes(["suggested-action"])
            .build();
        let next_stack = page_stack.clone();
        let next_index = index_cell.clone();
        let next_pages = pages_state.clone();
        let s_next = sender_cell.clone();
        let d_next = dialog.clone();
        next_btn.connect_clicked(move |_| {
            let page_count = next_pages.borrow().len();
            let mut index = next_index.borrow_mut();
            if *index + 1 < page_count {
                *index += 1;
                next_stack.set_visible_child_name(&index.to_string());
                return;
            }
            if let Some(tx) = s_next.borrow_mut().take() {
                let mut answers = Vec::new();
                for page in next_pages.borrow().iter() {
                    let mut selected = Vec::new();
                    for (label, check) in &page.checks {
                        if check.is_active() {
                            selected.push(label.clone());
                        }
                    }
                    answers.push(crate::protocol::build_question_answer(
                        &page.id,
                        selected,
                        &page.entry.text(),
                    ));
                }
                let _ = tx.send(SdkUserQuestionResult::answers(answers));
            }
            d_next.close();
        });
        button_box.append(&next_btn);

        let cancel_btn = gtk4::Button::builder().label("Cancel").build();
        let s_cancel = sender_cell.clone();
        let d_cancel = dialog.clone();
        cancel_btn.connect_clicked(move |_| {
            if let Some(tx) = s_cancel.borrow_mut().take() {
                let _ = tx.send(SdkUserQuestionResult::cancelled());
            }
            d_cancel.close();
        });
        button_box.append(&cancel_btn);

        content_box.append(&button_box);
    }

    dialog.set_child(Some(&content_box));

    let s_close = sender_cell.clone();
    dialog.connect_closed(move |_| {
        if let Some(tx) = s_close.borrow_mut().take() {
            let _ = tx.send(SdkUserQuestionResult::cancelled());
        }
    });

    dialog.present(Some(parent));
}
