use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

pub const DEFAULT_SERVICE_NAME: &str = "io.github.deepseek_ai.DshLibadwaita";
pub const DEFAULT_PROVIDER: &str = "DEEPSEEK_API_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    Backend(String),
    NotFound,
    EmptyKey,
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(s) => write!(f, "Credential backend error: {}", s),
            Self::NotFound => write!(f, "Credential not found"),
            Self::EmptyKey => write!(f, "Credential key cannot be empty"),
        }
    }
}

impl std::error::Error for CredentialError {}

#[derive(Debug, Clone)]
pub struct CredentialStore {
    service_name: String,
    in_memory: Arc<RwLock<HashMap<String, String>>>,
    force_in_memory: bool,
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore {
    pub fn new() -> Self {
        Self {
            service_name: DEFAULT_SERVICE_NAME.to_string(),
            in_memory: Arc::new(RwLock::new(HashMap::new())),
            force_in_memory: false,
        }
    }

    pub fn in_memory() -> Self {
        Self {
            service_name: DEFAULT_SERVICE_NAME.to_string(),
            in_memory: Arc::new(RwLock::new(HashMap::new())),
            force_in_memory: true,
        }
    }

    pub fn get_key(&self, provider: &str) -> Option<String> {
        if !self.force_in_memory {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, provider) {
                if let Ok(pass) = entry.get_password() {
                    if !pass.is_empty() {
                        return Some(pass);
                    }
                }
            }
        }

        self.in_memory
            .read()
            .ok()
            .and_then(|m| m.get(provider).cloned())
    }

    pub fn set_key(&self, provider: &str, key: &str) -> Result<(), CredentialError> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(CredentialError::EmptyKey);
        }

        if !self.force_in_memory {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, provider) {
                let _ = entry.set_password(trimmed);
            }
        }

        if let Ok(mut m) = self.in_memory.write() {
            m.insert(provider.to_string(), trimmed.to_string());
        }

        Ok(())
    }

    pub fn delete_key(&self, provider: &str) -> Result<(), CredentialError> {
        if !self.force_in_memory {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, provider) {
                let _ = entry.delete_credential();
            }
        }

        if let Ok(mut m) = self.in_memory.write() {
            m.remove(provider);
        }

        Ok(())
    }

    pub fn is_configured(&self, provider: &str) -> bool {
        self.get_key(provider).is_some()
    }

    pub fn mask_key(key: &str) -> String {
        let trimmed = key.trim();
        if trimmed.len() <= 6 {
            return "****".to_string();
        }

        if trimmed.starts_with("sk-") && trimmed.len() > 7 {
            let suffix = &trimmed[trimmed.len() - 4..];
            format!("sk-...{}", suffix)
        } else {
            let suffix = &trimmed[trimmed.len() - 4..];
            format!("...{}", suffix)
        }
    }
}

pub fn should_show_onboarding(items: &[crate::protocol::CredentialStatusItem]) -> bool {
    let deepseek_item = items.iter().find(|i| i.provider == DEFAULT_PROVIDER);
    match deepseek_item {
        Some(item) => !item.configured,
        None => true,
    }
}
