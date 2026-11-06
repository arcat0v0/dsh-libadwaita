use crate::protocol::{TerminalCreateResult, TerminalInfo, TerminalOutputResult};
use crate::ui::controller::SessionController;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub struct TerminalPanel {
    container: gtk4::Box,
    output: gtk4::TextView,
    input: gtk4::Entry,
    status: gtk4::Label,
    controller: RefCell<Option<Weak<SessionController>>>,
    session_id: Rc<RefCell<Option<String>>>,
    terminal_id: Rc<RefCell<Option<String>>>,
    next_offset: Rc<RefCell<Option<u64>>>,
    poll_started: Rc<RefCell<bool>>,
    cwd: Rc<RefCell<String>>,
}

impl TerminalPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .build();
        let status = gtk4::Label::builder()
            .label("No terminal")
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label", "caption"])
            .build();
        header.append(&status);
        let close = gtk4::Button::builder()
            .label("Close")
            .css_classes(["flat", "destructive-action"])
            .build();
        header.append(&close);
        container.append(&header);
        let scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .build();
        let output = gtk4::TextView::builder()
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::None)
            .build();
        scrolled.set_child(Some(&output));
        container.append(&scrolled);
        let input = gtk4::Entry::builder()
            .placeholder_text("Command, then Enter")
            .margin_start(8)
            .margin_end(8)
            .margin_bottom(8)
            .build();
        container.append(&input);

        let panel = Self {
            container,
            output,
            input,
            status,
            controller: RefCell::new(None),
            session_id: Rc::new(RefCell::new(None)),
            terminal_id: Rc::new(RefCell::new(None)),
            next_offset: Rc::new(RefCell::new(None)),
            poll_started: Rc::new(RefCell::new(false)),
            cwd: Rc::new(RefCell::new(String::new())),
        };
        panel.start_polling();
        panel.wire_input();
        panel.wire_close();
        panel
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        *self.controller.borrow_mut() = Some(Rc::downgrade(&controller));
        self.start_polling();
    }

    fn with_controller(&self, f: impl FnOnce(&SessionController)) {
        if let Some(controller) = self
            .controller
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
        {
            f(&controller);
        }
    }

    fn wire_input(&self) {
        let session = self.session_id.clone();
        let terminal = self.terminal_id.clone();
        let controller = self.controller.clone();
        let input = self.input.clone();
        self.input.connect_activate(move |entry| {
            let Some(session_id) = session.borrow().clone() else {
                return;
            };
            let Some(terminal_id) = terminal.borrow().clone() else {
                return;
            };
            let text = format!("{}\n", entry.text());
            entry.set_text("");
            if let Some(ctrl) = controller
                .borrow()
                .as_ref()
                .and_then(|weak| weak.upgrade())
            {
                ctrl.send_terminal_input(&session_id, &terminal_id, &text);
            }
            let _ = &input;
        });
    }

    fn wire_close(&self) {
        let session = self.session_id.clone();
        let terminal = self.terminal_id.clone();
        let controller = self.controller.clone();
        let status = self.status.clone();
        self.container
            .last_child()
            .and_then(|_| Some(()));
        let _ = status;
        let close = self
            .container
            .first_child()
            .and_then(|child| child.downcast::<gtk4::Box>().ok())
            .and_then(|row| row.last_child())
            .and_then(|child| child.downcast::<gtk4::Button>().ok());
        if let Some(close) = close {
            close.connect_clicked(move |_| {
                let Some(session_id) = session.borrow().clone() else {
                    return;
                };
                let Some(terminal_id) = terminal.borrow().clone() else {
                    return;
                };
                if let Some(ctrl) = controller
                    .borrow()
                    .as_ref()
                    .and_then(|weak| weak.upgrade())
                {
                    ctrl.close_terminal(&session_id, &terminal_id);
                }
            });
        }
    }

    fn start_polling(&self) {
        let mut started = self.poll_started.borrow_mut();
        if *started {
            return;
        }
        *started = true;
        let session = self.session_id.clone();
        let terminal = self.terminal_id.clone();
        let offset = self.next_offset.clone();
        let controller = self.controller.clone();
        let visible = self.container.clone();
        gtk4::glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            if visible.is_visible() {
                let session_id = session.borrow().clone();
                let terminal_id = terminal.borrow().clone();
                if let (Some(session_id), Some(terminal_id)) = (session_id, terminal_id) {
                    if let Some(ctrl) = controller
                        .borrow()
                        .as_ref()
                        .and_then(|weak| weak.upgrade())
                    {
                        ctrl.poll_terminal_output(
                            &session_id,
                            &terminal_id,
                            *offset.borrow(),
                        );
                    }
                }
            }
            gtk4::glib::ControlFlow::Continue
        });
    }

    pub fn restore(&self, session_id: &str, cwd: &str) {
        *self.session_id.borrow_mut() = Some(session_id.to_string());
        *self.cwd.borrow_mut() = cwd.to_string();
        *self.terminal_id.borrow_mut() = None;
        *self.next_offset.borrow_mut() = None;
        self.status.set_label("Restoring terminal…");
        self.with_controller(|ctrl| ctrl.load_terminals(session_id));
    }

    pub fn handle_list(&self, terminals: &[TerminalInfo]) {
        if let Some(existing) = terminals.first() {
            *self.terminal_id.borrow_mut() = Some(existing.terminal_id.clone());
            self.status
                .set_label(&format!("pid {} · {}", existing.pid, existing.cwd));
            self.poll_once();
            return;
        }
        let Some(session_id) = self.session_id.borrow().clone() else {
            return;
        };
        let cwd = self.cwd.borrow().clone();
        self.with_controller(|ctrl| ctrl.create_terminal(&session_id, &cwd));
    }

    pub fn handle_created(&self, created: &TerminalCreateResult) {
        *self.terminal_id.borrow_mut() = Some(created.terminal_id.clone());
        *self.next_offset.borrow_mut() = None;
        self.status
            .set_label(&format!("pid {} · {}", created.pid, created.cwd));
        self.output.buffer().set_text("");
        self.poll_once();
    }

    pub fn handle_output(&self, output: &TerminalOutputResult) {
        let buffer = self.output.buffer();
        if output.truncated {
            buffer.set_text("[earlier terminal output truncated]\n");
        }
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, &output.text);
        let line_count = buffer.line_count();
        if line_count > 5000 {
            let mut start = buffer.start_iter();
            let mut end = buffer.iter_at_line(line_count - 5000).unwrap_or(buffer.end_iter());
            buffer.delete(&mut start, &mut end);
        }
        *self.next_offset.borrow_mut() = Some(output.line_end);
    }

    fn poll_once(&self) {
        let Some(session_id) = self.session_id.borrow().clone() else {
            return;
        };
        let Some(terminal_id) = self.terminal_id.borrow().clone() else {
            return;
        };
        let offset = *self.next_offset.borrow();
        self.with_controller(|ctrl| {
            ctrl.poll_terminal_output(&session_id, &terminal_id, offset)
        });
    }
}

impl Default for TerminalPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalPanel {
    pub fn placeholder() -> adw::StatusPage {
        adw::StatusPage::builder()
            .title("Terminal")
            .description("Interactive terminal")
            .icon_name("utilities-terminal-symbolic")
            .build()
    }
}
