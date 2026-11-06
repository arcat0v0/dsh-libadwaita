use crate::connection::{ApprovalResponder, BridgeConnection, IncomingNotification, UserQuestionResponder};
use crate::credentials::{should_show_onboarding, CredentialStore, DEFAULT_PROVIDER};
use crate::protocol::*;
use crate::session::{SessionModel, SessionStatus};
use crate::supervisor::HarnessSupervisor;
use crate::ui::approval_dialog::{show_approval_dialog, show_user_question_dialog};
use crate::ui::conversation::ConversationView;
use crate::ui::desktop::{
    send_turn_completed_notification, should_notify_turn_completion, truncate_snippet,
    withdraw_notification,
};
use crate::ui::onboarding::show_onboarding_dialog;
use crate::ui::sidebar::Sidebar;
use crate::ui::window::MainWindow;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
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
    SessionFollowed(SessionFollowResult),
    Notification(IncomingNotification),
    Toast(String),
    ShowOnboarding,
    PluginRestartRequired,
    SupervisorRestarted(BridgeConnection),
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
    window_title: adw::WindowTitle,
    toast_overlay: adw::ToastOverlay,
    ui_sender: UnboundedSender<UiMessage>,
    credential_store: CredentialStore,
    running_sessions: Rc<RefCell<HashSet<String>>>,
    turn_start_times: Rc<RefCell<HashMap<String, Instant>>>,
    background_hold: Rc<RefCell<Option<gtk4::gio::ApplicationHoldGuard>>>,
    background_session: Rc<RefCell<Option<String>>>,
}

impl SessionController {
    pub fn new(
        main_window: &MainWindow,
        sidebar: Rc<Sidebar>,
        conversation_view: Rc<ConversationView>,
    ) -> Rc<Self> {
        let (ui_sender, ui_receiver) = unbounded_channel();
        let credential_store = CredentialStore::new();

        let controller = Rc::new(Self {
            connection: Arc::new(RwLock::new(None)),
            supervisor: Arc::new(RwLock::new(None)),
            active_session: Rc::new(RefCell::new(None)),
            active_model: Rc::new(RefCell::new("deepseek-chat".to_string())),
            active_provider: Rc::new(RefCell::new("deepseek".to_string())),
            window: main_window.window().clone().upcast(),
            sidebar,
            conversation_view,
            window_title: main_window.window_title().clone(),
            toast_overlay: main_window
                .window()
                .content()
                .and_downcast::<adw::ToastOverlay>()
                .unwrap(),
            ui_sender,
            credential_store,
            running_sessions: Rc::new(RefCell::new(HashSet::new())),
            turn_start_times: Rc::new(RefCell::new(HashMap::new())),
            background_hold: Rc::new(RefCell::new(None)),
            background_session: Rc::new(RefCell::new(None)),
        });

        controller.setup_ui_receiver(ui_receiver, Rc::downgrade(&controller));
        controller.setup_composer();
        controller.setup_double_esc();

        controller
    }

    pub fn credential_store(&self) -> &CredentialStore {
        &self.credential_store
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

    pub fn prompt_new_session_portal(&self) {
        let dialog = gtk4::FileDialog::builder()
            .title("Select Project Directory")
            .build();

        let controller_weak = Rc::downgrade(&Rc::new(self.clone_ref()));
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

    pub fn send_prompt(&self, text: &str) {
        let current_sess_id = {
            self.active_session
                .borrow()
                .as_ref()
                .map(|s| s.id.clone())
        };

        let sess_id = match current_sess_id {
            Some(id) => id,
            None => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                self.create_session(&home);
                return;
            }
        };

        let prompt_params = SessionPromptParams {
            session_id: sess_id,
            content_blocks: vec![PromptContentBlock::Text {
                text: text.to_string(),
            }],
            mode: Some("prompt".to_string()),
        };

        let conn_arc = self.connection.clone();
        tokio_runtime().spawn(async move {
            let conn_opt = conn_arc.read().await.clone();
            if let Some(conn) = conn_opt {
                let _ = conn
                    .call::<SessionPromptParams, SessionPromptResult>(
                        METHOD_SESSION_PROMPT,
                        prompt_params,
                        Duration::from_secs(10),
                    )
                    .await;
            }
        });
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

        let controller_weak = Rc::downgrade(&Rc::new(self.clone_ref()));

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
                        sidebar.load_sessions(&sessions);
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
                        conv_view.set_status(if status == "running" {
                            "Running"
                        } else {
                            "Idle"
                        });

                        if let Some(ctrl) = ctrl_weak.upgrade() {
                            ctrl.handle_follow_status(&session_id, &status);
                        }

                        let sess_model = SessionModel::from_follow(follow);
                        conv_view.update_transcript(&sess_model.transcript);
                        *active_sess.borrow_mut() = Some(sess_model);
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
                                let mut sess_guard = active_sess.borrow_mut();
                                if let Some(sess) = sess_guard.as_mut() {
                                    if p.session_id == sess.id {
                                        sess.handle_title(&p.title);
                                        window_title.set_subtitle(&p.title);
                                        conv_view.set_title(&p.title);
                                    }
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
                }
            }
        });
    }

    fn setup_composer(&self) {
        let composer = self.conversation_view.composer();
        let text_view = composer.text_view().clone();
        let controller_weak = Rc::downgrade(&Rc::new(self.clone_ref()));

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
                        c.send_prompt(&text);
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

        let enter_controller = gtk4::EventControllerKey::new();
        let c_weak_enter = controller_weak.clone();
        let tv_enter = text_view.clone();
        enter_controller.connect_key_pressed(move |_, key, _, modifier| {
            if key == gtk4::gdk::Key::Return || key == gtk4::gdk::Key::KP_Enter {
                let has_modifier = modifier
                    .contains(gtk4::gdk::ModifierType::SHIFT_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::ALT_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::SUPER_MASK)
                    || modifier.contains(gtk4::gdk::ModifierType::META_MASK);

                if !has_modifier {
                    if let Some(c) = c_weak_enter.upgrade() {
                        let is_running = {
                            c.active_session
                                .borrow()
                                .as_ref()
                                .map(|s| s.status == SessionStatus::Running)
                                .unwrap_or(false)
                        };
                        if !is_running {
                            let buf = tv_enter.buffer();
                            let (start, end) = buf.bounds();
                            let text = buf.text(&start, &end, false).to_string();
                            if !text.trim().is_empty() {
                                buf.set_text("");
                                c.send_prompt(&text);
                            }
                        }
                    }
                    return gtk4::glib::Propagation::Stop;
                }
            }
            gtk4::glib::Propagation::Proceed
        });
        text_view.add_controller(enter_controller);
    }

    fn setup_double_esc(&self) {
        let key_controller = gtk4::EventControllerKey::new();
        let last_esc = Rc::new(RefCell::new(None::<Instant>));
        let controller_weak = Rc::downgrade(&Rc::new(self.clone_ref()));

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

    fn clone_ref(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            supervisor: self.supervisor.clone(),
            active_session: self.active_session.clone(),
            active_model: self.active_model.clone(),
            active_provider: self.active_provider.clone(),
            window: self.window.clone(),
            sidebar: self.sidebar.clone(),
            conversation_view: self.conversation_view.clone(),
            window_title: self.window_title.clone(),
            toast_overlay: self.toast_overlay.clone(),
            ui_sender: self.ui_sender.clone(),
            credential_store: self.credential_store.clone(),
            running_sessions: self.running_sessions.clone(),
            turn_start_times: self.turn_start_times.clone(),
            background_hold: self.background_hold.clone(),
            background_session: self.background_session.clone(),
        }
    }
}
