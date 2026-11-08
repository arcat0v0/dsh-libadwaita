use crate::protocol::FileEntry;
use std::collections::{BTreeSet, HashMap, HashSet};

pub const ROOT_KEY: &str = ".";

pub struct ExpandPlan {
    pub to_expand: Vec<String>,
    pub to_fetch: Vec<String>,
}

#[derive(Default)]
pub struct FileTreeModel {
    workdir: String,
    levels: HashMap<String, Vec<FileEntry>>,
    failures: HashSet<String>,
    loading: HashSet<String>,
    expanded: BTreeSet<String>,
}

fn normalize_dir(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "." {
        ROOT_KEY.to_string()
    } else {
        trimmed.to_string()
    }
}

fn compare_entries(left: &FileEntry, right: &FileEntry) -> std::cmp::Ordering {
    let left_dir = left.kind == "directory";
    let right_dir = right.kind == "directory";
    if left_dir != right_dir {
        return if left_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        };
    }
    let lower = left.name.to_lowercase().cmp(&right.name.to_lowercase());
    lower.then_with(|| left.name.cmp(&right.name))
}

impl FileTreeModel {
    pub fn new(workdir: &str) -> Self {
        Self {
            workdir: workdir.to_string(),
            ..Default::default()
        }
    }

    pub fn root(&self) -> &str {
        &self.workdir
    }

    pub fn set_root(&mut self, workdir: &str) -> bool {
        if self.workdir == workdir {
            return false;
        }
        self.workdir = workdir.to_string();
        self.levels.clear();
        self.failures.clear();
        self.loading.clear();
        self.expanded.clear();
        true
    }

    pub fn mark_loading(&mut self, path: &str) {
        self.loading.insert(normalize_dir(path));
    }

    pub fn is_loading(&self, path: &str) -> bool {
        self.loading.contains(&normalize_dir(path))
    }

    pub fn apply_listing(&mut self, path: &str, entries: &[FileEntry]) {
        let path = normalize_dir(path);
        self.loading.remove(&path);
        self.failures.remove(&path);
        let mut sorted: Vec<FileEntry> = entries.to_vec();
        sorted.sort_by(compare_entries);
        if let Some(previous) = self.levels.get(&path) {
            let directories: HashSet<&str> = sorted
                .iter()
                .filter(|entry| entry.kind == "directory")
                .map(|entry| entry.path.as_str())
                .collect();
            let vanished: Vec<String> = previous
                .iter()
                .filter(|entry| {
                    entry.kind == "directory" && !directories.contains(entry.path.as_str())
                })
                .map(|entry| entry.path.clone())
                .collect();
            for removed in vanished {
                let prefix = format!("{}/", removed.trim_end_matches('/'));
                self.expanded
                    .retain(|value| *value != removed && !value.starts_with(&prefix));
                self.levels
                    .retain(|key, _| *key != removed && !key.starts_with(&prefix));
                self.failures
                    .retain(|key| *key != removed && !key.starts_with(&prefix));
            }
        }
        self.levels.insert(path, sorted);
    }

    pub fn apply_failure(&mut self, path: &str) {
        let path = normalize_dir(path);
        self.loading.remove(&path);
        self.failures.insert(path);
    }

    pub fn level(&self, path: &str) -> Option<&[FileEntry]> {
        self.levels
            .get(&normalize_dir(path))
            .map(|entries| entries.as_slice())
    }

    pub fn failure(&self, path: &str) -> bool {
        self.failures.contains(&normalize_dir(path))
    }

    pub fn toggle(&mut self, path: &str) -> bool {
        let path = normalize_dir(path);
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
            true
        } else {
            false
        }
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        self.expanded.contains(&normalize_dir(path))
    }

    pub fn expand(&mut self, path: &str) {
        self.expanded.insert(normalize_dir(path));
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.failures.clear();
        self.loading.clear();
        self.expanded.clear();
    }

    pub fn expand_ancestors(&mut self, file_path: &str) -> ExpandPlan {
        let relative =
            match file_path.strip_prefix(&format!("{}/", self.workdir.trim_end_matches('/'))) {
                Some(stripped) => stripped.to_string(),
                None => file_path.trim_start_matches("./").to_string(),
            };
        let mut to_expand = Vec::new();
        let mut to_fetch = Vec::new();
        let parts: Vec<&str> = relative.split('/').filter(|part| !part.is_empty()).collect();
        if parts.len() < 2 {
            return ExpandPlan {
                to_expand,
                to_fetch,
            };
        }
        let mut parent = String::new();
        for part in parts.iter().take(parts.len() - 1) {
            let path = if parent.is_empty() {
                part.to_string()
            } else {
                format!("{}/{}", parent, part)
            };
            let was_expanded = self.expanded.contains(&path);
            self.expanded.insert(path.clone());
            to_expand.push(path.clone());
            if !was_expanded && !self.levels.contains_key(&path) && !self.loading.contains(&path) {
                self.loading.insert(path.clone());
                to_fetch.push(path.clone());
            }
            parent = path;
        }
        ExpandPlan {
            to_expand,
            to_fetch,
        }
    }

    pub fn flatten(&self) -> Vec<FlattenRow> {
        let mut rows = Vec::new();
        let Some(root_entries) = self.levels.get(ROOT_KEY) else {
            return rows;
        };
        self.flatten_level(root_entries, 0, &mut rows);
        rows
    }

    fn flatten_level(&self, entries: &[FileEntry], depth: usize, rows: &mut Vec<FlattenRow>) {
        for entry in entries {
            rows.push(FlattenRow {
                entry: entry.clone(),
                depth,
            });
            if entry.kind != "directory" || !self.expanded.contains(&entry.path) {
                continue;
            }
            if let Some(children) = self.levels.get(&entry.path) {
                self.flatten_level(children, depth + 1, rows);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlattenRow {
    pub entry: FileEntry,
    pub depth: usize,
}

pub fn breadcrumb_segments(workdir: &str, file_path: &str) -> Vec<(String, String)> {
    let trimmed = workdir.trim_end_matches('/');
    if trimmed.is_empty() {
        return Vec::new();
    }
    let root_name = trimmed.rsplit('/').next().unwrap_or(trimmed).to_string();
    let relative = match file_path.strip_prefix(&format!("{}/", trimmed)) {
        Some(stripped) => stripped.to_string(),
        None if file_path.starts_with('/') => return Vec::new(),
        None => file_path.trim_start_matches("./").to_string(),
    };
    let mut segments = vec![(root_name, ROOT_KEY.to_string())];
    let parts: Vec<&str> = relative.split('/').filter(|part| !part.is_empty()).collect();
    let mut parent = String::new();
    for part in parts.iter().take(parts.len().saturating_sub(1)) {
        let path = if parent.is_empty() {
            part.to_string()
        } else {
            format!("{}/{}", parent, part)
        };
        segments.push((part.to_string(), path.clone()));
        parent = path;
    }
    segments
}
