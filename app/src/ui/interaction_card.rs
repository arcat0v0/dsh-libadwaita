use crate::interaction::{
    arbitrate_key, parse_recommended_label, ArbitratedDecision, ArbiterKey, FocusContext,
    QuestionDraft,
};
use crate::protocol::{
    ApprovalOutcome, ApprovalRequestParams, SdkUserQuestionRequest, SdkUserQuestionResult,
    UserQuestionIntent,
};
use crate::ui::message_renderer::render_markdown;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Copy {
    waiting_approval: &'static str,
    waiting_answer: &'static str,
    approve_escalation: &'static str,
    reject: &'static str,
    allow_once: &'static str,
    details: &'static str,
    risk_note: &'static str,
    recommended: &'static str,
    custom_answer: &'static str,
    back: &'static str,
    skip: &'static str,
    next: &'static str,
    submit: &'static str,
    cancel: &'static str,
    close: &'static str,
    expired: &'static str,
    request_changes: &'static str,
    plan_review: &'static str,
}

fn copy() -> Copy {
    if crate::ui::copy::system_locale().starts_with("zh") {
        Copy {
            waiting_approval: "等待审批",
            waiting_answer: "等待回答",
            approve_escalation: "工具 {tool} 请求越权执行",
            reject: "拒绝",
            allow_once: "允许一次",
            details: "详情",
            risk_note: "风险说明",
            recommended: "推荐",
            custom_answer: "自定义回答…",
            back: "返回",
            skip: "跳过",
            next: "下一步",
            submit: "提交",
            cancel: "取消",
            close: "关闭",
            expired: "该问题已过期，无法作答",
            request_changes: "要求修改",
            plan_review: "计划审查",
        }
    } else {
        Copy {
            waiting_approval: "Waiting for approval",
            waiting_answer: "Waiting for answer",
            approve_escalation: "Tool {tool} requests privileged execution",
            reject: "Reject",
            allow_once: "Allow once",
            details: "Details",
            risk_note: "Risk note",
            recommended: "Recommended",
            custom_answer: "Custom answer…",
            back: "Back",
            skip: "Skip",
            next: "Next",
            submit: "Submit",
            cancel: "Cancel",
            close: "Close",
            expired: "This question expired and can no longer be answered",
            request_changes: "Request changes",
            plan_review: "Plan review",
        }
    }
}

fn strip_box(text: &str) -> gtk4::Box {
    let strip = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .css_classes(["dsh-interaction-strip"])
        .build();
    let dot = gtk4::Label::builder()
        .label("●")
        .css_classes(["warning", "caption"])
        .build();
    strip.append(&dot);
    let label = gtk4::Label::builder()
        .label(text)
        .halign(gtk4::Align::Start)
        .css_classes(["caption"])
        .build();
    strip.append(&label);
    strip
}

fn card_box(kind: &str) -> gtk4::Box {
    gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .css_classes(["card", "dsh-interaction-card", kind])
        .build()
}

fn body_box() -> gtk4::Box {
    gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_start(16)
        .margin_end(16)
        .margin_top(10)
        .margin_bottom(10)
        .build()
}

fn action_row() -> gtk4::Box {
    gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .halign(gtk4::Align::End)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(12)
        .build()
}

