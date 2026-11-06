use crate::protocol::QueueItem;
use crate::transcript::{ConversationTranscript, RenderBlock};
use crate::ui::composer::Composer;
use crate::ui::desktop::{open_path_in_default_app, open_terminal_in_dir};
use crate::ui::message_renderer::{render_block, AssistantLiveHandle};
use crate::ui::goal_bar::GoalBar;
use crate::ui::queue_dock::QueueDock;
use crate::ui::schedules::SchedulesPage;
use crate::ui::todo_dock::TodoDock;
use libadwaita as adw;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

pub struct ConversationView {
    stack: gtk4::Stack,
    composer: Composer,
    goal_bar: GoalBar,
    queue_dock: QueueDock,
    todo_dock: TodoDock,
    transcript_box: gtk4::Box,
    welcome_bubble: gtk4::Box,
    status_label: gtk4::Label,
    title_label: gtk4::Label,
    breadcrumb: gtk4::Label,
    jobs_popover: gtk4::Popover,
    jobs_box: gtk4::Box,
    jobs_output: gtk4::Label,
    usage_label: gtk4::Label,
    context_button: gtk4::MenuButton,
    context_label: gtk4::Label,
    context_pressure: RefCell<Option<serde_json::Value>>,
    context_breakdown: RefCell<Option<serde_json::Value>>,
    transcript_scroll: gtk4::ScrolledWindow,
    on_load_older: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    live_handle: RefCell<Option<AssistantLiveHandle>>,
    schedules_page: SchedulesPage,
    trajectory_page: adw::ViewStackPage,
    rendered_count: Cell<usize>,
    active_cwd: Rc<RefCell<PathBuf>>,
    title_activate: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl ConversationView {
    pub fn new() -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let conversation_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let active_cwd = Rc::new(RefCell::new(PathBuf::from(
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        )));

        let session_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let title_label = gtk4::Label::builder()
            .label("New Session")
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .css_classes(["title-4"])
            .build();
        session_header.append(&title_label);

        let breadcrumb = gtk4::Label::builder()
            .label("")
            .css_classes(["dim-label", "caption"])
            .halign(gtk4::Align::Start)
            .build();
        session_header.append(&breadcrumb);

        let title_activate = Rc::new(RefCell::new(None::<Box<dyn Fn()>>));
        let title_click = gtk4::GestureClick::new();
        title_click.set_button(1);
        let title_activate_for_click = title_activate.clone();
        title_click.connect_pressed(move |_, n_press, _, _| {
            if n_press == 2 {
                if let Some(handler) = title_activate_for_click.borrow().as_ref() {
                    handler();
                }
            }
        });
        title_label.add_controller(title_click);

        let open_folder_btn = gtk4::Button::builder()
            .label("Open Folder")
            .tooltip_text("Open project folder in default file manager")
            .css_classes(["flat"])
            .build();
        let cwd_for_folder = active_cwd.clone();
        open_folder_btn.connect_clicked(move |_| {
            let path = cwd_for_folder.borrow().clone();
            open_path_in_default_app(&path);
        });
        session_header.append(&open_folder_btn);

        let open_terminal_btn = gtk4::Button::builder()
            .label("Open Terminal")
            .tooltip_text("Open terminal in project directory")
            .css_classes(["flat"])
            .build();
        let cwd_for_terminal = active_cwd.clone();
        open_terminal_btn.connect_clicked(move |_| {
            let path = cwd_for_terminal.borrow().clone();
            open_terminal_in_dir(&path);
        });
        session_header.append(&open_terminal_btn);

        let jobs_button = gtk4::MenuButton::builder()
            .label("Jobs")
            .css_classes(["flat"])
            .build();
        let jobs_popover = gtk4::Popover::builder().build();
        let jobs_outer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        let jobs_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        jobs_outer.append(&jobs_box);
        let jobs_output = gtk4::Label::builder()
            .label("")
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .css_classes(["monospace", "caption"])
            .build();
        jobs_outer.append(&jobs_output);
        jobs_popover.set_child(Some(&jobs_outer));
        jobs_button.set_popover(Some(&jobs_popover));
        session_header.append(&jobs_button);

        let usage_label = gtk4::Label::builder()
            .label("")
            .css_classes(["dim-label", "caption"])
            .build();
        session_header.append(&usage_label);

        let context_button = gtk4::MenuButton::builder()
            .label("Context")
            .css_classes(["flat"])
            .visible(false)
            .build();
        let context_popover = gtk4::Popover::builder().build();
        let context_label = gtk4::Label::builder()
            .label("")
            .wrap(true)
            .selectable(true)
            .xalign(0.0)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .css_classes(["monospace", "caption"])
            .build();
        context_popover.set_child(Some(&context_label));
        context_button.set_popover(Some(&context_popover));
        session_header.append(&context_button);

        let status_label = gtk4::Label::builder()
            .label("Idle")
            .css_classes(["dim-label", "caption"])
            .build();
        session_header.append(&status_label);

        conversation_box.append(&session_header);

        let view_stack = adw::ViewStack::new();

        let chat_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let transcript_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let load_slot_for_scroll: Rc<RefCell<Option<Box<dyn Fn()>>>> =
            Rc::new(RefCell::new(None));
        let load_slot_clone = load_slot_for_scroll.clone();
        let adjustment = transcript_scroll.vadjustment();
        adjustment.connect_value_changed(move |adj| {
            if adj.value() <= 8.0 {
                if let Some(handler) = load_slot_clone.borrow().as_ref() {
                    handler();
                }
            }
        });

        let transcript_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();

        let welcome_bubble = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .css_classes(["card"])
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let welcome_title = gtk4::Label::builder()
            .label("Welcome to dsh")
            .css_classes(["title-3"])
            .halign(gtk4::Align::Start)
            .build();
        welcome_bubble.append(&welcome_title);

        let welcome_text = gtk4::Label::builder()
            .label("Ask questions, inspect workspace diffs, or run developer agent commands.")
            .wrap(true)
            .halign(gtk4::Align::Start)
            .css_classes(["dim-label"])
            .build();
        welcome_bubble.append(&welcome_text);
        transcript_box.append(&welcome_bubble);

        transcript_scroll.set_child(Some(&transcript_box));
        chat_box.append(&transcript_scroll);

        let chat_page = view_stack.add_titled(&chat_box, Some("chat"), "Chat");
        chat_page.set_icon_name(Some("chat-bubbles-symbolic"));

        let trajectory_page = adw::StatusPage::builder()
            .title("Trajectory")
            .description("Agent tool calls and execution trace")
            .icon_name("view-list-tree-symbolic")
            .build();
        let traj_page = view_stack.add_titled(&trajectory_page, Some("trajectory"), "Trajectory");
        traj_page.set_icon_name(Some("view-list-tree-symbolic"));

        let view_switcher = adw::ViewSwitcher::builder()
            .stack(&view_stack)
            .policy(adw::ViewSwitcherPolicy::Wide)
            .halign(gtk4::Align::Center)
            .build();

        let switcher_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::Center)
            .margin_bottom(4)
            .build();
        switcher_bar.append(&view_switcher);
        conversation_box.append(&switcher_bar);

