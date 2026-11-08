use crate::connection::ConnectionError;
use crate::protocol::SettingsDescriptor;
use std::collections::HashMap;

/// Per-namespace expected revisions for `settings/mutate` CAS guards.
#[derive(Debug, Default)]
pub struct SettingsRevisionCache {
    revisions: HashMap<String, u64>,
}

impl SettingsRevisionCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed(&mut self, namespace: &str, revision: u64) {
        self.revisions.insert(namespace.to_string(), revision);
    }

    pub fn seed_from_descriptors(&mut self, descriptors: &[SettingsDescriptor]) {
        for descriptor in descriptors {
            self.revisions.insert(descriptor.ns.clone(), descriptor.revision);
        }
    }

    pub fn revision(&self, namespace: &str) -> Option<u64> {
        self.revisions.get(namespace).copied()
    }

    pub fn apply_updated(&mut self, namespace: &str, revision: u64) {
        self.revisions.insert(namespace.to_string(), revision);
    }

    pub fn apply_conflict_actual(&mut self, namespace: &str, actual: u64) {
        self.revisions.insert(namespace.to_string(), actual);
    }
}

/// The authoritative revision carried by a `-32020` settings conflict, when present.
pub fn conflict_revision(error: &ConnectionError) -> Option<u64> {
    let ConnectionError::Remote { code, data, .. } = error else {
        return None;
    };
    if *code != -32020 {
        return None;
    }
    data.as_ref()?["actual"].as_u64()
}

const LEGAL_API_KEY_END: std::ops::RangeInclusive<u8> = 0x21..=0x7E;

/// Twin of the host's `normalizeApiKey` charset rule and the web client's
/// `apiKeyFailure`: printable ASCII without space, no `NAME=value` paste, no
/// quote wrapping. An empty draft means "keep the stored key" and passes;
/// whitespace-only is a failure so typed input is never silently dropped.
pub fn api_key_failure(draft: &str) -> Option<&'static str> {
    if draft.is_empty() {
        return None;
    }
    let value = draft.trim();
    if value.is_empty() {
        return Some("keyBlank");
    }
    if is_env_line(value) || is_quoted(value) {
        return Some("keyIllegalCharacters");
    }
    if !value.bytes().all(|byte| LEGAL_API_KEY_END.contains(&byte)) {
        return Some("keyIllegalCharacters");
    }
    None
}

fn is_env_line(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_uppercase() {
        return false;
    }
    let mut rest = chars.as_str();
    loop {
        match rest.chars().next() {
            Some(c) if c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' => {
                rest = &rest[c.len_utf8()..];
            }
            Some('=') => {
                let after = &rest[1..];
                return !after.starts_with('=');
            }
            _ => return false,
        }
    }
}

fn is_quoted(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '"' && first != '\'' && first != '`' {
        return false;
    }
    value.len() > 1 && value.ends_with(first)
}
