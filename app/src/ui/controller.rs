use crate::connection::{
    ApprovalResponder, BridgeConnection, ConnectionError, IncomingNotification,
    UserQuestionResponder,
};
use crate::credentials::{CredentialStore, DEFAULT_PROVIDER, should_show_onboarding};
use crate::drafts::{DraftStore, default_drafts_path};
use crate::protocol::*;
use crate::schedule_model::{
    ScheduleDeliveryHistoryPage, SchedulePatch, ScheduleTargetRequest, ScheduleTiming,
};
use crate::session::{SessionModel, SessionStatus};
use crate::session_state::{SessionState, default_state_path};
use crate::supervisor::{HarnessSupervisor, SupervisorConfig, resolve_bridge_bundle_path};
use crate::ui::conversation::ConversationView;
use crate::ui::copy::Copybook;
use crate::ui::desktop::{
    send_turn_completed_notification, should_notify_turn_completion, truncate_snippet,
    withdraw_notification,
};
use crate::ui::onboarding::show_onboarding_dialog;
use crate::ui::right_dock::RightDock;
use crate::ui::schedules::model::{
    HISTORY_PAGE_LIMIT, ScheduleFailure, ScheduleHistoryMiss, ScheduleMutationOp,
    clear_ended_toast, copy as schedule_copy, create_prompt, delete_toast, failure_from_error,
    failure_from_result, failure_text, history_miss, run_failure,
};
use crate::ui::schedules::ui_locale;
use crate::ui::session_dialogs::{show_fork_dialog, show_rename_dialog};
use crate::ui::settings::{SettingsDialogHandle, show_settings_dialog};
use crate::ui::sidebar::{PanelState, Sidebar};
use crate::ui::trigger_popup::{TriggerOption, show_trigger_popup};
use crate::ui::window::MainWindow;
use crate::ui_prefs::{ArchiveFilter, SessionGrouping, SessionSort, UiPrefs, default_ui_prefs_path};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::{RwLock, oneshot};

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime")
});

pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
}

pub fn resolve_new_session_target<'a>(
    workspace_id: Option<&str>,
    active_session: Option<&str>,
    workspaces: &'a [WorkspaceView],
    sessions: &'a [SessionListItem],
) -> Option<&'a WorkspaceView> {
    if let Some(id) = workspace_id {
        return workspaces.iter().find(|workspace| workspace.id == id);
    }
    if let Some(session_id) = active_session {
        if let Some(host) = workspaces.iter().find(|workspace| {
            workspace
                .session_ids
                .iter()
                .any(|member| member == session_id)
        }) {
            return Some(host);
        }
    }
    most_recent_workspace(workspaces, sessions)
}

fn most_recent_workspace<'a>(
    workspaces: &'a [WorkspaceView],
    sessions: &'a [SessionListItem],
) -> Option<&'a WorkspaceView> {
    let mut best: Option<(&WorkspaceView, u64)> = None;
    for workspace in workspaces {
        let latest = workspace
            .session_ids
            .iter()
            .filter_map(|id| sessions.iter().find(|item| &item.id == id))
            .map(|item| item.updated_at)
            .max()
            .unwrap_or(0);
        if best.map_or(true, |(_, top)| latest > top) {
            best = Some((workspace, latest));
        }
    }
    best.map(|(workspace, _)| workspace)
}

