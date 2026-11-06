use crate::connection::{BridgeConnection, ConnectionError};
use crate::protocol::*;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorState {
    Uninitialized,
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed,
}

impl fmt::Display for SupervisorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uninitialized => write!(f, "Uninitialized"),
            Self::Starting => write!(f, "Starting"),
            Self::Ready => write!(f, "Ready"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug)]
pub enum SupervisorError {
    IllegalStateTransition {
        from: SupervisorState,
        to: SupervisorState,
    },
    AlreadyRunning,
    ProcessSpawnFailed(String),
    ConnectionFailed(ConnectionError),
    HandshakeFailed(String),
    ProtocolMismatch {
        expected: u32,
        actual: u32,
    },
    Io(std::io::Error),
    Timeout,
    LockFailed(String),
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalStateTransition { from, to } => {
                write!(f, "Illegal state transition from {} to {}", from, to)
            }
            Self::AlreadyRunning => write!(f, "Another dsh instance is already running"),
            Self::ProcessSpawnFailed(s) => write!(f, "Failed to spawn process: {}", s),
            Self::ConnectionFailed(e) => write!(f, "Connection failed: {}", e),
            Self::HandshakeFailed(s) => write!(f, "Handshake failed: {}", s),
            Self::ProtocolMismatch { expected, actual } => {
                write!(f, "Protocol version mismatch: expected {}, got {}", expected, actual)
            }
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::Timeout => write!(f, "Operation timed out"),
            Self::LockFailed(s) => write!(f, "Failed to acquire instance lock: {}", s),
        }
    }
}

impl std::error::Error for SupervisorError {}

impl From<std::io::Error> for SupervisorError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<ConnectionError> for SupervisorError {
    fn from(e: ConnectionError) -> Self {
        Self::ConnectionFailed(e)
    }
}

#[derive(Debug, Clone)]
pub struct SupervisorStateMachine {
    current: SupervisorState,
}

impl Default for SupervisorStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl SupervisorStateMachine {
    pub fn new() -> Self {
        Self {
            current: SupervisorState::Uninitialized,
        }
    }

    pub fn state(&self) -> SupervisorState {
        self.current
    }

