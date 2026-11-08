use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenWithApp {
    pub id: &'static str,
    pub label: &'static str,
    pub program: &'static str,
}

pub const OPEN_WITH_APPS: &[OpenWithApp] = &[
    OpenWithApp {
        id: "filemanager",
        label: "File Manager",
        program: "xdg-open",
    },
    OpenWithApp {
        id: "cursor",
        label: "Cursor",
        program: "cursor",
    },
    OpenWithApp {
        id: "vscode",
        label: "Visual Studio Code",
        program: "code",
    },
    OpenWithApp {
        id: "vscodeinsiders",
        label: "Visual Studio Code Insiders",
        program: "code-insiders",
    },
    OpenWithApp {
        id: "windsurf",
        label: "Windsurf",
        program: "windsurf",
    },
    OpenWithApp {
        id: "zed",
        label: "Zed",
        program: "zed",
    },
    OpenWithApp {
        id: "sublimetext",
        label: "Sublime Text",
        program: "subl",
    },
    OpenWithApp {
        id: "androidstudio",
        label: "Android Studio",
        program: "studio",
    },
    OpenWithApp {
        id: "intellij",
        label: "IntelliJ IDEA",
        program: "idea",
    },
    OpenWithApp {
        id: "pycharm",
        label: "PyCharm",
        program: "pycharm",
    },
    OpenWithApp {
        id: "webstorm",
        label: "WebStorm",
        program: "webstorm",
    },
    OpenWithApp {
        id: "phpstorm",
        label: "PhpStorm",
        program: "phpstorm",
    },
    OpenWithApp {
        id: "goland",
        label: "GoLand",
        program: "goland",
    },
    OpenWithApp {
        id: "rider",
        label: "Rider",
        program: "rider",
    },
    OpenWithApp {
        id: "rustrover",
        label: "RustRover",
        program: "rustrover",
    },
];

pub fn app_by_id(id: &str) -> Option<&'static OpenWithApp> {
    OPEN_WITH_APPS.iter().find(|app| app.id == id)
}

pub fn program_on_path(program: &str) -> bool {
    if program.contains('/') {
        return is_executable(Path::new(program));
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|entry| is_executable(&entry.join(program)))
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn detect_apps() -> Vec<&'static OpenWithApp> {
    OPEN_WITH_APPS
        .iter()
        .filter(|app| program_on_path(app.program))
        .collect()
}

pub fn launch(app: &OpenWithApp, path: &Path) -> std::io::Result<()> {
    let mut command = Command::new(app.program);
    command.arg(path);
    command.current_dir(path);
    let mut child = command.spawn()?;
    let _ = child.try_wait();
    Ok(())
}

pub fn default_app(apps: &[&'static OpenWithApp]) -> Option<&'static OpenWithApp> {
    apps.iter()
        .copied()
        .find(|app| app.id == "filemanager")
        .or_else(|| apps.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_ids_are_unique_and_known() {
        let mut ids: Vec<&str> = OPEN_WITH_APPS.iter().map(|app| app.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count);
        assert_eq!(app_by_id("vscode").map(|app| app.program), Some("code"));
        assert_eq!(app_by_id("filemanager").map(|app| app.program), Some("xdg-open"));
        assert!(app_by_id("finder").is_none());
    }

    #[test]
    fn test_default_prefers_file_manager() {
        let apps: Vec<&'static OpenWithApp> = vec![&OPEN_WITH_APPS[1], &OPEN_WITH_APPS[0]];
        assert_eq!(default_app(&apps).map(|app| app.id), Some("filemanager"));
        assert_eq!(default_app(&apps[..1]).map(|app| app.id), Some("cursor"));
        assert_eq!(default_app(&[]).map(|app| app.id), None);
    }
}