pub fn build_approval_card(
    req: &ApprovalRequestParams,
    command: Option<String>,
    on_answer: Rc<dyn Fn(ApprovalOutcome)>,
) -> gtk4::Widget {
    let copy = copy();
    let card = card_box("dsh-approval-card");
    card.append(&strip_box(copy.waiting_approval));

    let body = body_box();
    let title_text = req
        .display_reason
        .clone()
        .or_else(|| req.reason.clone())
        .unwrap_or_else(|| copy.approve_escalation.replace("{tool}", &req.tool_name));
    let title = gtk4::Label::builder()
        .label(&title_text)
        .xalign(0.0)
        .wrap(true)
        .css_classes(["title-4"])
        .build();
    body.append(&title);

    if let Some(command) = command {
        let command_label = gtk4::Label::builder()
            .label(&command)
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .css_classes(["monospace", "dim-label", "dsh-approval-command"])
            .build();
        body.append(&command_label);
    }

    if let Some(warning) = req.warning.as_ref() {
        let warning_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        let warning_label = gtk4::Label::builder()
            .label(warning)
            .xalign(0.0)
            .wrap(true)
            .hexpand(true)
            .css_classes(["dim-label", "caption"])
            .build();
        warning_row.append(&warning_label);
        let risk = gtk4::MenuButton::builder()
            .label(copy.risk_note)
            .css_classes(["flat", "caption"])
            .build();
        let popover = gtk4::Popover::builder().build();
        let popover_label = gtk4::Label::builder()
            .label(warning)
            .wrap(true)
            .max_width_chars(40)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        popover.set_child(Some(&popover_label));
        risk.set_popover(Some(&popover));
        warning_row.append(&risk);
        body.append(&warning_row);
    }

    if req.detail.is_some() || req.display_reason.is_some() {
        let raw = serde_json::to_string_pretty(req).unwrap_or_default();
        let raw_label = gtk4::Label::builder()
            .label(&raw)
            .selectable(true)
            .wrap(true)
            .xalign(0.0)
            .css_classes(["monospace", "caption"])
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .min_content_height(60)
            .max_content_height(220)
            .propagate_natural_height(true)
            .build();
        scroll.set_child(Some(&raw_label));
        let expander = gtk4::Expander::builder()
            .label(copy.details)
            .child(&scroll)
            .build();
        body.append(&expander);
    }
    card.append(&body);

    let actions = action_row();
    let reject_btn = gtk4::Button::builder()
        .label(copy.reject)
        .css_classes(["destructive-action"])
        .build();
    let allow_btn = gtk4::Button::builder()
        .label(copy.allow_once)
        .css_classes(["suggested-action"])
        .build();
    actions.append(&reject_btn);
    actions.append(&allow_btn);
    card.append(&actions);

    let answered = Rc::new(Cell::new(false));
    let answer = {
        let answered = answered.clone();
        let on_answer = on_answer.clone();
        let reject_btn = reject_btn.clone();
        let allow_btn = allow_btn.clone();
        move |outcome: ApprovalOutcome| {
            if answered.get() {
                return;
            }
            answered.set(true);
            reject_btn.set_sensitive(false);
            allow_btn.set_sensitive(false);
            on_answer(outcome);
        }
    };
    let answer_reject = answer.clone();
    reject_btn.connect_clicked(move |_| answer_reject(ApprovalOutcome::Rejected));
    let answer_allow = answer.clone();
    allow_btn.connect_clicked(move |_| answer_allow(ApprovalOutcome::AllowedOnce));

    let key_controller = gtk4::EventControllerKey::new();
    let key_card = card.clone();
    let key_answer = answer.clone();
    key_controller.connect_key_pressed(move |_, keyval, _, modifiers| {
        let key = match keyval {
            gtk4::gdk::Key::Return | gtk4::gdk::Key::KP_Enter => ArbiterKey::Enter,
            gtk4::gdk::Key::Escape => ArbiterKey::Escape,
            _ => ArbiterKey::Other,
        };
        if key == ArbiterKey::Other {
            return gtk4::glib::Propagation::Proceed;
        }
        let mut focus = FocusContext {
            in_card: true,
            is_text_input: false,
            is_button: false,
        };
        if let Some(focused) = key_card.root().and_then(|root| {
            root.downcast::<gtk4::Window>()
                .ok()
                .and_then(|window| gtk4::prelude::GtkWindowExt::focus(&window))
        }) {
            let mut in_card = false;
            let mut node: Option<gtk4::Widget> = Some(focused);
            while let Some(widget) = node {
                let is_card = widget == key_card.clone().upcast::<gtk4::Widget>();
                if is_card {
                    in_card = true;
                }
                if widget.is::<gtk4::Entry>()
                    || widget.is::<gtk4::TextView>()
                    || widget.is::<gtk4::SpinButton>()
                {
                    focus.is_text_input = true;
                }
                if widget.is::<gtk4::Button>() {
                    focus.is_button = true;
                }
                if is_card {
                    break;
                }
                node = widget.parent();
            }
            focus.in_card = in_card;
        }
        let has_modifiers = modifiers.intersects(
            gtk4::gdk::ModifierType::CONTROL_MASK
                | gtk4::gdk::ModifierType::ALT_MASK
                | gtk4::gdk::ModifierType::SUPER_MASK
                | gtk4::gdk::ModifierType::SHIFT_MASK,
        );
        match arbitrate_key(key, has_modifiers, focus) {
            ArbitratedDecision::Allow => {
                key_answer(ApprovalOutcome::AllowedOnce);
                gtk4::glib::Propagation::Stop
            }
            ArbitratedDecision::Reject => {
                key_answer(ApprovalOutcome::Rejected);
                gtk4::glib::Propagation::Stop
            }
            ArbitratedDecision::Pass => gtk4::glib::Propagation::Proceed,
        }
    });
    card.add_controller(key_controller);

    card.upcast()
}

