use crate::protocol::{QueueItem, WorkspaceView};
use crate::transcript::{ConversationTranscript, RenderBlock, TurnStat};
use crate::ui::composer::Composer;
use crate::ui::copy::Copybook;
use crate::ui::desktop::{open_path_in_default_app, open_terminal_in_dir};
use crate::ui::message_renderer::{render_block, AssistantLiveHandle};
use crate::ui::goal_bar::GoalBar;
use crate::ui::plugins_page::PluginsPage;
use crate::ui::presentation::{PresentationPolicy, StepGrouping};
use crate::ui::queue_dock::QueueDock;
use crate::ui::schedules::SchedulesPage;
use crate::ui::scroll_follow::ScrollFollow;
use crate::ui::trajectory::TrajectoryView;
use crate::ui::turn_nav::{active_turn, fork_target_index, turn_markers, TurnMarker};
use crate::ui_prefs::TranscriptView;
use crate::ui::todo_dock::TodoDock;
use libadwaita as adw;
use gtk4::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub fn compact_token_count(value: u64) -> String {
    if value < 1_000 {
        return value.to_string();
    }
    if value < 1_000_000 {
        return format!("{:.1}K", value as f64 / 1_000.0);
    }
    format!("{:.1}M", value as f64 / 1_000_000.0)
}

pub fn token_usage_total(usage: &serde_json::Value) -> Option<u64> {
    let buckets = [
        "uncachedInputTokens",
        "outputTokens",
        "cacheReadTokens",
        "cacheWriteTokens",
    ];
    let mut total = 0u64;
    let mut found = false;
    for key in buckets {
        if let Some(value) = usage.get(key).and_then(|value| value.as_u64()) {
            total = total.saturating_add(value);
            found = true;
        }
    }
    found.then_some(total)
}

pub fn context_occupancy(pressure: &serde_json::Value) -> Option<(u64, u64, u32)> {
    let used = pressure
        .get("projectedTokens")
        .and_then(|value| value.as_u64())
        .or_else(|| pressure.get("pressureTokens").and_then(|value| value.as_u64()))?;
    let window = pressure
        .get("contextWindow")
        .and_then(|value| value.as_u64())
        .filter(|window| *window > 0)?;
    let percent =
        (((used.saturating_mul(100) + window / 2) / window).min(100)) as u32;
    Some((used, window, percent))
}

pub fn context_breakdown_rows(breakdown: &serde_json::Value) -> Vec<(&'static str, u64)> {
    [
        ("系统", "systemTokens"),
        ("工具", "toolsTokens"),
        ("消息", "messageTokens"),
    ]
    .into_iter()
    .filter_map(|(label, key)| {
        breakdown
            .get(key)
            .and_then(|value| value.as_u64())
            .map(|value| (label, value))
    })
    .collect()
}

pub fn intermediate_detail(block: &RenderBlock) -> Option<String> {
    match block {
        RenderBlock::ToolResult { info, .. } => {
            Some(info.name.clone()).filter(|name| !name.is_empty())
        }
        RenderBlock::GenericEvent { event_type, .. } => Some(event_type.clone()),
        _ => None,
    }
}

fn update_active_turn(
    adjustment: &gtk4::Adjustment,
    transcript_box: &gtk4::Box,
    widgets: &Rc<RefCell<Vec<(u32, gtk4::Widget)>>>,
    buttons: &Rc<RefCell<Vec<(u32, gtk4::Button)>>>,
) {
    let offsets: Vec<(u32, f64)> = widgets
        .borrow()
        .iter()
        .filter_map(|(turn, widget)| {
            widget
                .compute_point(transcript_box, &gtk4::graphene::Point::new(0.0, 0.0))
                .map(|point| (*turn, f64::from(point.y())))
        })
        .collect();
    let Some(active) = active_turn(&offsets, adjustment.value()) else {
        return;
    };
    for (turn, button) in buttons.borrow().iter() {
        if *turn == active {
            button.add_css_class("dsh-turn-mark-active");
        } else {
            button.remove_css_class("dsh-turn-mark-active");
        }
    }
}

fn jump_to_turn(
    turn: u32,
    transcript_box: &gtk4::Box,
    adjustment: &gtk4::Adjustment,
    widgets: &Rc<RefCell<Vec<(u32, gtk4::Widget)>>>,
) {
    let Some(widget) = widgets
        .borrow()
        .iter()
        .find(|(marker, _)| *marker == turn)
        .map(|(_, widget)| widget.clone())
    else {
        return;
    };
    let Some(point) = widget.compute_point(transcript_box, &gtk4::graphene::Point::new(0.0, 0.0))
    else {
        return;
    };
    adjustment.set_value(f64::from(point.y()).max(0.0));
}