        conversation_box.append(&view_stack);

        let goal_bar = GoalBar::new();
        conversation_box.append(goal_bar.widget());

        let queue_dock = QueueDock::new();
        conversation_box.append(queue_dock.widget());

        let todo_dock = TodoDock::new();
        conversation_box.append(todo_dock.widget());

        let composer = Composer::new();
        conversation_box.append(composer.widget());

        stack.add_named(&conversation_box, Some("conversation"));

        let plugins_page = adw::StatusPage::builder()
            .title("Plugin Manager")
            .description("Discover and manage installed Cordis plugin bundles")
            .icon_name("application-x-addon-symbolic")
            .build();
        stack.add_named(&plugins_page, Some("plugins"));

        let schedules_page = SchedulesPage::new();
        stack.add_named(schedules_page.widget(), Some("schedules"));

        Self {
            stack,
            composer,
            goal_bar,
            queue_dock,
            todo_dock,
            transcript_box,
            welcome_bubble,
            status_label,
            title_label,
            breadcrumb,
            jobs_popover,
            jobs_box,
            jobs_output,
            usage_label,
            context_button,
            context_label,
            context_pressure: RefCell::new(None),
            context_breakdown: RefCell::new(None),
            transcript_scroll,
            on_load_older: load_slot_for_scroll,
            live_handle: RefCell::new(None),
            schedules_page,
            trajectory_page: traj_page,
            rendered_count: Cell::new(0),
            active_cwd,
            title_activate,
        }
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn set_trajectory_visible(&self, visible: bool) {
        self.trajectory_page.set_visible(visible);
    }

