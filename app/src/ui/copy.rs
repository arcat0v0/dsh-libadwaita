use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Copybook {
    pub new_session: &'static str,
    pub create_session: &'static str,
    pub plugins: &'static str,
    pub schedules: &'static str,
    pub more: &'static str,
    pub workspaces: &'static str,
    pub default_workspace: &'static str,
    pub search_sessions: &'static str,
    pub collapse_sidebar: &'static str,
    pub open_sidebar: &'static str,
    pub open_right_sidebar: &'static str,
    pub settings: &'static str,
    pub account: &'static str,
    pub hero_headline: &'static str,
    pub preview: &'static str,
    pub choose_workspace: &'static str,
    pub workspace_write: &'static str,
    pub select_model: &'static str,
    pub composer_placeholder: &'static str,
    pub add_files_or_commands: &'static str,
    pub send_message: &'static str,
    pub stop_generating: &'static str,
    pub standard_mode: &'static str,
    pub ptc_mode: &'static str,
    pub minimal_mode: &'static str,
    pub creator_mode: &'static str,
    pub browse_plugins: &'static str,
    pub create_plugin: &'static str,
}

impl Copybook {
    pub const fn zh() -> Self {
        Self {
            new_session: "新会话",
            create_session: "新建会话",
            plugins: "插件",
            schedules: "自动化任务",
            more: "更多",
            workspaces: "工作区",
            default_workspace: "默认工作区",
            search_sessions: "搜索会话",
            collapse_sidebar: "收起侧边栏",
            open_sidebar: "打开侧边栏",
            open_right_sidebar: "打开右侧边栏",
            settings: "设置",
            account: "账号",
            hero_headline: "探索未至之境",
            preview: "预览版",
            choose_workspace: "选择工作区",
            workspace_write: "工作区内修改",
            select_model: "请选择模型",
            composer_placeholder: "描述你想要构建的内容, / 调用指令, @ 文件或对话",
            add_files_or_commands: "添加文件或调用指令",
            send_message: "发送消息",
            stop_generating: "停止生成",
            standard_mode: "标准模式",
            ptc_mode: "PTC 模式",
            minimal_mode: "极简模式",
            creator_mode: "创造模式",
            browse_plugins: "浏览更多插件",
            create_plugin: "创建你的插件",
        }
    }

    pub const fn en() -> Self {
        Self {
            new_session: "New Session",
            create_session: "New session",
            plugins: "Plugins",
            schedules: "Automation tasks",
            more: "More",
            workspaces: "Workspaces",
            default_workspace: "Default workspace",
            search_sessions: "Search sessions",
            collapse_sidebar: "Collapse sidebar",
            open_sidebar: "Open sidebar",
            open_right_sidebar: "Open right sidebar",
            settings: "Settings",
            account: "Account",
            hero_headline: "Into the Unknown",
            preview: "Preview",
            choose_workspace: "Choose workspace",
            workspace_write: "Workspace Write",
            select_model: "Select model",
            composer_placeholder: "Describe what you want to build, / commands, @ files or sessions",
            add_files_or_commands: "Add files or run commands",
            send_message: "Send message",
            stop_generating: "Stop generating",
            standard_mode: "Standard mode",
            ptc_mode: "PTC mode",
            minimal_mode: "Minimal mode",
            creator_mode: "Creator mode",
            browse_plugins: "Browse other plugins",
            create_plugin: "Create your plugin",
        }
    }

    pub fn for_code(code: &str) -> Self {
        let normalized = code.trim().to_ascii_lowercase();
        if normalized == "zh" || normalized.starts_with("zh_") || normalized.starts_with("zh-") {
            Self::zh()
        } else if normalized == "system" || normalized.is_empty() {
            Self::for_code(&system_locale())
        } else {
            Self::en()
        }
    }
}

fn system_locale() -> String {
    for key in ["LC_MESSAGES", "LC_ALL", "LANG"] {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return value;
            }
        }
    }
    "en".to_string()
}

pub fn locale_from_settings(settings: &Value) -> Option<String> {
    settings
        .get("locale")
        .and_then(|locale| {
            locale
                .get("language")
                .or_else(|| locale.get("locale"))
                .or_else(|| locale.get("lang"))
        })
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub fn resolve_locale(preference: Option<&str>, settings: &Value) -> String {
    if let Some(preference) = preference.filter(|value| *value != "system") {
        return preference.to_string();
    }
    locale_from_settings(settings).unwrap_or_else(system_locale)
}
