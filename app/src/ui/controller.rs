use crate::connection::{ApprovalResponder, BridgeConnection, IncomingNotification, UserQuestionResponder};
use crate::credentials::{should_show_onboarding, CredentialStore, DEFAULT_PROVIDER};
use crate::protocol::*;
use crate::session::{SessionModel, SessionStatus};
use crate::session_state::{default_state_path, SessionState};
use crate::supervisor::{resolve_bridge_bundle_path, HarnessSupervisor, SupervisorConfig};
use crate::ui::approval_dialog::{show_approval_dialog, show_user_question_dialog};
use crate::ui::conversation::ConversationView;
use crate::ui::desktop::{
    send_turn_completed_notification, should_notify_turn_completion, truncate_snippet,
    withdraw_notification,
};
use crate::ui::onboarding::show_onboarding_dialog;
use crate::ui::right_dock::RightDock;
use crate::ui::session_dialogs::{show_fork_dialog, show_rename_dialog};
use crate::ui::settings::{show_settings_dialog, SettingsDialogHandle};
use crate::ui::sidebar::Sidebar;
use crate::ui::trigger_popup::{show_trigger_popup, TriggerOption};
use crate::ui::window::MainWindow;
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
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::{oneshot, RwLock};

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio runtime")
});

pub fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    &RUNTIME
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
    SessionFollowed(SessionFollowResult),
    Notification(IncomingNotification),
    Toast(String),
    ShowOnboarding,
    PluginRestartRequired,
    SupervisorRestarted(BridgeConnection),
    SupervisorStarted {
        supervisor: Arc<HarnessSupervisor>,
        connection: BridgeConnection,
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
        path: String,
        workdir: String,
        entries: Vec<FileEntry>,
    },
    FileStat(FilesStatResult),
    FilePreviewText {
        path: String,
        text: String,
        truncated: bool,
    },
    FilePreviewImage {
        path: String,
        bytes: Vec<u8>,
    },
    FilePreviewUnavailable(String),
    PermissionLoaded(PermissionGetResult),
    PermissionValueUpdated {
        session_id: String,
        current_value: String,
    },
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
            rx.await.unwrap_or_else(|_| SdkUserQuestionResult::cancelled())
        })
    }
}

pub struct SessionController {
    connection: Arc<RwLock<Option<BridgeConnection>>>,
    supervisor: Arc<RwLock<Option<Arc<HarnessSupervisor>>>>,
    active_session: Rc<RefCell<Option<SessionModel>>>,
    active_model: Rc<RefCell<String>>,
    active_provider: Rc<RefCell<String>>,
    window: gtk4::Window,
    sidebar: Rc<Sidebar>,
    conversation_view: Rc<ConversationView>,
    right_dock: Rc<RightDock>,
    workspaces: Rc<RefCell<Vec<WorkspaceView>>>,
    sessions_cache: Rc<RefCell<Vec<SessionListItem>>>,
    settings_dialog: Rc<RefCell<Option<SettingsDialogHandle>>>,
    permission_state: Rc<RefCell<Option<PermissionGetResult>>>,
    window_title: adw::WindowTitle,
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
            window: main_window.window().clone().upcast(),
            sidebar,
            conversation_view,
            right_dock: right_dock.clone(),
            workspaces: Rc::new(RefCell::new(Vec::new())),
            sessions_cache: Rc::new(RefCell::new(Vec::new())),
            settings_dialog: Rc::new(RefCell::new(None)),
            permission_state: Rc::new(RefCell::new(None)),
            window_title: main_window.window_title().clone(),
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
        controller.setup_ui_receiver(ui_receiver, Rc::downgrade(&controller));
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
        controller.conversation_view.queue_dock().set_on_edit(
            move |item_id, text| {
                if let Some(ctrl) = weak_edit.upgrade() {
                    ctrl.queue_edit(&item_id, &text);
                }
            },
        );