fn apply_open_with_label(
    button: &gtk4::Button,
    app: Option<&crate::ui::open_with::OpenWithApp>,
) {
    match app {
        Some(app) if app.id != "filemanager" => {
            button.set_label(&format!("Open in {}", app.label));
            button.set_tooltip_text(Some(&format!("Open project folder in {}", app.label)));
        }
        _ => {
            button.set_label("Open Folder");
            button.set_tooltip_text(Some("Open project folder in default file manager"));
        }
    }
}

pub struct ConversationView {
    stack: gtk4::Stack,
    composer: Composer,
    goal_bar: GoalBar,
    queue_dock: QueueDock,
    todo_dock: TodoDock,
    transcript_box: gtk4::Box,
    welcome_bubble: gtk4::Box,
    status_label: gtk4::Label,
    running_timer: gtk4::Label,
    stats_button: gtk4::MenuButton,
    stats_list: gtk4::Box,
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
    scroll_follow: Rc<Cell<ScrollFollow>>,
    back_to_bottom: gtk4::Button,
    turn_rail: gtk4::Box,
    turn_markers: RefCell<Vec<TurnMarker>>,
    turn_widgets: Rc<RefCell<Vec<(u32, gtk4::Widget)>>>,
    rail_buttons: Rc<RefCell<Vec<(u32, gtk4::Button)>>>,
    block_handles: RefCell<HashMap<usize, AssistantLiveHandle>>,
    fork_handle: RefCell<Option<(usize, AssistantLiveHandle)>>,
    schedules_page: SchedulesPage,
    plugins_page: PluginsPage,
    trajectory_page: adw::ViewStackPage,
    trajectory_view: TrajectoryView,
    view_mode: Cell<TranscriptView>,
    on_view_mode: Rc<RefCell<Option<Box<dyn Fn(TranscriptView)>>>>,
    view_dropdown: gtk4::DropDown,
    view_stack: adw::ViewStack,
    on_view_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    rendered_count: Cell<usize>,
    active_cwd: Rc<RefCell<PathBuf>>,
    title_activate: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    hero_composer_slot: gtk4::Box,
    hero_workspace_menu: gtk4::MenuButton,
    composer_dock: gtk4::Box,
    takeover_slot: gtk4::Box,
    session_header: gtk4::Box,
    dock_button: gtk4::Button,
    switcher_bar: gtk4::Box,
    preset_menu: gtk4::MenuButton,
    preset_label: gtk4::Label,
    open_with_apps: Vec<&'static crate::ui::open_with::OpenWithApp>,
    open_with_choice: Rc<RefCell<Option<&'static crate::ui::open_with::OpenWithApp>>>,
    open_with_button: gtk4::Button,
    on_open_with_choice: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
}

impl ConversationView {
    pub fn new() -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let conversation_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-conversation"])
            .build();

