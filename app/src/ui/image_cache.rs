use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageClaim {
    Ready,
    Pending,
    Load,
}

#[derive(Debug, Clone)]
struct Entry<V> {
    value: Option<V>,
    pending: bool,
}

#[derive(Debug, Default)]
pub struct ImageCache<V> {
    sessions: HashMap<String, HashMap<String, Entry<V>>>,
}

impl<V: Clone> ImageCache<V> {
    pub fn claim(&mut self, session_id: &str, attachment_id: &str) -> ImageClaim {
        let entries = self.sessions.entry(session_id.to_string()).or_default();
        match entries.get_mut(attachment_id) {
            Some(entry) if entry.value.is_some() => ImageClaim::Ready,
            Some(entry) if entry.pending => ImageClaim::Pending,
            Some(_) => {
                let entry = entries.get_mut(attachment_id).expect("entry present");
                entry.pending = true;
                ImageClaim::Load
            }
            None => {
                entries.insert(
                    attachment_id.to_string(),
                    Entry {
                        value: None,
                        pending: true,
                    },
                );
                ImageClaim::Load
            }
        }
    }

    pub fn peek(&self, session_id: &str, attachment_id: &str) -> Option<&V> {
        self.sessions
            .get(session_id)
            .and_then(|entries| entries.get(attachment_id))
            .and_then(|entry| entry.value.as_ref())
    }

    pub fn complete(&mut self, session_id: &str, attachment_id: &str, value: V) {
        self.sessions
            .entry(session_id.to_string())
            .or_default()
            .insert(
                attachment_id.to_string(),
                Entry {
                    value: Some(value),
                    pending: false,
                },
            );
    }

    pub fn fail(&mut self, session_id: &str, attachment_id: &str) {
        if let Some(entries) = self.sessions.get_mut(session_id) {
            entries.remove(attachment_id);
        }
    }

    pub fn seed(&mut self, session_id: &str, attachment_id: &str, value: V) -> bool {
        let entries = self.sessions.entry(session_id.to_string()).or_default();
        if entries.contains_key(attachment_id) {
            return false;
        }
        entries.insert(
            attachment_id.to_string(),
            Entry {
                value: Some(value),
                pending: false,
            },
        );
        true
    }

    pub fn release_session(&mut self, session_id: &str) -> usize {
        self.sessions
            .remove(session_id)
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    pub fn release_all(&mut self) -> usize {
        let count = self.sessions.values().map(HashMap::len).sum();
        self.sessions.clear();
        count
    }

    pub fn is_pending(&self, session_id: &str, attachment_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .and_then(|entries| entries.get(attachment_id))
            .map(|entry| entry.pending)
            .unwrap_or(false)
    }
}