pub fn find_reusable_blank<'a>(
    workspace: &WorkspaceView,
    sessions: &'a [SessionListItem],
    preset_compatible: impl Fn(&SessionListItem) -> bool,
) -> Option<&'a SessionListItem> {
    sessions.iter().find(|item| {
        item.turn_count == 0
            && !item.is_archived
            && item.cwd == workspace.path
            && workspace.session_ids.iter().any(|id| id == &item.id)
            && preset_compatible(item)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDraft {
    pub prompt: String,
    pub cwd: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftDisposition {
    Consume,
    Clear,
}

pub fn resolve_pending_draft(
    draft: &PendingDraft,
    followed_id: &str,
    followed_cwd: Option<&str>,
) -> DraftDisposition {
    if draft.session_id.as_deref() == Some(followed_id) {
        return DraftDisposition::Consume;
    }
    if draft.session_id.is_none() && draft.cwd.as_deref() == followed_cwd {
        return DraftDisposition::Consume;
    }
    DraftDisposition::Clear
}

pub enum UiMessage {
    Approval {
        req: ApprovalRequestParams,
        tx: oneshot::Sender<ApprovalOutcome>,
    },
    UserQuestion {
        req: SdkUserQuestionRequest,
        tx: oneshot::Sender<SdkUserQuestionResult>,
    },
    SessionListLoaded(Vec<SessionListItem>),
    WorkspacesLoaded(Vec<WorkspaceView>),
    WorkspaceCreated(String),
    WorkspaceChanged,
    SessionFollowed {
        follow: SessionFollowResult,
        generation: u64,
    },
    Notification(IncomingNotification),
    Toast(String),
    ShowOnboarding,
    PluginsInventoryLoaded(Result<PluginInventoryResult, String>),
    PluginsToggleDone {
        key: String,
        result: Result<PluginChangeResult, String>,
    },
    PluginsRemoveDone {
        name: String,
        result: Result<PluginChangeResult, String>,
    },
    PluginInspectDone(Result<PluginSpecInspection, String>),
    PluginInstallDone {
        request_id: Option<String>,
        result: Result<PluginChangeResult, String>,
    },
    PluginCancelInstallDone(Result<PluginCancellationResult, String>),
    PluginWaitInstallDone(Result<Option<PluginChangeResult>, String>),
    PluginRegistriesLoaded(Result<PluginRegistries, String>),
    PluginRegistryProbeDone(Result<Option<String>, String>),
    PluginVersionsLoaded {
        name: String,
        result: Result<PluginVersionList, String>,
    },
    PluginUpdateDone(Result<PluginUpdateResult, String>),
    SupervisorRestarted {
        connection: BridgeConnection,
        init: Option<InitializeResult>,
    },
    SupervisorStarted {
        supervisor: Arc<HarnessSupervisor>,
        connection: BridgeConnection,
        init: Option<InitializeResult>,
    },
    SupervisorStartFailed(String),
    ConnectionLost,
    SessionReleased,
    SessionForked(String),
    SessionRenamed {
        session_id: String,
        title: String,
    },
    QueueUpdated {
        session_id: String,
        items: Vec<QueueItem>,
    },
    CommandsLoaded(Vec<CommandDescriptor>),
    ReferencesLoaded(Vec<ReferenceCandidate>),
    SkillsLoaded(Vec<SkillSummaryItem>),
    FilesListed {
        session_id: String,
        path: String,
        workdir: String,
        entries: Vec<FileEntry>,
    },
    FileStat {
        session_id: String,
        stat: FilesStatResult,
    },
    FilePreviewText {
        session_id: String,
        path: String,
        text: String,
        truncated: bool,
    },
    FilePreviewImage {
        session_id: String,
        path: String,
        bytes: Vec<u8>,
    },
    FilePreviewUnavailable {
        session_id: String,
        message: String,
    },
    PermissionLoaded(PermissionGetResult),
    PermissionCatalogLoaded(Result<PermissionCatalogResult, String>),
    PermissionValueUpdated {
        session_id: String,
        current_value: String,
    },
    EchoAck {
        session_id: String,
    },
    EchoFailed {
        session_id: String,
        text: String,
        error: String,
    },
    TerminalListed {
        session_id: String,
        terminals: Vec<TerminalInfo>,
    },
    TerminalCreated {
        session_id: String,
        created: TerminalCreateResult,
    },
    TerminalOutput {
        session_id: String,
        output: TerminalOutputResult,
    },
    ScheduleCatalogLoaded(Vec<crate::schedule_model::ScheduleCatalogEntry>),
    ScheduleCatalogFailed,
    ScheduleHistoryLoaded {
        id: String,
        page: Result<ScheduleDeliveryHistoryPage, ScheduleHistoryMiss>,
    },
    ScheduleMutationDone {
        message: String,
        created: Option<crate::schedule_model::ScheduleCatalogEntry>,
    },
    ScheduleMutationFailed {
        op: ScheduleMutationOp,
        failure: ScheduleFailure,
    },
    ScheduleLimitsLoaded(Option<i64>),
    HistoryPageLoaded {
        session_id: String,
        events: Vec<SessionEvent>,
        has_more: bool,
    },
    SubagentsLoaded(Vec<serde_json::Value>),
    JobsLoaded(Vec<JobWireView>),
    JobFollowLoaded(JobFollowResult),
    FeedbackLoaded {
        session_id: String,
        items: Vec<MessageFeedbackItem>,
    },
    AccountLoaded {
        status: Result<AccountStatusResult, String>,
        profile: Result<AccountProfileResult, String>,
        balance: Result<AccountBalanceResult, String>,
    },
    CredentialRecordsLoaded(Vec<CredentialRecordItem>),
    QuestionReopened(SdkUserQuestionRequest),
    ImageAttached {
        name: String,
        mime_type: String,
        data: String,
    },
    FileAttached(String),
    SessionSearchLoaded(Vec<SessionSearchItem>),
    ProjectionsLoaded {
        session_id: String,
        projections: SessionProjectionsResult,
    },
    SessionModelLoaded {
        session_id: String,
        model: SessionModelGetResult,
    },
    ModelSwitchFailed {
        provider: String,
        model: String,
    },
    ModelCatalogLoaded(ModelCatalogResult),
    GoalLoaded(Option<GoalView>),
    SessionListRefresh,
    RefreshModelCatalog,
    SessionWindowLoaded {
        session_id: String,
        events: Vec<SessionEvent>,
    },
    GapRepairLoaded {
        session_id: String,
        events: Vec<SessionEvent>,
    },
    GapRepairFailed {
        session_id: String,
        error: String,
    },
    PendingDraftExpired,
}

#[derive(Clone)]
struct PendingImage {
    name: String,
    mime_type: String,
    data: String,
}

struct GapRepairState {
    session_id: String,
    to_seq: i64,
    buffered: Vec<SessionEvent>,
}

enum PendingInteraction {
    Approval {
        key: u64,
        req: ApprovalRequestParams,
        responder: Option<oneshot::Sender<ApprovalOutcome>>,
    },
    Question {
        key: u64,
        req: SdkUserQuestionRequest,
        responder: Option<oneshot::Sender<SdkUserQuestionResult>>,
    },
}

impl PendingInteraction {
    fn key(&self) -> u64 {
        match self {
            Self::Approval { key, .. } | Self::Question { key, .. } => *key,
        }
    }

    fn display(&self) -> DisplayInteraction {
        match self {
            Self::Approval { key, req, .. } => DisplayInteraction::Approval {
                key: *key,
                req: req.clone(),
            },
            Self::Question { key, req, responder } => DisplayInteraction::Question {
                key: *key,
                req: req.clone(),
                answerable: responder.is_some(),
            },
        }
    }
}

enum DisplayInteraction {
    Approval {
        key: u64,
        req: ApprovalRequestParams,
    },
    Question {
        key: u64,
        req: SdkUserQuestionRequest,
        answerable: bool,
    },
}

enum InteractionResolution {
    Approval(ApprovalOutcome),
    Question(SdkUserQuestionResult),
}

struct ApprovalHandler {
    ui_sender: UnboundedSender<UiMessage>,
}

impl ApprovalResponder for ApprovalHandler {
    fn respond(
        &self,
        req: ApprovalRequestParams,
    ) -> Pin<Box<dyn Future<Output = ApprovalOutcome> + Send>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.ui_sender.send(UiMessage::Approval { req, tx });
        Box::pin(async move { rx.await.unwrap_or(ApprovalOutcome::Rejected) })
    }
}

struct QuestionHandler {
    ui_sender: UnboundedSender<UiMessage>,
}

impl UserQuestionResponder for QuestionHandler {
    fn respond(
        &self,
        req: SdkUserQuestionRequest,
    ) -> Pin<Box<dyn Future<Output = SdkUserQuestionResult> + Send>> {
        let (tx, rx) = oneshot::channel();
        let _ = self.ui_sender.send(UiMessage::UserQuestion { req, tx });
        Box::pin(async move {
            rx.await
                .unwrap_or_else(|_| SdkUserQuestionResult::cancelled())
        })
    }
}

pub struct SessionController {
    connection: Arc<RwLock<Option<BridgeConnection>>>,
    supervisor: Arc<RwLock<Option<Arc<HarnessSupervisor>>>>,
    active_session: Rc<RefCell<Option<SessionModel>>>,
    active_model: Rc<RefCell<String>>,
    active_provider: Rc<RefCell<String>>,
    model_explicit: Rc<Cell<bool>>,
    model_catalog: Rc<RefCell<Option<ModelCatalogResult>>>,
    active_preset: Rc<RefCell<String>>,
    window: gtk4::Window,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    right_dock: Rc<RightDock>,
    dock_button: gtk4::Button,
    panel_state: Rc<PanelState>,
    pending_draft: Rc<RefCell<Option<PendingDraft>>>,
    drafts: Rc<RefCell<DraftStore>>,
    drafts_path: PathBuf,
    workspaces: Rc<RefCell<Vec<WorkspaceView>>>,
    sessions_cache: Rc<RefCell<Vec<SessionListItem>>>,
    settings_dialog: Rc<RefCell<Option<SettingsDialogHandle>>>,
    permission_state: Rc<RefCell<Option<PermissionGetResult>>>,
    permission_catalog: Rc<RefCell<Option<PermissionCatalogResult>>>,
    permission_catalog_error: Rc<RefCell<Option<String>>>,
    permission_catalog_loading: Rc<Cell<bool>>,
    permission_popover: Rc<RefCell<Option<gtk4::Popover>>>,
    permission_anchor: Rc<RefCell<Option<gtk4::Widget>>>,
    history_loading: Rc<Cell<bool>>,
    follow_generation: Rc<Cell<u64>>,
    gap_repair: Rc<RefCell<Option<GapRepairState>>>,
    pending_images: Rc<RefCell<Vec<PendingImage>>>,
    pending_auto_send: Rc<RefCell<Option<String>>>,
    reconnect_active: Rc<Cell<bool>>,
    reconnect_generation: Rc<Cell<u64>>,
    init_result: Rc<RefCell<Option<InitializeResult>>>,
    has_supervisor: Rc<Cell<bool>>,
    pending_interactions: Rc<RefCell<HashMap<String, Vec<PendingInteraction>>>>,
    takeover_key: Rc<RefCell<Option<(String, u64)>>>,
    interaction_seq: Rc<Cell<u64>>,
    prefs: Rc<RefCell<UiPrefs>>,
    prefs_path: PathBuf,
    unread_sessions: Rc<RefCell<HashSet<String>>>,
    search_generation: Rc<Cell<u64>>,
    toast_overlay: adw::ToastOverlay,
    ui_sender: UnboundedSender<UiMessage>,
    credential_store: CredentialStore,
    startup_banner: adw::Banner,
    content_stack: gtk4::Stack,
    startup_page: adw::StatusPage,
    startup_spinner: gtk4::Spinner,
    session_state: Rc<RefCell<SessionState>>,
    state_path: PathBuf,
    restore_pending: Rc<Cell<bool>>,
    session_titles: Rc<RefCell<HashMap<String, String>>>,
    running_sessions: Rc<RefCell<HashSet<String>>>,
    turn_start_times: Rc<RefCell<HashMap<String, Instant>>>,
    background_hold: Rc<RefCell<Option<gtk4::gio::ApplicationHoldGuard>>>,
    background_session: Rc<RefCell<Option<String>>>,
    self_weak: RefCell<Weak<SessionController>>,
}
impl SessionController {
    pub fn new(
        main_window: &MainWindow,
        sidebar: Rc<Sidebar>,
        conversation_view: Rc<ConversationView>,
        right_dock: Rc<RightDock>,
    ) -> Rc<Self> {
        let (ui_sender, ui_receiver) = unbounded_channel();
        let credential_store = CredentialStore::new();
        let startup_banner = main_window.startup_banner().clone();
        let banner_for_retry = startup_banner.clone();
        let content_stack = main_window.content_stack().clone();
        let startup_page = main_window.startup_page().clone();
        let startup_spinner = main_window.startup_spinner().clone();
        let state_path = default_state_path();
        let session_state = Rc::new(RefCell::new(SessionState::load(&state_path)));

        let controller = Rc::new(Self {
            connection: Arc::new(RwLock::new(None)),
            supervisor: Arc::new(RwLock::new(None)),
            active_session: Rc::new(RefCell::new(None)),
            active_model: Rc::new(RefCell::new("deepseek-chat".to_string())),
            active_provider: Rc::new(RefCell::new("deepseek".to_string())),
            model_explicit: Rc::new(Cell::new(false)),
            model_catalog: Rc::new(RefCell::new(None)),
            active_preset: Rc::new(RefCell::new("ptc".to_string())),
            window: main_window.window().clone().upcast(),
            sidebar,
            conversation_view,
            right_dock: right_dock.clone(),
            dock_button: main_window.dock_button().clone(),
            panel_state: main_window.panel_state().clone(),
            pending_draft: Rc::new(RefCell::new(None)),
            drafts: {
                let path = default_drafts_path();
                Rc::new(RefCell::new(DraftStore::load(&path)))
            },
            drafts_path: default_drafts_path(),
            workspaces: Rc::new(RefCell::new(Vec::new())),
            sessions_cache: Rc::new(RefCell::new(Vec::new())),
            settings_dialog: Rc::new(RefCell::new(None)),
            permission_state: Rc::new(RefCell::new(None)),
            permission_catalog: Rc::new(RefCell::new(None)),
            permission_catalog_error: Rc::new(RefCell::new(None)),
            permission_catalog_loading: Rc::new(Cell::new(false)),
            permission_popover: Rc::new(RefCell::new(None)),
            permission_anchor: Rc::new(RefCell::new(None)),
            history_loading: Rc::new(Cell::new(false)),
            follow_generation: Rc::new(Cell::new(0)),
            gap_repair: Rc::new(RefCell::new(None)),
            pending_images: Rc::new(RefCell::new(Vec::new())),
            pending_auto_send: Rc::new(RefCell::new(None)),
            reconnect_active: Rc::new(Cell::new(false)),
            reconnect_generation: Rc::new(Cell::new(0)),
            init_result: Rc::new(RefCell::new(None)),
            has_supervisor: Rc::new(Cell::new(false)),
            pending_interactions: Rc::new(RefCell::new(HashMap::new())),
            takeover_key: Rc::new(RefCell::new(None)),
            interaction_seq: Rc::new(Cell::new(1)),
            prefs: {
                let path = default_ui_prefs_path();
                Rc::new(RefCell::new(UiPrefs::load(&path)))
            },
            prefs_path: default_ui_prefs_path(),
            unread_sessions: Rc::new(RefCell::new(HashSet::new())),
            search_generation: Rc::new(Cell::new(0)),
            toast_overlay: main_window
                .window()
                .content()
                .and_downcast::<adw::ToastOverlay>()
                .unwrap(),
            ui_sender,
            credential_store,
            startup_banner,
            content_stack,
            startup_page,
            startup_spinner,
            session_state,
            state_path,
            restore_pending: Rc::new(Cell::new(false)),
            session_titles: Rc::new(RefCell::new(HashMap::new())),
            running_sessions: Rc::new(RefCell::new(HashSet::new())),
            turn_start_times: Rc::new(RefCell::new(HashMap::new())),
            background_hold: Rc::new(RefCell::new(None)),
            background_session: Rc::new(RefCell::new(None)),
            self_weak: RefCell::new(Weak::new()),
        });

        *controller.self_weak.borrow_mut() = Rc::downgrade(&controller);

        controller.right_dock.set_controller(controller.clone());
        controller
            .conversation_view
            .schedules_page()
            .set_controller(controller.clone());
        controller
            .conversation_view
            .plugins_page()
            .set_controller(controller.clone());
        controller.setup_ui_receiver(ui_receiver, Rc::downgrade(&controller));

        {
            let prefs = controller.prefs.borrow();
            controller
                .sidebar
                .set_session_options(prefs.session_grouping, prefs.session_sort);
        }

        {
            let weak = Rc::downgrade(&controller);
            let buffer = controller.conversation_view.composer().text_view().buffer();
            buffer.connect_changed(move |_| {
                if let Some(ctrl) = weak.upgrade() {
                    ctrl.update_continue_state();
                }
            });
        }
        controller.setup_composer();
        controller.setup_double_esc();

        let weak_title = Rc::downgrade(&controller);
        controller
            .conversation_view
            .set_title_double_click_handler(move || {
                if let Some(ctrl) = weak_title.upgrade() {
                    ctrl.open_rename_for_active();
                }
            });

        let weak_edit = Rc::downgrade(&controller);
        controller
            .conversation_view
            .queue_dock()
            .set_on_edit(move |item_id, text| {
                if let Some(ctrl) = weak_edit.upgrade() {
                    ctrl.queue_edit(&item_id, &text);
                }
            });

        let weak_remove = Rc::downgrade(&controller);
        controller
            .conversation_view
            .queue_dock()
            .set_on_remove(move |item_id| {
                if let Some(ctrl) = weak_remove.upgrade() {
                    ctrl.queue_remove(&item_id);
                }
            });

        let weak_steer = Rc::downgrade(&controller);
        controller
            .conversation_view
            .queue_dock()
            .set_on_steer_item(move |item_id| {
                if let Some(ctrl) = weak_steer.upgrade() {
                    ctrl.queue_steer_item(&item_id);
                }
            });

        let weak_steer_all = Rc::downgrade(&controller);
        controller
            .conversation_view
            .queue_dock()
            .set_on_steer_all(move |ids| {
                if let Some(ctrl) = weak_steer_all.upgrade() {
                    ctrl.queue_steer_all(ids);
                }
            });

        let ctrl_weak_banner = Rc::downgrade(&controller);
        banner_for_retry.connect_button_clicked(move |banner| {
            banner.set_revealed(false);
            if let Some(ctrl) = ctrl_weak_banner.upgrade() {
                if ctrl.has_supervisor() {
                    ctrl.start_reconnect_loop(true);
                } else {
                    ctrl.bootstrap_supervisor();
                }
            }
        });

        let search_weak = Rc::downgrade(&controller);
        let search_generation = controller.search_generation.clone();
        controller
            .sidebar
            .search_entry()
            .connect_search_changed(move |entry| {
                let text = entry.text().to_string();
                let generation = search_generation.get().wrapping_add(1);
                search_generation.set(generation);
                let weak = search_weak.clone();
                let generation_slot = search_generation.clone();
                gtk4::glib::timeout_add_local_once(
                    std::time::Duration::from_millis(250),
                    move || {
                        if generation_slot.get() != generation {
                            return;
                        }
                        let Some(ctrl) = weak.upgrade() else {
                            return;
                        };
                        if text.trim().is_empty() {
                            ctrl.load_session_list();
                        } else {
                            ctrl.search_sessions(text.trim());
                        }
                    },
                );
            });

        {
            let conv = controller.conversation_view.clone();
            let starts = controller.turn_start_times.clone();
            let active = controller.active_session.clone();
            gtk4::glib::timeout_add_seconds_local(1, move || {
                let id = active.borrow().as_ref().map(|session| session.id.clone());
                if let Some(id) = id {
                    if let Some(start) = starts.borrow().get(&id) {
                        conv.set_running_elapsed(start.elapsed().as_secs());
                    }
                }
                gtk4::glib::ControlFlow::Continue
            });
        }

        {
            let conv = controller.conversation_view.clone();
            let active = controller.active_session.clone();
            let stats_button = conv.stats_button().clone();
            stats_button.connect_active_notify(move |button| {
                if !button.is_active() {
                    return;
                }
                let events = active
                    .borrow()
                    .as_ref()
                    .map(|session| session.raw_events().iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                let stats = crate::transcript::turn_stats(&events);
                conv.set_turn_stats(&stats);
            });
        }

        crate::ui::message_renderer::set_controller(&controller);
        controller
            .conversation_view
            .set_trajectory_visible(controller.prefs.borrow().developer_tools);
        controller
            .conversation_view
            .set_view_mode(controller.prefs.borrow().transcript_view);
        let weak_view = Rc::downgrade(&controller);
        controller.conversation_view.set_on_view_mode(move |mode| {
            if let Some(ctrl) = weak_view.upgrade() {
                ctrl.set_transcript_view(mode);
            }
        });
        let weak_session_view = Rc::downgrade(&controller);
        controller
            .conversation_view
            .set_on_view_selected(move |name| {
                if let Some(ctrl) = weak_session_view.upgrade() {
                    ctrl.remember_session_view(name);
                }
            });
        if let Some(theme) = controller.prefs.borrow().theme.clone() {
            let manager = adw::StyleManager::default();
            manager.set_color_scheme(match theme.as_str() {
                "light" => adw::ColorScheme::ForceLight,
                "dark" => adw::ColorScheme::ForceDark,
                _ => adw::ColorScheme::Default,
            });
        }
        if let Some(size) = controller.prefs.borrow().font_size {
            if let Some(settings) = gtk4::Settings::default() {
                settings.set_gtk_font_name(Some(&format!("Sans {}", size)));
            }
        }
        let weak_history = Rc::downgrade(&controller);
        controller.conversation_view.set_on_load_older(move || {
            if let Some(ctrl) = weak_history.upgrade() {
                ctrl.load_older_history();
            }
        });
        controller
    }

    pub fn credential_store(&self) -> &CredentialStore {
        &self.credential_store
    }

    pub fn connection_handle(&self) -> Arc<RwLock<Option<BridgeConnection>>> {
        self.connection.clone()
    }

    pub fn open_settings(self: &Rc<Self>) {
        let handle = show_settings_dialog(
            &self.window,
            Some(self.clone()),
            Some(self.credential_store.clone()),
        );
        *self.settings_dialog.borrow_mut() = Some(handle);
    }

    pub fn bootstrap_supervisor(&self) {
        let ui_sender = self.ui_sender.clone();
        let supervisor_store = self.supervisor.clone();

        tokio_runtime().spawn(async move {
            let existing = supervisor_store.read().await.clone();
            let supervisor = if let Some(existing) = existing {
                let _ = existing.stop().await;
                existing
            } else {
                let mut config = SupervisorConfig::default();
                config.dsh_bin = std::env::var("DSH_BIN").unwrap_or_else(|_| "dsh".to_string());
                config.profile_config.bridge_bundle_path_or_url = resolve_bridge_bundle_path();
                Arc::new(HarnessSupervisor::new(config))
            };

            match supervisor.start().await {
                Ok(init) => {
                    if let Some(connection) = supervisor.connection().await {
                        let _ = ui_sender.send(UiMessage::SupervisorStarted {
                            supervisor,
                            connection,
                            init: Some(init),
                        });
                    } else {
                        let _ = ui_sender.send(UiMessage::SupervisorStartFailed(
                            "dsh started without a connection".to_string(),
                        ));
                    }
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::SupervisorStartFailed(err.to_string()));
                }
            }
        });
    }

    fn weak_self(&self) -> Weak<SessionController> {
        self.self_weak.borrow().clone()
    }

    pub fn has_supervisor(&self) -> bool {
        self.has_supervisor.get()
    }

    fn store_init_result(&self, init: Option<InitializeResult>) {
        if init.is_some() {
            *self.init_result.borrow_mut() = init;
        }
        if let Some(handle) = self.settings_dialog.borrow().as_ref() {
            handle.refresh_version_info(self.init_result.borrow().as_ref());
        }
    }

    pub fn active_session_id(&self) -> Option<String> {
        self.active_session
            .borrow()
            .as_ref()
            .map(|session| session.id.clone())
    }

    pub async fn connection(&self) -> Option<BridgeConnection> {
        self.connection.read().await.clone()
    }

    fn show_ready(&self) {
        self.content_stack.set_visible_child_name("main");
        self.startup_spinner.stop();
        self.startup_banner.set_revealed(false);
    }

    fn show_startup_error(&self, message: &str) {
        self.content_stack.set_visible_child_name("startup");
        self.startup_spinner.stop();
        self.startup_page.set_title("dsh failed to start");
        self.startup_page.set_description(Some(message));
        self.startup_banner.set_title("dsh failed to start");
        self.startup_banner.set_button_label(Some("Retry"));
        self.startup_banner.set_revealed(true);
    }

    fn show_connection_lost(&self) {
        self.startup_banner.set_title("dsh connection lost");
        self.startup_banner.set_button_label(Some("Reconnect"));
        self.startup_banner.set_revealed(true);
        self.conversation_view.set_disconnected();
    }

    fn mark_restore_pending(&self) {
        self.restore_pending.set(true);
    }

    fn cache_session_title(&self, session_id: &str, title: &str) {
        self.session_titles
            .borrow_mut()
            .insert(session_id.to_string(), title.to_string());
    }

    fn update_session_title(&self, session_id: &str, title: &str) {
        let updated = {
            let mut sessions = self.sessions_cache.borrow_mut();
            match sessions.iter_mut().find(|item| item.id == session_id) {
                Some(item) => {
                    item.title = title.to_string();
                    true
                }
                None => false,
            }
        };
        if updated {
            self.refresh_sidebar_sessions();
        }
    }

    fn session_title_for(&self, session_id: &str) -> String {
        if let Some(session) = self.active_session.borrow().as_ref() {
            if session.id == session_id {
                return session.title.clone();
            }
        }
        self.session_titles
            .borrow()
            .get(session_id)
            .cloned()
            .unwrap_or_else(|| "Session".to_string())
    }

    fn handle_session_list_loaded(&self, sessions: &[SessionListItem]) {
        for item in sessions {
            self.cache_session_title(&item.id, &item.title);
        }
        if !self.restore_pending.get() || self.active_session.borrow().is_some() {
            return;
        }
        self.restore_pending.set(false);
        let saved_id = self.session_state.borrow().last_session_id.clone();
        let target = saved_id
            .filter(|id| sessions.iter().any(|item| &item.id == id))
            .or_else(|| sessions.first().map(|item| item.id.clone()));
        if let Some(id) = target {
            self.follow_session(&id);
        }
    }

    fn persist_session(&self, session_id: &str, cwd: &str) {
        let mut state = self.session_state.borrow_mut();
        state.last_session_id = Some(session_id.to_string());
        state.last_cwd = Some(cwd.to_string());
        let _ = state.save(&self.state_path);
    }

    fn clear_persisted_session(&self) {
        let mut state = self.session_state.borrow_mut();
        state.last_session_id = None;
        state.last_cwd = None;
        let _ = state.save(&self.state_path);
    }

    pub fn open_rename_for_active(&self) {
        if let Some(session_id) = self.active_session_id() {
            self.open_rename_for_id(&session_id);
        }
    }

    pub fn open_rename_for_id(&self, session_id: &str) {
        let title = self.session_title_for(session_id);
        let weak = self.weak_self();
        let session_id = session_id.to_string();
        show_rename_dialog(&self.window, &title, move |new_title| {
            if let Some(ctrl) = weak.upgrade() {
                ctrl.rename_session(&session_id, &new_title);
            }
        });
    }

    pub fn rename_session(&self, session_id: &str, title: &str) {
        let params = SessionRenameParams {
            session_id: session_id.to_string(),
            title: title.to_string(),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                match conn
                    .call::<SessionRenameParams, SessionRenameResult>(
                        METHOD_SESSION_RENAME,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    Ok(result) => {
                        let _ = ui_sender.send(UiMessage::SessionRenamed {
                            session_id: result.session_id,
                            title: result.title,
                        });
                    }
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::Toast(format!("Rename failed: {}", err)));
                    }
                }
            }
        });
    }

    fn apply_rename(&self, session_id: &str, title: &str) {
        self.cache_session_title(session_id, title);
        self.update_session_title(session_id, title);
        let mut active = self.active_session.borrow_mut();
        if let Some(session) = active.as_mut() {
            if session.id == session_id {
                session.title = title.to_string();
                self.window.set_title(Some(title));
                self.conversation_view.set_title(title);
            }
        }
    }

    pub fn open_fork_dialog(&self, at_seq: i64) {
        let session_opt = self.active_session.borrow();
        let Some(session) = session_opt.as_ref() else {
            return;
        };
        let session_id = session.id.clone();
        let title = session.title.clone();
        drop(session_opt);
        let weak = self.weak_self();
        show_fork_dialog(&self.window, &title, at_seq, move || {
            if let Some(ctrl) = weak.upgrade() {
                ctrl.fork_session(&session_id, at_seq);
            }
        });
    }

    pub fn fork_session(&self, session_id: &str, at_seq: i64) {
        let params = SessionForkParams {
            session_id: session_id.to_string(),
            at_seq: Some(at_seq),
            increase_title: Some(true),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                match conn
                    .call::<SessionForkParams, SessionForkResult>(
                        METHOD_SESSION_FORK,
                        params,
                        Duration::from_secs(15),
                    )
                    .await
                {
                    Ok(result) => {
                        let _ = ui_sender.send(UiMessage::SessionForked(result.session_id));
                    }
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::Toast(format!("Fork failed: {}", err)));
                    }
                }
            }
        });
    }

    pub fn refresh_queue(&self, session_id: &str) {
        let session_id_for_ui = session_id.to_string();
        let params = QueueListParams {
            session_id: session_id.to_string(),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Ok(result) = conn
                    .call::<QueueListParams, QueueListResult>(
                        METHOD_QUEUE_LIST,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::QueueUpdated {
                        session_id: session_id_for_ui,
                        items: result.items,
                    });
                }
            }
        });
    }

    pub fn queue_edit(&self, item_id: &str, text: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = QueueEditParams {
            session_id,
            item_id: item_id.to_string(),
            content: vec![PromptContentBlock::Text {
                text: text.to_string(),
            }],
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Err(err) = conn
                    .call::<QueueEditParams, QueueMutationResult>(
                        METHOD_QUEUE_EDIT,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::Toast(format!("Queue edit failed: {}", err)));
                }
            }
        });
    }

    pub fn queue_remove(&self, item_id: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = QueueRemoveParams {
            session_id,
            item_id: item_id.to_string(),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Err(err) = conn
                    .call::<QueueRemoveParams, QueueMutationResult>(
                        METHOD_QUEUE_REMOVE,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ =
                        ui_sender.send(UiMessage::Toast(format!("Queue remove failed: {}", err)));
                }
            }
        });
    }

    pub fn queue_steer_item(&self, item_id: &str) {
        self.queue_steer_all(vec![item_id.to_string()]);
    }

    pub fn queue_steer_all(&self, item_ids: Vec<String>) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        if item_ids.is_empty() {
            return;
        }
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                for item_id in item_ids {
                    let params = QueueSteerParams {
                        session_id: session_id.clone(),
                        item_id,
                    };
                    if let Err(err) = conn
                        .call::<QueueSteerParams, QueueMutationResult>(
                            METHOD_QUEUE_STEER,
                            params,
                            Duration::from_secs(5),
                        )
                        .await
                    {
                        let _ = ui_sender.send(UiMessage::Toast(format!("Steer failed: {}", err)));
                        break;
                    }
                }
            }
        });
    }

    pub fn send_prompt(&self, text: &str) {
        self.send_prompt_with_mode(text, "prompt");
    }

    pub fn update_continue_state(&self) {
        let composer_empty = {
            let buffer = self.conversation_view.composer().text_view().buffer();
            let (start, end) = buffer.bounds();
            buffer.text(&start, &end, false).trim().is_empty()
        };
        let running = self
            .active_session
            .borrow()
            .as_ref()
            .map(|session| session.status == SessionStatus::Running)
            .unwrap_or(false);
        let last_error = self
            .active_session
            .borrow()
            .as_ref()
            .and_then(|session| session.last_error_code.clone());
        let eligible =
            crate::drafts::continuation_eligibility(last_error.as_deref(), composer_empty, running);
        self.conversation_view
            .composer()
            .set_continue_visible(eligible == crate::drafts::ContinuationEligibility::Eligible);
    }

    pub fn continue_generation(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        if let Some(session) = self.active_session.borrow_mut().as_mut() {
            session.last_error_code = None;
        }
        let params = SessionPromptParams {
            session_id,
            content_blocks: Vec::new(),
            mode: Some("queue".to_string()),
        };
        self.dispatch_call(
            METHOD_SESSION_PROMPT,
            params,
            Duration::from_secs(10),
            |result: Result<SessionPromptResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Continuing generation".to_string()),
                Err(err) => UiMessage::Toast(format!("Continue failed: {}", err)),
            },
        );
        self.conversation_view
            .composer()
            .set_continue_visible(false);
    }

    pub fn load_older_history(&self) {
        if self.history_loading.replace(true) {
            return;
        }
        let Some(session_id) = self.active_session_id() else {
            self.history_loading.set(false);
            return;
        };
        let oldest = self
            .active_session
            .borrow()
            .as_ref()
            .and_then(|session| session.transcript.first_seq());
        let Some(oldest) = oldest else {
            self.history_loading.set(false);
            return;
        };
        if oldest <= 0 {
            self.history_loading.set(false);
            return;
        }
        self.conversation_view.set_history_loading(true);
        let request_session = session_id.clone();
        let params = SessionPageParams {
            session_id,
            from_seq: (oldest - 50).max(0),
            to_seq: Some(oldest - 1),
            limit: Some(50),
        };
        self.dispatch_call(
            METHOD_SESSION_PAGE,
            params,
            Duration::from_secs(10),
            move |result: Result<SessionPageResult, ConnectionError>| match result {
                Ok(page) => UiMessage::HistoryPageLoaded {
                    session_id: request_session.clone(),
                    events: page.events,
                    has_more: page.has_more,
                },
                Err(err) => UiMessage::Toast(format!("History load failed: {}", err)),
            },
        );
    }
    pub fn send_prompt_with_mode(&self, text: &str, mode: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = SessionPromptParams {
            session_id,
            content_blocks: vec![PromptContentBlock::Text {
                text: text.to_string(),
            }],
            mode: Some(mode.to_string()),
        };
        let conn_arc = self.connection.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let _ = conn
                    .call::<SessionPromptParams, SessionPromptResult>(
                        METHOD_SESSION_PROMPT,
                        params,
                        Duration::from_secs(10),
                    )
                    .await;
            }
        });
    }

    pub fn pick_attachments(&self) {
        let dialog = gtk4::FileDialog::builder().title("Attach Files").build();
        let weak = self.weak_self();
        dialog.open_multiple(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |result| {
                let Ok(files) = result else {
                    return;
                };
                let Some(ctrl) = weak.upgrade() else {
                    return;
                };
                for index in 0..files.n_items() {
                    if let Some(file) = files
                        .item(index)
                        .and_then(|item| item.downcast::<gtk4::gio::File>().ok())
                    {
                        if let Some(path) = file.path() {
                            ctrl.attach_file_path(&path);
                        }
                    }
                }
            },
        );
    }

    pub fn attach_file_path(&self, path: &std::path::Path) {
        let Ok(bytes) = std::fs::read(path) else {
            let _ = self
                .ui_sender
                .send(UiMessage::Toast(format!("Cannot read {}", path.display())));
            return;
        };
        let name = crate::attachments::display_name(path);
        match crate::attachments::classify_path(path) {
            crate::attachments::AttachmentKind::Image(mime) => {
                self.upload_attachment(name, mime.to_string(), bytes, true);
            }
            crate::attachments::AttachmentKind::File => {
                let reference = crate::attachments::file_reference_text(path);
                self.upload_attachment(name, "application/octet-stream".to_string(), bytes, false);
                self.insert_composer_text(&reference);
            }
        }
    }

    pub fn attach_clipboard_texture(&self, texture: &gtk4::gdk::Texture) {
        let bytes = texture.save_to_png_bytes();
        self.upload_attachment(
            "clipboard.png".to_string(),
            "image/png".to_string(),
            bytes.as_ref().to_vec(),
            true,
        );
    }

    fn upload_attachment(&self, name: String, mime_type: String, bytes: Vec<u8>, image: bool) {
        let Some(session_id) = self.active_session_id() else {
            let _ = self.ui_sender.send(UiMessage::Toast(
                "Open a session before attaching".to_string(),
            ));
            return;
        };
        let data = crate::attachments::encode_base64(&bytes);
        let params = AttachmentUploadParams {
            session_id,
            name: name.clone(),
            data: data.clone(),
            mime_type: Some(mime_type.clone()),
        };
        let ui_sender = self.ui_sender.clone();
        self.dispatch_call(
            METHOD_ATTACHMENT_UPLOAD,
            params,
            Duration::from_secs(30),
            move |result: Result<AttachmentUploadResult, ConnectionError>| match result {
                Ok(_) if image => UiMessage::ImageAttached {
                    name,
                    mime_type,
                    data,
                },
                Ok(_) => UiMessage::FileAttached(String::new()),
                Err(err) => UiMessage::Toast(format!("Attachment upload failed: {}", err)),
            },
        );
        let _ = ui_sender;
    }

    pub fn submit_text(&self, text: &str, steer: bool) {
        let trimmed = text.trim();
        let images = self.pending_images.borrow().clone();
        if trimmed.is_empty() && images.is_empty() {
            return;
        }
        if trimmed.starts_with('/') {
            self.execute_command_line(trimmed);
            return;
        }
        let running = self
            .active_session
            .borrow()
            .as_ref()
            .map(|session| session.status == SessionStatus::Running)
            .unwrap_or(false);
        let mode = if running && steer { "steer" } else { "prompt" };
        let Some(session_id) = self.active_session_id() else {
            if !trimmed.is_empty() {
                *self.pending_auto_send.borrow_mut() = Some(trimmed.to_string());
                self.start_session_in(None, None, None);
            }
            return;
        };
        let pending_text = if text.trim().is_empty() {
            "[image]"
        } else {
            text
        };
        let pending_images: Vec<UserImageAttachment> = images
            .iter()
            .map(|image| UserImageAttachment {
                name: Some(image.name.clone()),
                mime_type: image.mime_type.clone(),
                data: Some(image.data.clone()),
                attachment_id: None,
            })
            .collect();
        {
            let mut guard = self.active_session.borrow_mut();
            if let Some(session) = guard.as_mut() {
                session
                    .transcript
                    .push_pending_user_with_images(pending_text, &pending_images);
                self.conversation_view
                    .update_transcript(&session.transcript);
            }
        }
        let mut content_blocks = Vec::new();
        if !text.trim().is_empty() {
            content_blocks.push(PromptContentBlock::Text {
                text: text.to_string(),
            });
        }
        for image in &images {
            content_blocks.push(PromptContentBlock::Image {
                data: image.data.clone(),
                mime_type: image.mime_type.clone(),
                name: Some(image.name.clone()),
            });
        }
        let params = SessionPromptParams {
            session_id: session_id.clone(),
            content_blocks,
            mode: Some(mode.to_string()),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let echoed = text.to_string();
        let echo_session_id = session_id.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            match conn
                .call::<SessionPromptParams, SessionPromptResult>(
                    METHOD_SESSION_PROMPT,
                    params,
                    Duration::from_secs(10),
                )
                .await
            {
                Ok(_) => {
                    let _ = ui_sender.send(UiMessage::EchoAck {
                        session_id: echo_session_id,
                    });
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::EchoFailed {
                        session_id: echo_session_id,
                        text: echoed,
                        error: err.to_string(),
                    });
                }
            }
        });
    }

    pub fn fetch_commands(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = CommandListParams { session_id };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Ok(result) = conn
                    .call::<CommandListParams, CommandListResult>(
                        METHOD_COMMAND_LIST,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::CommandsLoaded(result.commands));
                }
            }
        });
    }

    pub fn fetch_references(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = ReferenceListParams {
            session_id,
            query: String::new(),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Ok(result) = conn
                    .call::<ReferenceListParams, ReferenceListResult>(
                        METHOD_REFERENCE_LIST,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::ReferencesLoaded(result.candidates));
                }
            }
        });
    }

    pub fn fetch_skills(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = SkillListParams {
            session_id: Some(session_id),
            cwd: None,
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Ok(result) = conn
                    .call::<SkillListParams, SkillListResult>(
                        METHOD_SKILL_LIST,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::SkillsLoaded(result.skills));
                }
            }
        });
    }

    pub fn execute_command_line(&self, text: &str) {
        let rest = text.trim().trim_start_matches('/');
        let mut parts = rest.splitn(2, char::is_whitespace);
        let command = parts.next().unwrap_or_default();
        let args = parts.next().unwrap_or_default();
        if !command.is_empty() {
            self.execute_command(command, args);
        }
    }

    pub fn execute_command(&self, command: &str, args: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = CommandExecuteParams {
            session_id,
            command: command.to_string(),
            args: if args.trim().is_empty() {
                None
            } else {
                Some(args.trim().to_string())
            },
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                match conn
                    .call::<CommandExecuteParams, CommandExecuteResult>(
                        METHOD_COMMAND_EXECUTE,
                        params,
                        Duration::from_secs(10),
                    )
                    .await
                {
                    Ok(result) => {
                        if let Some(text) = result.text {
                            let message = if result.kind == "error" {
                                format!("Command failed: {}", text)
                            } else {
                                text
                            };
                            let _ = ui_sender.send(UiMessage::Toast(message));
                        }
                    }
                    Err(err) => {
                        let _ =
                            ui_sender.send(UiMessage::Toast(format!("Command failed: {}", err)));
                    }
                }
            }
        });
    }

    pub fn set_composer_text(&self, text: &str) {
        let buffer = self.conversation_view.composer().text_view().buffer();
        buffer.set_text(text);
        buffer.place_cursor(&buffer.end_iter());
        self.conversation_view.composer().text_view().grab_focus();
    }

    pub fn remember_composer_draft(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let buffer = self.conversation_view.composer().text_view().buffer();
        let (start, end) = buffer.bounds();
        let text = buffer.text(&start, &end, false).to_string();
        let mut drafts = self.drafts.borrow_mut();
        drafts.set(&session_id, &text);
        let _ = drafts.save(&self.drafts_path);
    }

    fn restore_composer_draft(&self, session_id: &str) {
        let text = self.drafts.borrow().text_for(session_id);
        self.set_composer_text(&text);
    }

    fn clear_composer_draft(&self, session_id: &str) {
        let mut drafts = self.drafts.borrow_mut();
        drafts.remove(session_id);
        let _ = drafts.save(&self.drafts_path);
    }

    fn store_composer_draft(&self, session_id: &str, text: &str) {
        let mut drafts = self.drafts.borrow_mut();
        drafts.set(session_id, text);
        let _ = drafts.save(&self.drafts_path);
    }

    pub fn insert_composer_text(&self, text: &str) {
        let buffer = self.conversation_view.composer().text_view().buffer();
        let mut end = buffer.end_iter();
        buffer.insert(&mut end, text);
        self.conversation_view.composer().text_view().grab_focus();
    }

    pub fn close_session(&self, session_id: &str) {
        let Some(active_id) = self.active_session_id() else {
            return;
        };
        if active_id != session_id {
            return;
        }
        let params = SessionReleaseParams {
            session_id: session_id.to_string(),
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                match conn
                    .call::<SessionReleaseParams, SessionReleaseResult>(
                        METHOD_SESSION_RELEASE,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    Ok(_) => {
                        let _ = ui_sender.send(UiMessage::SessionReleased);
                    }
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::Toast(format!("Close failed: {}", err)));
                    }
                }
            }
        });
    }

    fn show_blank_session_view(&self) {
        self.remember_composer_draft();
        self.next_follow_generation();
        *self.pending_auto_send.borrow_mut() = None;
        *self.gap_repair.borrow_mut() = None;
        *self.active_session.borrow_mut() = None;
        self.conversation_view.clear_session();
        self.right_dock.widget().set_visible(false);
        self.window.set_title(Some("dsh"));
        self.sidebar.set_active_session(None);
        self.update_dock_button();
    }

    fn clear_active_session_ui(&self) {
        self.show_blank_session_view();
        self.clear_persisted_session();
    }

    fn update_dock_button(&self) {
        self.dock_button.set_visible(
            self.active_session_id().is_some() && self.panel_state.active_panel().is_none(),
        );
    }

    pub fn running_session_info(&self) -> Option<(String, String)> {
        let running = self.running_sessions.borrow();
        if running.is_empty() {
            return None;
        }

        let active = self.active_session.borrow();
        if let Some(sess) = active.as_ref() {
            if running.contains(&sess.id) {
                return Some((sess.id.clone(), sess.title.clone()));
            }
        }

        running
            .iter()
            .next()
            .map(|id| (id.clone(), "dsh".to_string()))
    }

    pub fn any_session_running(&self) -> bool {
        !self.running_sessions.borrow().is_empty()
    }

    pub fn set_background_hold(&self, hold: gtk4::gio::ApplicationHoldGuard, session_id: &str) {
        *self.background_hold.borrow_mut() = Some(hold);
        *self.background_session.borrow_mut() = Some(session_id.to_string());
    }

    pub fn handle_follow_status(&self, session_id: &str, status: &str) {
        if status == "running" {
            let already_running = self.running_sessions.borrow().contains(session_id);
            self.running_sessions
                .borrow_mut()
                .insert(session_id.to_string());
            if !already_running {
                self.turn_start_times
                    .borrow_mut()
                    .insert(session_id.to_string(), Instant::now());
            }
            self.unread_sessions.borrow_mut().remove(session_id);
        } else {
            self.running_sessions.borrow_mut().remove(session_id);
            self.turn_start_times.borrow_mut().remove(session_id);
        }
    }

    pub fn handle_status_notification(&self, session_id: &str, status: &str) {
        let app_opt = self.window.application();
        let window_focused = self.window.is_active();

        if status == "running" {
            let already_running = self.running_sessions.borrow().contains(session_id);
            self.running_sessions
                .borrow_mut()
                .insert(session_id.to_string());
            if !already_running {
                self.turn_start_times
                    .borrow_mut()
                    .insert(session_id.to_string(), Instant::now());
            }
            self.unread_sessions.borrow_mut().remove(session_id);
        } else {
            let was_running = self.running_sessions.borrow_mut().remove(session_id);
            let start = self.turn_start_times.borrow_mut().remove(session_id);
            let duration_ms = start
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(0);

            let active_id = self.active_session_id();
            if was_running && active_id.as_deref() != Some(session_id) {
                self.unread_sessions
                    .borrow_mut()
                    .insert(session_id.to_string());
            }
            if was_running
                && self.prefs.borrow().notifications_enabled
                && should_notify_turn_completion(window_focused, duration_ms)
            {
                if let Some(app) = app_opt.as_ref() {
                    let (title, raw_snippet) = {
                        let active = self.active_session.borrow();
                        match active.as_ref() {
                            Some(sess) => (
                                sess.title.clone(),
                                sess.transcript
                                    .latest_snippet()
                                    .unwrap_or_else(|| "Turn finished.".to_string()),
                            ),
                            None => ("dsh".to_string(), "Turn finished.".to_string()),
                        }
                    };
                    let snippet = truncate_snippet(&raw_snippet, 100);
                    send_turn_completed_notification(app, session_id, &title, &snippet);
                }
            }

            if let Some(app) = app_opt.as_ref() {
                let background_id = self.background_session.borrow().clone();
                if let Some(background_id) = background_id {
                    withdraw_notification(app, &format!("{}-background", background_id));
                }
            }

            if self.running_sessions.borrow().is_empty() {
                self.background_hold.borrow_mut().take();
                self.background_session.borrow_mut().take();
            }
        }

        if let Some(sess) = self.active_session.borrow_mut().as_mut() {
            if sess.id == session_id {
                sess.handle_status(status);
                self.conversation_view.set_status(if status == "running" {
                    "Running"
                } else {
                    "Idle"
                });
            }
        }
        self.refresh_sidebar_sessions();
    }

    pub fn set_supervisor(&self, sup: Arc<HarnessSupervisor>) {
        let sup_store = self.supervisor.clone();
        tokio_runtime().spawn(async move {
            *sup_store.write().await = Some(sup);
        });
    }

    pub fn set_connection(&self, conn: BridgeConnection) {
        let sender1 = self.ui_sender.clone();
        conn.set_approval_responder(ApprovalHandler { ui_sender: sender1 });

        let sender2 = self.ui_sender.clone();
        conn.set_user_question_responder(QuestionHandler { ui_sender: sender2 });

        let conn_clone = conn.clone();
        let conn_store = self.connection.clone();
        tokio_runtime().spawn(async move {
            *conn_store.write().await = Some(conn_clone);
        });

        let mut notif_rx = conn.subscribe_notifications();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            while let Ok(notif) = notif_rx.recv().await {
                let _ = ui_sender.send(UiMessage::Notification(notif));
            }
        });

        let mut closed_rx = conn.subscribe_closed();
        let ui_sender_closed = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let _ = closed_rx.recv().await;
            let _ = ui_sender_closed.send(UiMessage::ConnectionLost);
        });

        let conn_for_cred = conn.clone();
        let ui_sender_for_cred = self.ui_sender.clone();
        let store_for_cred = self.credential_store.clone();

        tokio_runtime().spawn(async move {
            if let Ok(res) = conn_for_cred
                .call::<serde_json::Value, CredentialsStatusResult>(
                    METHOD_CREDENTIALS_STATUS,
                    serde_json::json!({}),
                    Duration::from_secs(5),
                )
                .await
            {
                if should_show_onboarding(&res.items) {
                    if let Some(saved_key) = store_for_cred.get_key(DEFAULT_PROVIDER) {
                        let _ = conn_for_cred
                            .call::<CredentialsInjectParams, CredentialsInjectResult>(
                                METHOD_CREDENTIALS_INJECT,
                                CredentialsInjectParams {
                                    provider: DEFAULT_PROVIDER.to_string(),
                                    key: saved_key,
                                    base_url: None,
                                },
                                Duration::from_secs(5),
                            )
                            .await;
                    } else {
                        let _ = ui_sender_for_cred.send(UiMessage::ShowOnboarding);
                    }
                }
            }
        });

        self.load_session_list();
        self.load_workspaces();
        self.load_schedules();
        self.adopt_default_model();
    }

    fn adopt_default_model(&self) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            if let Ok(catalog) = conn
                .call::<serde_json::Value, ModelCatalogResult>(
                    METHOD_MODEL_CATALOG,
                    serde_json::json!({}),
                    Duration::from_secs(10),
                )
                .await
            {
                let _ = ui_sender.send(UiMessage::ModelCatalogLoaded(catalog));
            }
        });
    }

    pub fn load_session_list(&self) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let include_archived = self.prefs.borrow().archive_filter != ArchiveFilter::Active;

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let params = serde_json::json!({"archived": include_archived, "limit": 200});
                if let Ok(res) = conn
                    .call::<serde_json::Value, SessionListResult>(
                        METHOD_SESSION_LIST,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::SessionListLoaded(res.sessions));
                }
            }
        });
    }

    pub fn load_goal(&self, session_id: &str) {
        self.dispatch_call(
            METHOD_GOAL_GET,
            GoalGetParams {
                session_id: session_id.to_string(),
            },
            Duration::from_secs(10),
            |result: Result<GoalGetResult, ConnectionError>| match result {
                Ok(goal) => UiMessage::GoalLoaded(goal.goal),
                Err(err) => UiMessage::Toast(format!("Goal unavailable: {}", err)),
            },
        );
    }

    pub fn goal_edit_dialog(&self) {
        let Some(goal) = self.conversation_view.goal_bar().goal() else {
            return;
        };
        let weak = self.weak_self();
        let goal_for_save = goal.clone();
        crate::ui::goal_bar::goal_edit_dialog(&self.window, &goal, move |objective| {
            if let Some(ctrl) = weak.upgrade() {
                ctrl.goal_edit(&goal_for_save, &objective);
            }
        });
    }

    fn goal_edit(&self, goal: &GoalView, objective: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        self.dispatch_call(
            METHOD_GOAL_EDIT,
            GoalEditParams {
                session_id,
                r#ref: GoalRef {
                    id: goal.id.clone(),
                    revision: goal.revision,
                },
                objective: Some(objective.to_string()),
                max_goal_rounds: None,
            },
            Duration::from_secs(10),
            |result: Result<GoalGetResult, ConnectionError>| match result {
                Ok(goal) => UiMessage::GoalLoaded(goal.goal),
                Err(err) => UiMessage::Toast(format!("Goal edit failed: {}", err)),
            },
        );
    }

    pub fn goal_pause(&self) {
        self.goal_action("pause");
    }

    pub fn goal_resume(&self) {
        self.goal_action("resume");
    }

    pub fn goal_clear(&self) {
        self.goal_action("clear");
    }

    fn goal_action(&self, action: &str) {
        let Some(goal) = self.conversation_view.goal_bar().goal() else {
            return;
        };
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let method = match action {
            "pause" => METHOD_GOAL_PAUSE,
            "resume" => METHOD_GOAL_RESUME,
            _ => METHOD_GOAL_CLEAR,
        };
        let params = GoalActionParams {
            session_id,
            r#ref: GoalRef {
                id: goal.id.clone(),
                revision: goal.revision,
            },
        };
        self.dispatch_call(
            method,
            params,
            Duration::from_secs(10),
            |result: Result<GoalGetResult, ConnectionError>| match result {
                Ok(goal) => UiMessage::GoalLoaded(goal.goal),
                Err(err) => UiMessage::Toast(format!("Goal action failed: {}", err)),
            },
        );
    }

    pub fn load_session_window(&self, session_id: &str, latest_seq: i64) {
        if latest_seq < 0 {
            return;
        }
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_SESSION_PAGE,
            SessionPageParams {
                session_id: session_id.to_string(),
                from_seq: (latest_seq - 200).max(0),
                to_seq: Some(latest_seq),
                limit: Some(200),
            },
            Duration::from_secs(10),
            move |result: Result<SessionPageResult, ConnectionError>| match result {
                Ok(page) => UiMessage::SessionWindowLoaded {
                    session_id: request_session.clone(),
                    events: page.events,
                },
                Err(_) => UiMessage::Toast("Session history unavailable".to_string()),
            },
        );
    }

    fn buffer_event_if_repairing(&self, session_id: &str, event: SessionEvent) -> bool {
        let mut repair = self.gap_repair.borrow_mut();
        match repair.as_mut() {
            Some(state) if state.session_id == session_id => {
                state.buffered.push(event);
                true
            }
            _ => false,
        }
    }

    fn begin_gap_repair(
        &self,
        session_id: &str,
        expected: i64,
        actual: i64,
        event: SessionEvent,
    ) {
        if let Some(state) = self.gap_repair.borrow_mut().as_mut() {
            if state.session_id == session_id {
                state.buffered.push(event);
            }
            return;
        }
        let to_seq = actual.saturating_sub(1).max(expected);
        *self.gap_repair.borrow_mut() = Some(GapRepairState {
            session_id: session_id.to_string(),
            to_seq,
            buffered: vec![event],
        });
        self.request_gap_page(session_id, expected, to_seq);
    }

    fn request_gap_page(&self, session_id: &str, from_seq: i64, to_seq: i64) {
        if from_seq > to_seq {
            return;
        }
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_SESSION_PAGE,
            SessionPageParams {
                session_id: session_id.to_string(),
                from_seq,
                to_seq: Some(to_seq),
                limit: Some(((to_seq - from_seq + 1).min(200)) as u32),
            },
            Duration::from_secs(10),
            move |result: Result<SessionPageResult, ConnectionError>| match result {
                Ok(page) => UiMessage::GapRepairLoaded {
                    session_id: request_session.clone(),
                    events: page.events,
                },
                Err(err) => UiMessage::GapRepairFailed {
                    session_id: request_session.clone(),
                    error: err.to_string(),
                },
            },
        );
    }

    fn abort_gap_repair(&self, session_id: &str) -> bool {
        let mut repair = self.gap_repair.borrow_mut();
        let matches = repair
            .as_ref()
            .map(|state| state.session_id == session_id)
            .unwrap_or(false);
        if matches {
            *repair = None;
        }
        matches
    }

    fn continue_gap_repair(&self, session_id: &str, events: Vec<SessionEvent>) {
        let mut guard = self.active_session.borrow_mut();
        let Some(session) = guard.as_mut() else {
            drop(guard);
            self.abort_gap_repair(session_id);
            return;
        };
        if session.id != session_id {
            drop(guard);
            self.abort_gap_repair(session_id);
            return;
        }
        let before = session.transcript.last_seq();
        for event in events {
            let _ = session.handle_event(event);
        }
        let after = session.transcript.last_seq();
        let to_seq = self
            .gap_repair
            .borrow()
            .as_ref()
            .filter(|state| state.session_id == session_id)
            .map(|state| state.to_seq);
        let Some(to_seq) = to_seq else {
            drop(guard);
            return;
        };
        if after < to_seq && after > before {
            drop(guard);
            self.request_gap_page(session_id, after + 1, to_seq);
            return;
        }
        if after < to_seq {
            drop(guard);
            self.abort_gap_repair(session_id);
            self.follow_session(session_id);
            return;
        }
        let buffered = self
            .gap_repair
            .borrow_mut()
            .take()
            .map(|state| state.buffered)
            .unwrap_or_default();
        let mut buffered = buffered;
        buffered.sort_by_key(|event| event.seq);
        for event in buffered {
            let _ = session.handle_event(event);
        }
        self.conversation_view
            .update_transcript(&session.transcript);
        self.conversation_view
            .composer()
            .set_plan_mode(session.plan_mode);
        let raw_events: Vec<SessionEvent> = session.raw_events().iter().cloned().collect();
        self.conversation_view
            .trajectory_view()
            .set_events(&raw_events);
    }

    pub fn load_projections(&self, session_id: &str) {
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_SESSION_PROJECTIONS,
            SessionProjectionsParams {
                session_id: session_id.to_string(),
            },
            Duration::from_secs(10),
            move |result: Result<SessionProjectionsResult, ConnectionError>| match result {
                Ok(projections) => UiMessage::ProjectionsLoaded {
                    session_id: request_session.clone(),
                    projections,
                },
                Err(err) => UiMessage::Toast(format!("Projections failed: {}", err)),
            },
        );
    }

    pub fn search_sessions(&self, query: &str) {
        let params = SessionSearchParams {
            query: query.to_string(),
            limit: Some(50),
            archived: Some(false),
        };
        self.dispatch_call(
            METHOD_SESSION_SEARCH,
            params,
            Duration::from_secs(10),
            |result: Result<SessionSearchResult, ConnectionError>| match result {
                Ok(search) => UiMessage::SessionSearchLoaded(search.items),
                Err(err) => UiMessage::Toast(format!("Search failed: {}", err)),
            },
        );
    }

    fn save_prefs(&self) {
        let _ = self.prefs.borrow().save(&self.prefs_path);
    }

    pub fn refresh_sidebar_sessions(&self) {
        let sessions = self.sessions_cache.borrow().clone();
        let prefs = self.prefs.borrow();
        self.sidebar.load_sessions(
            &sessions,
            &prefs.pinned_set(),
            prefs.archive_filter,
            &self.running_sessions.borrow(),
            &self.unread_sessions.borrow(),
        );
        self.sidebar.set_filter_label(match prefs.archive_filter {
            ArchiveFilter::Active => "Active",
            ArchiveFilter::Archived => "Archived",
            ArchiveFilter::All => "All",
        });
    }

    pub fn toggle_dock(&self) {
        let visible = self.right_dock.widget().is_visible();
        self.set_dock_visible(!visible);
    }

    pub fn set_dock_visible(&self, visible: bool) {
        self.right_dock.widget().set_visible(visible);
        if let Some(session_id) = self.active_session_id() {
            self.prefs
                .borrow_mut()
                .set_dock_visible(&session_id, visible);
            self.save_prefs();
        }
    }

    pub fn restore_dock_for_session(&self, session_id: &str) {
        let prefs = self.prefs.borrow();
        let visible = prefs.dock_visible(session_id);
        let tab = prefs.dock_tab(session_id).unwrap_or(0);
        drop(prefs);
        self.right_dock.widget().set_visible(visible);
        self.right_dock.select_tab(tab);
    }

    pub fn remember_dock_for_session(&self, session_id: &str) {
        let index = self.right_dock.selected_index();
        self.prefs.borrow_mut().set_dock_tab(session_id, index);
        let visible = self.right_dock.widget().is_visible();
        self.prefs
            .borrow_mut()
            .set_dock_visible(session_id, visible);
        self.save_prefs();
    }

    pub fn prefs_snapshot(&self) -> UiPrefs {
        self.prefs.borrow().clone()
    }

    pub fn set_transcript_view(&self, mode: crate::ui_prefs::TranscriptView) {
        self.prefs.borrow_mut().transcript_view = mode;
        self.save_prefs();
        self.conversation_view.set_view_mode(mode);
        if let Some(session) = self.active_session.borrow().as_ref() {
            self.conversation_view
                .rerender_transcript(&session.transcript);
        }
    }

    pub fn set_developer_tools(&self, enabled: bool) {
        self.prefs.borrow_mut().developer_tools = enabled;
        self.save_prefs();
        self.conversation_view.set_trajectory_visible(enabled);
        if !enabled {
            self.conversation_view.set_active_view("chat");
        }
    }

    fn remember_session_view(&self, name: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        {
            let mut prefs = self.prefs.borrow_mut();
            if prefs.session_view(&session_id) == Some(name) {
                return;
            }
            prefs.set_session_view(&session_id, name);
        }
        self.save_prefs();
    }

    fn apply_session_view(&self, session_id: &str) {
        let name = self
            .prefs
            .borrow()
            .session_view(session_id)
            .unwrap_or("chat")
            .to_string();
        self.conversation_view.set_active_view(&name);
    }

    pub fn set_notifications_enabled(&self, enabled: bool) {
        self.prefs.borrow_mut().notifications_enabled = enabled;
        self.save_prefs();
    }

    pub fn set_session_grouping(&self, grouping: SessionGrouping) {
        self.prefs.borrow_mut().session_grouping = grouping;
        self.save_prefs();
        let sort = self.prefs.borrow().session_sort;
        self.sidebar.set_session_options(grouping, sort);
    }

    pub fn set_session_sort(&self, sort: SessionSort) {
        self.prefs.borrow_mut().session_sort = sort;
        self.save_prefs();
        let grouping = self.prefs.borrow().session_grouping;
        self.sidebar.set_session_options(grouping, sort);
    }

    pub fn set_locale_preference(&self, code: &str) {
        self.prefs.borrow_mut().locale = Some(code.to_string());
        self.save_prefs();
    }

    pub fn set_theme_preference(&self, theme: &str) {
        self.prefs.borrow_mut().theme = Some(theme.to_string());
        self.save_prefs();
        let manager = adw::StyleManager::default();
        manager.set_color_scheme(match theme {
            "light" => adw::ColorScheme::ForceLight,
            "dark" => adw::ColorScheme::ForceDark,
            _ => adw::ColorScheme::Default,
        });
    }

    pub fn set_font_size(&self, size: u32) {
        self.prefs.borrow_mut().font_size = Some(size);
        self.save_prefs();
        if let Some(settings) = gtk4::Settings::default() {
            settings.set_gtk_font_name(Some(&format!("Sans {}", size)));
        }
    }

    pub fn toggle_pin_session(&self, session_id: &str) {
        self.prefs.borrow_mut().toggle_pin(session_id);
        self.save_prefs();
        self.refresh_sidebar_sessions();
    }

    pub fn set_archive_filter(&self, filter: ArchiveFilter) {
        self.prefs.borrow_mut().archive_filter = filter;
        self.save_prefs();
        self.refresh_sidebar_sessions();
        self.load_session_list();
    }

    pub fn unarchive_session(&self, session_id: &str) {
        let sess_id = session_id.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let params = SessionArchiveParams {
                session_id: sess_id,
                archived: false,
            };
            match conn
                .call::<SessionArchiveParams, SessionArchiveResult>(
                    METHOD_SESSION_ARCHIVE,
                    params,
                    Duration::from_secs(10),
                )
                .await
            {
                Ok(_) => {
                    let _ = ui_sender.send(UiMessage::Toast("Session unarchived".to_string()));
                    let _ = ui_sender.send(UiMessage::SessionListRefresh);
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::Toast(format!("Unarchive failed: {}", err)));
                }
            }
        });
    }

    pub fn load_workspaces(&self) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            if let Ok(result) = conn
                .call::<serde_json::Value, WorkspaceListResult>(
                    METHOD_WORKSPACE_LIST,
                    serde_json::json!({}),
                    Duration::from_secs(5),
                )
                .await
            {
                let _ = ui_sender.send(UiMessage::WorkspacesLoaded(result.workspaces));
            }
        });
    }

    pub fn prompt_new_workspace(&self) {
        let dialog = gtk4::FileDialog::builder()
            .title("Select Workspace Directory")
            .build();
        let weak = self.weak_self();
        dialog.select_folder(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |result| {
                let Ok(folder) = result else {
                    return;
                };
                let Some(path) = folder.path() else {
                    return;
                };
                let Some(ctrl) = weak.upgrade() else {
                    return;
                };
                let path_string = path.display().to_string();
                let title = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_string.clone());
                ctrl.create_workspace(&path_string, &title);
            },
        );
    }

    pub fn create_workspace(&self, path: &str, title: &str) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let path = path.to_string();
        let title = title.to_string();
        let _ = &title;
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let params = WorkspaceCreateParams {
                path: path.clone(),
                title: Some(title),
            };
            match conn
                .call::<WorkspaceCreateParams, WorkspaceCreateResult>(
                    METHOD_WORKSPACE_CREATE,
                    params,
                    Duration::from_secs(5),
                )
                .await
            {
                Ok(_) => {
                    if let Ok(list) = conn
                        .call::<serde_json::Value, WorkspaceListResult>(
                            METHOD_WORKSPACE_LIST,
                            serde_json::json!({}),
                            Duration::from_secs(5),
                        )
                        .await
                    {
                        let _ = ui_sender.send(UiMessage::WorkspacesLoaded(list.workspaces));
                    }
                    let _ = ui_sender.send(UiMessage::WorkspaceCreated(path));
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::Toast(format!(
                        "Create workspace failed: {}",
                        err
                    )));
                }
            }
        });
    }

    pub fn rename_workspace(&self, id: &str) {
        let Some(workspace) = self
            .workspaces
            .borrow()
            .iter()
            .find(|ws| ws.id == id)
            .cloned()
        else {
            return;
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let _ = &workspace;
        let id = id.to_string();
        show_rename_dialog(&self.window, &workspace.title, move |title| {
            let conn_arc = conn_arc.clone();
            let ui_sender = ui_sender.clone();
            let id = id.clone();
            let _ = &id;
            tokio_runtime().spawn(async move {
                let conn_opt = conn_arc.read().await.clone();
                let Some(conn) = conn_opt else {
                    return;
                };
                match conn
                    .call::<WorkspaceRenameParams, WorkspaceRenameResult>(
                        METHOD_WORKSPACE_RENAME,
                        WorkspaceRenameParams {
                            workspace_id: id,
                            title,
                        },
                        Duration::from_secs(5),
                    )
                    .await
                {
                    Ok(_) => {
                        let _ = ui_sender.send(UiMessage::WorkspaceChanged);
                    }
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::Toast(format!("Rename failed: {}", err)));
                    }
                }
            });
        });
    }

    pub fn remove_workspace(&self, id: &str) {
        let Some(workspace) = self
            .workspaces
            .borrow()
            .iter()
            .find(|ws| ws.id == id)
            .cloned()
        else {
            return;
        };
        let dialog = adw::AlertDialog::builder()
            .heading("Remove workspace?")
            .body(format!(
                "Remove “{}” from workspaces? Sessions are not deleted.",
                workspace.title
            ))
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("remove", "Remove");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let _ = &workspace;
        let id = id.to_string();
        dialog.choose(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |response| {
                if response != "remove" {
                    return;
                }
                let conn_arc = conn_arc.clone();
                let ui_sender = ui_sender.clone();
                let _ = &id;
                let id = id.clone();
                tokio_runtime().spawn(async move {
                    let conn_opt = conn_arc.read().await.clone();
                    let Some(conn) = conn_opt else {
                        return;
                    };
                    match conn
                        .call::<WorkspaceRemoveParams, serde_json::Value>(
                            METHOD_WORKSPACE_REMOVE,
                            WorkspaceRemoveParams { workspace_id: id },
                            Duration::from_secs(5),
                        )
                        .await
                    {
                        Ok(_) => {
                            let _ = ui_sender.send(UiMessage::WorkspaceChanged);
                        }
                        Err(err) => {
                            let _ =
                                ui_sender.send(UiMessage::Toast(format!("Remove failed: {}", err)));
                        }
                    }
                });
            },
        );
    }

    pub fn refresh_files(&self, path: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let path = path.to_string();
        let request_session = session_id.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let workdir = conn
                .call::<SessionWorkdirGetParams, SessionWorkdirResult>(
                    METHOD_SESSION_WORKDIR_GET,
                    SessionWorkdirGetParams {
                        session_id: session_id.clone(),
                    },
                    Duration::from_secs(5),
                )
                .await
                .ok()
                .map(|result| result.cwd)
                .unwrap_or_default();
            let params = FilesListParams {
                session_id,
                path: if path == "." { None } else { Some(path) },
            };
            match conn
                .call::<FilesListParams, FilesListResult>(
                    METHOD_FILES_LIST,
                    params,
                    Duration::from_secs(5),
                )
                .await
            {
                Ok(result) => {
                    let _ = ui_sender.send(UiMessage::FilesListed {
                        session_id: request_session,
                        path: result.path,
                        workdir,
                        entries: result.entries,
                    });
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::Toast(format!("Files list failed: {}", err)));
                }
            }
        });
    }

    pub fn preview_file(&self, path: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        self.right_dock.files_panel().locate(path);
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let path = path.to_string();
        let request_session = session_id.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let stat = conn
                .call::<FilesStatParams, FilesStatResult>(
                    METHOD_FILES_STAT,
                    FilesStatParams {
                        session_id: session_id.clone(),
                        path: path.clone(),
                    },
                    Duration::from_secs(5),
                )
                .await;
            match stat {
                Ok(stat) => {
                    let _ = ui_sender.send(UiMessage::FileStat { session_id: request_session.clone(), stat });
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::FilePreviewUnavailable {
                        session_id: request_session.clone(),
                        message: format!("Stat failed: {}", err),
                    });
                    return;
                }
            }
            if crate::ui::files_panel::is_image_path(&path) {
                match conn
                    .call::<FilesReadBytesParams, FilesReadBytesResult>(
                        METHOD_FILES_READ_BYTES,
                        FilesReadBytesParams {
                            session_id,
                            path: path.clone(),
                            max_bytes: None,
                        },
                        Duration::from_secs(10),
                    )
                    .await
                {
                    Ok(result) => match crate::ui::files_panel::read_bytes_result(&result) {
                        Some(bytes) => {
                            let _ = ui_sender.send(UiMessage::FilePreviewImage { session_id: request_session.clone(), path, bytes });
                        }
                        None => {
                            let _ = ui_sender.send(UiMessage::FilePreviewUnavailable {
                                session_id: request_session.clone(),
                                message: "Invalid image data".into(),
                            });
                        }
                    },
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::FilePreviewUnavailable {
                            session_id: request_session.clone(),
                            message: format!("Read failed: {}", err),
                        });
                    }
                }
            } else {
                match conn
                    .call::<FilesReadParams, FilesReadResult>(
                        METHOD_FILES_READ,
                        FilesReadParams {
                            session_id,
                            path: path.clone(),
                            max_bytes: None,
                        },
                        Duration::from_secs(10),
                    )
                    .await
                {
                    Ok(result) => match crate::ui::files_panel::read_text_result(&result) {
                        Some((text, truncated)) => {
                            let _ = ui_sender.send(UiMessage::FilePreviewText {
                                session_id: request_session.clone(),
                                path,
                                text,
                                truncated,
                            });
                        }
                        None => {
                            let _ = ui_sender.send(UiMessage::FilePreviewUnavailable {
                                session_id: request_session.clone(),
                                message: "Binary file — preview unavailable".to_string(),
                            });
                        }
                    },
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::FilePreviewUnavailable {
                            session_id: request_session.clone(),
                            message: format!("Read failed: {}", err),
                        });
                    }
                }
            }
        });
    }

    pub fn open_link_target(&self, target: &str) {
        match crate::links::classify_link(target) {
            crate::links::LinkTarget::External(uri) => {
                if let Err(err) = gtk4::gio::AppInfo::launch_default_for_uri(
                    &uri,
                    None::<&gtk4::gio::AppLaunchContext>,
                ) {
                    let _ = self
                        .ui_sender
                        .send(UiMessage::Toast(format!("Open link failed: {}", err)));
                }
            }
            crate::links::LinkTarget::File { path, line, .. } => {
                let resolved = if std::path::Path::new(&path).is_absolute() {
                    path
                } else {
                    let cwd = self
                        .active_session
                        .borrow()
                        .as_ref()
                        .map(|session| session.cwd.clone())
                        .unwrap_or_else(|| ".".to_string());
                    std::path::Path::new(&cwd).join(path).display().to_string()
                };
                self.right_dock.preview_panel().set_pending_line(line);
                self.preview_file(&resolved);
            }
            crate::links::LinkTarget::Blocked => {}
        }
    }

    pub fn save_file_as(&self, path: &str) {
        let dialog = gtk4::FileDialog::builder()
            .title("Save File As")
            .initial_name(
                std::path::Path::new(path)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "download".to_string()),
            )
            .build();
        let weak = self.weak_self();
        let path = path.to_string();
        dialog.save(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |result| {
                let Ok(file) = result else {
                    return;
                };
                let Some(destination) = file.path() else {
                    return;
                };
                let Some(ctrl) = weak.upgrade() else {
                    return;
                };
                ctrl.download_file(&path, &destination);
            },
        );
    }

    fn download_file(&self, path: &str, destination: &std::path::Path) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = FilesReadBytesParams {
            session_id,
            path: path.to_string(),
            max_bytes: Some(8 * 1024 * 1024),
        };
        let destination = destination.to_path_buf();
        let ui_sender = self.ui_sender.clone();
        self.dispatch_call(
            METHOD_FILES_READ_BYTES,
            params,
            Duration::from_secs(30),
            move |result: Result<FilesReadBytesResult, ConnectionError>| match result {
                Ok(read) => match crate::attachments::decode_base64(&read.data) {
                    Some(bytes) => match std::fs::write(&destination, bytes) {
                        Ok(_) => UiMessage::Toast(format!("Saved {}", destination.display())),
                        Err(err) => UiMessage::Toast(format!("Save failed: {}", err)),
                    },
                    None => UiMessage::Toast("Invalid file data".to_string()),
                },
                Err(err) => UiMessage::Toast(format!("Read failed: {}", err)),
            },
        );
        let _ = ui_sender;
    }

    pub fn select_agent_preset(&self, preset: &str) {
        *self.active_preset.borrow_mut() = preset.to_string();
        let label = match preset {
            "standard" => "标准模式",
            "ptc" => "PTC 模式",
            "minimal" => "极简模式",
            "cordis" => "创造模式",
            _ => preset,
        };
        self.conversation_view.set_preset_label(label);
    }

    pub fn start_new_session(&self) {
        self.start_session_in(None, None, None);
    }

    pub fn start_plugin_creator_session(&self) {
        self.start_session_in(
            None,
            Some("cordis"),
            Some("创建一个带左侧栏入口的 DSH 插件。"),
        );
    }

    pub fn start_session_in_workspace(&self, workspace_id: &str) {
        self.start_session_in(Some(workspace_id), None, None);
    }

    fn start_session_in(
        &self,
        workspace_id: Option<&str>,
        preset: Option<&str>,
        draft: Option<&str>,
    ) {
        let target = {
            let workspaces = self.workspaces.borrow();
            let sessions = self.sessions_cache.borrow();
            resolve_new_session_target(
                workspace_id,
                self.active_session_id().as_deref(),
                &workspaces,
                &sessions,
            )
            .cloned()
        };
        let Some(workspace) = target else {
            if preset.is_some() || draft.is_some() {
                let _ = self.ui_sender.send(UiMessage::Toast(
                    Copybook::for_code("system").workspace_required.to_string(),
                ));
            } else {
                self.panel_state.set_active_panel(None);
                self.show_blank_session_view();
            }
            return;
        };
        let blank = {
            let sessions = self.sessions_cache.borrow();
            find_reusable_blank(&workspace, &sessions, |_| true).map(|item| item.id.clone())
        };
        if let Some(prompt) = draft {
            *self.pending_draft.borrow_mut() = Some(PendingDraft {
                prompt: prompt.to_string(),
                cwd: Some(workspace.path.clone()),
                session_id: blank.clone(),
            });
        }
        self.create_and_follow_session(&workspace.path, blank, preset);
    }

    fn create_and_follow_session(
        &self,
        cwd: &str,
        session_id: Option<String>,
        preset: Option<&str>,
    ) {
        self.remember_composer_draft();
        let generation = self.next_follow_generation();
        let cwd_string = cwd.to_string();
        let conn_arc = self.connection.clone();
        let model = self.active_model.borrow().clone();
        let provider = self.active_provider.borrow().clone();
        let model_explicit = self.model_explicit.get();
        let preset = preset
            .map(str::to_string)
            .unwrap_or_else(|| self.active_preset.borrow().clone());
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let create_params = SessionCreateParams {
                    session_id,
                    cwd: cwd_string,
                    provider: if model_explicit { Some(provider) } else { None },
                    model: if model_explicit { Some(model) } else { None },
                    agent_preset: Some(preset),
                };

                if let Ok(create_res) = conn
                    .call::<SessionCreateParams, SessionCreateResult>(
                        METHOD_SESSION_CREATE,
                        create_params,
                        Duration::from_secs(10),
                    )
                    .await
                {
                    let sess_id = create_res.session_id;
                    match conn
                        .call::<SessionFollowParams, SessionFollowResult>(
                            METHOD_SESSION_FOLLOW,
                            SessionFollowParams {
                                session_id: sess_id,
                                from_seq: None,
                            },
                            Duration::from_secs(10),
                        )
                        .await
                    {
                        Ok(follow_res) => {
                            let _ = ui_sender.send(UiMessage::SessionFollowed {
                                follow: follow_res,
                                generation,
                            });
                        }
                        Err(_) => {
                            let _ = ui_sender.send(UiMessage::PendingDraftExpired);
                        }
                    }
                    let _ = ui_sender.send(UiMessage::SessionListRefresh);
                } else {
                    let _ = ui_sender.send(UiMessage::PendingDraftExpired);
                }
            } else {
                let _ = ui_sender.send(UiMessage::PendingDraftExpired);
            }
        });
        self.load_workspaces();
    }

    pub fn reveal_workspace(&self, workspace_id: &str) {
        let Some(workspace) = self
            .workspaces
            .borrow()
            .iter()
            .find(|ws| ws.id == workspace_id)
            .cloned()
        else {
            return;
        };
        let launcher = gtk4::FileLauncher::new(Some(&gtk4::gio::File::for_path(&workspace.path)));
        let window_for_reveal = self.window.clone();
        launcher.open_containing_folder(
            Some(&window_for_reveal),
            None::<&gtk4::gio::Cancellable>,
            move |result| {
                if let Err(err) = result {
                    eprintln!("reveal workspace failed: {err}");
                }
            },
        );
    }
    fn next_follow_generation(&self) -> u64 {
        let next = self.follow_generation.get().wrapping_add(1);
        self.follow_generation.set(next);
        next
    }

    pub fn follow_session(&self, session_id: &str) {
        *self.gap_repair.borrow_mut() = None;
        if self.active_session_id().as_deref() != Some(session_id) {
            self.remember_composer_draft();
        }
        self.conversation_view.set_breadcrumb("");
        let generation = self.next_follow_generation();
        let sess_id = session_id.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                if let Ok(follow_res) = conn
                    .call::<SessionFollowParams, SessionFollowResult>(
                        METHOD_SESSION_FOLLOW,
                        SessionFollowParams {
                            session_id: sess_id,
                            from_seq: None,
                        },
                        Duration::from_secs(10),
                    )
                    .await
                {
                    let _ = ui_sender.send(UiMessage::SessionFollowed {
                        follow: follow_res,
                        generation,
                    });
                }
            }
        });
    }

    pub fn open_child_session(&self, child_session_id: &str) {
        self.follow_session(child_session_id);
        self.conversation_view
            .set_breadcrumb(&format!("Subagent · {}", child_session_id));
    }

    pub fn archive_session(&self, session_id: &str) {
        let sess_id = session_id.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let params = SessionArchiveParams {
                    session_id: sess_id,
                    archived: true,
                };
                let _ = conn
                    .call::<SessionArchiveParams, SessionArchiveResult>(
                        METHOD_SESSION_ARCHIVE,
                        params,
                        Duration::from_secs(5),
                    )
                    .await;
                let _ = ui_sender.send(UiMessage::Toast("Session archived".to_string()));
            }
        });
        self.load_session_list();
    }

    pub fn cancel_current_turn(&self) {
        let sess_id_opt = self.active_session.borrow().as_ref().map(|s| s.id.clone());

        if let Some(sess_id) = sess_id_opt {
            let conn_arc = self.connection.clone();
            tokio_runtime().spawn(async move {
                let conn_opt = conn_arc.read().await.clone();
                if let Some(conn) = conn_opt {
                    let _ = conn
                        .call::<SessionCancelParams, SessionCancelResult>(
                            METHOD_SESSION_CANCEL,
                            SessionCancelParams {
                                session_id: sess_id,
                                reason: Some("user cancellation".to_string()),
                            },
                            Duration::from_secs(5),
                        )
                        .await;
                }
            });
        }
    }

    pub fn switch_model(&self, provider: &str, model: &str) {
        let previous_provider = self.active_provider.borrow().clone();
        let previous_model = self.active_model.borrow().clone();
        self.model_explicit.set(true);
        *self.active_model.borrow_mut() = model.to_string();
        *self.active_provider.borrow_mut() = provider.to_string();
        self.conversation_view.composer().set_model_label(model);

        let Some(sess_id) = self.active_session_id() else {
            return;
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let prov = provider.to_string();
        let mdl = model.to_string();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let params = SessionModelSetParams {
                session_id: sess_id,
                provider: prov.clone(),
                model: mdl.clone(),
                reasoning_effort: None,
            };
            if conn
                .call::<SessionModelSetParams, serde_json::Value>(
                    METHOD_SESSION_MODEL_SET,
                    params,
                    Duration::from_secs(5),
                )
                .await
                .is_err()
            {
                let _ = ui_sender.send(UiMessage::ModelSwitchFailed {
                    provider: previous_provider,
                    model: previous_model,
                });
            }
        });
    }

    pub fn refresh_session_model(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let request_session = session_id.clone();
        self.dispatch_call(
            METHOD_SESSION_MODEL_GET,
            SessionModelGetParams { session_id },
            Duration::from_secs(5),
            move |result: Result<SessionModelGetResult, ConnectionError>| match result {
                Ok(model) => UiMessage::SessionModelLoaded {
                    session_id: request_session.clone(),
                    model,
                },
                Err(_) => UiMessage::Toast("Model state unavailable".to_string()),
            },
        );
    }

    pub fn open_model_picker(&self, anchor: &impl IsA<gtk4::Widget>) {
        let popover = gtk4::Popover::builder().build();
        let box_widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        let loading = gtk4::Label::builder()
            .label("Loading model catalog…")
            .css_classes(["dim-label", "caption"])
            .build();
        box_widget.append(&loading);
        popover.set_child(Some(&box_widget));
        popover.set_parent(anchor);
        popover.popup();

        let conn_arc = self.connection.clone();
        let weak = self.weak_self();
        let (sender, mut receiver) =
            tokio::sync::mpsc::unbounded_channel::<Result<ModelCatalogResult, String>>();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                let _ = sender.send(Err("Not connected".to_string()));
                return;
            };
            let result = conn
                .call::<serde_json::Value, ModelCatalogResult>(
                    METHOD_MODEL_CATALOG,
                    serde_json::json!({}),
                    Duration::from_secs(10),
                )
                .await
                .map_err(|err| err.to_string());
            let _ = sender.send(result);
        });
        let box_for_ui = box_widget.clone();
        let popover_for_ui = popover.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let Some(result) = receiver.recv().await else {
                return;
            };
            while let Some(child) = box_for_ui.first_child() {
                box_for_ui.remove(&child);
            }
            match result {
                Ok(catalog) => {
                    for provider in catalog.providers {
                        let provider_badge = if provider.configured {
                            "configured"
                        } else {
                            "not configured"
                        };
                        let heading = gtk4::Label::builder()
                            .label(&format!("{} · {}", provider.name, provider_badge))
                            .css_classes(["heading"])
                            .halign(gtk4::Align::Start)
                            .build();
                        box_for_ui.append(&heading);
                        for model in provider.models {
                            let btn = gtk4::Button::builder()
                                .label(format!("{} ({})", model.display_name, model.id))
                                .css_classes(["flat"])
                                .halign(gtk4::Align::Fill)
                                .build();
                            let pop_clone = popover_for_ui.clone();
                            let weak_clone = weak.clone();
                            let provider_id = provider.id.clone();
                            let model_id = model.id.clone();
                            btn.connect_clicked(move |_| {
                                if let Some(ctrl) = weak_clone.upgrade() {
                                    ctrl.switch_model(&provider_id, &model_id);
                                }
                                pop_clone.popdown();
                            });
                            box_for_ui.append(&btn);
                        }
                    }
                }
                Err(err) => {
                    let label = gtk4::Label::builder()
                        .label(format!("Model catalog unavailable: {}", err))
                        .wrap(true)
                        .css_classes(["dimension", "caption"])
                        .build();
                    box_for_ui.append(&label);
                }
            }
        });
    }

    pub fn refresh_permission(&self, session_id: &str) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let session_id = session_id.to_string();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            if let Ok(result) = conn
                .call::<PermissionGetParams, PermissionGetResult>(
                    METHOD_PERMISSION_GET,
                    PermissionGetParams { session_id },
                    Duration::from_secs(5),
                )
                .await
            {
                let _ = ui_sender.send(UiMessage::PermissionLoaded(result));
            }
        });
    }

    pub fn switch_permission_preset(&self, preset: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let preset = preset.to_string();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            match conn
                .call::<PermissionSetParams, PermissionSetResult>(
                    METHOD_PERMISSION_SET,
                    PermissionSetParams {
                        session_id: session_id.clone(),
                        preset,
                    },
                    Duration::from_secs(5),
                )
                .await
            {
                Ok(result) => {
                    let _ = ui_sender.send(UiMessage::PermissionValueUpdated {
                        session_id,
                        current_value: result.current_value,
                    });
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::Toast(format!("Permission failed: {}", err)));
                }
            }
        });
    }

    pub fn refresh_permission_catalog(&self) {
        if self.permission_catalog_loading.get() {
            return;
        }
        self.permission_catalog_loading.set(true);
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                let _ = ui_sender.send(UiMessage::PermissionCatalogLoaded(Err(
                    "未连接".to_string(),
                )));
                return;
            };
            let result = conn
                .call::<serde_json::Value, PermissionCatalogResult>(
                    METHOD_PERMISSION_CATALOG,
                    serde_json::json!({}),
                    Duration::from_secs(5),
                )
                .await
                .map_err(|err| err.to_string());
            let _ = ui_sender.send(UiMessage::PermissionCatalogLoaded(result));
        });
    }

    pub fn handle_permission_catalog(
        &self,
        result: Result<PermissionCatalogResult, String>,
    ) {
        self.permission_catalog_loading.set(false);
        match result {
            Ok(catalog) => {
                *self.permission_catalog.borrow_mut() = Some(catalog);
                *self.permission_catalog_error.borrow_mut() = None;
            }
            Err(err) => {
                *self.permission_catalog_error.borrow_mut() = Some(err);
            }
        }
        let reopen = self
            .permission_popover
            .borrow()
            .as_ref()
            .map(|popover| popover.is_visible())
            .unwrap_or(false);
        if !reopen {
            return;
        }
        let anchor = self.permission_anchor.borrow().clone();
        if let Some(popover) = self.permission_popover.borrow_mut().take() {
            popover.popdown();
        }
        if let Some(anchor) = anchor {
            let weak = self.weak_self();
            gtk4::glib::idle_add_local_once(move || {
                if let Some(ctrl) = weak.upgrade() {
                    ctrl.open_permission_picker(&anchor);
                }
            });
        }
    }

    fn permission_catalog_options(&self) -> Vec<PresetOptionItem> {
        if let Some(catalog) = self.permission_catalog.borrow().as_ref() {
            if !catalog.options.is_empty() {
                return catalog.options.clone();
            }
            return catalog.default_options.clone();
        }
        if let Some(state) = self.permission_state.borrow().as_ref() {
            if !state.catalog.options.is_empty() {
                return state.catalog.options.clone();
            }
            return state.catalog.default_options.clone();
        }
        Vec::new()
    }

    fn fill_permission_popover(&self, popover: &gtk4::Popover) {
        let box_widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .width_request(300)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        let options = self.permission_catalog_options();
        if options.is_empty() {
            let error = self.permission_catalog_error.borrow().clone();
            match error {
                Some(message) => {
                    let label = gtk4::Label::builder()
                        .label(format!("无法读取权限预设：{message}"))
                        .wrap(true)
                        .xalign(0.0)
                        .css_classes(["dim-label", "caption"])
                        .margin_start(8)
                        .margin_end(8)
                        .margin_top(6)
                        .margin_bottom(6)
                        .build();
                    box_widget.append(&label);
                    let retry = gtk4::Button::builder()
                        .label("重试")
                        .css_classes(["flat"])
                        .halign(gtk4::Align::Start)
                        .build();
                    let weak = self.weak_self();
                    let popover_clone = popover.clone();
                    retry.connect_clicked(move |_| {
                        if let Some(ctrl) = weak.upgrade() {
                            *ctrl.permission_catalog_error.borrow_mut() = None;
                            ctrl.fill_permission_popover(&popover_clone);
                        }
                    });
                    box_widget.append(&retry);
                }
                None => {
                    let row = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Horizontal)
                        .spacing(8)
                        .margin_start(8)
                        .margin_end(8)
                        .margin_top(8)
                        .margin_bottom(8)
                        .build();
                    row.append(&gtk4::Spinner::builder().spinning(true).build());
                    row.append(&gtk4::Label::new(Some("正在载入权限设置…")));
                    box_widget.append(&row);
                    self.refresh_permission_catalog();
                }
            }
            popover.set_child(Some(&box_widget));
            return;
        }

        let has_session = self.active_session_id().is_some();
        let current = self
            .permission_state
            .borrow()
            .as_ref()
            .map(|state| state.current_value.clone());
        if !has_session {
            box_widget.append(
                &gtk4::Label::builder()
                    .label("打开一个会话后可以切换当前权限")
                    .wrap(true)
                    .xalign(0.0)
                    .css_classes(["dim-label", "caption"])
                    .margin_start(8)
                    .margin_end(8)
                    .margin_top(6)
                    .margin_bottom(6)
                    .build(),
            );
        }
        let weak = self.weak_self();
        for option in options {
            let is_current = current.as_deref() == Some(option.value.as_str());
            let button = gtk4::Button::builder()
                .css_classes(["flat", "dsh-permission-option"])
                .halign(gtk4::Align::Fill)
                .sensitive(has_session)
                .build();
            let content = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(10)
                .margin_start(4)
                .margin_end(4)
                .margin_top(4)
                .margin_bottom(4)
                .build();
            content.append(&gtk4::Image::from_icon_name(permission_preset_icon(
                &option.value,
            )));
            let text = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(1)
                .hexpand(true)
                .build();
            let title_row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .build();
            title_row.append(
                &gtk4::Label::builder()
                    .label(permission_preset_label(&option))
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            if option.value == "auto" {
                title_row.append(
                    &gtk4::Label::builder()
                        .label("EXP")
                        .css_classes(["dsh-plugins-tag", "dsh-plugins-tag-accent"])
                        .build(),
                );
            }
            text.append(&title_row);
            if let Some(description) = option
                .description
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                text.append(
                    &gtk4::Label::builder()
                        .label(description)
                        .wrap(true)
                        .xalign(0.0)
                        .css_classes(["dim-label", "caption"])
                        .build(),
                );
            }
            content.append(&text);
            if is_current {
                content.append(
                    &gtk4::Image::from_icon_name("object-select-symbolic"),
                );
            }
            button.set_child(Some(&content));
            let pop_clone = popover.clone();
            let weak_clone = weak.clone();
            let value = option.value.clone();
            button.connect_clicked(move |_| {
                pop_clone.popdown();
                if let Some(ctrl) = weak_clone.upgrade() {
                    if permission_preset_requires_confirmation(&value) {
                        ctrl.confirm_permission_preset(&value);
                    } else {
                        ctrl.switch_permission_preset(&value);
                    }
                }
            });
            box_widget.append(&button);
        }
        popover.set_child(Some(&box_widget));
    }

    pub fn open_permission_picker(&self, anchor: &impl IsA<gtk4::Widget>) {
        let anchor_widget = anchor.clone().upcast::<gtk4::Widget>();
        *self.permission_anchor.borrow_mut() = Some(anchor_widget);
        let popover = gtk4::Popover::builder()
            .css_classes(["dsh-permission-popover"])
            .build();
        self.fill_permission_popover(&popover);
        *self.permission_popover.borrow_mut() = Some(popover.clone());
        popover.set_parent(anchor);
        popover.popup();
    }

    fn confirm_permission_preset(&self, preset: &str) {
        if self.active_session_id().is_none() {
            let _ = self
                .ui_sender
                .send(UiMessage::Toast("请先打开一个会话".to_string()));
            return;
        }
        let auto = preset == "auto";
        let title = if auto {
            "启用自动审查（实验性）？"
        } else {
            "启用完全权限？"
        };
        let body = if auto {
            "自动审查会在每次工具调用前由同一模型判断是否放行，不进入沙箱。它可能误判放行或拦截，并消耗额外 token。"
        } else {
            "完全权限会减少确认步骤，允许 Agent 直接执行更多操作，包括敏感写入、文件修改和外部命令。仅在信任当前任务时使用。"
        };
        let enable_label = if auto { "启用自动审查" } else { "启用完全权限" };
        let dialog = adw::AlertDialog::builder()
            .heading(title)
            .body(body)
            .build();
        dialog.add_response("cancel", "取消");
        dialog.add_response("enable", enable_label);
        dialog.set_response_appearance("enable", adw::ResponseAppearance::Destructive);
        dialog.set_response_enabled("enable", false);
        dialog.set_close_response("cancel");
        let check = gtk4::CheckButton::with_label("我了解风险，仍要继续");
        dialog.set_extra_child(Some(&check));
        let dialog_for_check = dialog.clone();
        check.connect_toggled(move |check| {
            dialog_for_check.set_response_enabled("enable", check.is_active());
        });
        let weak = self.weak_self();
        let preset = preset.to_string();
        dialog.choose(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |response| {
                if response == "enable" {
                    if let Some(ctrl) = weak.upgrade() {
                        ctrl.switch_permission_preset(&preset);
                    }
                }
            },
        );
    }

    pub fn apply_permission_state(&self, state: PermissionGetResult) {
        let label = permission_display_label(&state);
        self.conversation_view
            .composer()
            .set_permission_label(&label);
        *self.permission_state.borrow_mut() = Some(state);
    }

    pub fn update_permission_value(&self, session_id: &str, current_value: &str) {
        if self.active_session_id().as_deref() != Some(session_id) {
            return;
        }
        let mut guard = self.permission_state.borrow_mut();
        if let Some(state) = guard.as_mut() {
            state.current_value = current_value.to_string();
            let label = permission_display_label(state);
            self.conversation_view
                .composer()
                .set_permission_label(&label);
        } else {
            let label = permission_preset_builtin_label(current_value)
                .map(str::to_string)
                .unwrap_or_else(|| current_value.to_string());
            self.conversation_view
                .composer()
                .set_permission_label(&label);
        }
    }

    pub fn inject_credential(&self, provider: &str, key: &str) {
        let prov = provider.to_string();
        let k = key.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let params = CredentialsInjectParams {
                    provider: prov.clone(),
                    key: k,
                    base_url: None,
                };
                let _ = conn
                    .call::<CredentialsInjectParams, CredentialsInjectResult>(
                        METHOD_CREDENTIALS_INJECT,
                        params,
                        Duration::from_secs(5),
                    )
                    .await;
                let _ = ui_sender.send(UiMessage::Toast(format!("{} injected", prov)));
            }
        });
    }

    pub fn rotate_credential(&self, provider: &str, key: &str) {
        let prov = provider.to_string();
        let k = key.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let params = CredentialsRotateParams {
                    provider: prov.clone(),
                    key: k,
                };
                let rotated = conn
                    .call::<CredentialsRotateParams, CredentialsInjectResult>(
                        METHOD_CREDENTIALS_ROTATE,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                    .is_ok();
                let _ = ui_sender.send(UiMessage::Toast(format!("{} rotated in memory", prov)));
                if rotated {
                    let _ = ui_sender.send(UiMessage::RefreshModelCatalog);
                }
            }
        });
    }

    pub fn load_plugin_inventory(&self) {
        self.dispatch_call(
            METHOD_PLUGIN_INVENTORY,
            serde_json::json!({}),
            Duration::from_secs(15),
            |result: Result<PluginInventoryResult, ConnectionError>| {
                UiMessage::PluginsInventoryLoaded(result.map_err(|err| err.to_string()))
            },
        );
    }

    pub fn toggle_plugin_bundle(&self, name: &str, enable: bool) {
        let method = if enable {
            METHOD_PLUGIN_BUNDLE_ENABLE
        } else {
            METHOD_PLUGIN_BUNDLE_DISABLE
        };
        let params = PluginBundleToggleParams {
            name: name.to_string(),
        };
        let key = name.to_string();
        self.dispatch_call(
            method,
            params,
            Duration::from_secs(30),
            move |result: Result<PluginChangeResult, ConnectionError>| {
                UiMessage::PluginsToggleDone {
                    key,
                    result: result.map_err(|err| err.to_string()),
                }
            },
        );
    }

    pub fn toggle_plugin_row(&self, entry_id: &str, enable: bool) {
        let method = if enable {
            METHOD_PLUGIN_ENABLE
        } else {
            METHOD_PLUGIN_DISABLE
        };
        let params = PluginToggleParams {
            id: entry_id.to_string(),
        };
        let key = format!("row:{entry_id}");
        self.dispatch_call(
            method,
            params,
            Duration::from_secs(30),
            move |result: Result<PluginChangeResult, ConnectionError>| {
                UiMessage::PluginsToggleDone {
                    key,
                    result: result.map_err(|err| err.to_string()),
                }
            },
        );
    }

    pub fn remove_plugin(&self, name: &str) {
        let params = PluginRemoveParams {
            name: name.to_string(),
        };
        let name = name.to_string();
        self.dispatch_call(
            METHOD_PLUGIN_REMOVE,
            params,
            Duration::from_secs(120),
            move |result: Result<PluginChangeResult, ConnectionError>| {
                UiMessage::PluginsRemoveDone {
                    name,
                    result: result.map_err(|err| err.to_string()),
                }
            },
        );
    }

    pub fn plugin_inspect(&self, spec: &str, registry: Option<String>) {
        let params = PluginInspectParams {
            spec: spec.to_string(),
            registry,
        };
        self.dispatch_call(
            METHOD_PLUGIN_INSPECT,
            params,
            Duration::from_secs(30),
            |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        serde_json::from_value::<PluginSpecInspection>(op.result)
                            .map_err(|e| e.to_string())
                    });
                UiMessage::PluginInspectDone(parsed)
            },
        );
    }

    pub fn plugin_install(
        &self,
        spec: &str,
        enabled: Option<bool>,
        save_exact: Option<bool>,
        request_id: Option<String>,
        approved_builds: Option<Vec<String>>,
        registry: Option<String>,
    ) {
        let req_id_clone = request_id.clone();
        let params = PluginInstallParams {
            spec: spec.to_string(),
            enabled,
            save_exact,
            request_id,
            approved_builds,
            registry,
        };
        self.dispatch_call(
            METHOD_PLUGIN_INSTALL,
            params,
            Duration::from_secs(600),
            move |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        serde_json::from_value::<PluginChangeResult>(op.result)
                            .map_err(|e| e.to_string())
                    });
                UiMessage::PluginInstallDone {
                    request_id: req_id_clone,
                    result: parsed,
                }
            },
        );
    }

    pub fn plugin_cancel_install(&self, request_id: &str) {
        let params = PluginRequestParams {
            request_id: request_id.to_string(),
        };
        self.dispatch_call(
            METHOD_PLUGIN_CANCEL_INSTALL,
            params,
            Duration::from_secs(30),
            |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        serde_json::from_value::<PluginCancellationResult>(op.result)
                            .map_err(|e| e.to_string())
                    });
                UiMessage::PluginCancelInstallDone(parsed)
            },
        );
    }

    pub fn plugin_wait_install(&self, request_id: &str) {
        let params = PluginRequestParams {
            request_id: request_id.to_string(),
        };
        self.dispatch_call(
            METHOD_PLUGIN_WAIT_INSTALL,
            params,
            Duration::from_secs(120),
            |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        if op.result.is_null() {
                            Ok(None)
                        } else {
                            serde_json::from_value::<PluginChangeResult>(op.result)
                                .map(Some)
                                .map_err(|e| e.to_string())
                        }
                    });
                UiMessage::PluginWaitInstallDone(parsed)
            },
        );
    }

    pub fn plugin_registries(&self) {
        self.dispatch_call(
            METHOD_PLUGIN_REGISTRIES,
            serde_json::json!({}),
            Duration::from_secs(15),
            |result: Result<PluginRegistriesResult, ConnectionError>| {
                UiMessage::PluginRegistriesLoaded(
                    result.map(|res| res.result).map_err(|err| err.to_string()),
                )
            },
        );
    }

    pub fn plugin_registry_probe(&self) {
        self.dispatch_call(
            METHOD_PLUGIN_REGISTRY_PROBE,
            serde_json::json!({}),
            Duration::from_secs(15),
            |result: Result<PluginRegistryProbeResult, ConnectionError>| {
                UiMessage::PluginRegistryProbeDone(
                    result.map(|res| res.result).map_err(|err| err.to_string()),
                )
            },
        );
    }

    pub fn plugin_versions(&self, name: &str, registry: Option<String>) {
        let params = PluginVersionsParams {
            name: name.to_string(),
            registry,
        };
        let pkg_name = name.to_string();
        self.dispatch_call(
            METHOD_PLUGIN_VERSIONS,
            params,
            Duration::from_secs(30),
            move |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        serde_json::from_value::<PluginVersionList>(op.result)
                            .map_err(|e| e.to_string())
                    });
                UiMessage::PluginVersionsLoaded {
                    name: pkg_name,
                    result: parsed,
                }
            },
        );
    }

    pub fn plugin_update(
        &self,
        name: &str,
        version: &str,
        request_id: Option<String>,
        registry: Option<String>,
    ) {
        let params = PluginUpdateParams {
            name: name.to_string(),
            version: version.to_string(),
            request_id,
            registry,
        };
        self.dispatch_call(
            METHOD_PLUGIN_UPDATE,
            params,
            Duration::from_secs(600),
            |result: Result<PluginOperationResult, ConnectionError>| {
                let parsed = result
                    .map_err(|err| err.to_string())
                    .and_then(|op| {
                        serde_json::from_value::<PluginUpdateResult>(op.result)
                            .map_err(|e| e.to_string())
                    });
                UiMessage::PluginUpdateDone(parsed)
            },
        );
    }

    pub fn restart_supervisor(&self) {
        let sup_arc = self.supervisor.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let sup_opt = sup_arc.read().await.clone();
            if let Some(sup) = sup_opt {
                let _ = sup.stop().await;
                if let Ok(init) = sup.start().await {
                    if let Some(conn) = sup.connection().await {
                        let _ = ui_sender.send(UiMessage::SupervisorRestarted {
                            connection: conn,
                            init: Some(init),
                        });
                    }
                }
            }
        });
    }

    pub fn next_reconnect_delay_ms(attempt: u32) -> u64 {
        const CAP_MS: u64 = 30_000;
        let step = attempt.min(5);
        1000u64.saturating_mul(1u64 << step).min(CAP_MS)
    }

    pub fn start_reconnect_loop(&self, immediate: bool) {
        if self.reconnect_active.get() {
            return;
        }
        self.reconnect_active.set(true);
        let generation = self.reconnect_generation.get().wrapping_add(1);
        self.reconnect_generation.set(generation);
        let sup_arc = self.supervisor.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let mut attempt: u32 = 0;
            loop {
                let delay_ms = if attempt == 0 && immediate {
                    0
                } else {
                    Self::next_reconnect_delay_ms(attempt)
                };
                if delay_ms > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                }
                let sup_opt = sup_arc.read().await.clone();
                let Some(sup) = sup_opt else {
                    return;
                };
                let mut connection = None;
                let mut init = None;
                if let Some(path) = sup.socket_path().await {
                    if let Ok(conn) = BridgeConnection::connect_unix(&path).await {
                        connection = Some(conn);
                    }
                }
                if connection.is_none() && attempt % 3 == 2 {
                    let _ = sup.stop().await;
                    if let Ok(result) = sup.start().await {
                        init = Some(result);
                        connection = sup.connection().await;
                    }
                }
                if let Some(conn) = connection {
                    let _ = ui_sender.send(UiMessage::SupervisorRestarted {
                        connection: conn,
                        init,
                    });
                    return;
                }
                attempt += 1;
            }
        });
    }

    fn setup_ui_receiver(
        &self,
        mut receiver: UnboundedReceiver<UiMessage>,
        ctrl_weak: std::rc::Weak<SessionController>,
    ) {
        let win = self.window.clone();
        let sidebar = self.sidebar.clone();
        let conv_view = self.conversation_view.clone();
        let window = self.window.clone();
        let toast_overlay = self.toast_overlay.clone();
        let active_sess = self.active_session.clone();
        let store = self.credential_store.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            while let Some(msg) = receiver.recv().await {
                match msg {
                    UiMessage::Approval { req, tx } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.enqueue_approval(req, Some(tx));
                        }
                    }
                    UiMessage::UserQuestion { req, tx } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.enqueue_question(req, Some(tx));
                        }
                    }
                    UiMessage::RefreshModelCatalog => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if let Some(handle) = ctrl.settings_dialog.borrow().as_ref() {
                                handle.refresh_model_catalog();
                            }
                        }
                    }
                    UiMessage::SessionListRefresh => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.load_session_list();
                        }
                    }
                    UiMessage::PendingDraftExpired => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.pending_draft.borrow_mut().take();
                        }
                    }
                    UiMessage::SessionListLoaded(sessions) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.handle_session_list_loaded(&sessions);
                        }
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            *ctrl.sessions_cache.borrow_mut() = sessions.clone();
                            ctrl.refresh_sidebar_sessions();
                            sidebar.load_workspaces(&ctrl.workspaces.borrow());
                        }
                    }
                    UiMessage::WorkspacesLoaded(workspaces) => {
                        conv_view.set_hero_workspaces(&workspaces);
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            *ctrl.workspaces.borrow_mut() = workspaces.clone();
                            sidebar.load_workspaces(&workspaces);
                        }
                    }
                    UiMessage::WorkspaceCreated(path) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            let workspace = ctrl
                                .workspaces
                                .borrow()
                                .iter()
                                .find(|ws| ws.path == path)
                                .map(|ws| ws.id.clone());
                            match workspace {
                                Some(id) => ctrl.start_session_in_workspace(&id),
                                None => ctrl.load_workspaces(),
                            }
                        }
                    }
                    UiMessage::WorkspaceChanged => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.load_workspaces();
                        }
                    }
                    UiMessage::SessionFollowed {
                        follow,
                        generation,
                    } => {
                        let current_generation = ctrl_weak
                            .upgrade()
                            .map(|ctrl| ctrl.follow_generation.get() == generation)
                            .unwrap_or(false);
                        if !current_generation {
                            continue;
                        }
                        let title = follow
                            .title
                            .as_ref()
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| "New Session".to_string());
                        let cwd = follow.header.cwd.clone();
                        let status = follow.status.clone();
                        let session_id = follow.header.id.clone();
                        let latest_seq = follow.latest_seq;

                        window.set_title(Some(&title));
                        conv_view.set_title(&title);
                        conv_view.set_cwd(&cwd);
                        conv_view.set_status(if status == "running" {
                            "Running"
                        } else {
                            "Idle"
                        });

                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            let previous_id = ctrl.active_session_id();
                            ctrl.unread_sessions.borrow_mut().remove(&session_id);
                            if let Some(previous) = ctrl.active_session_id() {
                                ctrl.remember_dock_for_session(&previous);
                            }
                            ctrl.handle_follow_status(&session_id, &status);
                            ctrl.persist_session(&session_id, &cwd);
                            ctrl.refresh_queue(&session_id);
                            ctrl.sidebar.set_active_session(Some(&session_id));
                            ctrl.panel_state.set_active_panel(None);
                            if let Some(pending) = ctrl.pending_draft.borrow_mut().take() {
                                if resolve_pending_draft(&pending, &session_id, Some(&cwd))
                                    == DraftDisposition::Consume
                                {
                                    ctrl.set_composer_text(&pending.prompt);
                                }
                            } else if previous_id.as_deref() != Some(session_id.as_str()) {
                                ctrl.restore_composer_draft(&session_id);
                            }
                        }

                        conv_view.reset_meters();
                        let switched = active_sess
                            .borrow()
                            .as_ref()
                            .map(|session| session.id != session_id)
                            .unwrap_or(true);
                        let sess_model = SessionModel::from_follow(follow);
                        if switched {
                            conv_view.reset_scroll_follow();
                            conv_view.rerender_transcript(&sess_model.transcript);
                        } else {
                            conv_view.update_transcript(&sess_model.transcript);
                        }
                        *active_sess.borrow_mut() = Some(sess_model);
                        if let Some(sess) = active_sess.borrow().as_ref() {
                            conv_view.composer().set_plan_mode(sess.plan_mode);
                            conv_view
                                .trajectory_view()
                                .set_events(&sess.raw_events().iter().cloned().collect::<Vec<_>>());
                        }
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.update_dock_button();
                            ctrl.right_dock.preview_panel().clear();
                            ctrl.refresh_files(".");
                            ctrl.refresh_permission(&session_id);
                            ctrl.apply_session_view(&session_id);
                            ctrl.update_continue_state();
                            ctrl.right_dock.on_session_followed(&session_id, &cwd);
                            ctrl.restore_dock_for_session(&session_id);
                            ctrl.load_session_window(&session_id, latest_seq);
                            ctrl.load_projections(&session_id);
                            ctrl.refresh_session_model();
                            ctrl.load_goal(&session_id);
                            ctrl.refresh_subagents();
                            ctrl.refresh_jobs();
                            ctrl.load_feedback();
                            if let Some(text) = ctrl.pending_auto_send.borrow_mut().take() {
                                let buffer = conv_view.composer().text_view().buffer();
                                buffer.set_text("");
                                ctrl.submit_text(&text, false);
                            }
                            ctrl.refresh_takeover();
                        }
                    }
                    UiMessage::Notification(notif) => match notif {
                        IncomingNotification::SessionEvent(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                if ctrl.buffer_event_if_repairing(&p.session_id, p.event.clone()) {
                                    continue;
                                }
                            }
                            let pending_gap = {
                                let mut sess_guard = active_sess.borrow_mut();
                                match sess_guard.as_mut() {
                                    Some(sess) if p.session_id == sess.id => {
                                        let event = p.event;
                                        let raw_event = event.clone();
                                        match sess.handle_event(event.clone()) {
                                            Ok(_) => {
                                                conv_view.update_transcript(&sess.transcript);
                                                conv_view
                                                    .composer()
                                                    .set_plan_mode(sess.plan_mode);
                                                conv_view
                                                    .trajectory_view()
                                                    .append_event(raw_event);
                                                None
                                            }
                                            Err(gap) => Some((gap, event)),
                                        }
                                    }
                                    _ => None,
                                }
                            };
                            if let Some((gap, event)) = pending_gap {
                                if let Some(ctrl) = ctrl_weak.upgrade() {
                                    ctrl.begin_gap_repair(
                                        &p.session_id,
                                        gap.expected,
                                        gap.actual,
                                        event,
                                    );
                                }
                            } else if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.update_continue_state();
                            }
                        }
                        IncomingNotification::AgentStream(p) => {
                            let mut sess_guard = active_sess.borrow_mut();
                            if let Some(sess) = sess_guard.as_mut() {
                                if p.session_id == sess.id {
                                    if let crate::protocol::AssistantStreamFrame::Chunk {
                                        chunk: crate::protocol::StreamChunk::Usage { usage },
                                        ..
                                    } = &p.frame
                                    {
                                        conv_view.set_usage(usage);
                                    }
                                    sess.handle_stream_frame(p.frame);
                                    conv_view.update_transcript(&sess.transcript);
                                }
                            }
                        }
                        IncomingNotification::SessionStatus(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.handle_status_notification(&p.session_id, &p.status);
                                ctrl.update_continue_state();
                            }
                        }
                        IncomingNotification::UserQuestionCancelled(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.cancel_pending_question(&p.session_id, &p.request_id);
                            }
                        }
                        IncomingNotification::SessionTitle(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.cache_session_title(&p.session_id, &p.title);
                                ctrl.update_session_title(&p.session_id, &p.title);
                            }
                            let mut sess_guard = active_sess.borrow_mut();
                            if let Some(sess) = sess_guard.as_mut() {
                                if p.session_id == sess.id {
                                    sess.handle_title(&p.title);
                                    window.set_title(Some(&p.title));
                                    conv_view.set_title(&p.title);
                                }
                            }
                        }
                        IncomingNotification::SessionInbox(p) => {
                            let matches_active = active_sess
                                .borrow()
                                .as_ref()
                                .map(|sess| sess.id == p.session_id)
                                .unwrap_or(false);
                            if matches_active {
                                conv_view.update_queue(&p.items);
                            }
                        }
                        IncomingNotification::SessionProjection(p) => {
                            if matches_active_session(&active_sess, &p.session_id) {
                                match p.kind.as_str() {
                                    "contextPressure" => {
                                        conv_view.set_context(Some(&p.value), None);
                                    }
                                    "contextBreakdown" => {
                                        conv_view.set_context(None, Some(&p.value));
                                    }
                                    "tokenUsage" => conv_view.set_usage(&p.value),
                                    _ => {}
                                }
                            }
                        }
                        IncomingNotification::JobOutput(p) => {
                            let matches_active = active_sess
                                .borrow()
                                .as_ref()
                                .map(|sess| sess.id == p.session_id)
                                .unwrap_or(false);
                            if matches_active {
                                conv_view.job_output_append(&p.chunks);
                            }
                        }
                        IncomingNotification::ScheduleChanged(_) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.load_schedules();
                            }
                        }
                        IncomingNotification::PluginInstallState(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.conversation_view
                                    .plugins_page()
                                    .handle_install_state(&p.request_id, &p.phase);
                            }
                        }
                        IncomingNotification::PluginInstallLog(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.conversation_view
                                    .plugins_page()
                                    .handle_install_log_chunk(
                                        p.request_id.as_deref(),
                                        &p.job_id,
                                        &p.argv,
                                        &p.cwd,
                                        &p.text,
                                        p.exit_code,
                                    );
                            }
                        }
                        IncomingNotification::PluginChanged(_) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.conversation_view.plugins_page().reload_after_change();
                            }
                        }
                        IncomingNotification::PluginUpdateState(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.conversation_view
                                    .plugins_page()
                                    .handle_update_state(&p.request_id, &p.step);
                            }
                        }
                        IncomingNotification::PluginMarketChanged(_) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.conversation_view.plugins_page().probe_market();
                            }
                        }
                        IncomingNotification::SettingsUpdated(params) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                if let Some(handle) = ctrl.settings_dialog.borrow().as_ref() {
                                    handle.apply_revision(&params.namespace, params.revision);
                                    handle.refresh();
                                }
                                ctrl.conversation_view.plugins_page().reload_forms();
                            }
                        }
                        IncomingNotification::GoalUpdated(p) => {
                            if matches_active_session(&active_sess, &p.session_id) {
                                conv_view.goal_bar().set_goal(p.goal);
                            }
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                if let Some(handle) = ctrl.settings_dialog.borrow().as_ref() {
                                    handle.refresh();
                                }
                            }
                        }
                        IncomingNotification::PermissionUpdated(p) => {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.update_permission_value(&p.session_id, &p.current_value);
                            }
                        }
                        _ => {}
                    },
                    UiMessage::Toast(msg) => {
                        toast_overlay.add_toast(adw::Toast::new(&msg));
                    }
                    UiMessage::ShowOnboarding => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            show_onboarding_dialog(&win, ctrl, store.clone());
                        }
                    }
                    UiMessage::SupervisorRestarted { connection, init } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.reconnect_active.set(false);
                            ctrl.store_init_result(init);
                            ctrl.show_ready();
                            ctrl.set_connection(connection);
                            let sess_id_opt =
                                { active_sess.borrow().as_ref().map(|s| s.id.clone()) };
                            if let Some(id) = sess_id_opt {
                                ctrl.follow_session(&id);
                            }
                            ctrl.conversation_view.plugins_page().on_connection_ready();
                            ctrl.refresh_permission_catalog();
                            if let Some(handle) = ctrl.settings_dialog.borrow().as_ref() {
                                handle.refresh_version_info(ctrl.init_result.borrow().as_ref());
                            }
                        }
                        toast_overlay.add_toast(adw::Toast::new("dsh reconnected"));
                    }
                    UiMessage::SupervisorStarted {
                        supervisor,
                        connection,
                        init,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.reconnect_active.set(false);
                            ctrl.has_supervisor.set(true);
                            ctrl.store_init_result(init);
                            ctrl.show_ready();
                            ctrl.mark_restore_pending();
                            ctrl.set_supervisor(supervisor);
                            ctrl.set_connection(connection);
                            ctrl.conversation_view.plugins_page().on_connection_ready();
                            ctrl.refresh_permission_catalog();
                        }
                    }
                    UiMessage::SupervisorStartFailed(message) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.pending_draft.borrow_mut().take();
                            ctrl.show_startup_error(&message);
                        }
                    }
                    UiMessage::ConnectionLost => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.pending_draft.borrow_mut().take();
                            ctrl.fail_pending_interactions();
                            ctrl.show_connection_lost();
                            ctrl.start_reconnect_loop(false);
                        }
                    }
                    UiMessage::SessionReleased => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.clear_active_session_ui();
                        }
                    }
                    UiMessage::SessionForked(session_id) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.follow_session(&session_id);
                        }
                    }
                    UiMessage::SessionRenamed { session_id, title } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.apply_rename(&session_id, &title);
                        }
                    }
                    UiMessage::QueueUpdated { session_id, items } => {
                        let matches_active = active_sess
                            .borrow()
                            .as_ref()
                            .map(|sess| sess.id == session_id)
                            .unwrap_or(false);
                        if matches_active {
                            conv_view.update_queue(&items);
                        }
                    }
                    UiMessage::CommandsLoaded(commands) => {
                        let options: Vec<TriggerOption> = commands
                            .iter()
                            .map(|command| TriggerOption {
                                label: format!("/{}", command.name),
                                description: command.description.clone(),
                                value: command.name.clone(),
                            })
                            .collect();
                        let commands_for_select = commands;
                        let ctrl_weak_select = ctrl_weak.clone();
                        let anchor = conv_view.composer().text_view().clone();
                        show_trigger_popup(&anchor, "Commands", options, move |value| {
                            if let Some(ctrl) = ctrl_weak_select.upgrade() {
                                if let Some(command) =
                                    commands_for_select.iter().find(|c| c.name == value)
                                {
                                    if command.input.is_some() {
                                        ctrl.set_composer_text(&format!("/{} ", command.name));
                                    } else {
                                        ctrl.set_composer_text("");
                                        ctrl.execute_command(&command.name, "");
                                    }
                                }
                            }
                        });
                    }
                    UiMessage::ReferencesLoaded(candidates) => {
                        let options: Vec<TriggerOption> = candidates
                            .iter()
                            .map(|candidate| TriggerOption {
                                label: format!("@{}", candidate.path),
                                description: candidate.kind.clone(),
                                value: candidate.path.clone(),
                            })
                            .collect();
                        let candidates_for_select = candidates;
                        let ctrl_weak_select = ctrl_weak.clone();
                        let anchor = conv_view.composer().text_view().clone();
                        show_trigger_popup(&anchor, "References", options, move |value| {
                            if let Some(ctrl) = ctrl_weak_select.upgrade() {
                                let suffix = candidates_for_select
                                    .iter()
                                    .find(|candidate| candidate.path == value)
                                    .filter(|candidate| candidate.kind == "directory")
                                    .map(|_| "/")
                                    .unwrap_or(" ");
                                ctrl.insert_composer_text(&format!("@{}{}", value, suffix));
                            }
                        });
                    }
                    UiMessage::SkillsLoaded(skills) => {
                        let options: Vec<TriggerOption> = skills
                            .iter()
                            .map(|skill| TriggerOption {
                                label: format!("#{}", skill.name),
                                description: skill.description.clone(),
                                value: skill.name.clone(),
                            })
                            .collect();
                        let ctrl_weak_select = ctrl_weak.clone();
                        let anchor = conv_view.composer().text_view().clone();
                        show_trigger_popup(&anchor, "Skills", options, move |value| {
                            if let Some(ctrl) = ctrl_weak_select.upgrade() {
                                ctrl.insert_composer_text(&format!("#{} ", value));
                            }
                        });
                    }
                    UiMessage::FilesListed {
                        session_id,
                        path,
                        workdir,
                        entries,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                let panel = ctrl.right_dock.files_panel();
                                panel.apply_listing(&path, &workdir, &entries);
                            }
                        }
                    }
                    UiMessage::FileStat { session_id, stat } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.files_panel().set_stat(&stat);
                            }
                        }
                    }
                    UiMessage::FilePreviewText {
                        session_id,
                        path,
                        text,
                        truncated,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock
                                    .preview_panel()
                                    .show_text(&path, &text, truncated);
                                ctrl.right_dock.show_preview();
                            }
                        }
                    }
                    UiMessage::FilePreviewImage {
                        session_id,
                        path,
                        bytes,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.preview_panel().show_image(&path, &bytes);
                                ctrl.right_dock.show_preview();
                            }
                        }
                    }
                    UiMessage::FilePreviewUnavailable { session_id, message } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.preview_panel().show_unavailable(&message);
                                ctrl.right_dock.show_preview();
                            }
                        }
                    }
                    UiMessage::PermissionLoaded(state) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.apply_permission_state(state);
                        }
                    }
                    UiMessage::PermissionCatalogLoaded(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.handle_permission_catalog(result);
                        }
                    }
                    UiMessage::PermissionValueUpdated {
                        session_id,
                        current_value,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.update_permission_value(&session_id, &current_value);
                        }
                    }
                    UiMessage::HistoryPageLoaded {
                        session_id,
                        events,
                        has_more,
                    } => {
                        conv_view.set_history_loading(false);
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.history_loading.set(false);
                        }
                        let active_id =
                            ctrl_weak.upgrade().and_then(|ctrl| ctrl.active_session_id());
                        if active_id.as_deref() != Some(session_id.as_str()) {
                            continue;
                        }
                        if events.is_empty() {
                            let _ = has_more;
                            continue;
                        }
                        let adjustment = conv_view.transcript_adjustment();
                        let distance_from_bottom = adjustment.upper() - adjustment.value();
                        let mut guard = active_sess.borrow_mut();
                        if let Some(session) = guard.as_mut() {
                            session.transcript.prepend_events(events.clone());
                            session.record_older_events(events);
                            conv_view.rerender_transcript(&session.transcript);
                            conv_view.trajectory_view().set_events(
                                &session.raw_events().iter().cloned().collect::<Vec<_>>(),
                            );
                        }
                        let adjustment = conv_view.transcript_adjustment();
                        adjustment.set_value((adjustment.upper() - distance_from_bottom).max(0.0));
                    }
                    UiMessage::EchoAck { session_id } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                let buffer = conv_view.composer().text_view().buffer();
                                buffer.set_text("");
                                ctrl.pending_images.borrow_mut().clear();
                                ctrl.clear_composer_draft(&session_id);
                            }
                        }
                    }
                    UiMessage::ImageAttached {
                        name,
                        mime_type,
                        data,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.pending_images.borrow_mut().push(PendingImage {
                                name: name.clone(),
                                mime_type,
                                data,
                            });
                        }
                        toast_overlay
                            .add_toast(adw::Toast::new(&format!("Image attached: {}", name)));
                    }
                    UiMessage::SessionSearchLoaded(items) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            let results = items
                                .into_iter()
                                .map(|item| SessionListItem {
                                    id: item.id,
                                    created_at: item.updated_at,
                                    updated_at: item.updated_at,
                                    cwd: item.cwd,
                                    title: item.title,
                                    turn_count: 0,
                                    is_archived: false,
                                })
                                .collect::<Vec<_>>();
                            ctrl.sidebar.show_search_results(&results);
                        }
                    }
                    UiMessage::FileAttached(path) => {
                        if !path.is_empty() {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.insert_composer_text(&format!("@{} ", path));
                            }
                        }
                    }
                    UiMessage::EchoFailed {
                        session_id,
                        text,
                        error,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                let mut guard = active_sess.borrow_mut();
                                if let Some(session) = guard.as_mut() {
                                    session.transcript.clear_pending_user();
                                    conv_view.update_transcript(&session.transcript);
                                }
                                drop(guard);
                                conv_view.composer().text_view().buffer().set_text(&text);
                                ctrl.store_composer_draft(&session_id, &text);
                            } else {
                                ctrl.store_composer_draft(&session_id, &text);
                            }
                        }
                        toast_overlay
                            .add_toast(adw::Toast::new(&format!("Send failed: {}", error)));
                    }
                    UiMessage::SubagentsLoaded(entries) => {
                        sidebar.load_subagents(&entries);
                    }
                    UiMessage::JobsLoaded(jobs) => {
                        conv_view.set_jobs(&jobs);
                    }
                    UiMessage::JobFollowLoaded(follow) => {
                        conv_view.show_job_follow(&follow);
                    }
                    UiMessage::TerminalListed {
                        session_id,
                        terminals,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.terminal_panel().handle_list(&terminals);
                            }
                        }
                    }
                    UiMessage::TerminalCreated {
                        session_id,
                        created,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.terminal_panel().handle_created(&created);
                            }
                        }
                    }
                    UiMessage::TerminalOutput { session_id, output } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.active_session_id().as_deref() == Some(session_id.as_str()) {
                                ctrl.right_dock.terminal_panel().handle_output(&output);
                            }
                        }
                    }
                    UiMessage::GoalLoaded(goal) => {
                        conv_view.goal_bar().set_goal(goal);
                    }
                    UiMessage::SessionModelLoaded { session_id, model } => {
                        let active_id =
                            ctrl_weak.upgrade().and_then(|ctrl| ctrl.active_session_id());
                        if active_id.as_deref() == Some(session_id.as_str()) {
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                let catalog = ctrl.model_catalog.borrow().clone();
                                let provider_known = catalog
                                    .as_ref()
                                    .map(|catalog| {
                                        catalog
                                            .providers
                                            .iter()
                                            .any(|provider| provider.id == model.provider)
                                    })
                                    .unwrap_or(true);
                                if !provider_known {
                                    if let Some(catalog) = catalog {
                                        ctrl.switch_model(
                                            &catalog.default_provider,
                                            &catalog.default_model,
                                        );
                                        continue;
                                    }
                                }
                                *ctrl.active_provider.borrow_mut() = model.provider;
                                *ctrl.active_model.borrow_mut() = model.model.clone();
                                conv_view.composer().set_model_label(&model.model);
                            }
                        }
                    }
                    UiMessage::ModelCatalogLoaded(catalog) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            let known = catalog
                                .providers
                                .iter()
                                .any(|provider| provider.id == *ctrl.active_provider.borrow());
                            if !known || !ctrl.model_explicit.get() {
                                *ctrl.active_provider.borrow_mut() =
                                    catalog.default_provider.clone();
                                *ctrl.active_model.borrow_mut() = catalog.default_model.clone();
                                conv_view
                                    .composer()
                                    .set_model_label(&catalog.default_model);
                            }
                            *ctrl.model_catalog.borrow_mut() = Some(catalog);
                        }
                    }
                    UiMessage::ModelSwitchFailed { provider, model } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            *ctrl.active_provider.borrow_mut() = provider;
                            *ctrl.active_model.borrow_mut() = model.clone();
                            conv_view.composer().set_model_label(&model);
                        }
                        toast_overlay.add_toast(adw::Toast::new(
                            "Model switch failed; previous model restored",
                        ));
                    }
                    UiMessage::GapRepairLoaded { session_id, events } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.continue_gap_repair(&session_id, events);
                        }
                    }
                    UiMessage::GapRepairFailed { session_id, error } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            if ctrl.abort_gap_repair(&session_id) {
                                toast_overlay.add_toast(adw::Toast::new(&format!(
                                    "History repair failed: {}",
                                    error
                                )));
                                ctrl.follow_session(&session_id);
                            }
                        }
                    }
                    UiMessage::SessionWindowLoaded { session_id, events } => {
                        let active_id = ctrl_weak.upgrade().and_then(|ctrl| ctrl.active_session_id());
                        if active_id.as_deref() == Some(session_id.as_str()) {
                            let mut guard = active_sess.borrow_mut();
                            if let Some(session) = guard.as_mut() {
                                if session.id == session_id {
                                    session.last_error_code =
                                        crate::session::terminal_error_code(&events);
                                }
                                if session.id == session_id
                                    && session.transcript.blocks().is_empty()
                                    && !events.is_empty()
                                {
                                    session.transcript.prepend_events(events.clone());
                                    conv_view.rerender_transcript(&session.transcript);
                                    conv_view.scroll_transcript_to_bottom();
                                }
                                session.record_older_events(events);
                                conv_view.trajectory_view().set_events(
                                    &session.raw_events().iter().cloned().collect::<Vec<_>>(),
                                );
                            }
                            drop(guard);
                            if let Some(ctrl) = ctrl_weak.upgrade() {
                                ctrl.update_continue_state();
                            }
                        }
                    }
                    UiMessage::ProjectionsLoaded {
                        session_id,
                        projections,
                    } => {
                        let active_id =
                            ctrl_weak.upgrade().and_then(|ctrl| ctrl.active_session_id());
                        if active_id.as_deref() == Some(session_id.as_str()) {
                            if let Some(usage) = projections.token_usage.as_ref() {
                                conv_view.set_usage(usage);
                            }
                            conv_view.set_context(
                                projections.context_pressure.as_ref(),
                                projections.context_breakdown.as_ref(),
                            );
                        }
                    }
                    UiMessage::ScheduleCatalogLoaded(entries) => {
                        conv_view.schedules_page().handle_catalog(&entries);
                    }
                    UiMessage::ScheduleCatalogFailed => {
                        conv_view.schedules_page().handle_catalog_failed();
                    }
                    UiMessage::ScheduleHistoryLoaded { id, page } => {
                        conv_view.schedules_page().handle_history(&id, page);
                    }
                    UiMessage::ScheduleMutationDone { message, created } => {
                        if !message.is_empty() {
                            toast_overlay.add_toast(adw::Toast::new(&message));
                        }
                        let page = conv_view.schedules_page();
                        if let Some(entry) = created.as_ref() {
                            page.handle_created(entry);
                        } else {
                            page.handle_mutation_settled();
                        }
                        page.load();
                    }
                    UiMessage::ScheduleMutationFailed { op, failure } => {
                        let locale = ui_locale();
                        let message = match &op {
                            ScheduleMutationOp::Delete { .. } => Some(match failure {
                                ScheduleFailure::Missing | ScheduleFailure::Conflict => {
                                    failure_text(failure, false, locale).to_string()
                                }
                                _ => schedule_copy(locale, "toast.deleteFailed").to_string(),
                            }),
                            ScheduleMutationOp::ClearEnded => {
                                Some(schedule_copy(locale, "toast.clearFailed").to_string())
                            }
                            _ => None,
                        };
                        if let Some(message) = message {
                            toast_overlay.add_toast(adw::Toast::new(&message));
                        }
                        conv_view
                            .schedules_page()
                            .handle_mutation_failed(&op, failure);
                    }
                    UiMessage::ScheduleLimitsLoaded(value) => {
                        conv_view.schedules_page().handle_limits(value);
                    }
                    UiMessage::QuestionReopened(request) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.enqueue_question(request, None);
                        }
                    }
                    UiMessage::FeedbackLoaded { session_id, items } => {
                        let active_id =
                            ctrl_weak.upgrade().and_then(|ctrl| ctrl.active_session_id());
                        if active_id.as_deref() == Some(session_id.as_str()) {
                            crate::ui::message_renderer::set_feedback_items(&items);
                        }
                    }
                    UiMessage::PluginsInventoryLoaded(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_inventory(result);
                        }
                    }
                    UiMessage::PluginsToggleDone { key, result } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_toggle_done(key, result);
                        }
                    }
                    UiMessage::PluginsRemoveDone { name, result } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_remove_done(name, result);
                        }
                    }
                    UiMessage::PluginInspectDone(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_inspect_done(result);
                        }
                    }
                    UiMessage::PluginInstallDone { request_id, result } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_install_done(request_id, result);
                        }
                    }
                    UiMessage::PluginCancelInstallDone(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_cancel_install_done(result);
                        }
                    }
                    UiMessage::PluginWaitInstallDone(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_wait_install_done(result);
                        }
                    }
                    UiMessage::PluginRegistriesLoaded(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_registries_loaded(result);
                        }
                    }
                    UiMessage::PluginRegistryProbeDone(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_registry_probe_done(result);
                        }
                    }
                    UiMessage::PluginVersionsLoaded { name, result } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_versions_loaded(name, result);
                        }
                    }
                    UiMessage::PluginUpdateDone(result) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.conversation_view
                                .plugins_page()
                                .handle_update_done(result);
                        }
                    }
                    UiMessage::AccountLoaded { .. } | UiMessage::CredentialRecordsLoaded(_) => {}
                }
            }
        });
    }

    fn setup_composer(&self) {
        let composer = self.conversation_view.composer();
        let text_view = composer.text_view().clone();
        let controller_weak = self.weak_self();

        let c_weak_send = controller_weak.clone();
        let tv_send = text_view.clone();
        composer.send_button().connect_clicked(move |_| {
            if let Some(c) = c_weak_send.upgrade() {
                let is_running = {
                    c.active_session
                        .borrow()
                        .as_ref()
                        .map(|s| s.status == SessionStatus::Running)
                        .unwrap_or(false)
                };

                if is_running {
                    c.cancel_current_turn();
                } else {
                    let buf = tv_send.buffer();
                    let (start, end) = buf.bounds();
                    let text = buf.text(&start, &end, false).to_string();
                    if !text.trim().is_empty() {
                        c.submit_text(&text, false);
                    }
                }
            }
        });

        let c_weak_model = controller_weak.clone();
        composer.model_button().connect_clicked(move |btn| {
            if let Some(c) = c_weak_model.upgrade() {
                c.open_model_picker(btn);
            }
        });
        let c_weak_perm = controller_weak.clone();
        composer.permission_button().connect_clicked(move |btn| {
            if let Some(c) = c_weak_perm.upgrade() {
                c.open_permission_picker(btn);
            }
        });

        let c_weak_continue = controller_weak.clone();
        composer.continue_button().connect_clicked(move |_| {
            if let Some(c) = c_weak_continue.upgrade() {
                c.continue_generation();
            }
        });

        let c_weak_plan = controller_weak.clone();
        composer.plan_button().connect_clicked(move |_| {
            if let Some(c) = c_weak_plan.upgrade() {
                c.execute_command("plan", "off");
            }
        });

        let c_weak_attach = controller_weak.clone();
        composer.attach_button().connect_clicked(move |_| {
            if let Some(c) = c_weak_attach.upgrade() {
                c.pick_attachments();
            }
        });

        let c_weak_drop = controller_weak.clone();
        let drop = gtk4::DropTarget::new(
            gtk4::gdk::FileList::static_type(),
            gtk4::gdk::DragAction::COPY,
        );
        drop.connect_drop(move |_, value, _, _| {
            if let Ok(list) = value.get::<gtk4::gdk::FileList>() {
                if let Some(c) = c_weak_drop.upgrade() {
                    for file in list.files() {
                        if let Some(path) = file.path() {
                            c.attach_file_path(&path);
                        }
                    }
                }
            }
            true
        });
        composer.widget().add_controller(drop);

        let c_weak_paste = controller_weak.clone();
        let paste_controller = gtk4::EventControllerKey::new();
        paste_controller.connect_key_pressed(move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::v
                && (modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::SUPER_MASK))
            {
                if let Some(display) = gtk4::gdk::Display::default() {
                    let clipboard = display.clipboard();
                    if clipboard.formats().contain_mime_type("image/png") {
                        let c = c_weak_paste.clone();
                        clipboard.read_texture_async(
                            None::<&gtk4::gio::Cancellable>,
                            move |result| {
                                if let Ok(Some(texture)) = result {
                                    if let Some(ctrl) = c.upgrade() {
                                        ctrl.attach_clipboard_texture(&texture);
                                    }
                                }
                            },
                        );
                        return gtk4::glib::Propagation::Stop;
                    }
                }
            }
            gtk4::glib::Propagation::Proceed
        });
        text_view.add_controller(paste_controller);

        let c_weak_enter = controller_weak.clone();
        let tv_enter = text_view.clone();
        let enter_controller = gtk4::EventControllerKey::new();
        enter_controller.connect_key_pressed(move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                if modifier.contains(gtk4::gdk::ModifierType::SHIFT_MASK) {
                    return gtk4::glib::Propagation::Proceed;
                }
                let steer = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::SUPER_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::META_MASK);
                if let Some(c) = c_weak_enter.upgrade() {
                    let buf = tv_enter.buffer();
                    let (start, end) = buf.bounds();
                    let text = buf.text(&start, &end, false).to_string();
                    if !text.trim().is_empty() {
                        c.submit_text(&text, steer);
                    }
                }
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });
        text_view.add_controller(enter_controller);

        let c_weak_trigger = controller_weak.clone();
        let trigger_controller = gtk4::EventControllerKey::new();
        trigger_controller.connect_key_pressed(move |_, key, _, modifier| {
            let has_modifier = modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                || modifier.contains(gtk4::gdk::ModifierType::ALT_MASK)
                || modifier.contains(gtk4::gdk::ModifierType::SUPER_MASK)
                || modifier.contains(gtk4::gdk::ModifierType::META_MASK);
            if !has_modifier {
                if let Some(c) = c_weak_trigger.upgrade() {
                    if key == gtk4::gdk::Key::slash {
                        c.fetch_commands();
                    } else if key == gtk4::gdk::Key::at {
                        c.fetch_references();
                    } else if key == gtk4::gdk::Key::numbersign {
                        c.fetch_skills();
                    }
                }
            }
            gtk4::glib::Propagation::Proceed
        });
        text_view.add_controller(trigger_controller);
    }

    fn setup_double_esc(&self) {
        let key_controller = gtk4::EventControllerKey::new();
        let last_esc = Rc::new(RefCell::new(None::<Instant>));
        let controller_weak = self.weak_self();

        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                let now = Instant::now();
                let is_double_esc = if let Some(prev) = *last_esc.borrow() {
                    now.duration_since(prev) <= Duration::from_millis(500)
                } else {
                    false
                };
                *last_esc.borrow_mut() = Some(now);

                if is_double_esc {
                    if let Some(c) = controller_weak.upgrade() {
                        c.cancel_current_turn();
                    }
                    return gtk4::glib::Propagation::Stop;
                }
            }
            gtk4::glib::Propagation::Proceed
        });

        self.conversation_view
            .widget()
            .add_controller(key_controller);
    }

    fn dispatch_call<P, R, F>(&self, method: &'static str, params: P, timeout: Duration, map: F)
    where
        P: serde::Serialize + Send + 'static,
        R: serde::de::DeserializeOwned + Send + 'static,
        F: FnOnce(Result<R, ConnectionError>) -> UiMessage + Send + 'static,
    {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let result = conn.call::<P, R>(method, params, timeout).await;
            let _ = ui_sender.send(map(result));
        });
    }

    pub fn refresh_subagents(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        self.dispatch_call(
            METHOD_SUBAGENT_LIST,
            SubagentListParams { session_id },
            Duration::from_secs(10),
            |result: Result<SubagentListResult, ConnectionError>| match result {
                Ok(list) => UiMessage::SubagentsLoaded(list.entries),
                Err(err) => UiMessage::Toast(format!("Subagent list failed: {}", err)),
            },
        );
    }

    pub fn interrupt_subagent(&self, child_session_id: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = SubagentInterruptParams {
            session_id,
            child_session_id: child_session_id.to_string(),
        };
        self.dispatch_call(
            METHOD_SUBAGENT_INTERRUPT,
            params,
            Duration::from_secs(10),
            |result: Result<SubagentInterruptResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Subagent interrupted".to_string()),
                Err(err) => UiMessage::Toast(format!("Interrupt failed: {}", err)),
            },
        );
    }

    pub fn refresh_jobs(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        self.dispatch_call(
            METHOD_JOB_LIST,
            JobListParams { session_id },
            Duration::from_secs(10),
            |result: Result<JobListResult, ConnectionError>| match result {
                Ok(list) => UiMessage::JobsLoaded(list.jobs),
                Err(err) => UiMessage::Toast(format!("Job list failed: {}", err)),
            },
        );
    }

    pub fn follow_job(&self, job_id: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = JobFollowParams {
            session_id,
            job_id: job_id.to_string(),
            from: None,
        };
        self.dispatch_call(
            METHOD_JOB_FOLLOW,
            params,
            Duration::from_secs(10),
            |result: Result<JobFollowResult, ConnectionError>| match result {
                Ok(follow) => UiMessage::JobFollowLoaded(follow),
                Err(err) => UiMessage::Toast(format!("Job follow failed: {}", err)),
            },
        );
    }

    pub fn kill_job(&self, job_id: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = JobKillParams {
            session_id,
            job_id: job_id.to_string(),
            reason: Some("user requested".to_string()),
        };
        self.dispatch_call(
            METHOD_JOB_KILL,
            params,
            Duration::from_secs(10),
            |result: Result<JobKillResult, ConnectionError>| match result {
                Ok(kill) => UiMessage::Toast(format!("Job kill: {}", kill.status)),
                Err(err) => UiMessage::Toast(format!("Job kill failed: {}", err)),
            },
        );
    }

    pub fn load_feedback(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let request_session = session_id.clone();
        self.dispatch_call(
            METHOD_MESSAGE_FEEDBACK_LIST,
            MessageFeedbackListParams { session_id },
            Duration::from_secs(10),
            move |result: Result<MessageFeedbackListResult, ConnectionError>| match result {
                Ok(list) => UiMessage::FeedbackLoaded {
                    session_id: request_session.clone(),
                    items: list.items,
                },
                Err(err) => UiMessage::Toast(format!("Feedback load failed: {}", err)),
            },
        );
    }

    pub fn put_feedback(
        &self,
        message_id: &str,
        rating: &str,
        if_version: Option<String>,
        note: Option<String>,
        category: Option<String>,
    ) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = MessageFeedbackPutParams {
            session_id,
            message_id: message_id.to_string(),
            rating: rating.to_string(),
            note,
            category,
            if_version,
        };
        self.dispatch_call(
            METHOD_MESSAGE_FEEDBACK_PUT,
            params,
            Duration::from_secs(10),
            |result: Result<MessageFeedbackPutResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Feedback saved".to_string()),
                Err(err) => UiMessage::Toast(format!("Feedback failed: {}", err)),
            },
        );
    }

    pub fn delete_feedback(&self, message_id: &str, if_version: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = MessageFeedbackDeleteParams {
            session_id,
            message_id: message_id.to_string(),
            if_version: if_version.to_string(),
        };
        self.dispatch_call(
            METHOD_MESSAGE_FEEDBACK_DELETE,
            params,
            Duration::from_secs(10),
            |result: Result<MessageFeedbackDeleteResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Feedback removed".to_string()),
                Err(err) => UiMessage::Toast(format!("Feedback delete failed: {}", err)),
            },
        );
    }

    pub fn record_session_feedback(&self, text: &str) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let params = SessionFeedbackRecordParams {
            session_id,
            text: Some(text.to_string()),
            category: None,
        };
        self.dispatch_call(
            METHOD_SESSION_FEEDBACK_RECORD,
            params,
            Duration::from_secs(10),
            |result: Result<SessionFeedbackRecordResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Feedback recorded".to_string()),
                Err(err) => UiMessage::Toast(format!("Feedback record failed: {}", err)),
            },
        );
    }

    pub fn reopen_question(&self) {
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        self.dispatch_call(
            METHOD_QUESTION_REOPEN,
            QuestionReopenParams { session_id },
            Duration::from_secs(10),
            |result: Result<QuestionReopenResult, ConnectionError>| match result {
                Ok(reopen) => UiMessage::QuestionReopened(reopen.request),
                Err(err) => UiMessage::Toast(format!("No pending question: {}", err)),
            },
        );
    }

    fn next_interaction_key(&self) -> u64 {
        let key = self.interaction_seq.get();
        self.interaction_seq.set(key + 1);
        key
    }

    pub fn enqueue_approval(
        &self,
        req: ApprovalRequestParams,
        responder: Option<oneshot::Sender<ApprovalOutcome>>,
    ) {
        let key = self.next_interaction_key();
        let session_id = req.session_id.clone();
        self.pending_interactions
            .borrow_mut()
            .entry(session_id)
            .or_default()
            .push(PendingInteraction::Approval {
                key,
                req,
                responder,
            });
        self.refresh_takeover();
    }

    pub fn enqueue_question(
        &self,
        req: SdkUserQuestionRequest,
        responder: Option<oneshot::Sender<SdkUserQuestionResult>>,
    ) {
        let key = self.next_interaction_key();
        let session_id = req.session_id.clone();
        self.pending_interactions
            .borrow_mut()
            .entry(session_id)
            .or_default()
            .push(PendingInteraction::Question {
                key,
                req,
                responder,
            });
        self.refresh_takeover();
    }

    pub fn cancel_pending_question(&self, session_id: &str, request_id: &str) {
        {
            let mut map = self.pending_interactions.borrow_mut();
            let mut empty = false;
            if let Some(queue) = map.get_mut(session_id) {
                queue.retain(|item| match item {
                    PendingInteraction::Question { req, .. } => req.request_id != request_id,
                    PendingInteraction::Approval { .. } => true,
                });
                empty = queue.is_empty();
            }
            if empty {
                map.remove(session_id);
            }
        }
        self.refresh_takeover();
    }

    pub fn fail_pending_interactions(&self) {
        let drained: Vec<PendingInteraction> = {
            let mut map = self.pending_interactions.borrow_mut();
            map.drain()
                .flat_map(|(_, queue)| queue)
                .collect::<Vec<_>>()
        };
        for item in drained {
            match item {
                PendingInteraction::Approval {
                    responder: Some(tx), ..
                } => {
                    let _ = tx.send(ApprovalOutcome::Rejected);
                }
                PendingInteraction::Question {
                    responder: Some(tx), ..
                } => {
                    let _ = tx.send(SdkUserQuestionResult::cancelled());
                }
                _ => {}
            }
        }
        *self.takeover_key.borrow_mut() = None;
        self.conversation_view.set_takeover(None);
    }

    fn active_session_events(&self) -> Vec<SessionEvent> {
        let session = self.active_session.borrow();
        let Some(session) = session.as_ref() else {
            return Vec::new();
        };
        let mut events: Vec<SessionEvent> = Vec::new();
        let (front, back) = session.raw_events().as_slices();
        events.extend_from_slice(front);
        events.extend_from_slice(back);
        events
    }

    pub fn refresh_takeover(&self) {
        if let Some((session_id, key)) = self.takeover_key.borrow().clone() {
            let active = self.active_session_id();
            let still_pending = self
                .pending_interactions
                .borrow()
                .get(&session_id)
                .map(|queue| queue.iter().any(|item| item.key() == key))
                .unwrap_or(false);
            if still_pending && active.as_deref() == Some(session_id.as_str()) {
                return;
            }
            self.conversation_view.set_takeover(None);
            *self.takeover_key.borrow_mut() = None;
        }
        let Some(session_id) = self.active_session_id() else {
            return;
        };
        let Some(display) = self
            .pending_interactions
            .borrow()
            .get(&session_id)
            .and_then(|queue| queue.first())
            .map(PendingInteraction::display)
        else {
            return;
        };
        let weak = self.self_weak.borrow().clone();
        match display {
            DisplayInteraction::Approval { key, req } => {
                let command = crate::interaction::approval_command(
                    &self.active_session_events(),
                    req.call_id.as_deref(),
                );
                let session = session_id.clone();
                let on_answer: Rc<dyn Fn(ApprovalOutcome)> = Rc::new(move |outcome| {
                    if let Some(ctrl) = weak.upgrade() {
                        ctrl.resolve_interaction(
                            &session,
                            key,
                            InteractionResolution::Approval(outcome),
                        );
                    }
                });
                let card =
                    crate::ui::interaction_card::build_approval_card(&req, command, on_answer);
                self.conversation_view.set_takeover(Some(card));
                *self.takeover_key.borrow_mut() = Some((session_id, key));
            }
            DisplayInteraction::Question {
                key,
                req,
                answerable,
            } => {
                let draft = crate::interaction::QuestionDraft::new(&req);
                let session = session_id.clone();
                let on_answer: Rc<dyn Fn(SdkUserQuestionResult)> = Rc::new(move |result| {
                    if let Some(ctrl) = weak.upgrade() {
                        ctrl.resolve_interaction(
                            &session,
                            key,
                            InteractionResolution::Question(result),
                        );
                    }
                });
                let card = crate::ui::interaction_card::build_question_card(
                    &req, draft, answerable, on_answer,
                );
                self.conversation_view.set_takeover(Some(card));
                *self.takeover_key.borrow_mut() = Some((session_id, key));
            }
        }
    }

    fn resolve_interaction(&self, session_id: &str, key: u64, resolution: InteractionResolution) {
        let pending = {
            let mut map = self.pending_interactions.borrow_mut();
            let mut removed = None;
            let mut empty = false;
            if let Some(queue) = map.get_mut(session_id) {
                if let Some(position) = queue.iter().position(|item| item.key() == key) {
                    removed = Some(queue.remove(position));
                }
                empty = queue.is_empty();
            }
            if empty {
                map.remove(session_id);
            }
            removed
        };
        match (pending, resolution) {
            (
                Some(PendingInteraction::Approval {
                    responder: Some(tx), ..
                }),
                InteractionResolution::Approval(outcome),
            ) => {
                let _ = tx.send(outcome);
            }
            (
                Some(PendingInteraction::Question {
                    responder: Some(tx), ..
                }),
                InteractionResolution::Question(result),
            ) => {
                let _ = tx.send(result);
            }
            _ => {}
        }
        let mounted = self
            .takeover_key
            .borrow()
            .as_ref()
            .map(|(session, mounted_key)| session == session_id && *mounted_key == key)
            .unwrap_or(false);
        if mounted {
            *self.takeover_key.borrow_mut() = None;
            self.conversation_view.set_takeover(None);
        }
        self.refresh_takeover();
    }

    pub fn load_account(&self) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            let Some(conn) = conn_opt else {
                return;
            };
            let status = conn
                .call::<serde_json::Value, AccountStatusResult>(
                    METHOD_ACCOUNT_STATUS,
                    serde_json::json!({}),
                    Duration::from_secs(10),
                )
                .await
                .map_err(|err| err.to_string());
            let profile = conn
                .call::<serde_json::Value, AccountProfileResult>(
                    METHOD_ACCOUNT_PROFILE,
                    serde_json::json!({}),
                    Duration::from_secs(10),
                )
                .await
                .map_err(|err| err.to_string());
            let balance = conn
                .call::<serde_json::Value, AccountBalanceResult>(
                    METHOD_ACCOUNT_BALANCE,
                    serde_json::json!({}),
                    Duration::from_secs(10),
                )
                .await
                .map_err(|err| err.to_string());
            let _ = ui_sender.send(UiMessage::AccountLoaded {
                status,
                profile,
                balance,
            });
        });
    }

    pub fn sign_out_account(&self) {
        self.dispatch_call(
            METHOD_ACCOUNT_SIGN_OUT,
            serde_json::json!({}),
            Duration::from_secs(10),
            |result: Result<AccountSignOutResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Signed out".to_string()),
                Err(err) => UiMessage::Toast(format!("Sign out failed: {}", err)),
            },
        );
    }

    pub fn load_credential_records(&self) {
        self.dispatch_call(
            METHOD_CREDENTIALS_RECORDS_LIST,
            serde_json::json!({}),
            Duration::from_secs(10),
            |result: Result<CredentialRecordsListResult, ConnectionError>| match result {
                Ok(list) => UiMessage::CredentialRecordsLoaded(list.records),
                Err(err) => UiMessage::Toast(format!("Credential records failed: {}", err)),
            },
        );
    }

    pub fn load_terminals(&self, session_id: &str) {
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_TERMINAL_LIST,
            TerminalListParams {
                session_id: session_id.to_string(),
            },
            Duration::from_secs(10),
            move |result: Result<TerminalListResult, ConnectionError>| match result {
                Ok(list) => UiMessage::TerminalListed {
                    session_id: request_session.clone(),
                    terminals: list.terminals,
                },
                Err(err) => UiMessage::Toast(format!("Terminal list failed: {}", err)),
            },
        );
    }

    pub fn create_terminal(&self, session_id: &str, cwd: &str) {
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_TERMINAL_CREATE,
            TerminalCreateParams {
                session_id: session_id.to_string(),
                cwd: Some(cwd.to_string()),
            },
            Duration::from_secs(15),
            move |result: Result<TerminalCreateResult, ConnectionError>| match result {
                Ok(created) => UiMessage::TerminalCreated {
                    session_id: request_session.clone(),
                    created,
                },
                Err(err) => UiMessage::Toast(format!("Terminal create failed: {}", err)),
            },
        );
    }

    pub fn send_terminal_input(&self, session_id: &str, terminal_id: &str, data: &str) {
        self.dispatch_call(
            METHOD_TERMINAL_INPUT,
            TerminalInputParams {
                session_id: session_id.to_string(),
                terminal_id: terminal_id.to_string(),
                data: data.to_string(),
            },
            Duration::from_secs(10),
            |result: Result<serde_json::Value, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast(String::new()),
                Err(err) => UiMessage::Toast(format!("Terminal input failed: {}", err)),
            },
        );
    }

    pub fn poll_terminal_output(&self, session_id: &str, terminal_id: &str, offset: Option<u64>) {
        let request_session = session_id.to_string();
        self.dispatch_call(
            METHOD_TERMINAL_OUTPUT,
            TerminalOutputParams {
                session_id: session_id.to_string(),
                terminal_id: terminal_id.to_string(),
                offset,
                count: Some(400),
            },
            Duration::from_secs(10),
            move |result: Result<TerminalOutputResult, ConnectionError>| match result {
                Ok(output) => UiMessage::TerminalOutput {
                    session_id: request_session.clone(),
                    output,
                },
                Err(err) => UiMessage::Toast(format!("Terminal output failed: {}", err)),
            },
        );
    }

    pub fn close_terminal(&self, session_id: &str, terminal_id: &str) {
        self.dispatch_call(
            METHOD_TERMINAL_CLOSE,
            TerminalCloseParams {
                session_id: session_id.to_string(),
                terminal_id: terminal_id.to_string(),
            },
            Duration::from_secs(10),
            |result: Result<TerminalCloseResult, ConnectionError>| match result {
                Ok(_) => UiMessage::Toast("Terminal closed".to_string()),
                Err(err) => UiMessage::Toast(format!("Terminal close failed: {}", err)),
            },
        );
    }

    pub fn load_schedules(&self) {
        self.dispatch_call(
            METHOD_SCHEDULE_CATALOG,
            serde_json::json!({}),
            Duration::from_secs(10),
            |result: Result<ScheduleCatalogResult, ConnectionError>| match result {
                Ok(catalog) => UiMessage::ScheduleCatalogLoaded(catalog.result),
                Err(_) => UiMessage::ScheduleCatalogFailed,
            },
        );
    }

    pub fn schedule_create(
        &self,
        title: &str,
        prompt: &str,
        timing: ScheduleTiming,
        target: Option<ScheduleTargetRequest>,
    ) {
        self.dispatch_call(
            METHOD_SCHEDULE_CREATE,
            ScheduleCreateParams {
                title: title.to_string(),
                prompt: prompt.to_string(),
                timing,
                target,
            },
            Duration::from_secs(15),
            move |result: Result<ScheduleOperationResult, ConnectionError>| match result {
                Ok(value) => match serde_json::from_value::<
                    crate::schedule_model::ScheduleCatalogEntry,
                >(value.result)
                {
                    Ok(entry) => UiMessage::ScheduleMutationDone {
                        message: String::new(),
                        created: Some(entry),
                    },
                    Err(_) => UiMessage::ScheduleMutationFailed {
                        op: ScheduleMutationOp::Create,
                        failure: ScheduleFailure::SaveError,
                    },
                },
                Err(err) => UiMessage::ScheduleMutationFailed {
                    op: ScheduleMutationOp::Create,
                    failure: failure_from_error(&err),
                },
            },
        );
    }

    pub fn schedule_run(&self, id: &str) {
        let op = ScheduleMutationOp::Run { id: id.to_string() };
        self.dispatch_call(
            METHOD_SCHEDULE_RUN,
            ScheduleRunParams { id: id.to_string() },
            Duration::from_secs(15),
            move |result: Result<ScheduleOperationResult, ConnectionError>| {
                let failure = match result {
                    Ok(value) => failure_from_result(&value.result),
                    Err(err) => Some(failure_from_error(&err)),
                };
                match failure {
                    Some(failure) => UiMessage::ScheduleMutationFailed {
                        op,
                        failure: run_failure(failure),
                    },
                    None => UiMessage::ScheduleMutationDone {
                        message: String::new(),
                        created: None,
                    },
                }
            },
        );
    }

    pub fn schedule_history(&self, id: &str, before: Option<String>) {
        let id_owned = id.to_string();
        self.dispatch_call(
            METHOD_SCHEDULE_HISTORY,
            ScheduleHistoryParams {
                id: id.to_string(),
                limit: Some(HISTORY_PAGE_LIMIT),
                before,
            },
            Duration::from_secs(10),
            move |result: Result<ScheduleHistoryResult, ConnectionError>| {
                let page = match result {
                    Ok(value) => match value.result {
                        ScheduleHistoryOutcome::Page(page) => Ok(page),
                        ScheduleHistoryOutcome::Miss { code } => {
                            Err(history_miss(&code))
                        }
                    },
                    Err(_) => Err(ScheduleHistoryMiss::Failed),
                };
                UiMessage::ScheduleHistoryLoaded { id: id_owned, page }
            },
        );
    }

    pub fn schedule_delete(&self, id: &str, expected_updated_at: Option<String>) {
        let locale = ui_locale();
        let id_owned = id.to_string();
        self.dispatch_call(
            METHOD_SCHEDULE_DELETE,
            ScheduleDeleteParams {
                id: id.to_string(),
                expected_updated_at,
            },
            Duration::from_secs(10),
            move |result: Result<ScheduleOperationResult, ConnectionError>| {
                let failure = match result {
                    Ok(value) => failure_from_result(&value.result),
                    Err(err) => Some(failure_from_error(&err)),
                };
                match failure {
                    Some(failure) => UiMessage::ScheduleMutationFailed {
                        op: ScheduleMutationOp::Delete { id: id_owned },
                        failure,
                    },
                    None => UiMessage::ScheduleMutationDone {
                        message: delete_toast(locale).to_string(),
                        created: None,
                    },
                }
            },
        );
    }

    pub fn schedule_edit(
        &self,
        id: &str,
        patch: SchedulePatch,
        expected_updated_at: Option<String>,
    ) {
        let op = ScheduleMutationOp::Edit { id: id.to_string() };
        self.dispatch_call(
            METHOD_SCHEDULE_EDIT,
            ScheduleEditParams {
                id: id.to_string(),
                action: "update".to_string(),
                patch: Some(patch),
                expected_updated_at,
            },
            Duration::from_secs(15),
            move |result: Result<ScheduleOperationResult, ConnectionError>| {
                let failure = match result {
                    Ok(value) => failure_from_result(&value.result),
                    Err(err) => Some(failure_from_error(&err)),
                };
                match failure {
                    Some(failure) => UiMessage::ScheduleMutationFailed { op, failure },
                    None => UiMessage::ScheduleMutationDone {
                        message: String::new(),
                        created: None,
                    },
                }
            },
        );
    }

    pub fn schedule_delete_inactive(&self) {
        let locale = ui_locale();
        self.dispatch_call(
            METHOD_SCHEDULE_DELETE_INACTIVE,
            serde_json::json!({}),
            Duration::from_secs(15),
            move |result: Result<ScheduleOperationResult, ConnectionError>| match result {
                Ok(value) => {
                    let deleted = value
                        .result
                        .get("deleted")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(0);
                    UiMessage::ScheduleMutationDone {
                        message: clear_ended_toast(deleted, locale),
                        created: None,
                    }
                }
                Err(_) => UiMessage::ScheduleMutationFailed {
                    op: ScheduleMutationOp::ClearEnded,
                    failure: ScheduleFailure::SaveError,
                },
            },
        );
    }

    pub fn schedule_delivery_limits(&self) {
        self.dispatch_call(
            METHOD_SCHEDULE_DELIVERY_LIMITS,
            serde_json::json!({}),
            Duration::from_secs(10),
            |result: Result<ScheduleOperationResult, ConnectionError>| match result {
                Ok(value) => UiMessage::ScheduleLimitsLoaded(
                    value
                        .result
                        .get("newSessionIntervalSeconds")
                        .and_then(serde_json::Value::as_i64),
                ),
                Err(_) => UiMessage::ScheduleLimitsLoaded(None),
            },
        );
    }

    pub fn start_schedule_session(&self) {
        self.start_session_in(None, None, Some(create_prompt(ui_locale())));
    }

    pub fn open_schedule_session(&self, session_id: &str) {
        self.panel_state.set_active_panel(None);
        self.follow_session(session_id);
    }

    pub fn workspace_choices(&self) -> Vec<WorkspaceView> {
        self.workspaces.borrow().clone()
    }

    pub fn session_archived(&self, session_id: &str) -> Option<bool> {
        self.sessions_cache
            .borrow()
            .iter()
            .find(|item| item.id == session_id)
            .map(|item| item.is_archived)
    }

    pub fn session_display_title(&self, session_id: &str) -> Option<String> {
        self.sessions_cache
            .borrow()
            .iter()
            .find(|item| item.id == session_id)
            .map(|item| item.title.clone())
    }
}

