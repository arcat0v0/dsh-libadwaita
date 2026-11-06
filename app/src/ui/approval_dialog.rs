use crate::protocol::*;
use crate::ui::message_renderer::render_markdown;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
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

    let call_row = adw::ActionRow::builder()
        .title("Call ID")
        .subtitle(&req.call_id)
        .build();
    details_card.append(&call_row);

    if let Some(ref reason) = req.reason {
        let reason_row = adw::ActionRow::builder()
            .title("Reason")
            .subtitle(reason)
            .build();
        details_card.append(&reason_row);
    }
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
        let mut answer_entries = Vec::new();

        for q in &req.questions {
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

            let entry = gtk4::Entry::builder()
                .placeholder_text("Your answer...")
                .build();
            q_box.append(&entry);
            answer_entries.push((q.id.clone(), entry));

            content_box.append(&q_box);
        }

        let button_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .halign(gtk4::Align::End)
            .margin_top(8)
            .build();

        let cancel_btn = gtk4::Button::builder()
            .label("Cancel")
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

        let submit_btn = gtk4::Button::builder()
            .label("Submit")
            .css_classes(["suggested-action"])
            .build();

        let s_submit = sender_cell.clone();
        let d_submit = dialog.clone();
        submit_btn.connect_clicked(move |_| {
            if let Some(tx) = s_submit.borrow_mut().take() {
                let mut answers = Vec::new();
                for (id, entry) in &answer_entries {
                    let text = entry.text().to_string();
                    answers.push(UserQuestionAnswer {
                        id: id.clone(),
                        selected: vec![text],
                    });
                }
                let _ = tx.send(SdkUserQuestionResult::answers(answers));
            }
            d_submit.close();
        });
        button_box.append(&submit_btn);

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