        let weak_remove = Rc::downgrade(&controller);
        controller.conversation_view.queue_dock().set_on_remove(
            move |item_id| {
                if let Some(ctrl) = weak_remove.upgrade() {
                    ctrl.queue_remove(&item_id);
                }
            },
        );

        let weak_steer = Rc::downgrade(&controller);
        controller.conversation_view.queue_dock().set_on_steer_item(
            move |item_id| {
                if let Some(ctrl) = weak_steer.upgrade() {
                    ctrl.queue_steer_item(&item_id);
                }
            },
        );

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
                ctrl.bootstrap_supervisor();
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
                config.dsh_bin =
                    std::env::var("DSH_BIN").unwrap_or_else(|_| "dsh".to_string());
                config.profile_config.bridge_bundle_path_or_url =
                    resolve_bridge_bundle_path();
                Arc::new(HarnessSupervisor::new(config))
            };

            match supervisor.start().await {
                Ok(_) => {
                    if let Some(connection) = supervisor.connection().await {
                        let _ = ui_sender.send(UiMessage::SupervisorStarted {
                            supervisor,
                            connection,
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
                        let _ = ui_sender
                            .send(UiMessage::Toast(format!("Rename failed: {}", err)));
                    }
                }
            }
        });
    }

    fn apply_rename(&self, session_id: &str, title: &str) {
        self.cache_session_title(session_id, title);
        let mut active = self.active_session.borrow_mut();
        if let Some(session) = active.as_mut() {
            if session.id == session_id {
                session.title = title.to_string();
                self.window_title.set_subtitle(title);
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
                        let _ = ui_sender
                            .send(UiMessage::Toast(format!("Fork failed: {}", err)));
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
                    let _ = ui_sender
                        .send(UiMessage::Toast(format!("Queue edit failed: {}", err)));
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
                    let _ = ui_sender
                        .send(UiMessage::Toast(format!("Queue remove failed: {}", err)));
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
                        let _ = ui_sender
                            .send(UiMessage::Toast(format!("Steer failed: {}", err)));
                        break;
                    }
                }
            }
        });
    }

    pub fn send_prompt(&self, text: &str) {
        self.send_prompt_with_mode(text, "prompt");
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

    pub fn submit_text(&self, text: &str, steer: bool) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
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
        self.send_prompt_with_mode(text, mode);
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
                        let _ = ui_sender
                            .send(UiMessage::Toast(format!("Command failed: {}", err)));
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
                        let _ = ui_sender
                            .send(UiMessage::Toast(format!("Close failed: {}", err)));
                    }
                }
            }
        });
    }

    fn clear_active_session_ui(&self) {
        *self.active_session.borrow_mut() = None;
        self.conversation_view.clear_session();
        self.window_title.set_subtitle("New Session");
        self.clear_persisted_session();
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

    pub fn set_background_hold(
        &self,
        hold: gtk4::gio::ApplicationHoldGuard,
        session_id: &str,
    ) {
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
        } else {
            let was_running = self.running_sessions.borrow_mut().remove(session_id);
            let start = self.turn_start_times.borrow_mut().remove(session_id);
            let duration_ms = start
                .map(|started| started.elapsed().as_millis() as u64)
                .unwrap_or(0);

            if was_running
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
                    withdraw_notification(
                        app,
                        &format!("{}-background", background_id),
                    );
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
                self.conversation_view
                    .set_status(if status == "running" { "Running" } else { "Idle" });
            }
        }
    }

    pub fn set_supervisor(&self, sup: Arc<HarnessSupervisor>) {
        let sup_store = self.supervisor.clone();
        tokio_runtime().spawn(async move {
            *sup_store.write().await = Some(sup);
        });
    }

    pub fn set_connection(&self, conn: BridgeConnection) {
        let sender1 = self.ui_sender.clone();
        conn.set_approval_responder(ApprovalHandler {
            ui_sender: sender1,
        });

        let sender2 = self.ui_sender.clone();
        conn.set_user_question_responder(QuestionHandler {
            ui_sender: sender2,
        });

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
    }

    pub fn load_session_list(&self) {
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let params = serde_json::json!({"archived": false, "limit": 50});
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
                    let _ = ui_sender.send(UiMessage::WorkspaceCreated(path));
                }
                Err(err) => {
                    let _ = ui_sender.send(UiMessage::Toast(format!("Create workspace failed: {}", err)));
                }
            }
        });
    }

    pub fn switch_workspace(&self, id: &str) {
        let workspace = self.workspaces.borrow().iter().find(|ws| ws.id == id).cloned();
        let Some(workspace) = workspace else {
            return;
        };
        self.switch_to_workspace_path(&workspace.path);
    }

    fn switch_to_workspace_path(&self, path: &str) {
        if let Some(active) = self.active_session_id() {
            self.close_session(&active);
        }
        self.create_session(path);
    }

    pub fn rename_workspace(&self, id: &str) {
        let Some(workspace) = self.workspaces.borrow().iter().find(|ws| ws.id == id).cloned()
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
                        WorkspaceRenameParams { workspace_id: id, title },
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
        let Some(workspace) = self.workspaces.borrow().iter().find(|ws| ws.id == id).cloned()
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
                            let _ = ui_sender.send(UiMessage::Toast(format!("Remove failed: {}", err)));
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
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();
        let path = path.to_string();
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
                    let _ = ui_sender.send(UiMessage::FileStat(stat));
                }
                Err(err) => {
                    let _ = ui_sender
                        .send(UiMessage::FilePreviewUnavailable(format!("Stat failed: {}", err)));
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
                            let _ = ui_sender.send(UiMessage::FilePreviewImage { path, bytes });
                        }
                        None => {
                            let _ = ui_sender
                                .send(UiMessage::FilePreviewUnavailable("Invalid image data".into()));
                        }
                    },
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::FilePreviewUnavailable(format!(
                            "Read failed: {}",
                            err
                        )));
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
                                path,
                                text,
                                truncated,
                            });
                        }
                        None => {
                            let _ = ui_sender.send(UiMessage::FilePreviewUnavailable(
                                "Binary file — preview unavailable".to_string(),
                            ));
                        }
                    },
                    Err(err) => {
                        let _ = ui_sender.send(UiMessage::FilePreviewUnavailable(format!(
                            "Read failed: {}",
                            err
                        )));
                    }
                }
            }
        });
    }

    pub fn prompt_new_session_portal(&self) {
        let dialog = gtk4::FileDialog::builder()
            .title("Select Project Directory")
            .build();

        let controller_weak = self.weak_self();
        dialog.select_folder(
            Some(&self.window),
            None::<&gtk4::gio::Cancellable>,
            move |res| {
                if let Ok(folder) = res {
                    if let Some(path) = folder.path() {
                        if let Some(c) = controller_weak.upgrade() {
                            c.create_session(&path.display().to_string());
                        }
                    }
                }
            },
        );
    }

    pub fn create_session(&self, cwd: &str) {
        let cwd_string = cwd.to_string();
        let conn_arc = self.connection.clone();
        let model = self.active_model.borrow().clone();
        let provider = self.active_provider.borrow().clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let create_params = SessionCreateParams {
                    session_id: None,
                    cwd: cwd_string,
                    provider: Some(provider),
                    model: Some(model),
                    agent_preset: None,
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
                        let _ = ui_sender.send(UiMessage::SessionFollowed(follow_res));
                    }
                }
            }
        });
    }

    pub fn follow_session(&self, session_id: &str) {
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
                    let _ = ui_sender.send(UiMessage::SessionFollowed(follow_res));
                }
            }
        });
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
        let sess_id_opt = self
            .active_session
            .borrow()
            .as_ref()
            .map(|s| s.id.clone());

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
        *self.active_model.borrow_mut() = model.to_string();
        *self.active_provider.borrow_mut() = provider.to_string();

        let sess_id_opt = self
            .active_session
            .borrow()
            .as_ref()
            .map(|s| s.id.clone());

        if let Some(sess_id) = sess_id_opt {
            let conn_arc = self.connection.clone();
            let prov = provider.to_string();
            let mdl = model.to_string();

            tokio_runtime().spawn(async move {
                let conn_opt = conn_arc.read().await.clone();
                if let Some(conn) = conn_opt {
                    let params = SessionModelSetParams {
                        session_id: sess_id,
                        provider: prov,
                        model: mdl,
                        reasoning_effort: None,
                    };
                    let _ = conn
                        .call::<SessionModelSetParams, serde_json::Value>(
                            METHOD_SESSION_MODEL_SET,
                            params,
                            Duration::from_secs(5),
                        )
                        .await;
                }
            });
        }

        self.conversation_view
            .composer()
            .model_button()
            .set_label(model);
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

        let models = vec![
            ("deepseek", "deepseek-chat"),
            ("deepseek", "deepseek-coder"),
            ("deepseek", "deepseek-reasoner"),
        ];

        let controller_weak = self.weak_self();

        for (prov, mdl) in models {
            let btn = gtk4::Button::builder()
                .label(mdl)
                .css_classes(["flat"])
                .halign(gtk4::Align::Fill)
                .build();

            let pop_clone = popover.clone();
            let ctrl_weak = controller_weak.clone();
            let p_str = prov.to_string();
            let m_str = mdl.to_string();
            btn.connect_clicked(move |_| {
                if let Some(c) = ctrl_weak.upgrade() {
                    c.switch_model(&p_str, &m_str);
                }
                pop_clone.popdown();
            });
            box_widget.append(&btn);
        }

        popover.set_child(Some(&box_widget));
        popover.set_parent(anchor);
        popover.popup();
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

    pub fn open_permission_picker(&self, anchor: &impl IsA<gtk4::Widget>) {
        let popover = gtk4::Popover::builder().build();
        let box_widget = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_start(6)
            .margin_end(6)
            .margin_top(6)
            .margin_bottom(6)
            .build();
        let options = self
            .permission_state
            .borrow()
            .as_ref()
            .map(|state| {
                if !state.catalog.options.is_empty() {
                    state.catalog.options.clone()
                } else {
                    state.catalog.default_options.clone()
                }
            })
            .unwrap_or_default();
        let weak = self.weak_self();
        for option in options {
            let label = match &option.description {
                Some(description) if !description.is_empty() => {
                    format!("{} — {}", option.name, description)
                }
                _ => option.name.clone(),
            };
            let btn = gtk4::Button::builder()
                .label(&label)
                .css_classes(["flat"])
                .halign(gtk4::Align::Fill)
                .build();
            let pop_clone = popover.clone();
            let weak_clone = weak.clone();
            let value = option.value.clone();
            btn.connect_clicked(move |_| {
                if let Some(ctrl) = weak_clone.upgrade() {
                    ctrl.switch_permission_preset(&value);
                }
                pop_clone.popdown();
            });
            box_widget.append(&btn);
        }
        popover.set_child(Some(&box_widget));
        popover.set_parent(anchor);
        popover.popup();
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
            self.conversation_view
                .composer()
                .set_permission_label(current_value);
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
                let _ = conn
                    .call::<CredentialsRotateParams, CredentialsInjectResult>(
                        METHOD_CREDENTIALS_ROTATE,
                        params,
                        Duration::from_secs(5),
                    )
                    .await;
                let _ = ui_sender.send(UiMessage::Toast(format!("{} rotated in memory", prov)));
            }
        });
    }

    pub fn toggle_plugin(&self, plugin_id: &str, enable: bool) {
        let p_id = plugin_id.to_string();
        let conn_arc = self.connection.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let method = if enable {
                    METHOD_PLUGIN_ENABLE
                } else {
                    METHOD_PLUGIN_DISABLE
                };
                let params = PluginToggleParams { id: p_id };
                if let Ok(res) = conn
                    .call::<PluginToggleParams, PluginToggleResult>(
                        method,
                        params,
                        Duration::from_secs(5),
                    )
                    .await
                {
                    if res.restart_required {
                        let _ = ui_sender.send(UiMessage::PluginRestartRequired);
                    }
                }
            }
        });
    }

    pub fn restart_supervisor(&self) {
        let sup_arc = self.supervisor.clone();
        let ui_sender = self.ui_sender.clone();

        tokio_runtime().spawn(async move {
            let sup_opt = sup_arc.read().await.clone();
            if let Some(sup) = sup_opt {
                let _ = sup.stop().await;
                if let Ok(_init) = sup.start().await {
                    if let Some(conn) = sup.connection().await {
                        let _ = ui_sender.send(UiMessage::SupervisorRestarted(conn));
                    }
                }
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
        let window_title = self.window_title.clone();
        let toast_overlay = self.toast_overlay.clone();
        let active_sess = self.active_session.clone();
        let store = self.credential_store.clone();

        gtk4::glib::MainContext::default().spawn_local(async move {
            while let Some(msg) = receiver.recv().await {
                match msg {
                    UiMessage::Approval { req, tx } => {
                        show_approval_dialog(&win, req, tx);
                    }
                    UiMessage::UserQuestion { req, tx } => {
                        show_user_question_dialog(&win, req, tx);
                    }
                    UiMessage::SessionListLoaded(sessions) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.handle_session_list_loaded(&sessions);
                        }
                        sidebar.load_sessions(&sessions);
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            *ctrl.sessions_cache.borrow_mut() = sessions.clone();
                            let workspaces = ctrl.workspaces.borrow().clone();
                            sidebar.load_workspaces(&workspaces, &sessions);
                        }
                    }
                    UiMessage::WorkspacesLoaded(workspaces) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            *ctrl.workspaces.borrow_mut() = workspaces.clone();
                            let sessions = ctrl.sessions_cache.borrow().clone();
                            sidebar.load_workspaces(&workspaces, &sessions);
                        }
                    }
                    UiMessage::WorkspaceCreated(path) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.switch_to_workspace_path(&path);
                            ctrl.load_workspaces();
                        }
                    }
                    UiMessage::WorkspaceChanged => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.load_workspaces();
                        }
                    }
                    UiMessage::SessionFollowed(follow) => {
                        let title = follow
                            .title
                            .as_ref()
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| "New Session".to_string());
                        let cwd = follow.header.cwd.clone();
                        let status = follow.status.clone();
                        let session_id = follow.header.id.clone();

                        window_title.set_subtitle(&title);
                        conv_view.set_title(&title);
                        conv_view.set_cwd(&cwd);
                        conv_view.select_panel("conversation");
                        conv_view.set_status(if status == "running" {
                            "Running"
                        } else {
                            "Idle"
                        });

                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.handle_follow_status(&session_id, &status);
                            ctrl.persist_session(&session_id, &cwd);
                            ctrl.refresh_queue(&session_id);
                        }

                        let sess_model = SessionModel::from_follow(follow);
                        conv_view.update_transcript(&sess_model.transcript);
                        *active_sess.borrow_mut() = Some(sess_model);
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.right_dock.preview_panel().clear();
                            ctrl.refresh_files(".");
                            ctrl.refresh_permission(&session_id);
                        }
                    }
                    UiMessage::Notification(notif) => {
                        match notif {
                            IncomingNotification::SessionEvent(p) => {
                                let mut sess_guard = active_sess.borrow_mut();
                                if let Some(sess) = sess_guard.as_mut() {
                                    if p.session_id == sess.id {
                                        let _ = sess.handle_event(p.event);
                                        conv_view.update_transcript(&sess.transcript);
                                    }
                                }
                            }
                            IncomingNotification::AgentStream(p) => {
                                let mut sess_guard = active_sess.borrow_mut();
                                if let Some(sess) = sess_guard.as_mut() {
                                    if p.session_id == sess.id {
                                        sess.handle_stream_frame(p.frame);
                                        conv_view.update_transcript(&sess.transcript);
                                    }
                                }
                            }
                            IncomingNotification::SessionStatus(p) => {
                                if let Some(ctrl) = ctrl_weak.upgrade() {
                                    ctrl.handle_status_notification(&p.session_id, &p.status);
                                }
                            }
                            IncomingNotification::SessionTitle(p) => {
                                if let Some(ctrl) = ctrl_weak.upgrade() {
                                    ctrl.cache_session_title(&p.session_id, &p.title);
                                }
                                let mut sess_guard = active_sess.borrow_mut();
                                if let Some(sess) = sess_guard.as_mut() {
                                    if p.session_id == sess.id {
                                        sess.handle_title(&p.title);
                                        window_title.set_subtitle(&p.title);
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
                            IncomingNotification::SettingsUpdated(_)
                            | IncomingNotification::GoalUpdated(_) => {
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
                        }
                    }
                    UiMessage::Toast(msg) => {
                        toast_overlay.add_toast(adw::Toast::new(&msg));
                    }
                    UiMessage::ShowOnboarding => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            show_onboarding_dialog(&win, ctrl, store.clone());
                        }
                    }
                    UiMessage::PluginRestartRequired => {
                        toast_overlay
                            .add_toast(adw::Toast::new("Plugin updated. Restart dsh to apply."));
                    }
                    UiMessage::SupervisorRestarted(new_conn) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.show_ready();
                            ctrl.set_connection(new_conn);
                            let sess_id_opt = {
                                active_sess
                                    .borrow()
                                    .as_ref()
                                    .map(|s| s.id.clone())
                            };
                            if let Some(id) = sess_id_opt {
                                ctrl.follow_session(&id);
                            }
                        }
                        toast_overlay.add_toast(adw::Toast::new("dsh restarted successfully"));
                    }
                    UiMessage::SupervisorStarted {
                        supervisor,
                        connection,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.show_ready();
                            ctrl.mark_restore_pending();
                            ctrl.set_supervisor(supervisor);
                            ctrl.set_connection(connection);
                        }
                    }
                    UiMessage::SupervisorStartFailed(message) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.show_startup_error(&message);
                        }
                    }
                    UiMessage::ConnectionLost => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.show_connection_lost();
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
                    UiMessage::SessionRenamed {
                        session_id,
                        title,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.apply_rename(&session_id, &title);
                        }
                    }
                    UiMessage::QueueUpdated {
                        session_id,
                        items,
                    } => {
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
                        path,
                        workdir,
                        entries,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            let panel = ctrl.right_dock.files_panel();
                            panel.apply_listing(&path, &entries);
                            if !workdir.is_empty() {
                                panel.set_status(&format!(
                                    "{} entries under {}",
                                    entries.len(),
                                    workdir
                                ));
                            }
                        }
                    }
                    UiMessage::FileStat(stat) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.right_dock.files_panel().set_stat(&stat);
                        }
                    }
                    UiMessage::FilePreviewText {
                        path,
                        text,
                        truncated,
                    } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.right_dock
                                .preview_panel()
                                .show_text(&path, &text, truncated);
                            ctrl.right_dock.show_preview();
                        }
                    }
                    UiMessage::FilePreviewImage { path, bytes } => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.right_dock.preview_panel().show_image(&path, &bytes);
                            ctrl.right_dock.show_preview();
                        }
                    }
                    UiMessage::FilePreviewUnavailable(reason) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.right_dock.preview_panel().show_unavailable(&reason);
                            ctrl.right_dock.show_preview();
                        }
                    }
                    UiMessage::PermissionLoaded(state) => {
                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.apply_permission_state(state);
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
                        buf.set_text("");
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
                        buf.set_text("");
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

}

pub fn permission_display_label(state: &PermissionGetResult) -> String {
    state
        .catalog
        .options
        .iter()
        .chain(state.catalog.default_options.iter())
        .find(|option| option.value == state.current_value)
        .map(|option| option.name.clone())
        .unwrap_or_else(|| state.current_value.clone())
}