fn matches_active_session(active: &Rc<RefCell<Option<SessionModel>>>, session_id: &str) -> bool {
    active
        .borrow()
        .as_ref()
        .map(|session| session.id == session_id)
        .unwrap_or(false)
}

pub fn permission_display_label(state: &PermissionGetResult) -> String {
    if let Some(label) = permission_preset_builtin_label(&state.current_value) {
        return label.to_string();
    }
    state
        .catalog
        .options
        .iter()
        .chain(state.catalog.default_options.iter())
        .find(|option| option.value == state.current_value)
        .map(|option| option.name.clone())
        .unwrap_or_else(|| state.current_value.clone())
}

fn permission_preset_builtin_label(value: &str) -> Option<&'static str> {
    match value {
        "read-only" => Some("仅可查看"),
        "workspace-write" => Some("工作区内修改"),
        "danger-full-access" => Some("完全权限"),
        "auto" => Some("自动审查"),
        _ => None,
    }
}

fn permission_preset_label(option: &PresetOptionItem) -> String {
    permission_preset_builtin_label(&option.value)
        .map(str::to_string)
        .unwrap_or_else(|| option.name.clone())
}

fn permission_preset_icon(value: &str) -> &'static str {
    match value {
        "read-only" => "changes-prevent-symbolic",
        "workspace-write" => "document-edit-symbolic",
        "danger-full-access" => "dialog-warning-symbolic",
        "auto" => "emblem-system-symbolic",
        _ => "dialog-question-symbolic",
    }
}

fn permission_preset_requires_confirmation(value: &str) -> bool {
    matches!(value, "danger-full-access" | "auto")
}