    pub fn transition(&mut self, target: SupervisorState) -> Result<(), SupervisorError> {
        let valid = match (self.current, target) {
            (SupervisorState::Uninitialized, SupervisorState::Starting) => true,
            (SupervisorState::Starting, SupervisorState::Ready) => true,
            (SupervisorState::Starting, SupervisorState::Failed) => true,
            (SupervisorState::Ready, SupervisorState::Stopping) => true,
            (SupervisorState::Ready, SupervisorState::Failed) => true,
            (SupervisorState::Stopping, SupervisorState::Stopped) => true,
            (SupervisorState::Stopping, SupervisorState::Failed) => true,
            (SupervisorState::Stopped, SupervisorState::Starting) => true,
            (SupervisorState::Failed, SupervisorState::Starting) => true,
            _ => false,
        };

        if valid {
            self.current = target;
            Ok(())
        } else {
            Err(SupervisorError::IllegalStateTransition {
                from: self.current,
                to: target,
            })
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenerationTracker {
    current: Option<String>,
}

impl GenerationTracker {
    pub fn new() -> Self {
        Self { current: None }
    }

    pub fn current_generation(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn record_generation(&mut self, generation: String) -> bool {
        let changed = match &self.current {
            Some(existing) => existing != &generation,
            None => true,
        };
        self.current = Some(generation);
        changed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileTemplateConfig {
    pub profile_name: String,
    pub bridge_bundle_path_or_url: String,
    pub disabled_rows: Vec<String>,
    pub llm_base_url: Option<String>,
    pub schema_version: u32,
}

impl Default for ProfileTemplateConfig {
    fn default() -> Self {
        Self {
            profile_name: "libadwaita".to_string(),
            bridge_bundle_path_or_url: "file:///usr/lib/dsh-libadwaita/bridge".to_string(),
            disabled_rows: vec!["hmr".to_string(), "credentials".to_string()],
            llm_base_url: None,
            schema_version: 1,
        }
    }
}

pub fn resolve_bridge_bundle_path() -> String {
    if let Ok(value) = std::env::var("DSH_BRIDGE_BUNDLE") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    let system_path = Path::new("/usr/lib/dsh-libadwaita/bridge");
    if system_path.exists() {
        return system_path.display().to_string();
    }

    let dev_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bridge");
    dev_path
        .canonicalize()
        .unwrap_or(dev_path)
        .display()
        .to_string()
}

pub fn generate_profile_package_json(config: &ProfileTemplateConfig) -> String {
    let bridge_dep = if config.bridge_bundle_path_or_url.starts_with("file:") {
        config.bridge_bundle_path_or_url.clone()
    } else {
        format!("file:{}", config.bridge_bundle_path_or_url)
    };

    let root = serde_json::json!({
        "name": format!("{}-profile", config.profile_name),
        "version": "0.1.0",
        "private": true,
        "dsh": {
            "profile": {
                "bundles": [
                    "@deepseek-ai/dsh-base",
                    "@dsh-libadwaita/bridge"
                ]
            }
        },
        "dependencies": {
            "@deepseek-ai/dsh-base": "0.2.2-rc.1",
            "@dsh-libadwaita/bridge": bridge_dep
        },
        "managedProfile": {
            "schemaVersion": config.schema_version
        }
    });

    serde_json::to_string_pretty(&root).unwrap_or_else(|_| "{}".to_string())
}

pub fn generate_cordis_patch_yml(config: &ProfileTemplateConfig) -> String {
    let mut out = String::new();
    for row in &config.disabled_rows {
        if row == "hmr" {
            out.push_str("- id: hmr\n  name: '@deepseek-ai/dsh-hmr'\n  disabled: true\n  config:\n    root: []\n");
        } else if row == "credentials" {
            out.push_str("- id: credentials\n  name: '@deepseek-ai/dsh-credentials-local'\n  disabled: true\n");
        } else {
            out.push_str(&format!("- id: {}\n  disabled: true\n", row));
        }
    }
    if let Some(base_url) = &config.llm_base_url {
        out.push_str(&format!(
            "- id: llm-deepseek\n  name: '@deepseek-ai/dsh-llm-deepseek-api-key'\n  config:\n    baseURL: {}\n",
            base_url
        ));
    }
    out
}

pub fn ensure_managed_profile(
    dsh_home: &Path,
    config: &ProfileTemplateConfig,
) -> Result<bool, SupervisorError> {
    let profile_dir = dsh_home.join("profiles").join(&config.profile_name);
    fs::create_dir_all(&profile_dir)?;

    let nm_scope_dir = profile_dir.join("node_modules/@dsh-libadwaita");
    fs::create_dir_all(&nm_scope_dir)?;
    let link_path = nm_scope_dir.join("bridge");
    if link_path.is_symlink() || link_path.exists() {
        let _ = fs::remove_file(&link_path);
    }
    let target_dir = if let Some(stripped) = config.bridge_bundle_path_or_url.strip_prefix("file:") {
        PathBuf::from(stripped)
    } else {
        PathBuf::from(&config.bridge_bundle_path_or_url)
    };
    let _ = std::os::unix::fs::symlink(&target_dir, &link_path);

    let pkg_path = profile_dir.join("package.json");
    let patch_path = profile_dir.join("cordis.patch.yml");

    let needs_update = if pkg_path.exists() && patch_path.exists() {
        let existing = fs::read_to_string(&pkg_path).unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&existing) {
            let cur_ver = parsed["managedProfile"]["schemaVersion"].as_u64().unwrap_or(0);
            let bridge_dep = parsed["dependencies"]["@dsh-libadwaita/bridge"].as_str().unwrap_or_default();
            let expected_dep = if config.bridge_bundle_path_or_url.starts_with("file:") {
                config.bridge_bundle_path_or_url.clone()
            } else {
                format!("file:{}", config.bridge_bundle_path_or_url)
            };
            let patch_content = fs::read_to_string(&patch_path).unwrap_or_default();
            let expected_patch = generate_cordis_patch_yml(config);
            cur_ver != config.schema_version as u64 || bridge_dep != expected_dep || patch_content != expected_patch
        } else {
            true
        }
    } else {
        true
    };

    if needs_update {
        let pkg_content = generate_profile_package_json(config);
        let patch_content = generate_cordis_patch_yml(config);

        fs::write(&pkg_path, pkg_content)?;
        fs::write(&patch_path, patch_content)?;
        Ok(true)
    } else {
        Ok(false)
    }
}
pub fn set_socket_permissions_0600(path: &Path) -> Result<(), SupervisorError> {
    let permissions = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SupervisorConfig {
    pub dsh_home: PathBuf,
    pub runtime_dir: PathBuf,
    pub dsh_bin: String,
    pub client_version: String,
    pub profile_config: ProfileTemplateConfig,
    pub handshake_timeout: Duration,
    pub shutdown_timeout: Duration,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        let dsh_home = match std::env::var("XDG_DATA_HOME") {
            Ok(val) if !val.is_empty() => PathBuf::from(val).join("dsh-libadwaita"),
            _ => {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".local/share/dsh-libadwaita")
            }
        };

        let runtime_dir = match std::env::var("XDG_RUNTIME_DIR") {
            Ok(val) if !val.is_empty() => PathBuf::from(val).join("dsh-libadwaita"),
            _ => {
                let uid = unsafe { libc::getuid() };
                PathBuf::from(format!("/tmp/dsh-libadwaita-{}", uid))
            }
        };

        let dsh_bin = std::env::var("DSH_BIN").unwrap_or_else(|_| "dsh".to_string());

        Self {
            dsh_home,
            runtime_dir,
            dsh_bin,
            client_version: "0.1.0".to_string(),
            profile_config: ProfileTemplateConfig::default(),
            handshake_timeout: Duration::from_secs(20),
            shutdown_timeout: Duration::from_secs(5),
        }
    }
}

pub struct HarnessSupervisor {
    config: SupervisorConfig,
    state_machine: Mutex<SupervisorStateMachine>,
    generation_tracker: Mutex<GenerationTracker>,
    connection: RwLock<Option<BridgeConnection>>,
    child: Mutex<Option<Child>>,
    lock_file: Mutex<Option<File>>,
    socket_path: RwLock<Option<PathBuf>>,
    stdout_purity_violations: Arc<AtomicUsize>,
}

impl HarnessSupervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            config,
            state_machine: Mutex::new(SupervisorStateMachine::new()),
            generation_tracker: Mutex::new(GenerationTracker::new()),
            connection: RwLock::new(None),
            child: Mutex::new(None),
            lock_file: Mutex::new(None),
            socket_path: RwLock::new(None),
            stdout_purity_violations: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn state(&self) -> SupervisorState {
        self.state_machine.lock().await.state()
    }

    pub async fn generation_id(&self) -> Option<String> {
        self.generation_tracker.lock().await.current_generation().map(str::to_string)
    }

    pub async fn connection(&self) -> Option<BridgeConnection> {
        self.connection.read().await.clone()
    }

    pub fn stdout_purity_violations(&self) -> usize {
        self.stdout_purity_violations.load(Ordering::Acquire)
    }

    pub async fn start(&self) -> Result<InitializeResult, SupervisorError> {
        {
            let mut sm = self.state_machine.lock().await;
            sm.transition(SupervisorState::Starting)?;
        }

        ensure_managed_profile(&self.config.dsh_home, &self.config.profile_config)?;

        let lock_path = self.config.dsh_home.join(".instance.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|e| SupervisorError::LockFailed(e.to_string()))?;

        unsafe {
            if libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&lock), libc::LOCK_EX | libc::LOCK_NB) != 0 {
                let mut sm = self.state_machine.lock().await;
                let _ = sm.transition(SupervisorState::Failed);
                return Err(SupervisorError::AlreadyRunning);
            }
        }
        *self.lock_file.lock().await = Some(lock);

        fs::create_dir_all(&self.config.runtime_dir)?;
        let _ = fs::set_permissions(&self.config.runtime_dir, fs::Permissions::from_mode(0o700));

        let sock_name = format!(
            "dsh-{}-{}.sock",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let socket_path = self.config.runtime_dir.join(sock_name);
        *self.socket_path.write().await = Some(socket_path.clone());

        let mut cmd = if self.config.dsh_bin.ends_with(".js") {
            let mut c = Command::new("node");
            c.arg(&self.config.dsh_bin);
            c
        } else {
            Command::new(&self.config.dsh_bin)
        };
        cmd.arg("--profile")
            .arg(&self.config.profile_config.profile_name)
            .env("DSH_HOME", &self.config.dsh_home)
            .env("XDG_RUNTIME_DIR", &self.config.runtime_dir)
            .env("DSH_LIBADWAITA_SOCKET", &socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        unsafe {
            cmd.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        let mut child = cmd.spawn().map_err(|e| {
            let _ = self.cleanup_lock();
            SupervisorError::ProcessSpawnFailed(e.to_string())
        })?;

        if let Some(mut stdout) = child.stdout.take() {
            let violations = self.stdout_purity_violations.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                use tokio::io::AsyncReadExt;
                while let Ok(n) = stdout.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    violations.fetch_add(n, Ordering::Release);
                }
            });
        }

        *self.child.lock().await = Some(child);

        let connect_deadline = tokio::time::Instant::now() + self.config.handshake_timeout;
        let mut connection = None;

        while tokio::time::Instant::now() < connect_deadline {
            {
                let mut guard = self.child.lock().await;
                if let Some(c) = guard.as_mut() {
                    if let Ok(Some(status)) = c.try_wait() {
                        return Err(SupervisorError::ProcessSpawnFailed(format!(
                            "child process exited prematurely with status: {}",
                            status
                        )));
                    }
                }
            }
            if socket_path.exists() {
                let _ = set_socket_permissions_0600(&socket_path);
                if let Ok(conn) = BridgeConnection::connect_unix(&socket_path).await {
                    connection = Some(conn);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let conn = match connection {
            Some(c) => c,
            None => {
                let _ = self.stop().await;
                return Err(SupervisorError::Timeout);
            }
        };

        let init_params = InitializeParams {
            client_version: self.config.client_version.clone(),
            protocol_version: PROTOCOL_VERSION,
            capabilities: Some(ClientCapabilities::default()),
        };

        let init_result: InitializeResult = match conn
            .call(METHOD_INITIALIZE, init_params, self.config.handshake_timeout)
            .await
        {
            Ok(res) => res,
            Err(e) => {
                let _ = self.stop().await;
                return Err(SupervisorError::HandshakeFailed(e.to_string()));
            }
        };

        if init_result.protocol_version != PROTOCOL_VERSION {
            let _ = self.stop().await;
            return Err(SupervisorError::ProtocolMismatch {
                expected: PROTOCOL_VERSION,
                actual: init_result.protocol_version,
            });
        }

        self.generation_tracker
            .lock()
            .await
            .record_generation(init_result.generation_id.clone());

        *self.connection.write().await = Some(conn);

        {
            let mut sm = self.state_machine.lock().await;
            sm.transition(SupervisorState::Ready)?;
        }

        Ok(init_result)
    }

    pub async fn stop(&self) -> Result<(), SupervisorError> {
        {
            let mut sm = self.state_machine.lock().await;
            let _ = sm.transition(SupervisorState::Stopping);
        }

        if let Some(conn) = self.connection.write().await.take() {
            let _ = conn
                .call::<serde_json::Value, ShutdownResult>(
                    METHOD_SHUTDOWN,
                    serde_json::json!({}),
                    self.config.shutdown_timeout,
                )
                .await;
            let _ = conn.close().await;
        }

        let mut child_guard = self.child.lock().await;
        if let Some(mut child) = child_guard.take() {
            let wait_fut = child.wait();
            match tokio::time::timeout(self.config.shutdown_timeout, wait_fut).await {
                Ok(_) => {}
                Err(_) => {
                    let _ = child.kill().await;
                }
            }
        }

        if let Some(path) = self.socket_path.write().await.take() {
            let _ = fs::remove_file(path);
        }

        self.cleanup_lock();

        {
            let mut sm = self.state_machine.lock().await;
            let _ = sm.transition(SupervisorState::Stopped);
        }

        Ok(())
    }

    fn cleanup_lock(&self) {
        if let Ok(mut guard) = self.lock_file.try_lock() {
            if let Some(file) = guard.take() {
                unsafe {
                    libc::flock(std::os::unix::io::AsRawFd::as_raw_fd(&file), libc::LOCK_UN);
                }
            }
        }
        let lock_path = self.config.dsh_home.join(".instance.lock");
        let _ = fs::remove_file(lock_path);
    }
}