struct QuestionPage {
    checks: Vec<(String, gtk4::CheckButton)>,
    entry: gtk4::Entry,
}

pub fn build_question_card(
    req: &SdkUserQuestionRequest,
    draft: QuestionDraft,
    answerable: bool,
    on_answer: Rc<dyn Fn(SdkUserQuestionResult)>,
) -> gtk4::Widget {
    let copy = copy();
    if let Some(UserQuestionIntent::PlanReview { approve, .. }) =
        req.questions.first().and_then(|q| q.intent.as_ref())
    {
        return build_plan_review_card(req, approve.clone(), answerable, on_answer, &copy);
    }

    let card = card_box("dsh-question-card");
    card.append(&strip_box(if answerable {
        copy.waiting_answer
    } else {
        copy.expired
    }));

    let draft = Rc::new(RefCell::new(draft));
    let stack = gtk4::Stack::builder()
        .transition_type(gtk4::StackTransitionType::SlideLeftRight)
        .vexpand(true)
        .build();
    let pages: Rc<RefCell<Vec<QuestionPage>>> = Rc::new(RefCell::new(Vec::new()));
    let page_index = Rc::new(Cell::new(0usize));
    let page_count = req.questions.len().max(1);
    let question_count = req.questions.len();
    let refresh_next: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    for (index, question) in req.questions.iter().enumerate() {
        let page_box = body_box();
        let prompt = gtk4::Label::builder()
            .label(&question.question)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["heading"])
            .build();
        page_box.append(&prompt);
        if let Some(detail) = question.detail.as_ref() {
            let markdown = render_markdown(detail);
            page_box.append(&markdown);
        }

        let mut checks = Vec::new();
        let group: Rc<RefCell<Option<gtk4::CheckButton>>> = Rc::new(RefCell::new(None));
        if let Some(options) = question.options.as_ref() {
            let multi = question.multi_select.unwrap_or(false);
            for option in options {
                let (display, recommended) = parse_recommended_label(&option.label);
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(6)
                    .build();
                let text = match option.description.as_ref() {
                    Some(description) if !description.is_empty() => {
                        format!("{} — {}", display, description)
                    }
                    _ => display,
                };
                let label = gtk4::Label::builder()
                    .label(&text)
                    .xalign(0.0)
                    .wrap(true)
                    .build();
                row.append(&label);
                if recommended {
                    let badge = gtk4::Label::builder()
                        .label(copy.recommended)
                        .css_classes(["dsh-preview-badge", "caption"])
                        .build();
                    row.append(&badge);
                }
                let check = gtk4::CheckButton::builder()
                    .css_classes(["flat"])
                    .build();
                check.set_child(Some(&row));
                let preselected = draft.borrow().selected[index].contains(&option.label);
                check.set_active(preselected);
                if !multi {
                    if let Some(first) = group.borrow().as_ref() {
                        check.set_group(Some(first));
                    } else {
                        *group.borrow_mut() = Some(check.clone());
                    }
                }
                let draft_toggle = draft.clone();
                let label_value = option.label.clone();
                let checks_toggle = pages.clone();
                let index_toggle = index;
                let stack_advance = stack.clone();
                let page_index_advance = page_index.clone();
                let refresh_advance = refresh_next.clone();
                let single_select = !multi;
                check.connect_toggled(move |check| {
                    if !check.is_active() {
                        return;
                    }
                    let active: Vec<String> = {
                        let pages = checks_toggle.borrow();
                        match pages.get(index_toggle) {
                            Some(page) => page
                                .checks
                                .iter()
                                .filter(|(_, item)| item.is_active())
                                .map(|(label, _)| label.clone())
                                .collect(),
                            None => vec![label_value.clone()],
                        }
                    };
                    if let Some(selected) = draft_toggle.borrow_mut().selected.get_mut(index_toggle) {
                        *selected = active;
                    }
                    let is_last = index_toggle + 1 >= question_count;
                    if single_select && !is_last {
                        page_index_advance.set(index_toggle + 1);
                        stack_advance.set_visible_child_name(&(index_toggle + 1).to_string());
                        if let Some(refresh) = refresh_advance.borrow().as_ref() {
                            refresh();
                        }
                    }
                });
                checks.push((option.label.clone(), check.clone()));
                page_box.append(&check);
            }
        }

        let entry = gtk4::Entry::builder()
            .placeholder_text(copy.custom_answer)
            .build();
        entry.set_text(&draft.borrow().custom[index]);
        let draft_text = draft.clone();
        entry.connect_changed(move |entry| {
            if let Some(custom) = draft_text.borrow_mut().custom.get_mut(index) {
                *custom = entry.text().to_string();
            }
        });
        page_box.append(&entry);

        stack.add_named(&page_box, Some(&index.to_string()));
        pages.borrow_mut().push(QuestionPage {
            checks,
            entry,
        });
    }
    card.append(&stack);

    let actions = action_row();
    let back_btn = gtk4::Button::builder().label(copy.back).build();
    let skip_btn = gtk4::Button::builder().label(copy.skip).build();
    let next_btn = gtk4::Button::builder()
        .label(if page_count > 1 { copy.next } else { copy.submit })
        .css_classes(["suggested-action"])
        .build();
    let cancel_btn = gtk4::Button::builder().label(copy.cancel).build();
    actions.append(&back_btn);
    actions.append(&skip_btn);
    actions.append(&cancel_btn);
    actions.append(&next_btn);
    card.append(&actions);

    {
        let submit = copy.submit;
        let next = copy.next;
        let next_btn = next_btn.clone();
        let page_index = page_index.clone();
        *refresh_next.borrow_mut() = Some(Box::new(move || {
            let label = if page_index.get() + 1 >= page_count {
                submit
            } else {
                next
            };
            next_btn.set_label(label);
        }));
    }

    {
        let stack = stack.clone();
        let page_index = page_index.clone();
        let refresh_next = refresh_next.clone();
        back_btn.connect_clicked(move |_| {
            let current = page_index.get();
            if current > 0 {
                let next = current - 1;
                page_index.set(next);
                stack.set_visible_child_name(&next.to_string());
                if let Some(refresh) = refresh_next.borrow().as_ref() {
                    refresh();
                }
            }
        });
    }
    {
        let stack = stack.clone();
        let page_index = page_index.clone();
        let pages = pages.clone();
        let draft = draft.clone();
        let refresh_next = refresh_next.clone();
        skip_btn.connect_clicked(move |_| {
            let current = page_index.get();
            if let Some(selected) = draft.borrow_mut().selected.get_mut(current) {
                selected.clear();
            }
            if let Some(custom) = draft.borrow_mut().custom.get_mut(current) {
                custom.clear();
            }
            if let Some(page) = pages.borrow().get(current) {
                for (_, check) in &page.checks {
                    check.set_active(false);
                }
                page.entry.set_text("");
            }
            if current + 1 < pages.borrow().len() {
                let next = current + 1;
                page_index.set(next);
                stack.set_visible_child_name(&next.to_string());
                if let Some(refresh) = refresh_next.borrow().as_ref() {
                    refresh();
                }
            }
        });
    }
    {
        let stack = stack.clone();
        let page_index = page_index.clone();
        let draft = draft.clone();
        let req = req.clone();
        let on_answer = on_answer.clone();
        let refresh_next = refresh_next.clone();
        next_btn.connect_clicked(move |_| {
            let current = page_index.get();
            if current + 1 < page_count {
                let next = current + 1;
                page_index.set(next);
                stack.set_visible_child_name(&next.to_string());
                if let Some(refresh) = refresh_next.borrow().as_ref() {
                    refresh();
                }
                return;
            }
            let answers = draft.borrow().answers(&req);
            on_answer(SdkUserQuestionResult::answers(answers));
        });
    }
    {
        let on_answer = on_answer.clone();
        cancel_btn.connect_clicked(move |_| {
            on_answer(SdkUserQuestionResult::cancelled());
        });
    }

    if !answerable {
        skip_btn.set_sensitive(false);
        next_btn.set_sensitive(false);
        for page in pages.borrow().iter() {
            for (_, check) in &page.checks {
                check.set_sensitive(false);
            }
            page.entry.set_sensitive(false);
        }
        back_btn.set_sensitive(false);
        cancel_btn.set_label(copy.close);
    }

    card.upcast()
}