    pub fn schedules_page(&self) -> &SchedulesPage {
        &self.schedules_page
    }

    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    pub fn queue_dock(&self) -> &QueueDock {
        &self.queue_dock
    }

    pub fn set_title_double_click_handler<F: Fn() + 'static>(&self, handler: F) {
        *self.title_activate.borrow_mut() = Some(Box::new(handler));
    }

    pub fn transcript_box(&self) -> &gtk4::Box {
        &self.transcript_box
    }

    pub fn set_title(&self, title: &str) {
        self.title_label.set_label(title);
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_label(status);
        self.composer.set_is_running(status == "Running");
    }

    pub fn set_disconnected(&self) {
        let current = self.status_label.label().to_string();
        if current == "Running" || current == "Running (paused)" {
            self.status_label.set_label("Running (paused)");
        } else {
            self.status_label.set_label("Disconnected");
        }
    }

    pub fn set_cwd(&self, cwd: &str) {
        *self.active_cwd.borrow_mut() = PathBuf::from(cwd);
    }

    pub fn goal_bar(&self) -> &GoalBar {
        &self.goal_bar
    }

    pub fn update_queue(&self, items: &[QueueItem]) {
        self.queue_dock.update(items);
    }

    pub fn todo_dock(&self) -> &TodoDock {
        &self.todo_dock
    }

    pub fn update_todo(&self, items: &[crate::protocol::TodoItem]) {
        self.todo_dock.update(items);
    }

    pub fn set_on_load_older<F: Fn() + 'static>(&self, handler: F) {
        *self.on_load_older.borrow_mut() = Some(Box::new(handler));
    }

    pub fn transcript_adjustment(&self) -> gtk4::Adjustment {
        self.transcript_scroll.vadjustment()
    }

    pub fn rerender_transcript(&self, transcript: &ConversationTranscript) {
        while let Some(child) = self.transcript_box.first_child() {
            self.transcript_box.remove(&child);
        }
        self.live_handle.borrow_mut().take();
        self.rendered_count.set(0);
        self.update_transcript(transcript);
    }

    pub fn set_history_loading(&self, loading: bool) {
        let existing = self
            .transcript_box
            .first_child()
            .filter(|child| child.widget_name() == "history-loading");
        if loading {
            if existing.is_none() {
                let row = gtk4::Label::builder()
                    .label("Loading earlier messages…")
                    .css_classes(["dim-label", "caption"])
                    .build();
                row.set_widget_name("history-loading");
                self.transcript_box.prepend(&row);
            }
        } else if let Some(row) = existing {
            self.transcript_box.remove(&row);
        }
    }

    pub fn set_breadcrumb(&self, text: &str) {
        self.breadcrumb.set_label(text);
    }

    pub fn set_jobs(&self, jobs: &[crate::protocol::JobWireView]) {
        while let Some(child) = self.jobs_box.first_child() {
            self.jobs_box.remove(&child);
        }
        if jobs.is_empty() {
            let row = gtk4::Label::builder()
                .label("No background jobs")
                .css_classes(["dim-label", "caption"])
                .halign(gtk4::Align::Start)
                .build();
            self.jobs_box.append(&row);
            return;
        }
        for job in jobs {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .build();
            let label = gtk4::Button::builder()
                .label(format!("{} · {}", job.label, job.status))
                .css_classes(["flat"])
                .action_name("win.follow-job")
                .build();
            label.set_action_target_value(Some(&job.id.to_variant()));
            row.append(&label);
            let kill = gtk4::Button::builder()
                .label("Kill")
                .css_classes(["flat", "destructive-action"])
                .action_name("win.kill-job")
                .build();
            kill.set_action_target_value(Some(&job.id.to_variant()));
            row.append(&kill);
            self.jobs_box.append(&row);
        }
    }

    pub fn show_job_follow(&self, follow: &crate::protocol::JobFollowResult) {
        let text = follow
            .chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<String>();
        if follow.lossy {
            self.jobs_output
                .set_label(&format!("[earlier output lost]\n{}", text));
        } else {
            self.jobs_output.set_label(&text);
        }
        self.jobs_popover.popup();
    }

    pub fn set_usage(&self, usage: &serde_json::Value) {
        let text = usage
            .get("totalTokens")
            .and_then(|value| value.as_u64())
            .or_else(|| usage.get("total").and_then(|value| value.as_u64()))
            .map(|value| format!("{} tok", value))
            .unwrap_or_else(|| serde_json::to_string(usage).unwrap_or_default());
        self.usage_label.set_label(&text);
    }

    pub fn set_context(&self, pressure: Option<&serde_json::Value>, breakdown: Option<&serde_json::Value>) {
        if let Some(pressure) = pressure {
            *self.context_pressure.borrow_mut() = Some(pressure.clone());
        }
        if let Some(breakdown) = breakdown {
            *self.context_breakdown.borrow_mut() = Some(breakdown.clone());
        }
        let pressure = self.context_pressure.borrow().clone();
        let Some(pressure) = pressure else {
            self.context_button.set_visible(false);
            return;
        };
        let text = pressure
            .get("used")
            .and_then(|value| value.as_u64())
            .zip(pressure.get("limit").and_then(|value| value.as_u64()))
            .map(|(used, limit)| format!("{}%", used.saturating_mul(100) / limit.max(1)))
            .unwrap_or_else(|| serde_json::to_string(&pressure).unwrap_or_default());
        self.context_button.set_label(&format!("Context {}", text));
        self.context_button.set_visible(true);
        let details = self
            .context_breakdown
            .borrow()
            .as_ref()
            .map(|value| serde_json::to_string_pretty(value).unwrap_or_default())
            .unwrap_or_else(|| serde_json::to_string_pretty(&pressure).unwrap_or_default());
        self.context_label.set_label(&details);
    }

    pub fn job_output_append(&self, chunks: &[crate::protocol::JobChunkWire]) {
        let addition = chunks
            .iter()
            .map(|chunk| chunk.text.as_str())
            .collect::<String>();
        let existing = self.jobs_output.label().to_string();
        self.jobs_output
            .set_label(&format!("{}{}", existing, addition));
    }

    pub fn clear_session(&self) {
        while let Some(child) = self.transcript_box.first_child() {
            self.transcript_box.remove(&child);
        }
        self.transcript_box.append(&self.welcome_bubble);
        self.live_handle.borrow_mut().take();
        self.rendered_count.set(0);
        self.set_title("New Session");
        self.set_status("Idle");
        self.queue_dock.update(&[]);
    }

    pub fn select_panel(&self, panel_name: &str) {
        self.stack.set_visible_child_name(panel_name);
    }

    pub fn update_transcript(&self, transcript: &ConversationTranscript) {
        let blocks = transcript.blocks();
        if blocks.is_empty() {
            while let Some(child) = self.transcript_box.first_child() {
                self.transcript_box.remove(&child);
            }
            self.live_handle.borrow_mut().take();
            self.rendered_count.set(0);
            return;
        }

        if self.rendered_count.get() == 0 {
            while let Some(child) = self.transcript_box.first_child() {
                self.transcript_box.remove(&child);
            }
        }

        let current_count = self.rendered_count.get();
        if blocks.len() > current_count {
            for block in &blocks[current_count..] {
                let (widget, handle) = render_block(block);
                self.transcript_box.append(&widget);
                if let Some(h) = handle {
                    *self.live_handle.borrow_mut() = Some(h);
                }
            }
            self.rendered_count.set(blocks.len());
        }

        if let Some(last_block) = blocks.last() {
            match last_block {
                RenderBlock::AssistantMessage {
                    text,
                    reasoning,
                    is_live,
                    ..
                } => {
                    let handle_opt = self.live_handle.borrow().clone();
                    if let Some(handle) = handle_opt {
                        if *is_live {
                            handle.update_text(text);
                            if let Some(r) = reasoning {
                                handle.update_reasoning(r);
                            }
                        } else {
                            handle.finalize();
                            *self.live_handle.borrow_mut() = None;
                        }
                    }
                }
                _ => {
                    if let Some(handle) = self.live_handle.borrow_mut().take() {
                        handle.finalize();
                    }
                }
            }
        }
    }
}

impl Default for ConversationView {
    fn default() -> Self {
        Self::new()
    }
}