        let active_cwd = Rc::new(RefCell::new(PathBuf::from(
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string()),
        )));

        let session_header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_start(12)
            .margin_end(112)
            .margin_top(6)
            .margin_bottom(8)
            .visible(false)
            .build();

        let title_label = gtk4::Label::builder()
            .label(Copybook::for_code("system").new_session)
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

        let open_with_apps = crate::ui::open_with::detect_apps();
        let open_with_choice: Rc<RefCell<Option<&'static crate::ui::open_with::OpenWithApp>>> =
            Rc::new(RefCell::new(crate::ui::open_with::default_app(
                &open_with_apps,
            )));
        let open_with_button = gtk4::Button::builder()
            .label("Open Folder")
            .tooltip_text("Open project folder in default file manager")
            .css_classes(["flat"])
            .build();
        apply_open_with_label(&open_with_button, *open_with_choice.borrow());
        let cwd_for_folder = active_cwd.clone();
        let choice_for_folder = open_with_choice.clone();
        open_with_button.connect_clicked(move |_| {
            let path = cwd_for_folder.borrow().clone();
            if let Some(app) = *choice_for_folder.borrow() {
                let _ = crate::ui::open_with::launch(app, &path);
            } else {
                open_path_in_default_app(&path);
            }
        });
        session_header.append(&open_with_button);
        let on_open_with_choice: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> =
            Rc::new(RefCell::new(None));
        if open_with_apps.len() > 1 {
            let open_with_menu = gtk4::MenuButton::builder()
                .icon_name("pan-down-symbolic")
                .tooltip_text("Choose an application")
                .css_classes(["flat"])
                .build();
            let open_with_popover = gtk4::Popover::builder().build();
            let open_with_list = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .margin_start(4)
                .margin_end(4)
                .margin_top(4)
                .margin_bottom(4)
                .build();
            for app in &open_with_apps {
                let row = gtk4::Button::builder()
                    .label(app.label)
                    .css_classes(["flat"])
                    .halign(gtk4::Align::Fill)
                    .build();
                let cwd_for_app = active_cwd.clone();
                let choice_for_app = open_with_choice.clone();
                let button_for_app = open_with_button.clone();
                let popover_for_app = open_with_popover.clone();
                let on_choice_for_app = on_open_with_choice.clone();
                let app = *app;
                row.connect_clicked(move |_| {
                    *choice_for_app.borrow_mut() = Some(app);
                    apply_open_with_label(&button_for_app, Some(app));
                    popover_for_app.popdown();
                    let path = cwd_for_app.borrow().clone();
                    let _ = crate::ui::open_with::launch(app, &path);
                    if let Some(handler) = on_choice_for_app.borrow().as_ref() {
                        handler(app.id);
                    }
                });
                open_with_list.append(&row);
            }
            open_with_popover.set_child(Some(&open_with_list));
            open_with_menu.set_popover(Some(&open_with_popover));
            session_header.append(&open_with_menu);
        }

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
            .visible(false)
            .css_classes(["dim-label", "caption"])
            .build();
        session_header.append(&usage_label);

        let stats_button = gtk4::MenuButton::builder()
            .label("Stats")
            .css_classes(["flat", "caption"])
            .visible(false)
            .build();
        let stats_popover = gtk4::Popover::builder().build();
        let stats_scroll = gtk4::ScrolledWindow::builder()
            .min_content_height(80)
            .max_content_height(320)
            .min_content_width(280)
            .propagate_natural_height(true)
            .build();
        let stats_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(8)
            .margin_end(8)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        stats_scroll.set_child(Some(&stats_list));
        stats_popover.set_child(Some(&stats_scroll));
        stats_button.set_popover(Some(&stats_popover));
        session_header.append(&stats_button);

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

        let view_model = gtk4::StringList::new(&["Concise", "Standard", "Detailed", "Expanded"]);
        let view_dropdown = gtk4::DropDown::builder()
            .model(&view_model)
            .selected(1)
            .tooltip_text("Transcript detail level")
            .build();
        session_header.append(&view_dropdown);
        let on_view_mode: Rc<RefCell<Option<Box<dyn Fn(TranscriptView)>>>> =
            Rc::new(RefCell::new(None));
        let view_callback = on_view_mode.clone();
        view_dropdown.connect_selected_notify(move |dropdown| {
            let mode = match dropdown.selected() {
                0 => TranscriptView::Concise,
                2 => TranscriptView::Detailed,
                3 => TranscriptView::Expanded,
                _ => TranscriptView::Standard,
            };
            if let Some(callback) = view_callback.borrow().as_ref() {
                callback(mode);
            }
        });

        let status_label = gtk4::Label::builder()
            .label("Idle")
            .css_classes(["dim-label", "caption"])
            .build();
        session_header.append(&status_label);

        let running_timer = gtk4::Label::builder()
            .label("")
            .visible(false)
            .css_classes(["dim-label", "caption", "monospace"])
            .build();
        session_header.append(&running_timer);

        let dock_button = gtk4::Button::builder()
            .icon_name("sidebar-show-right-symbolic")
            .tooltip_text(Copybook::for_code("system").open_right_sidebar)
            .action_name("win.toggle-dock")
            .css_classes(["dsh-icon-button"])
            .visible(false)
            .build();
        session_header.append(&dock_button);

        conversation_box.append(&session_header);

        let view_stack = adw::ViewStack::new();

        let chat_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();

        let transcript_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_top(16)
            .margin_bottom(16)
            .build();

        let welcome_bubble = crate::ui::hero::build_hero();
        let hero_composer_slot = welcome_bubble.composer_slot.clone();
        let hero_workspace_menu = welcome_bubble.workspace_menu.clone();
        let preset_menu = welcome_bubble.preset_menu.clone();
        let preset_label = welcome_bubble.preset_label.clone();
        let welcome_bubble = welcome_bubble.container;
        transcript_box.append(&welcome_bubble);

        let transcript_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();
        transcript_scroll.set_child(Some(&transcript_box));

        let back_to_bottom = gtk4::Button::builder()
            .icon_name("go-bottom-symbolic")
            .tooltip_text("Back to bottom")
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::End)
            .margin_end(18)
            .margin_bottom(18)
            .css_classes(["circular", "osd", "dsh-scroll-bottom"])
            .visible(false)
            .build();

        let turn_rail = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(3)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::Center)
            .margin_end(8)
            .css_classes(["dsh-turn-rail"])
            .visible(false)
            .build();

        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&transcript_scroll));
        overlay.set_vexpand(true);
        overlay.add_overlay(&back_to_bottom);
        overlay.add_overlay(&turn_rail);
        chat_box.append(&overlay);

        let scroll_follow = Rc::new(Cell::new(ScrollFollow::new(true)));
        let turn_widgets: Rc<RefCell<Vec<(u32, gtk4::Widget)>>> =
            Rc::new(RefCell::new(Vec::new()));
        let rail_buttons: Rc<RefCell<Vec<(u32, gtk4::Button)>>> =
            Rc::new(RefCell::new(Vec::new()));

        let load_slot_for_scroll: Rc<RefCell<Option<Box<dyn Fn()>>>> =
            Rc::new(RefCell::new(None));

        let back_click_adjustment = transcript_scroll.vadjustment();
        let back_click_follow = scroll_follow.clone();
        back_to_bottom.connect_clicked(move |_| {
            back_click_follow.set(ScrollFollow::new(true));
            back_click_adjustment.set_value(
                (back_click_adjustment.upper() - back_click_adjustment.page_size()).max(0.0),
            );
        });

        let adjustment = transcript_scroll.vadjustment();
        let follow_for_scroll = scroll_follow.clone();
        let button_for_scroll = back_to_bottom.clone();
        let box_for_scroll = transcript_box.clone();
        let widgets_for_scroll = turn_widgets.clone();
        let buttons_for_scroll = rail_buttons.clone();
        let load_slot_clone = load_slot_for_scroll.clone();
        adjustment.connect_value_changed(move |adj| {
            let mut follow = follow_for_scroll.get();
            follow.sample(adj.value(), adj.upper(), adj.page_size());
            follow_for_scroll.set(follow);
            button_for_scroll.set_visible(!follow.following());
            update_active_turn(
                adj,
                &box_for_scroll,
                &widgets_for_scroll,
                &buttons_for_scroll,
            );
            if adj.value() <= 8.0 {
                if let Some(handler) = load_slot_clone.borrow().as_ref() {
                    handler();
                }
            }
        });

        let follow_for_changed = scroll_follow.clone();
        let adjustment_for_changed = transcript_scroll.vadjustment();
        adjustment.connect_changed(move |_| {
            if !follow_for_changed.get().following() {
                return;
            }
            let adjust = adjustment_for_changed.clone();
            let follow = follow_for_changed.clone();
            gtk4::glib::idle_add_local_once(move || {
                if follow.get().following() {
                    adjust.set_value((adjust.upper() - adjust.page_size()).max(0.0));
                }
            });
        });

        let chat_page = view_stack.add_titled(&chat_box, Some("chat"), "Chat");
        chat_page.set_icon_name(Some("chat-bubbles-symbolic"));

        let trajectory_view = TrajectoryView::new();
        let traj_page = view_stack.add_titled(
            trajectory_view.widget(),
            Some("trajectory"),
            "Trajectory",
        );
        traj_page.set_icon_name(Some("view-list-tree-symbolic"));

        let view_switcher = adw::ViewSwitcher::builder()
            .stack(&view_stack)
            .policy(adw::ViewSwitcherPolicy::Wide)
            .halign(gtk4::Align::Center)
            .build();

        let on_view_selected: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> =
            Rc::new(RefCell::new(None));
        let on_view_selected_notify = on_view_selected.clone();
        view_stack.connect_visible_child_name_notify(move |stack| {
            if let Some(name) = stack.visible_child_name() {
                if let Some(callback) = on_view_selected_notify.borrow().as_ref() {
                    callback(name.as_str());
                }
            }
        });

        let switcher_bar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::Center)
            .margin_bottom(4)
            .visible(false)
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
        let composer_dock = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .css_classes(["dsh-composer-shell"])
            .build();
        let takeover_slot = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .visible(false)
            .build();
        composer_dock.prepend(&takeover_slot);
        conversation_box.append(&composer_dock);
        hero_composer_slot.append(composer.widget());

        stack.add_named(&conversation_box, Some("conversation"));

        let plugins_page = PluginsPage::new();
        stack.add_named(plugins_page.widget(), Some("plugins"));

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
            running_timer,
            stats_button,
            stats_list,
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
            scroll_follow,
            back_to_bottom,
            turn_rail,
            turn_markers: RefCell::new(Vec::new()),
            turn_widgets,
            rail_buttons,
            block_handles: RefCell::new(HashMap::new()),
            fork_handle: RefCell::new(None),
            schedules_page,
            plugins_page,
            trajectory_page: traj_page,
            trajectory_view,
            view_mode: Cell::new(TranscriptView::Standard),
            on_view_mode,
            view_dropdown,
            view_stack,
            on_view_selected,
            rendered_count: Cell::new(0),
            active_cwd,
            title_activate,
            hero_composer_slot,
            hero_workspace_menu,
            composer_dock,
            takeover_slot,
            session_header,
            dock_button,
            switcher_bar,
            preset_menu,
            preset_label,
            open_with_apps,
            open_with_choice,
            open_with_button,
            on_open_with_choice,
        }
    }

    pub fn widget(&self) -> &gtk4::Stack {
        &self.stack
    }

    pub fn set_on_view_mode<F: Fn(TranscriptView) + 'static>(&self, callback: F) {
        *self.on_view_mode.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_view_mode(&self, mode: TranscriptView) {
        self.view_mode.set(mode);
        let index = match mode {
            TranscriptView::Concise => 0,
            TranscriptView::Standard => 1,
            TranscriptView::Detailed => 2,
            TranscriptView::Expanded => 3,
        };
        if self.view_dropdown.selected() != index {
            self.view_dropdown.set_selected(index);
        }
    }

    pub fn set_on_view_selected<F: Fn(&str) + 'static>(&self, callback: F) {
        *self.on_view_selected.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_active_view(&self, name: &str) {
        let name = if name == "trajectory" && !self.trajectory_page.is_visible() {
            "chat"
        } else {
            name
        };
        if self.view_stack.visible_child_name().as_deref() != Some(name) {
            self.view_stack.set_visible_child_name(name);
        }
    }

    pub fn trajectory_view(&self) -> &TrajectoryView {
        &self.trajectory_view
    }

    pub fn set_trajectory_visible(&self, visible: bool) {
        self.trajectory_page.set_visible(visible);
    }

    pub fn schedules_page(&self) -> &SchedulesPage {
        &self.schedules_page
    }

    pub fn plugins_page(&self) -> &PluginsPage {
        &self.plugins_page
    }

    pub fn composer(&self) -> &Composer {
        &self.composer
    }

    pub fn header_dock_button(&self) -> &gtk4::Button {
        &self.dock_button
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

    pub fn set_drawer_opener_visible(&self, visible: bool) {
        self.session_header
            .set_margin_start(if visible { 48 } else { 12 });
    }

    pub fn set_status(&self, status: &str) {
        self.status_label.set_label(match status {
            "Running" => "运行中",
            "Idle" => "就绪",
            other => other,
        });
        self.composer.set_is_running(status == "Running");
        if status != "Running" {
            self.running_timer.set_visible(false);
        }
    }

    pub fn set_disconnected(&self) {
        let current = self.status_label.label().to_string();
        if current == "运行中" || current == "运行中（已暂停）" {
            self.status_label.set_label("运行中（已暂停）");
        } else {
            self.status_label.set_label("已断开连接");
        }
        self.running_timer.set_visible(false);
    }

    pub fn set_running_elapsed(&self, seconds: u64) {
        let text = if seconds < 60 {
            format!("{}s", seconds)
        } else {
            format!("{}m{:02}s", seconds / 60, seconds % 60)
        };
        self.running_timer.set_label(&text);
        self.running_timer.set_visible(true);
    }

    pub fn stats_button(&self) -> &gtk4::MenuButton {
        &self.stats_button
    }

    pub fn set_turn_stats(&self, stats: &[TurnStat]) {
        while let Some(child) = self.stats_list.first_child() {
            self.stats_list.remove(&child);
        }
        self.stats_button.set_visible(!stats.is_empty());
        if stats.is_empty() {
            return;
        }
        for stat in stats.iter().rev() {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .css_classes(["card"])
                .margin_top(2)
                .margin_bottom(2)
                .build();
            let title = gtk4::Label::builder()
                .label(&format!(
                    "Turn {} · {:.1} tok/s · {:.1}s",
                    stat.turn,
                    stat.tokens_per_sec,
                    stat.elapsed_ms as f64 / 1000.0
                ))
                .xalign(0.0)
                .css_classes(["heading", "caption"])
                .build();
            row.append(&title);
            let detail = gtk4::Label::builder()
                .label(&format!(
                    "in {} · out {} · reasoning {} · cache {}",
                    compact_token_count(stat.input_tokens),
                    compact_token_count(stat.output_tokens),
                    compact_token_count(stat.reasoning_tokens),
                    compact_token_count(stat.cache_read_tokens)
                ))
                .xalign(0.0)
                .wrap(true)
                .css_classes(["dim-label", "caption"])
                .build();
            row.append(&detail);
            self.stats_list.append(&row);
        }
    }

    pub fn set_cwd(&self, cwd: &str) {
        *self.active_cwd.borrow_mut() = PathBuf::from(cwd);
    }

    pub fn set_on_open_with_choice<F: Fn(&str) + 'static>(&self, callback: F) {
        *self.on_open_with_choice.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_open_with_choice(&self, id: Option<&str>) {
        let app = id.and_then(|id| {
            self.open_with_apps
                .iter()
                .copied()
                .find(|app| app.id == id)
        });
        let app = app.or_else(|| crate::ui::open_with::default_app(&self.open_with_apps));
        *self.open_with_choice.borrow_mut() = app;
        apply_open_with_label(&self.open_with_button, app);
    }

    pub fn goal_bar(&self) -> &GoalBar {
        &self.goal_bar
    }

    pub fn update_queue(&self, session_id: &str, items: &[QueueItem]) {
        self.queue_dock.update(session_id, items);
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

    pub fn set_hero_workspaces(&self, workspaces: &[WorkspaceView]) {
        let items = gtk4::gio::Menu::new();
        for workspace in workspaces {
            let item = gtk4::gio::MenuItem::new(Some(&workspace.title), None);
            item.set_action_and_target_value(
                Some("win.new-session-in-workspace"),
                Some(&workspace.id.to_variant()),
            );
            items.append_item(&item);
        }
        let choose = gtk4::gio::MenuItem::new(
            Some(&Copybook::for_code("system").choose_workspace),
            Some("win.new-workspace"),
        );
        items.append_item(&choose);
        self.hero_workspace_menu.set_menu_model(Some(&items));
    }

    pub fn scroll_transcript_to_bottom(&self) {
        let adjustment = self.transcript_scroll.vadjustment();
        gtk4::glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
            adjustment.set_value((adjustment.upper() - adjustment.page_size()).max(0.0));
        });
    }

    pub fn reset_scroll_follow(&self) {
        self.scroll_follow.set(ScrollFollow::new(true));
        self.back_to_bottom.set_visible(false);
        self.scroll_transcript_to_bottom();
    }

    pub fn rerender_transcript(&self, transcript: &ConversationTranscript) {
        while let Some(child) = self.transcript_box.first_child() {
            self.transcript_box.remove(&child);
        }
        self.live_handle.borrow_mut().take();
        self.block_handles.borrow_mut().clear();
        *self.fork_handle.borrow_mut() = None;
        self.turn_widgets.borrow_mut().clear();
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
        match token_usage_total(usage) {
            Some(total) if total > 0 => {
                self.usage_label
                    .set_label(&format!("{} tok", compact_token_count(total)));
                self.usage_label.set_visible(true);
            }
            _ => {
                self.usage_label.set_label("");
                self.usage_label.set_visible(false);
            }
        }
    }

    pub fn set_context(
        &self,
        pressure: Option<&serde_json::Value>,
        breakdown: Option<&serde_json::Value>,
    ) {
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
        let Some((used, window, percent)) = context_occupancy(&pressure) else {
            self.context_button.set_visible(false);
            return;
        };
        self.context_button
            .set_label(&format!("Context {}%", percent));
        self.context_button.set_visible(true);
        let occupancy = format!(
            "已用 {} / {} tokens（{}%）",
            compact_token_count(used),
            compact_token_count(window),
            percent
        );
        let rows = self
            .context_breakdown
            .borrow()
            .as_ref()
            .map(context_breakdown_rows)
            .unwrap_or_default();
        let details = if rows.is_empty() {
            occupancy
        } else {
            let mut lines = vec![occupancy];
            let mut total = 0u64;
            for (label, value) in rows {
                total = total.saturating_add(value);
                lines.push(format!("{}：{}", label, compact_token_count(value)));
            }
            lines.push(format!("合计：{}", compact_token_count(total)));
            lines.join("\n")
        };
        self.context_label.set_label(&details);
    }

    pub fn reset_meters(&self) {
        *self.context_pressure.borrow_mut() = None;
        *self.context_breakdown.borrow_mut() = None;
        self.usage_label.set_label("");
        self.usage_label.set_visible(false);
        self.context_button.set_label("Context");
        self.context_button.set_visible(false);
        self.context_label.set_label("");
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

    fn render_grouped(&self, blocks: &[RenderBlock]) {
        while let Some(child) = self.transcript_box.first_child() {
            self.transcript_box.remove(&child);
        }
        self.live_handle.borrow_mut().take();
        self.block_handles.borrow_mut().clear();
        self.turn_widgets.borrow_mut().clear();
        *self.fork_handle.borrow_mut() = None;
        self.rendered_count.set(0);
        let policy = PresentationPolicy::for_mode(self.view_mode.get());
        let last_turn_start = blocks
            .iter()
            .rposition(|block| matches!(block, RenderBlock::UserMessage { .. }))
            .unwrap_or(0);
        let markers = turn_markers(blocks);
        let mut group: Option<(gtk4::Expander, gtk4::Box, usize, bool)> = None;
        for (index, block) in blocks.iter().enumerate() {
            let intermediate = matches!(
                block,
                RenderBlock::ToolResult { info, .. } if !info.is_error
            ) || matches!(
                block,
                RenderBlock::GenericEvent { .. }
                    | RenderBlock::TodoBlock { .. }
            );
            let in_live_turn = index >= last_turn_start;
            let grouped = policy.groups_intermediate()
                && (policy.step_grouping != StepGrouping::History || !in_live_turn);
            if intermediate && grouped {
                if group.is_none() {
                    let inner = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Vertical)
                        .spacing(6)
                        .build();
                    let expander = gtk4::Expander::builder()
                        .label("Intermediate steps")
                        .child(&inner)
                        .expanded(false)
                        .build();
                    self.transcript_box.append(&expander);
                    group = Some((expander, inner, 0, in_live_turn));
                }
                if let Some((expander, inner, count, live_group)) = group.as_mut() {
                    let (widget, handle) = render_block(block, policy);
                    if let Some(handle) = handle {
                        self.block_handles
                            .borrow_mut()
                            .insert(index, handle.clone());
                        *self.live_handle.borrow_mut() = Some(handle);
                    }
                    inner.append(&widget);
                    *count += 1;
                    let mut label = format!(
                        "{} intermediate step{}",
                        *count,
                        if *count == 1 { "" } else { "s" }
                    );
                    if policy.live_process_detail && *live_group {
                        if let Some(detail) = intermediate_detail(block) {
                            label.push_str(" · ");
                            label.push_str(&detail);
                        }
                    }
                    expander.set_label(Some(&label));
                }
            } else {
                group = None;
                let (widget, handle) = render_block(block, policy);
                self.transcript_box.append(&widget);
                if let Some(handle) = handle {
                    self.block_handles
                        .borrow_mut()
                        .insert(index, handle.clone());
                    *self.live_handle.borrow_mut() = Some(handle);
                }
                if let Some(marker) = markers.iter().find(|marker| marker.block_index == index) {
                    self.turn_widgets
                        .borrow_mut()
                        .push((marker.turn, widget.clone().upcast()));
                }
            }
        }
        self.rendered_count.set(blocks.len());
        if let Some(last_block) = blocks.last() {
            if let RenderBlock::AssistantMessage {
                text,
                reasoning,
                is_live,
                seq,
                message_id,
                time,
                ..
            } = last_block
            {
                if let Some(handle) = self.live_handle.borrow().clone() {
                    if *is_live {
                        handle.update_text(text);
                        if let Some(reasoning) = reasoning {
                            handle.update_reasoning(reasoning);
                        }
                    } else {
                        handle.finalize(*seq, message_id.as_deref(), *time);
                    }
                }
            }
        }
        self.refresh_turn_rail(blocks);
        self.refresh_fork_target(blocks);
    }

    fn refresh_turn_rail(&self, blocks: &[RenderBlock]) {
        let markers = turn_markers(blocks);
        if *self.turn_markers.borrow() == markers {
            return;
        }
        *self.turn_markers.borrow_mut() = markers.clone();
        while let Some(child) = self.turn_rail.first_child() {
            self.turn_rail.remove(&child);
        }
        let mut buttons = self.rail_buttons.borrow_mut();
        buttons.clear();
        for marker in &markers {
            let tooltip = if marker.preview.is_empty() {
                format!("Turn {}", marker.turn)
            } else {
                format!("Turn {} · {}", marker.turn, marker.preview)
            };
            let button = gtk4::Button::builder()
                .tooltip_text(&tooltip)
                .css_classes(["dsh-turn-mark"])
                .build();
            let turn = marker.turn;
            let box_for_click = self.transcript_box.clone();
            let adjustment_for_click = self.transcript_scroll.vadjustment();
            let widgets_for_click = self.turn_widgets.clone();
            button.connect_clicked(move |_| {
                jump_to_turn(
                    turn,
                    &box_for_click,
                    &adjustment_for_click,
                    &widgets_for_click,
                );
            });
            self.turn_rail.append(&button);
            buttons.push((marker.turn, button));
        }
        self.turn_rail.set_visible(markers.len() >= 2);
        if let Some(last) = markers.last() {
            if let Some((_, button)) = buttons.iter().find(|(turn, _)| *turn == last.turn) {
                button.add_css_class("dsh-turn-mark-active");
            }
        }
    }

    fn refresh_fork_target(&self, blocks: &[RenderBlock]) {
        let target = fork_target_index(blocks);
        let mut slot = self.fork_handle.borrow_mut();
        if slot.as_ref().map(|(index, _)| *index) == target {
            return;
        }
        if let Some((_, handle)) = slot.take() {
            handle.set_fork_visible(false);
        }
        if let Some(index) = target {
            if let Some(handle) = self.block_handles.borrow().get(&index) {
                handle.set_fork_visible(true);
                *slot = Some((index, handle.clone()));
            }
        }
    }

    fn clear_turn_state(&self) {
        *self.turn_markers.borrow_mut() = Vec::new();
        self.turn_widgets.borrow_mut().clear();
        while let Some(child) = self.turn_rail.first_child() {
            self.turn_rail.remove(&child);
        }
        self.rail_buttons.borrow_mut().clear();
        self.turn_rail.set_visible(false);
        self.block_handles.borrow_mut().clear();
        *self.fork_handle.borrow_mut() = None;
    }

    pub fn set_hero(&self, hero: bool) {
        self.session_header.set_visible(!hero);
        self.switcher_bar.set_visible(!hero);
        if hero {
            if self.welcome_bubble.parent().is_none() {
                self.transcript_box.append(&self.welcome_bubble);
            }
            self.welcome_bubble.set_visible(true);
            if let Some(parent) = self.composer.widget().parent() {
                if let Some(box_parent) = parent.downcast_ref::<gtk4::Box>() {
                    box_parent.remove(self.composer.widget());
                }
            }
            self.hero_composer_slot.append(self.composer.widget());
        } else {
            self.welcome_bubble.set_visible(false);
            if let Some(parent) = self.composer.widget().parent() {
                if let Some(box_parent) = parent.downcast_ref::<gtk4::Box>() {
                    box_parent.remove(self.composer.widget());
                }
            }
            self.composer_dock.append(self.composer.widget());
        }
        if self.takeover_slot.is_visible() {
            self.composer.widget().set_visible(false);
        }
    }

    pub fn set_takeover(&self, widget: Option<gtk4::Widget>) {
        while let Some(child) = self.takeover_slot.first_child() {
            self.takeover_slot.remove(&child);
        }
        match widget {
            Some(widget) => {
                self.takeover_slot.append(&widget);
                self.takeover_slot.set_visible(true);
                self.composer.widget().set_visible(false);
            }
            None => {
                self.takeover_slot.set_visible(false);
                self.composer.widget().set_visible(true);
            }
        }
    }

    pub fn set_preset_label(&self, label: &str) {
        self.preset_label.set_label(label);
        self.preset_menu.set_tooltip_text(Some(label));
    }

    pub fn clear_session(&self) {
        while let Some(child) = self.transcript_box.first_child() {
            self.transcript_box.remove(&child);
        }
        self.transcript_box.append(&self.welcome_bubble);
        self.live_handle.borrow_mut().take();
        self.rendered_count.set(0);
        self.clear_turn_state();
        self.scroll_follow.set(ScrollFollow::new(true));
        self.back_to_bottom.set_visible(false);
        self.set_hero(true);
        self.set_title(Copybook::for_code("system").new_session);
        self.set_status("Idle");
        self.queue_dock.update("", &[]);
        self.reset_meters();
    }

    pub fn select_panel(&self, panel_name: &str) {
        self.stack.set_visible_child_name(panel_name);
    }

    pub fn update_transcript(&self, transcript: &ConversationTranscript) {
        let blocks = transcript.blocks();
        let policy = PresentationPolicy::for_mode(self.view_mode.get());
        self.set_hero(blocks.is_empty());
        if policy.groups_intermediate() && blocks.len() > self.rendered_count.get() {
            self.render_grouped(blocks);
        }
        if blocks.is_empty() {
            while let Some(child) = self.transcript_box.first_child() {
                self.transcript_box.remove(&child);
            }
            self.live_handle.borrow_mut().take();
            self.rendered_count.set(0);
            self.clear_turn_state();
            self.set_hero(true);
            return;
        }

        if self.rendered_count.get() == 0 {
            while let Some(child) = self.transcript_box.first_child() {
                self.transcript_box.remove(&child);
            }
            self.turn_widgets.borrow_mut().clear();
            self.block_handles.borrow_mut().clear();
            *self.fork_handle.borrow_mut() = None;
        }

        let current_count = self.rendered_count.get();
        if blocks.len() > current_count {
            let markers = turn_markers(blocks);
            for (index, block) in blocks.iter().enumerate().skip(current_count) {
                let (widget, handle) = render_block(block, policy);
                self.transcript_box.append(&widget);
                if let Some(handle) = handle {
                    self.block_handles
                        .borrow_mut()
                        .insert(index, handle.clone());
                    *self.live_handle.borrow_mut() = Some(handle);
                }
                if let Some(marker) = markers.iter().find(|marker| marker.block_index == index) {
                    self.turn_widgets
                        .borrow_mut()
                        .push((marker.turn, widget.clone().upcast()));
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
                    seq,
                    message_id,
                    time,
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
                            handle.finalize(*seq, message_id.as_deref(), *time);
                            *self.live_handle.borrow_mut() = None;
                        }
                    }
                }
                _ => {
                    if let Some(handle) = self.live_handle.borrow_mut().take() {
                        handle.finalize(None, None, 0);
                    }
                }
            }
        }
        self.refresh_turn_rail(blocks);
        self.refresh_fork_target(blocks);
    }
}

impl Default for ConversationView {
    fn default() -> Self {
        Self::new()
    }
}