fn build_plan_review_card(
    req: &SdkUserQuestionRequest,
    approve_label: String,
    answerable: bool,
    on_answer: Rc<dyn Fn(SdkUserQuestionResult)>,
    copy: &Copy,
) -> gtk4::Widget {
    let card = card_box("dsh-plan-review-card");
    card.append(&strip_box(if answerable {
        copy.plan_review
    } else {
        copy.expired
    }));

    let body = body_box();
    let question = req.questions.first().cloned();
    if let Some(question) = question.as_ref() {
        let prompt = gtk4::Label::builder()
            .label(&question.question)
            .xalign(0.0)
            .wrap(true)
            .css_classes(["title-4"])
            .build();
        body.append(&prompt);
        if let Some(detail) = question.detail.as_ref() {
            let markdown = render_markdown(detail);
            body.append(&markdown);
        }
    }
    card.append(&body);

    let actions = action_row();
    let changes_btn = gtk4::Button::builder()
        .label(copy.request_changes)
        .build();
    let approve_btn = gtk4::Button::builder()
        .label(&approve_label)
        .css_classes(["suggested-action"])
        .build();
    actions.append(&changes_btn);
    actions.append(&approve_btn);
    card.append(&actions);

    let answerable_flag = Rc::new(Cell::new(answerable));
    {
        let on_answer = on_answer.clone();
        let answerable = answerable_flag.clone();
        let changes_btn_clone = changes_btn.clone();
        let approve_btn_clone = approve_btn.clone();
        changes_btn.connect_clicked(move |_| {
            if !answerable.get() {
                return;
            }
            answerable.set(false);
            changes_btn_clone.set_sensitive(false);
            approve_btn_clone.set_sensitive(false);
            on_answer(SdkUserQuestionResult::cancelled());
        });
    }
    {
        let q_id = req
            .questions
            .first()
            .map(|q| q.id.clone())
            .unwrap_or_default();
        let label = approve_label.clone();
        let on_answer = on_answer.clone();
        let answerable = answerable_flag.clone();
        let changes_btn_clone = changes_btn.clone();
        approve_btn.connect_clicked(move |_| {
            if !answerable.get() {
                return;
            }
            answerable.set(false);
            changes_btn_clone.set_sensitive(false);
            on_answer(SdkUserQuestionResult::answers(vec![
                crate::protocol::UserQuestionAnswer {
                    id: q_id.clone(),
                    selected: vec![label.clone()],
                },
            ]));
        });
    }
    if !answerable {
        changes_btn.set_sensitive(false);
        approve_btn.set_sensitive(false);
    }

    card.upcast()
}
