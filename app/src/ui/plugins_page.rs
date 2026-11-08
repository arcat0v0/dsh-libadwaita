use crate::protocol::*;
use crate::ui::controller::SessionController;
use crate::ui::settings::rpc_receiver;
use adw::prelude::*;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

pub mod model {
    use crate::protocol::*;
    use serde::{Deserialize, Serialize};
    use serde_json::Value;
    use std::collections::HashSet;

    pub const HIDDEN_BUNDLES: [&str; 8] = [
        "@deepseek-ai/dsh-base",
        "@deepseek-ai/dsh-web-app",
        "@deepseek-ai/dsh-headless",
        "@deepseek-ai/dsh-sdk-app",
        "@deepseek-ai/dsh-acp-app",
        "@deepseek-ai/dsh-sdk-minimal",
        "@deepseek-ai/dsh-experimental-terminal-bundle",
        "@deepseek-ai/dsh-experimental-tool-worktree",
    ];

    pub const EXTENSION_BUNDLES: [&str; 3] = [
        "@deepseek-ai/dsh-experimental-voice-input-bundle",
        "@deepseek-ai/dsh-experimental-plugin-market-bundle",
        "@deepseek-ai/dsh-experimental-auto-review",
    ];

    pub const MORE_BUNDLES: [&str; 10] = [
        "@deepseek-ai/dsh-personalized-recommendations",
        "@deepseek-ai/dsh-experimental-inspector-profile",
        "@deepseek-ai/dsh-cua-bundle",
        "@deepseek-ai/dsh-experimental-cot-translation-bundle",
        "@deepseek-ai/dsh-experimental-session-search",
        "@deepseek-ai/dsh-experimental-session-titles-bundle",
        "@deepseek-ai/dsh-experimental-badge-skill-bundle",
        "@deepseek-ai/dsh-experimental-ralph-bundle",
        "@deepseek-ai/dsh-subagent-codex",
        "@deepseek-ai/dsh-subagent-claude-code",
    ];

    pub const EXPERIMENTAL_BUNDLES: [&str; 14] = [
        "@deepseek-ai/dsh-personalized-recommendations",
        "@deepseek-ai/dsh-cua-bundle",
        "@deepseek-ai/dsh-experimental-voice-input-bundle",
        "@deepseek-ai/dsh-experimental-auto-review",
        "@deepseek-ai/dsh-experimental-inspector-profile",
        "@deepseek-ai/dsh-experimental-forge-bundle",
        "@deepseek-ai/dsh-experimental-tool-worktree",
        "@deepseek-ai/dsh-experimental-cot-translation-bundle",
        "@deepseek-ai/dsh-experimental-session-search",
        "@deepseek-ai/dsh-experimental-session-titles-bundle",
        "@deepseek-ai/dsh-experimental-badge-skill-bundle",
        "@deepseek-ai/dsh-experimental-ralph-bundle",
        "@deepseek-ai/dsh-experimental-plugin-market-bundle",
        "@deepseek-ai/dsh-experimental-liveshare",
    ];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum RowPhase {
        Pending,
        Loading,
        Active,
        Failed,
        Unloading,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DotState {
        Idle,
        Ongoing,
        Done,
        Error,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PackageError {
        pub code: String,
        pub diagnostic: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InstallTarget {
        pub spec: String,
        pub version: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PackageRow {
        pub entry_id: Option<String>,
        pub row_id: String,
        pub module_name: String,
        pub title: Option<String>,
        pub description: Option<String>,
        pub icon: Option<String>,
        pub meta_error: Option<String>,
        pub enabled: bool,
        pub phase: Option<RowPhase>,
        pub diagnostic: Option<String>,
        pub read_only_reason: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PackageView {
        pub name: String,
        pub official: bool,
        pub availability: String,
        pub install_target: Option<InstallTarget>,
        pub version: Option<String>,
        pub description: Option<String>,
        pub title: Option<String>,
        pub meta_description: Option<String>,
        pub icon: Option<String>,
        pub meta_error: Option<String>,
        pub installed: bool,
        pub listed_by_default: Option<bool>,
        pub source: Option<String>,
        pub optional: bool,
        pub removable: bool,
        pub updatable: bool,
        pub enabled: bool,
        pub read_only_reason: Option<String>,
        pub error: Option<PackageError>,
        pub rows: Vec<PackageRow>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PackageStatus {
        Running,
        Disabled,
        Problem,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PackageTag {
        Experimental,
        Problem,
        MissingFiles,
        NotInstalled,
        Updatable,
    }

    impl PackageTag {
        pub fn text(self) -> &'static str {
            match self {
                PackageTag::Experimental => "实验性",
                PackageTag::Problem => "异常",
                PackageTag::MissingFiles => "包不可用",
                PackageTag::NotInstalled => "未安装",
                PackageTag::Updatable => "可更新",
            }
        }

        pub fn tone(self) -> &'static str {
            match self {
                PackageTag::Problem => "error",
                PackageTag::Experimental | PackageTag::Updatable => "accent",
                _ => "neutral",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum FailedAction {
        Enable,
        Disable,
        Uninstall,
        RowEnable,
        RowDisable,
    }

    impl FailedAction {
        pub fn label(self) -> &'static str {
            match self {
                FailedAction::Enable => "启用失败",
                FailedAction::Disable => "停用失败",
                FailedAction::Uninstall => "卸载失败",
                FailedAction::RowEnable => "组件启用失败",
                FailedAction::RowDisable => "组件停用失败",
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct OfficialItem {
        pub id: &'static str,
        pub order: u32,
        pub label: &'static str,
        pub description: &'static str,
        pub namespaces: &'static [&'static str],
    }

    pub const BUILTIN_ITEMS: [OfficialItem; 4] = [
        OfficialItem {
            id: "subagent",
            order: 10,
            label: "子智能体",
            description: "设置子智能体的递归层级、数量和模型。",
            namespaces: &["subagent"],
        },
        OfficialItem {
            id: "web-search",
            order: 20,
            label: "网页搜索与获取",
            description: "选择网页搜索与网页获取使用的提供方。",
            namespaces: &["web"],
        },
        OfficialItem {
            id: "agent-loop",
            order: 30,
            label: "Agent 循环",
            description: "控制 Agent 派发工具调用的方式。",
            namespaces: &["agent-loop"],
        },
        OfficialItem {
            id: "shell",
            order: 40,
            label: "终端",
            description: "限制每条命令最多能跑多久、最多输出多少内容。",
            namespaces: &["bash-sandbox", "pwsh-sandbox"],
        },
    ];

    pub fn builtin_item(id: &str) -> Option<&'static OfficialItem> {
        BUILTIN_ITEMS.iter().find(|item| item.id == id)
    }

    pub fn short_name(name: &str) -> String {
        let unscoped = match name.strip_prefix('@').and_then(|rest| rest.split_once('/')) {
            Some((_, tail)) => tail,
            None => name,
        };
        let stripped = unscoped
            .strip_prefix("dsh-host-")
            .or_else(|| unscoped.strip_prefix("dsh-client-"))
            .unwrap_or(unscoped);
        stripped.strip_prefix("dsh-").unwrap_or(stripped).to_string()
    }

    pub fn resolve_text(value: Option<&Value>, lang: &str) -> Option<String> {
        let value = value?;
        if let Some(text) = value.as_str() {
            return Some(text.to_string());
        }
        let object = value.as_object()?;
        if let Some(text) = object.get(lang).and_then(Value::as_str) {
            return Some(text.to_string());
        }
        let prefix = format!("{lang}-");
        for (key, text) in object {
            if key.starts_with(&prefix) {
                if let Some(text) = text.as_str() {
                    return Some(text.to_string());
                }
            }
        }
        object.get("en").and_then(Value::as_str).map(str::to_string)
    }

    fn parse_phase(value: Option<&str>) -> Option<RowPhase> {
        match value {
            Some("pending") => Some(RowPhase::Pending),
            Some("loading") => Some(RowPhase::Loading),
            Some("active") => Some(RowPhase::Active),
            Some("failed") => Some(RowPhase::Failed),
            Some("unloading") => Some(RowPhase::Unloading),
            _ => None,
        }
    }

    fn parse_row(row: &Value, plugins: &[Value], lang: &str) -> PackageRow {
        let entry_id = row["entryId"].as_str().map(str::to_string);
        let live = entry_id
            .as_deref()
            .and_then(|id| plugins.iter().find(|plugin| plugin["entryId"].as_str() == Some(id)));
        let meta = &row["meta"];
        PackageRow {
            entry_id,
            row_id: row["rowId"].as_str().unwrap_or_default().to_string(),
            module_name: row["moduleName"].as_str().unwrap_or_default().to_string(),
            title: resolve_text(meta.get("title"), lang),
            description: resolve_text(meta.get("description"), lang),
            icon: meta.get("icon").and_then(Value::as_str).map(str::to_string),
            meta_error: meta.get("error").and_then(Value::as_str).map(str::to_string),
            enabled: live.and_then(|plugin| plugin["enabled"].as_bool()).unwrap_or(false),
            phase: live
                .and_then(|plugin| plugin["fiberPhase"].as_str())
                .and_then(|phase| parse_phase(Some(phase))),
            diagnostic: live
                .and_then(|plugin| plugin["diagnostic"].as_str())
                .map(str::to_string),
            read_only_reason: live
                .and_then(|plugin| plugin["readOnlyReason"].as_str())
                .map(str::to_string),
        }
    }

    pub fn parse_bundle(bundle: &Value, plugins: &[Value], lang: &str) -> PackageView {
        let meta = &bundle["meta"];
        let rows = bundle["rows"]
            .as_array()
            .map(|rows| rows.iter().map(|row| parse_row(row, plugins, lang)).collect())
            .unwrap_or_default();
        PackageView {
            name: bundle["name"].as_str().unwrap_or_default().to_string(),
            official: bundle["official"].as_bool().unwrap_or(false),
            availability: bundle["availability"]
                .as_str()
                .unwrap_or("profile")
                .to_string(),
            install_target: match bundle.get("installTarget") {
                Some(target) => Some(InstallTarget {
                    spec: target["spec"].as_str().unwrap_or_default().to_string(),
                    version: target["version"].as_str().unwrap_or_default().to_string(),
                }),
                None => None,
            },
            version: bundle["version"].as_str().map(str::to_string),
            description: bundle["description"].as_str().map(str::to_string),
            title: resolve_text(meta.get("title"), lang),
            meta_description: resolve_text(meta.get("description"), lang),
            icon: meta.get("icon").and_then(Value::as_str).map(str::to_string),
            meta_error: meta.get("error").and_then(Value::as_str).map(str::to_string),
            installed: bundle["installed"].as_bool().unwrap_or(false),
            listed_by_default: bundle["listedByDefault"].as_bool(),
            source: bundle["source"].as_str().map(str::to_string),
            optional: bundle["optional"].as_bool().unwrap_or(false),
            removable: bundle["removable"].as_bool().unwrap_or(false),
            updatable: bundle["updatable"].as_bool().unwrap_or(false),
            enabled: bundle["enabled"].as_bool().unwrap_or(false),
            read_only_reason: bundle["readOnlyReason"].as_str().map(str::to_string),
            error: bundle.get("error").and_then(|error| {
                Some(PackageError {
                    code: error["code"].as_str()?.to_string(),
                    diagnostic: error["diagnostic"].as_str().map(str::to_string),
                })
            }),
            rows,
        }
    }

    pub fn parse_inventory_lang(bundles: &[Value], plugins: &[Value], lang: &str) -> Vec<PackageView> {
        let mut packages: Vec<PackageView> = bundles
            .iter()
            .map(|bundle| parse_bundle(bundle, plugins, lang))
            .collect();
        packages.sort_by(|a, b| short_name(&a.name).cmp(&short_name(&b.name)));
        packages
    }

    pub fn parse_inventory(bundles: &[Value], plugins: &[Value]) -> Vec<PackageView> {
        parse_inventory_lang(bundles, plugins, "zh")
    }

    pub fn package_status(pkg: &PackageView) -> PackageStatus {
        if pkg.error.is_some()
            || (pkg.enabled && pkg.rows.iter().any(|row| row.phase == Some(RowPhase::Failed)))
        {
            return PackageStatus::Problem;
        }
        if pkg.enabled {
            PackageStatus::Running
        } else {
            PackageStatus::Disabled
        }
    }

    pub fn update_available(pkg: &PackageView) -> bool {
        let Some(target) = pkg.install_target.as_ref() else {
            return false;
        };
        if !pkg.installed || !pkg.removable || pkg.availability == "installation" {
            return false;
        }
        pkg.version.as_deref() != Some(target.version.as_str())
            || ((target.spec.starts_with("link:")
                || pkg.source.as_deref().is_some_and(|spec| spec.starts_with("link:")))
                && pkg.source.as_deref() != Some(target.spec.as_str()))
    }

    pub fn package_tags(pkg: &PackageView) -> Vec<PackageTag> {
        let mut tags = Vec::new();
        if EXPERIMENTAL_BUNDLES.contains(&pkg.name.as_str()) {
            tags.push(PackageTag::Experimental);
        }
        if package_status(pkg) == PackageStatus::Problem {
            tags.push(PackageTag::Problem);
        }
        if pkg.availability == "missing" {
            tags.push(if pkg.installed {
                PackageTag::MissingFiles
            } else {
                PackageTag::NotInstalled
            });
        }
        if update_available(pkg) {
            tags.push(PackageTag::Updatable);
        }
        tags
    }

    pub fn row_state_text(row: &PackageRow) -> &'static str {
        match (row.enabled, row.phase) {
            (false, _) => "已关闭",
            (true, None) => "未运行",
            (true, Some(RowPhase::Pending)) => "等待依赖",
            (true, Some(RowPhase::Loading)) => "加载中",
            (true, Some(RowPhase::Active)) => "运行中",
            (true, Some(RowPhase::Failed)) => "异常",
            (true, Some(RowPhase::Unloading)) => "卸载中",
        }
    }

    pub fn row_dot(row: &PackageRow) -> DotState {
        match (row.enabled, row.phase) {
            (false, _) | (true, None) | (true, Some(RowPhase::Pending)) => DotState::Idle,
            (true, Some(RowPhase::Loading)) | (true, Some(RowPhase::Unloading)) => DotState::Ongoing,
            (true, Some(RowPhase::Active)) => DotState::Done,
            (true, Some(RowPhase::Failed)) => DotState::Error,
        }
    }

    pub fn rows_running(pkg: &PackageView) -> bool {
        pkg.rows.iter().any(|row| row.enabled)
    }

    pub fn pinned_to_main_list(pkg: &PackageView) -> bool {
        !MORE_BUNDLES.contains(&pkg.name.as_str()) || pkg.enabled
    }

    pub fn optional_bundle_rank(name: &str) -> usize {
        match EXTENSION_BUNDLES.iter().position(|entry| *entry == name) {
            Some(index) => index,
            None => match MORE_BUNDLES.iter().position(|entry| *entry == name) {
                Some(index) => EXTENSION_BUNDLES.len() + 1 + index,
                None => EXTENSION_BUNDLES.len(),
            },
        }
    }

    pub fn served_builtin_items(namespaces: &[String]) -> Vec<&'static OfficialItem> {
        let mut items: Vec<&'static OfficialItem> = BUILTIN_ITEMS
            .iter()
            .filter(|item| {
                item.namespaces
                    .iter()
                    .any(|ns| namespaces.iter().any(|served| served == ns))
            })
            .collect();
        items.sort_by_key(|item| item.order);
        items
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct PackageGroups {
        pub items: Vec<&'static OfficialItem>,
        pub extensions: Vec<PackageView>,
        pub more: Vec<PackageView>,
        pub installed: Vec<PackageView>,
    }

    pub fn group_packages(packages: &[PackageView], served: &[&'static OfficialItem]) -> PackageGroups {
        let listed: Vec<PackageView> = packages
            .iter()
            .filter(|pkg| {
                !HIDDEN_BUNDLES.contains(&pkg.name.as_str())
                    && (pkg.listed_by_default != Some(false)
                        || pkg.enabled
                        || pkg.installed
                        || rows_running(pkg))
                    && (pkg.installed || pkg.official || pkg.error.is_some())
            })
            .cloned()
            .collect();
        let installed = listed.iter().filter(|pkg| !pkg.official).cloned().collect();
        let mut ordered: Vec<PackageView> = listed
            .iter()
            .filter(|pkg| pkg.official)
            .cloned()
            .collect();
        ordered.sort_by_key(|pkg| optional_bundle_rank(&pkg.name));
        let pinned: HashSet<&str> = ordered
            .iter()
            .filter(|pkg| pinned_to_main_list(pkg))
            .map(|pkg| pkg.name.as_str())
            .collect();
        let extensions = ordered
            .iter()
            .filter(|pkg| pinned.contains(pkg.name.as_str()))
            .cloned()
            .collect();
        let more = ordered;
        PackageGroups {
            items: served.to_vec(),
            extensions,
            more,
            installed,
        }
    }

    pub fn management_reason_text(code: &str) -> &'static str {
        match code {
            "management-required" => "插件管理所需，不能停用或卸载",
            "unaddressable" => "当前 profile 的 patch 无法唯一定位这一项",
            "unknown-plugin" => "找不到该插件",
            "invalid-spec" => "请输入有效的包名或地址",
            "already-installed" => "这个包已经安装；要安装其他版本，请先卸载它",
            "ambiguous-install" => "无法从依赖变更中确定安装了哪一个包",
            "not-bundle" => "这个包没有声明组合包，不能作为插件管理",
            "not-removable" => "这个包不属于当前 profile，或者是插件管理所需的组件",
            "stop-profile" => "这个 profile 没有启用 HMR，正在使用的包要停止后用 dsh plugin 卸载",
            "bundle-in-use" => "其他配置仍在使用这个组合包的组件，请先停用它们",
            "stale-approval" => "待允许的安装脚本列表已变化，请重新安装以刷新",
            "incompatible-version" => "这个插件与当前 DSH 版本不兼容，运行它可能导致崩溃或数据丢失。",
            "unsupported-platform" => "此插件不支持运行 DSH 的主机操作系统",
            _ => "Host 报告了一个错误",
        }
    }

    pub fn restart_reason_text(reason: Option<&str>) -> &'static str {
        match reason {
            Some("no-hmr") => "这个 profile 只在启动时应用更改。",
            Some("package-moved") => "已安装的插件包解析到了另一个目录，或者更改替换了 DSH 安装自带的包。从其他位置安装前请先卸载。",
            Some("host-module") => "更改涉及 DSH 自身已加载的模块。",
            Some("unmanaged-plugin") => "更改涉及一个不受插件列表管理的插件。",
            Some("module-comparison") => "DSH 无法将已加载的插件与磁盘上的文件进行比较。",
            _ => "",
        }
    }

    pub fn change_notice_text(result: &PluginChangeResult, name: &str) -> Option<String> {
        match result.application.as_str() {
            "restart-required" => Some(format!(
                "请重启 DSH 使更改生效。{}",
                restart_reason_text(result.restart_reason.as_deref())
            )),
            "overridden" => Some(format!(
                "{name} 已保存，但被更高优先级的配置覆盖，当前未生效"
            )),
            "cancelled" => Some("已取消安装，插件未启用，下载的文件可能保留".to_string()),
            _ => None,
        }
    }

    pub fn change_failure_text(result: &PluginChangeResult, action: FailedAction) -> String {
        let reason = match result.error.as_ref() {
            Some(error) => error
                .diagnostic
                .clone()
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| management_reason_text(&error.code).to_string()),
            None => String::new(),
        };
        let reason = if reason.is_empty() {
            "Host 报告了一个错误".to_string()
        } else {
            reason
        };
        format!("{}：{reason}", action.label())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SpecKind {
        Npm,
        Git,
        Url,
        PathOrOther,
    }

    fn is_npm_spec(s: &str) -> bool {
        if s.is_empty() || s.starts_with('.') || s.starts_with('/') || s.starts_with('\\') {
            return false;
        }
        let (pkg_part, _version_part) = if let Some(stripped) = s.strip_prefix('@') {
            let Some((scope, rest)) = stripped.split_once('/') else {
                return false;
            };
            if scope.is_empty()
                || !scope
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
            {
                return false;
            }
            match rest.split_once('@') {
                Some((name, ver)) => (name, Some(ver)),
                None => (rest, None),
            }
        } else {
            match s.split_once('@') {
                Some((name, ver)) => (name, Some(ver)),
                None => (s, None),
            }
        };
        if pkg_part.is_empty()
            || pkg_part.contains('/')
            || pkg_part.contains('\\')
            || pkg_part.contains(':')
        {
            return false;
        }
        let mut chars = pkg_part.chars();
        let first = chars.next().unwrap();
        if !first.is_ascii_alphanumeric() {
            return false;
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    }

    pub fn classify_spec_input(spec: &str) -> SpecKind {
        let lower = spec.trim().to_ascii_lowercase();
        if lower.starts_with("git+")
            || lower.starts_with("git:")
            || lower.starts_with("git@")
            || lower.starts_with("github:")
            || lower.starts_with("gitlab:")
            || lower.starts_with("bitbucket:")
        {
            return SpecKind::Git;
        }
        if let Some((scheme, _)) = lower.split_once("://") {
            if !scheme.is_empty()
                && scheme.chars().next().unwrap().is_ascii_alphabetic()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '.' || c == '-')
            {
                return SpecKind::Url;
            }
        }
        let trimmed = spec.trim();
        if is_npm_spec(trimmed) {
            return SpecKind::Npm;
        }
        SpecKind::PathOrOther
    }

    pub fn sanitize_install_input(spec: &str) -> String {
        match classify_spec_input(spec) {
            SpecKind::Npm => spec.trim().to_string(),
            SpecKind::Git => "[git]".to_string(),
            SpecKind::Url => "[url]".to_string(),
            SpecKind::PathOrOther => "[path-or-other]".to_string(),
        }
    }

    pub fn input_problem_sentence(
        problem: &str,
        reason: &str,
        registries: &[Option<String>],
    ) -> String {
        match problem {
            "invalid-spec" => format!("无法识别这个包名或地址：{reason}"),
            "already-installed" => "该插件已安装。如需升级，请卸载后重新安装".to_string(),
            "shipped" => "该插件随 DSH 提供，升级 DSH 即可获得新版本".to_string(),
            "not-found" => "未找到相关插件".to_string(),
            "not-a-package" => "该路径不存在或不是有效的插件包".to_string(),
            "not-a-bundle" => format!("这个包没有声明组合包，无法作为插件安装：{reason}"),
            "network" => {
                let asked: Vec<&str> = registries.iter().filter_map(|r| r.as_deref()).collect();
                if asked.len() > 1 {
                    format!(
                        "所有安装源都无法连接（已尝试：{}），请检查网络或代理设置，或更换安装源",
                        asked.join("、")
                    )
                } else {
                    "无法连接插件源，请检查网络后重试".to_string()
                }
            }
            "unknown" | _ => {
                if reason.is_empty() {
                    "无法获取插件信息".to_string()
                } else {
                    format!("无法获取插件信息：{reason}")
                }
            }
        }
    }

    pub fn validate_custom_registry_url(url: &str) -> Result<String, &'static str> {
        let trimmed = url.trim();
        if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
            return Err("请输入以 http:// 或 https:// 开头的地址");
        }
        let rest = if let Some(r) = trimmed.strip_prefix("https://") {
            r
        } else {
            trimmed.strip_prefix("http://").unwrap()
        };
        let host = rest.split('/').next().unwrap_or("").trim();
        if host.is_empty() {
            return Err("请输入以 http:// 或 https:// 开头的地址");
        }
        let mut normalized = trimmed.to_string();
        if !normalized.ends_with('/') {
            normalized.push('/');
        }
        Ok(normalized)
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum RegistryChoice {
        Default,
        Offered(String),
        Custom(String),
    }

    impl RegistryChoice {
        pub fn resolved_url<'a>(&'a self, registries: &'a PluginRegistries) -> Option<&'a str> {
            match self {
                RegistryChoice::Default => registries.resolved.as_deref(),
                RegistryChoice::Offered(url) => Some(url.as_str()),
                RegistryChoice::Custom(url) => Some(url.as_str()),
            }
        }

        pub fn display_name(&self) -> String {
            match self {
                RegistryChoice::Default => "默认安装源".to_string(),
                RegistryChoice::Offered(url) => url.clone(),
                RegistryChoice::Custom(url) => format!("自定义源 ({url})"),
            }
        }
    }

    pub fn reconcile_registry_choice(
        saved_choice: Option<&str>,
        registries: &PluginRegistries,
        probe_fastest: Option<&str>,
    ) -> RegistryChoice {
        if let Some(saved) = saved_choice.map(str::trim).filter(|s| !s.is_empty()) {
            if saved == "default"
                || (registries.resolved.as_deref() == Some(saved) && registries.registry.is_none())
            {
                return RegistryChoice::Default;
            }
            if registries.registry.as_deref() == Some(saved)
                || registries.fallback_registries.iter().any(|r| r == saved)
            {
                return RegistryChoice::Offered(saved.to_string());
            }
            if saved.starts_with("http://") || saved.starts_with("https://") {
                return RegistryChoice::Custom(saved.to_string());
            }
            return RegistryChoice::Default;
        }
        if let Some(fastest) = probe_fastest.map(str::trim).filter(|s| !s.is_empty()) {
            if registries.fallback_registries.iter().any(|r| r == fastest)
                || registries.registry.as_deref() == Some(fastest)
            {
                return RegistryChoice::Offered(fastest.to_string());
            }
            if let Some(ref primary) = registries.registry {
                return RegistryChoice::Offered(primary.clone());
            }
            return RegistryChoice::Default;
        }
        if let Some(ref primary) = registries.registry {
            RegistryChoice::Offered(primary.clone())
        } else {
            RegistryChoice::Default
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum InstallPhase {
        Idle,
        Checking,
        Confirmed,
        Running,
        Cancelling,
        Applying,
        Unconfirmed,
        Done,
        Failed,
    }

    pub fn is_install_pending(phase: InstallPhase) -> bool {
        matches!(
            phase,
            InstallPhase::Running
                | InstallPhase::Cancelling
                | InstallPhase::Applying
                | InstallPhase::Unconfirmed
        )
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InstallSubjectInfo {
        pub spec: String,
        pub kind: String,
        pub name: Option<String>,
        pub version: Option<String>,
        pub description: Option<String>,
        pub bundle: Option<bool>,
        pub registry: Option<String>,
        pub host: Option<String>,
        pub bundled: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InstallRunJob {
        pub job_id: String,
        pub argv: Vec<String>,
        pub cwd: String,
        pub lines: Vec<String>,
        pub exit_code: Option<i64>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InstallFailureDetails {
        pub reason: String,
        pub code: Option<String>,
        pub kind: Option<String>,
        pub stage: Option<String>,
        pub failed_at: Option<String>,
        pub pending_builds: Vec<String>,
        pub uncertainty: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct InstallStateModel {
        pub phase: InstallPhase,
        pub spec: String,
        pub registry_choice: RegistryChoice,
        pub request_id: Option<String>,
        pub input_error: Option<String>,
        pub subject: Option<InstallSubjectInfo>,
        pub market_title: Option<String>,
        pub runs: Vec<InstallRunJob>,
        pub log_path: Option<String>,
        pub installed_name: Option<String>,
        pub installed_version: Option<String>,
        pub restart_required: bool,
        pub restart_reason: Option<String>,
        pub failure: Option<InstallFailureDetails>,
        pub approved_builds: Vec<String>,
    }

    impl Default for InstallStateModel {
        fn default() -> Self {
            Self {
                phase: InstallPhase::Idle,
                spec: String::new(),
                registry_choice: RegistryChoice::Default,
                request_id: None,
                input_error: None,
                subject: None,
                market_title: None,
                runs: Vec::new(),
                log_path: None,
                installed_name: None,
                installed_version: None,
                restart_required: false,
                restart_reason: None,
                failure: None,
                approved_builds: Vec::new(),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum InstallAction {
        StartCheck {
            spec: String,
            choice: RegistryChoice,
        },
        InspectRefused {
            problem: String,
            reason: String,
            registries: Vec<Option<String>>,
        },
        InspectBundled {
            subject: InstallSubjectInfo,
        },
        InspectAccepted {
            subject: InstallSubjectInfo,
        },
        StartInstall {
            request_id: String,
            approved_builds: Vec<String>,
        },
        ProgressPhase {
            phase: String,
        },
        AppendLog {
            job_id: String,
            argv: Vec<String>,
            cwd: String,
            text: String,
            exit_code: Option<i64>,
        },
        RequestCancel,
        CancelResult {
            status: String,
        },
        InstallResult {
            result: PluginChangeResult,
        },
        WaitInstallResult {
            result: Option<PluginChangeResult>,
        },
        ApproveBuildsAndRetry {
            request_id: String,
        },
        Reset,
    }

    pub fn extract_pending_builds(change_result: &PluginChangeResult) -> Vec<String> {
        let mut builds = Vec::new();
        if let Some(list) = &change_result.pending_builds {
            for name in list {
                if !builds.contains(name) {
                    builds.push(name.clone());
                }
            }
        }
        builds
    }

    pub fn classify_install_failure(change: &PluginChangeResult) -> InstallFailureDetails {
        let reason = if change.error.as_ref().map(|e| e.code.as_str())
            == Some("incompatible-version")
        {
            "这个插件与当前 DSH 版本不兼容，运行它可能导致崩溃或数据丢失。".to_string()
        } else if let Some(ref pkg_res) = change.package_result {
            match pkg_res.kind.as_deref() {
                Some("pnpm-missing") => "没有找到 pnpm，无法安装依赖".to_string(),
                Some("timeout") => "安装超时，请检查网络后重试".to_string(),
                Some("not-found") => "未找到相关插件".to_string(),
                Some("no-matching-version") => "没有匹配的版本".to_string(),
                Some("network") => {
                    if change.failed_at.as_deref() == Some("spec-host") {
                        "无法连接目标主机。GitHub 地址和 .tgz 直链不经过安装源，需要本机能直接访问它或配置代理".to_string()
                    } else {
                        "网络连接失败，请检查网络或更换安装源后重试".to_string()
                    }
                }
                Some("disk-full") => "磁盘空间不足，安装已停止".to_string(),
                Some("permission") => "没有写入权限，无法安装".to_string(),
                Some("build-blocked") => "有依赖的安装脚本需要你允许后才能继续".to_string(),
                Some("integrity") => "下载的安装包校验失败".to_string(),
                _ => {
                    if !pkg_res.output.trim().is_empty() {
                        let first_line = pkg_res.output.lines().next().unwrap_or("安装失败");
                        first_line.to_string()
                    } else {
                        "安装失败".to_string()
                    }
                }
            }
        } else if let Some(ref err) = change.error {
            err.diagnostic
                .clone()
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| management_reason_text(&err.code).to_string())
        } else {
            "安装失败".to_string()
        };

        InstallFailureDetails {
            reason,
            code: change.error.as_ref().map(|e| e.code.clone()),
            kind: change.package_result.as_ref().and_then(|p| p.kind.clone()),
            stage: Some(change.stage.clone()),
            failed_at: change.failed_at.clone(),
            pending_builds: extract_pending_builds(change),
            uncertainty: None,
        }
    }

    pub fn reduce_install(state: &mut InstallStateModel, action: InstallAction) {
        match action {
            InstallAction::StartCheck { spec, choice } => {
                state.spec = spec;
                state.registry_choice = choice;
                state.phase = InstallPhase::Checking;
                state.input_error = None;
                state.subject = None;
            }
            InstallAction::InspectRefused {
                problem,
                reason,
                registries,
            } => {
                state.phase = InstallPhase::Idle;
                state.input_error = Some(input_problem_sentence(&problem, &reason, &registries));
            }
            InstallAction::InspectBundled { subject } => {
                state.installed_name = subject.name.clone();
                state.installed_version = subject.version.clone();
                state.restart_required = false;
                state.restart_reason = None;
                state.subject = Some(subject);
                state.phase = InstallPhase::Done;
            }
            InstallAction::InspectAccepted { subject } => {
                state.subject = Some(subject);
                state.phase = InstallPhase::Confirmed;
            }
            InstallAction::StartInstall {
                request_id,
                approved_builds,
            } => {
                state.request_id = Some(request_id);
                state.approved_builds = approved_builds;
                state.phase = InstallPhase::Running;
                state.runs.clear();
                state.failure = None;
            }
            InstallAction::ProgressPhase { phase } => {
                if phase == "applying" {
                    state.phase = InstallPhase::Applying;
                }
            }
            InstallAction::AppendLog {
                job_id,
                argv,
                cwd,
                text,
                exit_code,
            } => {
                let split_lines: Vec<String> = text
                    .lines()
                    .filter(|l| !l.is_empty())
                    .map(str::to_string)
                    .collect();
                if let Some(run) = state.runs.iter_mut().find(|r| r.job_id == job_id) {
                    run.lines.extend(split_lines);
                    if exit_code.is_some() {
                        run.exit_code = exit_code;
                    }
                } else {
                    state.runs.push(InstallRunJob {
                        job_id,
                        argv,
                        cwd,
                        lines: split_lines,
                        exit_code,
                    });
                }
            }
            InstallAction::RequestCancel => {
                state.phase = InstallPhase::Cancelling;
            }
            InstallAction::CancelResult { status } => match status.as_str() {
                "cancelled" => {
                    state.phase = InstallPhase::Idle;
                }
                "too-late" => {
                    state.phase = InstallPhase::Applying;
                }
                _ => {
                    state.phase = InstallPhase::Unconfirmed;
                }
            },
            InstallAction::InstallResult { result } => {
                if result.application == "applied" || result.application == "restart-required" {
                    state.phase = InstallPhase::Done;
                    state.installed_name = result
                        .bundle
                        .clone()
                        .or_else(|| state.subject.as_ref().and_then(|s| s.name.clone()));
                    state.installed_version = result
                        .version
                        .clone()
                        .or_else(|| state.subject.as_ref().and_then(|s| s.version.clone()));
                    state.restart_required = result.application == "restart-required";
                    state.restart_reason = result.restart_reason.clone();
                } else if result.application == "cancelled" {
                    state.phase = InstallPhase::Idle;
                } else {
                    state.phase = InstallPhase::Failed;
                    state.failure = Some(classify_install_failure(&result));
                }
            }
            InstallAction::WaitInstallResult { result } => match result {
                Some(res) => {
                    reduce_install(state, InstallAction::InstallResult { result: res });
                }
                None => {
                    state.phase = InstallPhase::Unconfirmed;
                }
            },
            InstallAction::ApproveBuildsAndRetry { request_id } => {
                if let Some(failure) = &state.failure {
                    for pkg in &failure.pending_builds {
                        if !state.approved_builds.contains(pkg) {
                            state.approved_builds.push(pkg.clone());
                        }
                    }
                }
                state.request_id = Some(request_id);
                state.phase = InstallPhase::Running;
                state.failure = None;
            }
            InstallAction::Reset => {
                *state = InstallStateModel::default();
            }
        }
    }

    pub fn installable_version(v: &PluginVersionItem) -> bool {
        v.bundle && v.incompatible.is_none()
    }

    pub fn update_candidate(versions: &[PluginVersionItem]) -> Option<&PluginVersionItem> {
        versions
            .iter()
            .find(|v| installable_version(v) && v.deprecated.is_none() && !v.prerelease)
    }

    pub fn filter_version_items<'a>(
        versions: &'a [PluginVersionItem],
        show_prereleases: bool,
    ) -> Vec<&'a PluginVersionItem> {
        versions
            .iter()
            .filter(|v| show_prereleases || !v.prerelease)
            .collect()
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum VersionSummaryKind {
        Checking,
        Refused {
            title: String,
            sub: String,
        },
        Available {
            version: String,
            published_at: Option<String>,
        },
        Latest {
            checked_at: Option<String>,
        },
        NoneInstallable,
        BlockedDsh {
            version: String,
            peers: String,
            runtime: String,
        },
    }

    pub fn evaluate_version_summary(
        list: Option<&PluginVersionList>,
        checking: bool,
        checked_time: Option<&str>,
    ) -> VersionSummaryKind {
        if checking || list.is_none() {
            return VersionSummaryKind::Checking;
        }
        match list.unwrap() {
            PluginVersionList::Refused {
                problem,
                reason,
                attempts,
            } => {
                let failed = attempts.last();
                let sub = if let Some(attempt) = failed {
                    let reg_name = attempt.registry.as_deref().unwrap_or("默认安装源");
                    let prob = match attempt.problem.as_deref() {
                        Some("network") => "无法连接",
                        Some("not-found") => "未找到",
                        _ => "失败",
                    };
                    format!("{reg_name} {prob}")
                } else {
                    reason.clone()
                };
                let title = if problem == "network" {
                    "无法连接安装源".to_string()
                } else {
                    "无法检查更新".to_string()
                };
                VersionSummaryKind::Refused { title, sub }
            }
            PluginVersionList::Listed { versions, .. } => {
                if let Some(candidate) = update_candidate(versions) {
                    VersionSummaryKind::Available {
                        version: candidate.version.clone(),
                        published_at: candidate.published_at.clone(),
                    }
                } else if versions.is_empty() {
                    VersionSummaryKind::Latest {
                        checked_at: checked_time.map(str::to_string),
                    }
                } else if let Some(blocked) =
                    versions.iter().find(|v| !v.prerelease && v.incompatible.is_some())
                {
                    let incomp = blocked.incompatible.as_ref().unwrap();
                    let peers = incomp
                        .requirements
                        .iter()
                        .map(|(k, v)| format!("{k}@{v}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    VersionSummaryKind::BlockedDsh {
                        version: blocked.version.clone(),
                        peers,
                        runtime: incomp.runtime_version.clone(),
                    }
                } else {
                    VersionSummaryKind::NoneInstallable
                }
            }
        }
    }

    pub fn ordered_update_steps(
        announced: &[String],
        result_step: Option<&str>,
        restore_present: bool,
    ) -> Vec<String> {
        const STEP_ORDER: [&str; 4] = ["checking", "removing", "installing", "restoring"];
        let mut implied = HashSet::new();
        implied.insert("checking".to_string());
        for s in announced {
            implied.insert(s.clone());
        }
        if result_step == Some("remove") {
            implied.insert("removing".to_string());
        }
        if result_step == Some("install") {
            implied.insert("installing".to_string());
        }
        if restore_present {
            implied.insert("restoring".to_string());
        }
        STEP_ORDER
            .iter()
            .filter(|step| implied.contains(&step.to_string()))
            .map(|s| s.to_string())
            .collect()
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum UpdateOutcomeKind {
        Done {
            version: String,
        },
        RestartRequired {
            version: String,
            reason: Option<String>,
        },
        Refused {
            version: String,
            reason: String,
        },
        RemoveFailed {
            version: String,
            reason: String,
        },
        Restored {
            from: String,
            to: String,
            reason: String,
        },
        Kept {
            from: String,
            to: String,
            reason: String,
        },
        Broken {
            from: String,
            to: String,
        },
        Lost {
            reason: String,
        },
    }

    pub fn evaluate_update_outcome(
        result: Option<&PluginUpdateResult>,
        lost_reason: Option<&str>,
    ) -> UpdateOutcomeKind {
        let Some(up) = result else {
            return UpdateOutcomeKind::Lost {
                reason: lost_reason.unwrap_or("与 DSH 的连接中断").to_string(),
            };
        };
        let reason_fn = |res: &PluginChangeResult| -> String {
            res.error
                .as_ref()
                .and_then(|e| e.diagnostic.clone())
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| {
                    management_reason_text(
                        res.error.as_ref().map(|e| e.code.as_str()).unwrap_or(""),
                    )
                    .to_string()
                })
        };
        if up.step == "check" {
            return UpdateOutcomeKind::Refused {
                version: up.to.clone(),
                reason: reason_fn(&up.result),
            };
        }
        if up.step == "remove" {
            return UpdateOutcomeKind::RemoveFailed {
                version: up.from.clone(),
                reason: reason_fn(&up.result),
            };
        }
        let app = up.result.application.as_str();
        if app != "failed" && app != "cancelled" {
            if app == "restart-required" {
                return UpdateOutcomeKind::RestartRequired {
                    version: up.to.clone(),
                    reason: up.result.restart_reason.clone(),
                };
            }
            return UpdateOutcomeKind::Done {
                version: up.to.clone(),
            };
        }
        if up.restore.is_none() {
            return UpdateOutcomeKind::Kept {
                from: up.from.clone(),
                to: up.to.clone(),
                reason: reason_fn(&up.result),
            };
        }
        let restore = up.restore.as_ref().unwrap();
        if restore.application == "failed" || restore.application == "cancelled" {
            UpdateOutcomeKind::Broken {
                from: up.from.clone(),
                to: up.to.clone(),
            }
        } else {
            UpdateOutcomeKind::Restored {
                from: up.from.clone(),
                to: up.to.clone(),
                reason: reason_fn(&up.result),
            }
        }
    }

    pub fn fold_terminal_lines(
        lines: &[String],
        limit: usize,
    ) -> (Vec<String>, usize, Vec<String>) {
        if lines.len() <= limit {
            return (lines.to_vec(), 0, Vec::new());
        }
        let half = limit / 2;
        let head = lines[..half].to_vec();
        let folded = lines.len() - limit;
        let tail = lines[lines.len() - half..].to_vec();
        (head, folded, tail)
    }
}

use model::*;

pub mod market {
    use crate::protocol::{
        MarketCompatibility, MarketIndexMetadata, MarketIndexSummary, MarketListingEntry,
        MarketListingVersion, MarketQuery, MarketSort, MarketSortDirection,
    };
    use super::model::resolve_text;

    pub const MARKET_PAGE_SIZE: usize = 50;
    pub const MARKET_MAX_PAGE: usize = 100;

    pub const MARKET_TEXT_DELAY_MS: u64 = 250;

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct DiscoverQuery {
        pub text: String,
        pub category: Option<String>,
        pub labels: Vec<String>,
        pub sort: MarketSort,
    }

    impl Default for MarketSort {
        fn default() -> Self {
            MarketSort::Index
        }
    }

    pub fn initial_query() -> DiscoverQuery {
        DiscoverQuery {
            text: String::new(),
            category: None,
            labels: Vec::new(),
            sort: MarketSort::Index,
        }
    }

    pub fn with_text(query: &DiscoverQuery, text: &str) -> DiscoverQuery {
        let was_blank = query.text.trim().is_empty();
        let blank = text.trim().is_empty();
        let sort = match &query.sort {
            MarketSort::Index if was_blank && !blank => MarketSort::Relevance,
            MarketSort::Relevance if blank => MarketSort::Index,
            other => other.clone(),
        };
        DiscoverQuery {
            text: text.to_string(),
            category: query.category.clone(),
            labels: query.labels.clone(),
            sort,
        }
    }

    pub fn declared_query(query: &DiscoverQuery, metadata: &MarketIndexMetadata) -> DiscoverQuery {
        let category = match &query.category {
            Some(id) if metadata.categories.contains_key(id) => Some(id.clone()),
            _ => None,
        };
        let labels: Vec<String> = query
            .labels
            .iter()
            .filter(|label| metadata.labels.contains_key(*label))
            .cloned()
            .collect();
        let sort = match &query.sort {
            MarketSort::Key { id, .. } if !metadata.sort_keys.contains_key(id) => MarketSort::Index,
            other => other.clone(),
        };
        DiscoverQuery {
            text: query.text.clone(),
            category,
            labels,
            sort,
        }
    }

    pub fn market_query_params(
        query: &DiscoverQuery,
        offset: usize,
        limit: usize,
        fetched_at: Option<u64>,
        refresh: bool,
    ) -> MarketQuery {
        MarketQuery {
            text: (!query.text.trim().is_empty()).then(|| query.text.clone()),
            category: query.category.clone(),
            labels: (!query.labels.is_empty()).then(|| query.labels.clone()),
            sort: query.sort.clone(),
            offset: offset as u32,
            limit: limit as u32,
            refresh: refresh.then_some(true),
            fetched_at,
        }
    }

    pub fn presented_version(entry: &MarketListingEntry) -> Option<&MarketListingVersion> {
        entry
            .versions
            .iter()
            .find(|version| Some(version.version.as_str()) == entry.default_version.as_deref())
            .or_else(|| entry.versions.first())
    }

    pub fn installable(compatibility: &MarketCompatibility) -> bool {
        compatibility.compatible || compatibility.exempted
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MarketInstallStatus {
        Installed,
        UpdateAvailable,
        Provided,
    }

    impl MarketInstallStatus {
        pub fn text(self) -> &'static str {
            match self {
                MarketInstallStatus::Installed => "已安装",
                MarketInstallStatus::UpdateAvailable => "有新版本",
                MarketInstallStatus::Provided => "随 DSH 提供",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MarketEntryAction {
        Install,
        Status(MarketInstallStatus),
    }

    pub fn entry_action(entry: &MarketListingEntry) -> MarketEntryAction {
        match entry.installation.status.as_str() {
            "not-installed" => MarketEntryAction::Install,
            "update-available" => MarketEntryAction::Status(MarketInstallStatus::UpdateAvailable),
            "provided" => MarketEntryAction::Status(MarketInstallStatus::Provided),
            _ => MarketEntryAction::Status(MarketInstallStatus::Installed),
        }
    }

    pub fn entry_install_blocked_reason(entry: &MarketListingEntry) -> Option<String> {
        let version = presented_version(entry)?;
        if installable(&version.compatibility) {
            return None;
        }
        let runtime = version
            .compatibility
            .runtime_version
            .as_deref()
            .unwrap_or("未知");
        Some(format!("没有与当前 DSH {runtime} 兼容的版本"))
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MarketVersionTag {
        Recommended,
        Prerelease,
        Installed,
        Incompatible,
        Exempted,
    }

    impl MarketVersionTag {
        pub fn text(self) -> &'static str {
            match self {
                MarketVersionTag::Recommended => "推荐",
                MarketVersionTag::Prerelease => "预发布",
                MarketVersionTag::Installed => "当前已安装",
                MarketVersionTag::Incompatible => "不兼容",
                MarketVersionTag::Exempted => "已豁免",
            }
        }

        pub fn tone(self) -> &'static str {
            match self {
                MarketVersionTag::Recommended => "accent",
                MarketVersionTag::Installed => "success",
                MarketVersionTag::Incompatible | MarketVersionTag::Exempted => "warning",
                MarketVersionTag::Prerelease => "neutral",
            }
        }
    }

    pub fn version_tags(
        entry: &MarketListingEntry,
        version: &MarketListingVersion,
    ) -> Vec<MarketVersionTag> {
        let mut tags = Vec::new();
        if entry.default_version.as_deref() == Some(version.version.as_str()) {
            tags.push(MarketVersionTag::Recommended);
        }
        let base = version.version.split('+').next().unwrap_or(&version.version);
        if base.contains('-') {
            tags.push(MarketVersionTag::Prerelease);
        }
        if entry.installation.installed_version.as_deref() == Some(version.version.as_str()) {
            tags.push(MarketVersionTag::Installed);
        }
        if !version.compatibility.compatible {
            tags.push(if version.compatibility.exempted {
                MarketVersionTag::Exempted
            } else {
                MarketVersionTag::Incompatible
            });
        }
        tags
    }

    pub fn version_install_offered(
        entry: &MarketListingEntry,
        version: &MarketListingVersion,
    ) -> bool {
        let current = entry.installation.installed_version.as_deref() == Some(version.version.as_str());
        !current && entry.installation.status != "provided" && installable(&version.compatibility)
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MarketSourceFact {
        Package(String),
        Registry(String),
        Repository(String),
        Commit(String),
        Directory(String),
    }

    impl MarketSourceFact {
        pub fn label(&self) -> &'static str {
            match self {
                MarketSourceFact::Package(_) => "软件包",
                MarketSourceFact::Registry(_) => "注册表",
                MarketSourceFact::Repository(_) => "仓库",
                MarketSourceFact::Commit(_) => "提交",
                MarketSourceFact::Directory(_) => "目录",
            }
        }

        pub fn value(&self) -> &str {
            match self {
                MarketSourceFact::Package(value)
                | MarketSourceFact::Registry(value)
                | MarketSourceFact::Repository(value)
                | MarketSourceFact::Commit(value)
                | MarketSourceFact::Directory(value) => value,
            }
        }
    }

    pub fn source_facts(
        name: &str,
        version: &MarketListingVersion,
    ) -> Vec<MarketSourceFact> {
        match &version.source {
            crate::protocol::MarketSource::Npm { registry } => {
                let mut facts = vec![MarketSourceFact::Package(format!(
                    "{name}@{}",
                    version.version
                ))];
                if let Some(registry) = registry {
                    facts.push(MarketSourceFact::Registry(registry.clone()));
                }
                facts
            }
            crate::protocol::MarketSource::Git { url, commit, path } => {
                let mut facts = vec![
                    MarketSourceFact::Repository(url.clone()),
                    MarketSourceFact::Commit(commit.clone()),
                ];
                if let Some(path) = path {
                    facts.push(MarketSourceFact::Directory(path.clone()));
                }
                facts
            }
        }
    }

    pub fn resolve_localized(value: Option<&serde_json::Value>) -> Option<String> {
        resolve_text(value, "zh")
    }

    pub fn entry_title(entry: &MarketListingEntry) -> String {
        resolve_localized(entry.title.as_ref()).unwrap_or_else(|| entry.name.clone())
    }

    pub fn entry_description(entry: &MarketListingEntry) -> Option<String> {
        resolve_localized(entry.description.as_ref())
    }

    fn url_host(url: &str) -> Option<String> {
        let rest = url.split_once("://")?.1;
        let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
        (!host.is_empty()).then(|| host.to_string())
    }

    pub fn index_label(index: &MarketIndexSummary) -> String {
        if let Some(title) = resolve_localized(index.title.as_ref()) {
            return title;
        }
        url_host(&index.url).unwrap_or_else(|| index.url.clone())
    }

    pub fn listing_title(result: &crate::protocol::MarketSearchResult) -> String {
        resolve_localized(result.title.as_ref()).unwrap_or_else(|| {
            url_host(&result.url).unwrap_or_else(|| result.url.clone())
        })
    }

    pub fn format_publish_date(timestamp: u64) -> String {
        let days = (timestamp / 86400) as i64;
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{y:04}-{m:02}-{d:02}")
    }

    pub fn entry_meta(entry: &MarketListingEntry, version: &MarketListingVersion) -> String {
        let date = format_publish_date(version.publish_timestamp);
        match &entry.author {
            Some(author) => format!("{} · {author} · {date}", version.version),
            None => format!("{} · {date}", version.version),
        }
    }

    pub struct MarketSortOption {
        pub id: String,
        pub label: String,
        pub sort: MarketSort,
        pub directed: bool,
    }

    pub fn sort_options(query: &DiscoverQuery, metadata: &MarketIndexMetadata) -> Vec<MarketSortOption> {
        let mut options = Vec::new();
        if !query.text.trim().is_empty() {
            options.push(MarketSortOption {
                id: "relevance".to_string(),
                label: "最佳匹配".to_string(),
                sort: MarketSort::Relevance,
                directed: false,
            });
        }
        options.push(MarketSortOption {
            id: "index".to_string(),
            label: "默认顺序".to_string(),
            sort: MarketSort::Index,
            directed: false,
        });
        options.push(MarketSortOption {
            id: "published".to_string(),
            label: "发布时间".to_string(),
            sort: MarketSort::Published {
                direction: MarketSortDirection::Desc,
            },
            directed: true,
        });
        let mut keys: Vec<(&String, &crate::protocol::MarketSortKeyDeclaration)> =
            metadata.sort_keys.iter().collect();
        keys.sort_by(|a, b| {
            let title_a = resolve_localized(Some(&a.1.title)).unwrap_or_default();
            let title_b = resolve_localized(Some(&b.1.title)).unwrap_or_default();
            title_a.cmp(&title_b).then_with(|| a.0.cmp(b.0))
        });
        for (id, declaration) in keys {
            options.push(MarketSortOption {
                id: format!("key:{id}"),
                label: resolve_localized(Some(&declaration.title)).unwrap_or_else(|| id.clone()),
                sort: MarketSort::Key {
                    id: id.clone(),
                    direction: declaration.order,
                },
                directed: true,
            });
        }
        options
    }

    pub fn sort_id(sort: &MarketSort) -> String {
        match sort {
            MarketSort::Index => "index".to_string(),
            MarketSort::Relevance => "relevance".to_string(),
            MarketSort::Published { .. } => "published".to_string(),
            MarketSort::Key { id, .. } => format!("key:{id}"),
        }
    }

    pub fn sort_label(query: &DiscoverQuery, metadata: &MarketIndexMetadata) -> String {
        let effective_sort = match &query.sort {
            MarketSort::Index if !query.text.trim().is_empty() => MarketSort::Relevance,
            MarketSort::Relevance if query.text.trim().is_empty() => MarketSort::Index,
            other => other.clone(),
        };
        let id = sort_id(&effective_sort);
        sort_options(query, metadata)
            .into_iter()
            .find(|option| option.id == id)
            .map(|option| option.label)
            .unwrap_or_else(|| "默认顺序".to_string())
    }

    pub fn flipped_direction(sort: &MarketSort) -> Option<MarketSort> {
        match sort {
            MarketSort::Published { direction } => Some(MarketSort::Published {
                direction: flip(*direction),
            }),
            MarketSort::Key { id, direction } => Some(MarketSort::Key {
                id: id.clone(),
                direction: flip(*direction),
            }),
            _ => None,
        }
    }

    fn flip(direction: MarketSortDirection) -> MarketSortDirection {
        match direction {
            MarketSortDirection::Asc => MarketSortDirection::Desc,
            MarketSortDirection::Desc => MarketSortDirection::Asc,
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ListingFailure {
        Fetch,
        Invalid,
        Changed,
    }

    impl ListingFailure {
        pub fn text(self) -> &'static str {
            match self {
                ListingFailure::Fetch => "无法读取此索引",
                ListingFailure::Invalid => "此索引的格式无效",
                ListingFailure::Changed => "此索引在加载过程中发生了变化",
            }
        }
    }

    pub fn listing_failure(message: &str) -> ListingFailure {
        let lowered = message.to_lowercase();
        if lowered.contains("could not fetch")
            || lowered.contains("timed out")
            || lowered.contains("transport error")
            || lowered.contains("connection is closed")
            || lowered.contains("not a configured index source")
            || lowered.contains("not available")
        {
            ListingFailure::Fetch
        } else {
            ListingFailure::Invalid
        }
    }

    pub fn listing_failure_text(failure: ListingFailure) -> &'static str {
        failure.text()
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SourceFailure {
        Invalid,
        Duplicate,
        Unavailable,
        Other,
    }

    impl SourceFailure {
        pub fn text(self) -> &'static str {
            match self {
                SourceFailure::Invalid => {
                    "请使用 http 或 https 地址，且不要包含用户名、密码或 # 片段"
                }
                SourceFailure::Duplicate => "已添加过这个索引源",
                SourceFailure::Unavailable => "当前环境无法保存索引源",
                SourceFailure::Other => "无法保存索引源",
            }
        }
    }

    pub fn source_failure(message: &str) -> SourceFailure {
        let lowered = message.to_lowercase();
        if lowered.contains("listed twice") {
            SourceFailure::Duplicate
        } else if lowered.contains("settings-unavailable") || lowered.contains("settings service") {
            SourceFailure::Unavailable
        } else if lowered.contains("invalid-index-url")
            || lowered.contains("absolute url")
            || lowered.contains("http or https")
            || lowered.contains("credentials or fragment")
            || lowered.contains("index urls")
        {
            SourceFailure::Invalid
        } else {
            SourceFailure::Other
        }
    }

    pub fn source_failure_text(failure: SourceFailure) -> &'static str {
        failure.text()
    }

    pub fn detail_failure_text(message: &str) -> &'static str {
        let lowered = message.to_lowercase();
        if lowered.contains("exceeds") && lowered.contains("bytes") {
            "详细介绍或它所在的仓库超过大小限制，未加载"
        } else if lowered.contains("could not fetch")
            || lowered.contains("could not read")
            || lowered.contains("timed out")
            || lowered.contains("transport error")
        {
            "无法连接到详细介绍的来源"
        } else if lowered.contains("denied") || lowered.contains("403") || lowered.contains("auth") {
            "无法访问详细介绍所在的仓库，请检查 Git 凭据"
        } else if lowered.contains("git") {
            "详细介绍所在的仓库需要用 Git 读取，但无法运行 Git。请安装 Git 后重试"
        } else if lowered.contains("does not exist") || lowered.contains("invalid") {
            "插件声明的详细介绍文件无效"
        } else {
            "无法读取详细介绍"
        }
    }

    #[derive(Debug, Clone, Default, PartialEq)]
    pub struct MarketPages {
        pub entries: Vec<MarketListingEntry>,
        pub page: Option<crate::protocol::MarketSearchResult>,
        pub fetched_at: Option<u64>,
        pub restarted: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PageMerge {
        Continue { done: bool },
        Restart,
        Changed,
    }

    impl MarketPages {
        pub fn merge(&mut self, page: crate::protocol::MarketSearchResult, target: usize) -> PageMerge {
            if let Some(at) = self.fetched_at {
                if at != page.fetched_at {
                    if self.restarted {
                        return PageMerge::Changed;
                    }
                    self.restarted = true;
                    self.entries.clear();
                    self.page = None;
                    self.fetched_at = None;
                    return PageMerge::Restart;
                }
            }
            self.fetched_at = Some(page.fetched_at);
            for entry in &page.entries {
                if self.entries.iter().all(|kept| kept.name != entry.name) {
                    self.entries.push(entry.clone());
                }
            }
            self.page = Some(page);
            PageMerge::Continue {
                done: self.complete(target),
            }
        }

        pub fn complete(&self, target: usize) -> bool {
            match self.page.as_ref() {
                Some(page) => page.next_offset.is_none() || self.entries.len() >= target,
                None => false,
            }
        }

        pub fn next_request(&self, target: usize) -> Option<(usize, usize)> {
            if self.complete(target) {
                return None;
            }
            let offset = self.entries.len();
            Some((offset, (target - offset).min(MARKET_MAX_PAGE)))
        }

        pub fn total(&self) -> u64 {
            self.page.as_ref().map(|page| page.total).unwrap_or(0)
        }
    }
}

const MIN_REFRESH_SPINNER: Duration = Duration::from_millis(400);
const ROW_FILTER_THRESHOLD: usize = 10;
const NPM_FILE_SOURCES: [&str; 2] = ["jsdelivr", "npmmirror"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadStatus {
    Idle,
    Loading,
    Ready,
    Error,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
enum View {
    List,
    More,
    Package { name: String, from_more: bool },
    Item(&'static OfficialItem),
}

struct PluginsState {
    status: Cell<LoadStatus>,
    refreshing: Cell<bool>,
    refresh_started: Cell<Option<Instant>>,
    packages: RefCell<Vec<PackageView>>,
    busy: RefCell<HashMap<String, FailedAction>>,
    market: Cell<bool>,
    market_state: RefCell<MarketState>,
    details: RefCell<HashMap<String, PluginBundleDetail>>,
    forms: RefCell<Vec<SettingsDescriptor>>,
    drafts: RefCell<HashMap<String, serde_json::Map<String, serde_json::Value>>>,
    providers: RefCell<Option<(Vec<ProviderOption>, Vec<ProviderOption>)>>,
    providers_failed: Cell<bool>,
    install_state: RefCell<Option<(String, String)>>,
    install_log: RefCell<String>,
    update_state: RefCell<Option<(String, String)>>,
    install_model: RefCell<InstallStateModel>,
    install_dialog: RefCell<Option<adw::Dialog>>,
    install_dialog_content: RefCell<Option<gtk4::Box>>,
    install_output_open: Rc<Cell<bool>>,
    registries: RefCell<Option<PluginRegistries>>,
    probe_fastest: RefCell<Option<String>>,
    versions_cache: RefCell<HashMap<String, PluginVersionList>>,
    versions_checking: RefCell<HashSet<String>>,
    versions_checked_at: RefCell<HashMap<String, String>>,
    versions_show_prerelease: RefCell<HashMap<String, bool>>,
    update_result: RefCell<Option<PluginUpdateResult>>,
    update_steps: RefCell<Vec<String>>,
    update_request_id: RefCell<Option<String>>,
    update_running: Cell<bool>,
    update_dialog: RefCell<Option<adw::Dialog>>,
    update_dialog_content: RefCell<Option<gtk4::Box>>,
    update_pkg_info: RefCell<Option<(String, String, String, Option<String>)>>,
    highlight_card: RefCell<Option<String>>,
    view: RefCell<View>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)]
enum MarketSourcesStatus {
    #[default]
    Idle,
    Loading,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarketListingStatus {
    Loading,
    More,
    Ready,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MarketRetry {
    #[default]
    Load,
    More,
}

#[derive(Debug, Clone)]
enum MarketDetailState {
    Loading,
    Ready(PluginMarketDetail),
    Error(&'static str),
}

#[derive(Default)]
struct MarketState {
    available: bool,
    sources_status: MarketSourcesStatus,
    indexes: Vec<MarketIndexSummary>,
    selected: Option<String>,
    query: market::DiscoverQuery,
    answered: Option<market::DiscoverQuery>,
    listing: Option<market::MarketPages>,
    listing_status: Option<MarketListingStatus>,
    failure: Option<market::ListingFailure>,
    open_entry: Option<String>,
    details: HashMap<String, MarketDetailState>,
    search_gen: u64,
    install_request: Option<(String, String)>,
    saving_sources: bool,
    source_failure: Option<market::SourceFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderOption {
    id: String,
    title: Option<String>,
}

impl ProviderOption {
    fn label(&self, missing: bool) -> String {
        let name = match self.id.as_str() {
            "deepseek-search-api" => "DeepSeek Search API".to_string(),
            "deepseek-official" => "DeepSeek（旧版）".to_string(),
            "http" => "本机直接获取".to_string(),
            "deepseek-fetch-api" => "DeepSeek Fetch API".to_string(),
            _ => self.title.clone().unwrap_or_else(|| self.id.clone()),
        };
        if missing {
            format!("{name}（未加载）")
        } else {
            name
        }
    }
}

#[derive(Clone)]
pub struct PluginsPage {
    overlay: adw::ToastOverlay,
    stack: gtk4::Stack,
    tabs_switcher: gtk4::StackSwitcher,
    status_area: gtk4::Box,
    groups_area: gtk4::Box,
    refresh_button: gtk4::Button,
    refresh_spinner: gtk4::Spinner,
    add_button: gtk4::Button,
    add_menu: gtk4::MenuButton,
    more_list: gtk4::Box,
    package_box: gtk4::Box,
    item_box: gtk4::Box,
    market_stack: gtk4::Stack,
    market_index_button: gtk4::MenuButton,
    market_index_label: gtk4::Label,
    market_search: gtk4::SearchEntry,
    market_filter_button: gtk4::MenuButton,
    market_filter_label: gtk4::Label,
    market_sort_button: gtk4::MenuButton,
    market_sort_label: gtk4::Label,
    market_refresh_button: gtk4::Button,
    market_refresh_spinner: gtk4::Spinner,
    market_categories: gtk4::FlowBox,
    market_results: gtk4::Box,
    market_entry_box: gtk4::Box,
    market_debounce: Rc<RefCell<Option<gtk4::glib::SourceId>>>,
    market_pills: Rc<RefCell<Vec<(Option<String>, gtk4::ToggleButton)>>>,
    market_pills_updating: Rc<Cell<bool>>,
    state: Rc<PluginsState>,
    controller: RefCell<Option<Weak<SessionController>>>,
}

fn clear_children(container: &gtk4::Box) {
    let mut child = container.first_child();
    while let Some(widget) = child {
        child = widget.next_sibling();
        container.remove(&widget);
    }
}

fn dim_label(text: &str) -> gtk4::Label {
    gtk4::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dim-label", "caption"])
        .build()
}

fn tag_label(text: &str, tone: &str) -> gtk4::Label {
    let classes = match tone {
        "error" => vec!["dsh-plugins-tag", "dsh-plugins-tag-error"],
        "accent" => vec!["dsh-plugins-tag", "dsh-plugins-tag-accent"],
        "success" => vec!["dsh-plugins-tag", "dsh-plugins-tag-success"],
        "warning" => vec!["dsh-plugins-tag", "dsh-plugins-tag-warning"],
        _ => vec!["dsh-plugins-tag"],
    };
    gtk4::Label::builder()
        .label(text)
        .css_classes(classes)
        .valign(gtk4::Align::Center)
        .build()
}

fn dot_widget(state: DotState) -> gtk4::Box {
    let dot = gtk4::Box::builder()
        .css_classes(["dsh-plugins-dot"])
        .valign(gtk4::Align::Center)
        .build();
    match state {
        DotState::Done => dot.add_css_class("dsh-plugins-dot-success"),
        DotState::Ongoing => dot.add_css_class("dsh-plugins-dot-ongoing"),
        DotState::Error => dot.add_css_class("dsh-plugins-dot-error"),
        DotState::Idle => {}
    }
    dot
}

fn package_icon(meta_icon: Option<&str>, size: i32) -> gtk4::Image {
    if let Some(data) = meta_icon {
        if let Some(rest) = data.split_once("base64,").map(|(_, rest)| rest) {
            use base64::Engine as _;
            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(rest) {
                if let Ok(texture) = gtk4::gdk::Texture::from_bytes(&gtk4::glib::Bytes::from_owned(
                    bytes,
                )) {
                    return gtk4::Image::builder()
                        .paintable(&texture)
                        .pixel_size(size)
                        .build();
                }
            }
        }
    }
    gtk4::Image::builder()
        .icon_name("application-x-addon-symbolic")
        .pixel_size(size)
        .build()
}

fn item_icon(id: &str, size: i32) -> gtk4::Image {
    let name = match id {
        "shell" => "utilities-terminal-symbolic",
        "agent-loop" => "object-rotate-right-symbolic",
        "subagent" => "system-users-symbolic",
        "web-search" => "web-browser-symbolic",
        _ => "application-x-addon-symbolic",
    };
    gtk4::Image::builder()
        .icon_name(name)
        .pixel_size(size)
        .build()
}

impl PluginsPage {
    pub fn new() -> Self {
        let stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let tabs_stack = gtk4::Stack::new();
        let tabs_switcher = gtk4::StackSwitcher::builder()
            .stack(&tabs_stack)
            .visible(false)
            .halign(gtk4::Align::Start)
            .css_classes(["flat"])
            .build();

        let status_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        let groups_area = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(24)
            .build();

        let installed_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        installed_box.append(&status_area);
        installed_box.append(&groups_area);
        tabs_stack.add_titled(&installed_box, Some("installed"), "已安装");

        let market_stack = gtk4::Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .build();

        let market_index_label = gtk4::Label::builder()
            .label("插件索引")
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .max_width_chars(18)
            .build();
        let index_button_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        index_button_content.append(&market_index_label);
        index_button_content.append(
            &gtk4::Image::builder()
                .icon_name("pan-down-symbolic")
                .css_classes(["dim-label"])
                .build(),
        );
        let index_popover_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        let index_popover = gtk4::Popover::builder().child(&index_popover_box).build();
        let market_index_button = gtk4::MenuButton::builder()
            .css_classes(["flat"])
            .tooltip_text("切换插件索引")
            .popover(&index_popover)
            .build();
        market_index_button.set_child(Some(&index_button_content));

        let market_search = gtk4::SearchEntry::builder()
            .placeholder_text("搜索插件")
            .hexpand(true)
            .build();

        let market_filter_label = gtk4::Label::new(Some("标签"));
        let filter_button_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        filter_button_content.append(
            &gtk4::Image::builder()
                .icon_name("funnel-symbolic")
                .css_classes(["dim-label"])
                .build(),
        );
        filter_button_content.append(&market_filter_label);
        let filter_popover_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        let filter_popover = gtk4::Popover::builder().child(&filter_popover_box).build();
        let market_filter_button = gtk4::MenuButton::builder()
            .css_classes(["flat"])
            .tooltip_text("按标签筛选")
            .popover(&filter_popover)
            .build();
        market_filter_button.set_child(Some(&filter_button_content));

        let market_sort_label = gtk4::Label::new(Some("排序：默认顺序"));
        let sort_popover_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        let sort_popover = gtk4::Popover::builder().child(&sort_popover_box).build();
        let market_sort_button = gtk4::MenuButton::builder()
            .css_classes(["flat"])
            .popover(&sort_popover)
            .build();
        market_sort_button.set_child(Some(&market_sort_label));

        let market_refresh_spinner = gtk4::Spinner::builder().visible(false).build();
        let market_refresh_button = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .css_classes(["flat"])
            .tooltip_text("刷新")
            .build();

        let market_toolbar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        market_toolbar.append(&market_index_button);
        market_toolbar.append(&market_search);
        market_toolbar.append(&market_filter_button);
        market_toolbar.append(&market_sort_button);
        market_toolbar.append(&market_refresh_button);
        market_toolbar.append(&market_refresh_spinner);

        let market_categories = gtk4::FlowBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .activate_on_single_click(false)
            .homogeneous(true)
            .max_children_per_line(12)
            .min_children_per_line(1)
            .visible(false)
            .build();

        let market_results = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();

        let market_browser = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        market_browser.append(&market_toolbar);
        market_browser.append(&market_categories);
        market_browser.append(&market_results);
        market_stack.add_named(&market_browser, Some("browser"));

        let market_entry_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        market_stack.add_named(&market_entry_box, Some("entry"));
        market_stack.set_visible_child_name("browser");

        let discover_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        discover_box.append(&market_stack);
        tabs_stack.add_titled(&discover_box, Some("discover"), "发现");

        let refresh_spinner = gtk4::Spinner::builder().visible(false).build();
        let refresh_button = gtk4::Button::builder()
            .css_classes(["flat"])
            .tooltip_text("刷新")
            .sensitive(false)
            .build();
        let refresh_content = gtk4::Box::builder().spacing(6).build();
        let refresh_label = gtk4::Label::new(Some("刷新"));
        refresh_content.append(&refresh_spinner);
        refresh_content.append(&refresh_label);
        refresh_button.set_child(Some(&refresh_content));

        let add_button = gtk4::Button::builder()
            .label("添加插件")
            .css_classes(["flat"])
            .tooltip_text("添加插件")
            .build();
        let add_menu = gtk4::MenuButton::builder()
            .icon_name("pan-down-symbolic")
            .css_classes(["flat"])
            .tooltip_text("选择添加插件方式")
            .build();
        let add_popover_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();
        let add_popover = gtk4::Popover::builder()
            .child(&add_popover_box)
            .build();
        add_menu.set_popover(Some(&add_popover));

        let toolbar = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .halign(gtk4::Align::End)
            .valign(gtk4::Align::Center)
            .spacing(6)
            .build();
        toolbar.append(&refresh_button);
        toolbar.append(&add_button);
        toolbar.append(&add_menu);

        let info_popover_label = gtk4::Label::builder()
            .label(
                "在这里配置官方插件，安装和管理其他插件。内置插件列表及运行状态可在「设置 → 内置插件」中查看",
            )
            .wrap(true)
            .max_width_chars(34)
            .xalign(0.0)
            .build();
        let info_popover = gtk4::Popover::builder()
            .child(&info_popover_label)
            .build();
        let info_button = gtk4::MenuButton::builder()
            .icon_name("dialog-information-symbolic")
            .css_classes(["flat"])
            .tooltip_text("插件说明")
            .valign(gtk4::Align::Center)
            .popover(&info_popover)
            .build();

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .margin_top(32)
            .build();
        let header_text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        header_text.append(
            &gtk4::Label::builder()
                .label("插件")
                .css_classes(["title-2"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        header_text.append(&dim_label("安装、启用和配置插件"));
        header.append(&header_text);
        header.append(&info_button);
        header.append(&toolbar);

        let list_scroll_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(0)
            .build();
        list_scroll_box.append(&header);
        tabs_switcher.set_margin_top(18);
        list_scroll_box.append(&tabs_switcher);
        tabs_stack.set_margin_top(24);
        list_scroll_box.append(&tabs_stack);

        let list_clamp = adw::Clamp::builder()
            .maximum_size(960)
            .tightening_threshold(600)
            .child(&list_scroll_box)
            .build();
        let list_page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        let list_scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&list_clamp)
            .build();
        list_page.append(&list_scroll);
        stack.add_named(&list_page, Some("list"));

        let more_list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        let more_page = Self::detail_shell("实验性插件", &more_list);
        stack.add_named(&more_page, Some("more"));

        let package_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        let package_page = Self::detail_shell("", &package_box);
        stack.add_named(&package_page, Some("package"));

        let item_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        let item_page = Self::detail_shell("", &item_box);
        stack.add_named(&item_page, Some("item"));

        let overlay = adw::ToastOverlay::new();
        overlay.set_child(Some(&stack));

        let state = Rc::new(PluginsState {
            status: Cell::new(LoadStatus::Idle),
            refreshing: Cell::new(false),
            refresh_started: Cell::new(None),
            packages: RefCell::new(Vec::new()),
            busy: RefCell::new(HashMap::new()),
            market: Cell::new(false),
            market_state: RefCell::new(MarketState::default()),
            details: RefCell::new(HashMap::new()),
            forms: RefCell::new(Vec::new()),
            drafts: RefCell::new(HashMap::new()),
            providers: RefCell::new(None),
            providers_failed: Cell::new(false),
            install_state: RefCell::new(None),
            install_log: RefCell::new(String::new()),
            update_state: RefCell::new(None),
            install_model: RefCell::new(InstallStateModel::default()),
            install_dialog: RefCell::new(None),
            install_dialog_content: RefCell::new(None),
            install_output_open: Rc::new(Cell::new(false)),
            registries: RefCell::new(None),
            probe_fastest: RefCell::new(None),
            versions_cache: RefCell::new(HashMap::new()),
            versions_checking: RefCell::new(HashSet::new()),
            versions_checked_at: RefCell::new(HashMap::new()),
            versions_show_prerelease: RefCell::new(HashMap::new()),
            update_result: RefCell::new(None),
            update_steps: RefCell::new(Vec::new()),
            update_request_id: RefCell::new(None),
            update_running: Cell::new(false),
            update_dialog: RefCell::new(None),
            update_dialog_content: RefCell::new(None),
            update_pkg_info: RefCell::new(None),
            highlight_card: RefCell::new(None),
            view: RefCell::new(View::List),
        });

        let page = Self {
            overlay,
            stack,
            tabs_switcher,
            status_area,
            groups_area,
            refresh_button,
            refresh_spinner,
            add_button,
            add_menu,
            more_list,
            package_box,
            item_box,
            market_stack,
            market_index_button,
            market_index_label,
            market_search,
            market_filter_button,
            market_filter_label,
            market_sort_button,
            market_sort_label,
            market_refresh_button,
            market_refresh_spinner,
            market_categories,
            market_results,
            market_entry_box,
            market_debounce: Rc::new(RefCell::new(None)),
            market_pills: Rc::new(RefCell::new(Vec::new())),
            market_pills_updating: Rc::new(Cell::new(false)),
            state,
            controller: RefCell::new(None),
        };
        page.wire_toolbar(&add_popover_box);
        page.wire_market();
        {
            let page = page.clone();
            tabs_stack.clone().connect_visible_child_name_notify(move |stack| {
                let name = stack.visible_child_name().map(|name| name.to_string());
                page.on_market_tab_visibility(name.as_deref());
            });
        }
        page
    }

    fn detail_shell(title: &str, content: &gtk4::Box) -> gtk4::Box {
        let page = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .build();
        let inner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .build();
        if !title.is_empty() {
            inner.append(
                &gtk4::Label::builder()
                    .label(title)
                    .css_classes(["title-2"])
                    .halign(gtk4::Align::Start)
                    .build(),
            );
        }
        inner.append(content);
        let clamp = adw::Clamp::builder()
            .maximum_size(960)
            .tightening_threshold(600)
            .child(&inner)
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&clamp)
            .build();
        page.append(&scroll);
        page
    }

    pub fn widget(&self) -> &adw::ToastOverlay {
        &self.overlay
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        *self.controller.borrow_mut() = Some(Rc::downgrade(&controller));
        self.load_inventory();
        self.probe_market();
        self.probe_settings_namespaces();
        self.load_registries();
        self.probe_registry_speed();
    }

    fn with_controller(&self, f: impl FnOnce(&SessionController)) {
        if let Some(controller) = self
            .controller
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
        {
            f(&controller);
        }
    }

    fn controller_weak(&self) -> Option<Weak<SessionController>> {
        self.controller.borrow().clone()
    }

    fn toast(&self, text: &str) {
        let toast = adw::Toast::new(text);
        toast.set_timeout(5);
        self.overlay.add_toast(toast);
    }

    fn wire_toolbar(&self, add_popover_box: &gtk4::Box) {
        let page = self.clone();
        self.refresh_button.connect_clicked(move |_| {
            page.user_refresh();
        });

        let menu_item = gtk4::Button::builder().css_classes(["flat"]).build();
        let menu_content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(1)
            .build();
        menu_content.append(
            &gtk4::Label::builder()
                .label("安装第三方插件")
                .css_classes(["heading"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        menu_content.append(&dim_label("支持 npm 包、Git 仓库和本地目录"));
        menu_item.set_child(Some(&menu_content));
        add_popover_box.append(&menu_item);

        let page = self.clone();
        self.add_button.connect_clicked(move |_| {
            page.open_install_dialog();
        });

        let page = self.clone();
        let popover = add_popover_box
            .parent()
            .and_downcast::<gtk4::Popover>();
        menu_item.connect_clicked(move |_| {
            if let Some(popover) = popover.as_ref() {
                popover.popdown();
            }
            page.open_install_dialog();
        });
        self.update_add_button();
    }

    pub fn update_add_button(&self) {
        let pending = is_install_pending(self.state.install_model.borrow().phase);
        if pending {
            self.add_button.set_label("查看安装任务");
            self.add_button.add_css_class("suggested-action");
            self.add_menu.set_visible(false);
        } else {
            self.add_button.set_label("添加插件");
            self.add_button.remove_css_class("suggested-action");
            self.add_menu.set_visible(true);
        }
    }

    pub fn user_refresh(&self) {
        if self.state.refreshing.get() {
            return;
        }
        self.state.refreshing.set(true);
        self.state.refresh_started.set(Some(Instant::now()));
        self.update_refresh_button();
        self.load_inventory();
    }

    fn update_refresh_button(&self) {
        let refreshing = self.state.refreshing.get();
        self.refresh_spinner.set_visible(refreshing);
        if refreshing {
            self.refresh_spinner.start();
        } else {
            self.refresh_spinner.stop();
        }
    }

    pub fn load_inventory(&self) {
        if self.state.status.get() == LoadStatus::Idle {
            self.state.status.set(LoadStatus::Loading);
            self.render_list();
        }
        self.with_controller(|ctrl| ctrl.load_plugin_inventory());
    }

    pub fn handle_inventory(&self, result: Result<PluginInventoryResult, String>) {
        let started = self.state.refresh_started.take();
        match result {
            Ok(inventory) => {
                let packages = parse_inventory(&inventory.bundles, &inventory.plugins);
                self.state.busy.borrow_mut().retain(|key, _| {
                    match key.strip_prefix("row:") {
                        Some(entry_id) => packages.iter().any(|pkg| {
                            pkg.rows
                                .iter()
                                .any(|row| row.entry_id.as_deref() == Some(entry_id))
                        }),
                        None => packages.iter().any(|pkg| &pkg.name == key),
                    }
                });
                *self.state.packages.borrow_mut() = packages;
                self.state.status.set(LoadStatus::Ready);
            }
            Err(err) => {
                let message = err.to_lowercase();
                if self.state.packages.borrow().is_empty() {
                    if message.contains("profile") || message.contains("unavailable") {
                        self.state.status.set(LoadStatus::Unavailable);
                    } else {
                        self.state.status.set(LoadStatus::Error);
                    }
                } else {
                    self.state.status.set(LoadStatus::Ready);
                    self.toast("刷新失败，请重试");
                }
            }
        }
        self.refresh_current_view();
        if let Some(started) = started {
            let page = self.clone();
            let elapsed = started.elapsed();
            if elapsed < MIN_REFRESH_SPINNER {
                gtk4::glib::timeout_add_local_once(
                    MIN_REFRESH_SPINNER - elapsed,
                    move || page.finish_refresh(),
                );
            } else {
                page.finish_refresh();
            }
        }
    }

    fn finish_refresh(&self) {
        self.state.refreshing.set(false);
        self.state.refresh_started.set(None);
        self.update_refresh_button();
    }

    pub fn on_connection_ready(&self) {
        self.load_inventory();
        self.probe_market();
        self.probe_settings_namespaces();
        self.load_registries();
        self.probe_registry_speed();
    }

    pub fn load_registries(&self) {
        self.with_controller(|ctrl| ctrl.plugin_registries());
    }

    pub fn probe_registry_speed(&self) {
        self.with_controller(|ctrl| ctrl.plugin_registry_probe());
    }

    pub fn probe_market(&self) {
        {
            let mut state = self.state.market_state.borrow_mut();
            if state.sources_status == MarketSourcesStatus::Idle {
                state.sources_status = MarketSourcesStatus::Loading;
            }
        }
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            return;
        };
        let rx = rpc_receiver::<serde_json::Value, PluginMarketListIndexesResult>(
            conn_arc,
            METHOD_PLUGIN_MARKET_LIST_INDEXES,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            match rx.recv().await {
                Some(Ok(result)) => {
                    let selected = {
                        let mut state = page.state.market_state.borrow_mut();
                        state.available = true;
                        state.sources_status = MarketSourcesStatus::Ready;
                        state.indexes = result.result;
                        let previous = state.selected.clone();
                        let selected = previous
                            .filter(|url| state.indexes.iter().any(|index| &index.url == url))
                            .or_else(|| state.indexes.first().map(|index| index.url.clone()));
                        state.selected = selected.clone();
                        selected
                    };
                    page.state.market.set(true);
                    page.tabs_switcher.set_visible(true);
                    let reload_count = {
                        let state = page.state.market_state.borrow();
                        match state.listing.as_ref() {
                            Some(listing) if listing.page.as_ref().map(|p| p.url.as_str())
                                == selected.as_deref() =>
                            {
                                Some(listing.entries.len().max(market::MARKET_PAGE_SIZE))
                            }
                            _ => None,
                        }
                    };
                    match reload_count {
                        Some(count) => page.market_load(count, false),
                        None => {
                            page.state.market_state.borrow_mut().open_entry = None;
                            page.market_load(market::MARKET_PAGE_SIZE, false);
                        }
                    }
                    page.render_market();
                }
                _ => {
                    page.state.market.set(false);
                    let mut state = page.state.market_state.borrow_mut();
                    state.available = false;
                    state.sources_status = MarketSourcesStatus::Error;
                    drop(state);
                    page.tabs_switcher.set_visible(false);
                }
            }
        });
    }

    fn probe_settings_namespaces(&self) {
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            return;
        };
        let rx = rpc_receiver::<serde_json::Value, SettingsDescribeResult>(
            conn_arc,
            METHOD_SETTINGS_DESCRIBE,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            if let Some(Ok(result)) = rx.recv().await {
                *page.state.forms.borrow_mut() = result.namespaces;
                if page.state.status.get() == LoadStatus::Ready {
                    page.render_list();
                }
            }
        });
    }

    pub fn reload_after_change(&self) {
        self.state.details.borrow_mut().clear();
        self.load_inventory();
        self.refresh_current_view();
        self.market_reload_install_status();
    }

    fn market_reload_install_status(&self) {
        let reload_count = {
            let state = self.state.market_state.borrow();
            match state.listing.as_ref() {
                Some(listing)
                    if state.listing_status == Some(MarketListingStatus::Ready)
                        && state.answered.as_ref() == Some(&state.query) =>
                {
                    Some(listing.entries.len().max(market::MARKET_PAGE_SIZE))
                }
                _ => None,
            }
        };
        if let Some(count) = reload_count {
            self.market_load(count, false);
        }
    }

    fn clear_flowbox(flow: &gtk4::FlowBox) {
        let mut child = flow.first_child();
        while let Some(widget) = child {
            child = widget.next_sibling();
            if let Ok(wrapper) = widget.downcast::<gtk4::FlowBoxChild>() {
                flow.remove(&wrapper);
            }
        }
    }

    fn wire_market(&self) {
        let page = self.clone();
        self.market_index_button
            .connect_active_notify(move |button| {
                if button.is_active() {
                    page.fill_index_menu();
                }
            });
        let page = self.clone();
        self.market_filter_button
            .connect_active_notify(move |button| {
                if button.is_active() {
                    page.fill_filter_menu();
                }
            });
        let page = self.clone();
        self.market_sort_button
            .connect_active_notify(move |button| {
                if button.is_active() {
                    page.fill_sort_menu();
                }
            });

        let page = self.clone();
        self.market_search.connect_search_changed(move |entry| {
            let text = entry.text().to_string();
            if let Some(id) = page.market_debounce.borrow_mut().take() {
                id.remove();
            }
            let page_cb = page.clone();
            let id = gtk4::glib::timeout_add_local_once(
                Duration::from_millis(market::MARKET_TEXT_DELAY_MS),
                move || {
                    *page_cb.market_debounce.borrow_mut() = None;
                    let query = {
                        let state = page_cb.state.market_state.borrow();
                        market::with_text(&state.query, &text)
                    };
                    page_cb.market_search(query);
                },
            );
            *page.market_debounce.borrow_mut() = Some(id);
        });

        let page = self.clone();
        self.market_refresh_button
            .connect_clicked(move |_| page.market_refresh());
    }

    fn on_market_tab_visibility(&self, name: Option<&str>) {
        let entering = name == Some("discover");
        if !entering {
            let mut state = self.state.market_state.borrow_mut();
            if state.open_entry.take().is_some() {
                drop(state);
                self.market_stack.set_visible_child_name("browser");
            }
            return;
        }
        let available = self.state.market_state.borrow().available;
        if available {
            self.market_load(market::MARKET_PAGE_SIZE, false);
        }
    }

    fn fill_index_menu(&self) {
        let Some(popover) = self
            .market_index_button
            .popover()
            .and_downcast::<gtk4::Popover>()
        else {
            return;
        };
        let Some(box_) = popover.child().and_downcast::<gtk4::Box>() else {
            return;
        };
        clear_children(&box_);
        let indexes = self.state.market_state.borrow().indexes.clone();
        for index in &indexes {
            let item = gtk4::Button::builder()
                .label(&market::index_label(index))
                .css_classes(["flat"])
                .build();
            let page = self.clone();
            let url = index.url.clone();
            let popdown = popover.clone();
            item.connect_clicked(move |_| {
                popdown.popdown();
                page.market_select(&url);
            });
            box_.append(&item);
        }
        box_.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
        let manage = gtk4::Button::builder()
            .label("管理索引源…")
            .css_classes(["flat"])
            .build();
        let page = self.clone();
        let popdown = popover.clone();
        manage.connect_clicked(move |_| {
            popdown.popdown();
            page.open_market_sources();
        });
        box_.append(&manage);
    }

    fn market_declared_labels(&self) -> Vec<(String, String)> {
        let state = self.state.market_state.borrow();
        let Some(pages) = state.listing.as_ref() else {
            return Vec::new();
        };
        let Some(page) = pages.page.as_ref() else {
            return Vec::new();
        };
        let mut labels: Vec<(String, String)> = page
            .metadata
            .labels
            .iter()
            .map(|(id, declaration)| {
                (
                    id.clone(),
                    market::resolve_localized(Some(&declaration.title)).unwrap_or_else(|| id.clone()),
                )
            })
            .collect();
        labels.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        labels
    }

    fn fill_filter_menu(&self) {
        let Some(popover) = self
            .market_filter_button
            .popover()
            .and_downcast::<gtk4::Popover>()
        else {
            return;
        };
        let Some(box_) = popover.child().and_downcast::<gtk4::Box>() else {
            return;
        };
        clear_children(&box_);
        let selected = self.state.market_state.borrow().query.labels.clone();
        for (id, title) in self.market_declared_labels() {
            let check = gtk4::CheckButton::builder()
                .label(&title)
                .active(selected.contains(&id))
                .build();
            let page = self.clone();
            let id = id.clone();
            check.connect_toggled(move |check| {
                page.market_toggle_label(&id, check.is_active());
            });
            box_.append(&check);
        }
    }

    fn fill_sort_menu(&self) {
        let Some(popover) = self
            .market_sort_button
            .popover()
            .and_downcast::<gtk4::Popover>()
        else {
            return;
        };
        let Some(box_) = popover.child().and_downcast::<gtk4::Box>() else {
            return;
        };
        clear_children(&box_);
        let (query, metadata) = {
            let state = self.state.market_state.borrow();
            (
                state.query.clone(),
                state
                    .listing
                    .as_ref()
                    .and_then(|pages| pages.page.as_ref())
                    .map(|page| page.metadata.clone())
                    .unwrap_or_default(),
            )
        };
        let current = market::sort_id(&query.sort);
        for option in market::sort_options(&query, &metadata) {
            let selected = option.id == current;
            let item = gtk4::Button::builder()
                .css_classes(["flat"])
                .halign(gtk4::Align::Start)
                .build();
            let content = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .build();
            content.append(
                &gtk4::Image::builder()
                    .icon_name("object-select-symbolic")
                    .visible(selected)
                    .build(),
            );
            content.append(&gtk4::Label::new(Some(&option.label)));
            item.set_child(Some(&content));
            let page = self.clone();
            let sort = option.sort;
            let popdown = popover.clone();
            item.connect_clicked(move |_| {
                popdown.popdown();
                let query = {
                    let mut query = page.state.market_state.borrow().query.clone();
                    query.sort = sort.clone();
                    query
                };
                page.market_search(query);
            });
            box_.append(&item);
        }
        if let Some(flipped) = market::flipped_direction(&query.sort) {
            box_.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
            let direction = match flipped {
                MarketSort::Published { direction } | MarketSort::Key { direction, .. } => direction,
                _ => MarketSortDirection::Desc,
            };
            let label = match direction {
                MarketSortDirection::Asc => "改为升序",
                MarketSortDirection::Desc => "改为降序",
            };
            let footer = gtk4::Button::builder()
                .label(label)
                .css_classes(["flat"])
                .halign(gtk4::Align::Start)
                .build();
            let page = self.clone();
            let popdown = popover.clone();
            footer.connect_clicked(move |_| {
                popdown.popdown();
                let query = {
                    let mut query = page.state.market_state.borrow().query.clone();
                    if let Some(sort) = market::flipped_direction(&query.sort) {
                        query.sort = sort;
                    }
                    query
                };
                page.market_search(query);
            });
            box_.append(&footer);
        }
    }

    fn render_market(&self) {
        let (open_entry, listing) = {
            let state = self.state.market_state.borrow();
            (state.open_entry.clone(), state.listing.clone())
        };
        let entry = open_entry
            .as_deref()
            .zip(listing.as_ref())
            .and_then(|(name, pages)| {
                pages
                    .entries
                    .iter()
                    .find(|entry| entry.name == name)
                    .cloned()
            });
        match (entry, listing.as_ref().and_then(|pages| pages.page.clone())) {
            (Some(entry), Some(facts)) => {
                self.render_market_entry(&entry, &facts);
                self.market_stack.set_visible_child_name("entry");
            }
            _ => {
                if open_entry.is_some() {
                    self.state.market_state.borrow_mut().open_entry = None;
                }
                self.render_market_browser();
                self.market_stack.set_visible_child_name("browser");
            }
        }
    }

    fn render_market_browser(&self) {
        let (sources_status, indexes, selected, query, listing, listing_status, failure) = {
            let state = self.state.market_state.borrow();
            (
                state.sources_status,
                state.indexes.clone(),
                state.selected.clone(),
                state.query.clone(),
                state.listing.clone(),
                state.listing_status,
                state.failure,
            )
        };
        let metadata = listing
            .as_ref()
            .and_then(|pages| pages.page.as_ref())
            .map(|page| page.metadata.clone())
            .unwrap_or_default();

        let selected_index = indexes
            .iter()
            .find(|index| Some(&index.url) == selected.as_ref());
        self.market_index_label.set_label(
            &selected_index
                .map(market::index_label)
                .unwrap_or_else(|| "插件索引".to_string()),
        );
        let filter_count = query.labels.len();
        let label = if filter_count == 0 {
            "标签".to_string()
        } else {
            format!("标签 · {filter_count}")
        };
        self.market_filter_label.set_label(&label);
        self.market_filter_button
            .set_visible(!metadata.labels.is_empty());
        self.market_sort_label
            .set_label(&format!("排序：{}", market::sort_label(&query, &metadata)));
        let busy = matches!(
            listing_status,
            Some(MarketListingStatus::Loading) | Some(MarketListingStatus::More)
        );
        self.market_refresh_spinner.set_visible(busy);
        if busy {
            self.market_refresh_spinner.start();
        } else {
            self.market_refresh_spinner.stop();
        }
        self.market_refresh_button
            .set_sensitive(listing.is_some() && !busy);

        self.render_market_categories(&query, &metadata);

        clear_children(&self.market_results);
        match sources_status {
            MarketSourcesStatus::Idle | MarketSourcesStatus::Loading => {
                self.market_results.append(&self.skeleton());
            }
            MarketSourcesStatus::Error => {
                let err_box = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Vertical)
                    .spacing(8)
                    .build();
                err_box.append(&gtk4::Label::new(Some("无法读取索引源")));
                self.market_results.append(&err_box);
            }
            MarketSourcesStatus::Ready => {
                if indexes.is_empty() {
                    let empty = gtk4::Box::builder()
                        .orientation(gtk4::Orientation::Vertical)
                        .spacing(8)
                        .valign(gtk4::Align::Center)
                        .halign(gtk4::Align::Center)
                        .margin_top(48)
                        .margin_bottom(48)
                        .build();
                    empty.append(
                        &gtk4::Image::builder()
                            .icon_name("application-x-addon-symbolic")
                            .pixel_size(48)
                            .build(),
                    );
                    empty.append(
                        &gtk4::Label::builder()
                            .label("还没有插件索引")
                            .css_classes(["title-3"])
                            .build(),
                    );
                    empty.append(&dim_label(
                        "添加一个插件索引，即可在这里浏览和安装其中的插件",
                    ));
                    let add = gtk4::Button::builder()
                        .label("添加索引源")
                        .css_classes(["suggested-action"])
                        .halign(gtk4::Align::Center)
                        .build();
                    let page = self.clone();
                    add.connect_clicked(move |_| page.open_market_sources());
                    empty.append(&add);
                    self.market_results.append(&empty);
                    return;
                }
                if let Some(failure) = failure {
                    let retry = if failure == market::ListingFailure::Changed
                        || listing.is_none()
                        || listing_status != Some(MarketListingStatus::Error)
                    {
                        MarketRetry::Load
                    } else {
                        MarketRetry::More
                    };
                    self.market_results
                        .append(&self.market_failure_row(failure.text(), retry));
                }
                match listing.as_ref() {
                    None if listing_status == Some(MarketListingStatus::Loading) => {
                        self.market_results.append(&self.skeleton());
                    }
                    None => {}
                    Some(pages) => {
                        if pages.total() == 0 {
                            let unfiltered = query.text.trim().is_empty()
                                && query.category.is_none()
                                && query.labels.is_empty();
                            self.market_results.append(&dim_label(if unfiltered {
                                "此索引没有插件"
                            } else {
                                "没有符合条件的插件"
                            }));
                        } else {
                            let list = gtk4::ListBox::builder()
                                .selection_mode(gtk4::SelectionMode::None)
                                .css_classes(["boxed-list"])
                                .build();
                            let declared_labels = self.market_declared_labels();
                            for entry in &pages.entries {
                                list.append(&self.market_entry_card(entry, &declared_labels));
                            }
                            self.market_results.append(&list);
                            {
                                let more = gtk4::Button::builder()
                                    .label("显示更多")
                                    .css_classes(["flat"])
                                    .halign(gtk4::Align::Center)
                                    .sensitive(listing_status == Some(MarketListingStatus::Ready))
                                    .build();
                                let page_handle = self.clone();
                                more.connect_clicked(move |_| page_handle.market_more());
                                self.market_results.append(&more);
                            }
                        }
                    }
                }
            }
        }
    }

    fn market_failure_row(&self, text: &str, retry: MarketRetry) -> gtk4::Box {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        row.append(&dot_widget(DotState::Error));
        row.append(
            &gtk4::Label::builder()
                .label(text)
                .wrap(true)
                .xalign(0.0)
                .hexpand(true)
                .halign(gtk4::Align::Start)
                .css_classes(["error", "caption"])
                .build(),
        );
        let button = gtk4::Button::builder()
            .label("重试")
            .css_classes(["flat"])
            .build();
        let page = self.clone();
        button.connect_clicked(move |_| match retry {
            MarketRetry::Load => page.market_load(market::MARKET_PAGE_SIZE, false),
            MarketRetry::More => page.market_more(),
        });
        row.append(&button);
        row
    }

    fn render_market_categories(&self, query: &market::DiscoverQuery, metadata: &MarketIndexMetadata) {
        Self::clear_flowbox(&self.market_categories);
        self.market_pills.borrow_mut().clear();
        let mut categories: Vec<(String, String)> = metadata
            .categories
            .iter()
            .map(|(id, declaration)| {
                (
                    id.clone(),
                    market::resolve_localized(Some(&declaration.title)).unwrap_or_else(|| id.clone()),
                )
            })
            .collect();
        categories.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
        if categories.is_empty() {
            self.market_categories.set_visible(false);
            return;
        }
        self.market_categories.set_visible(true);
        let all = gtk4::ToggleButton::builder()
            .label("全部")
            .css_classes(["flat", "dsh-market-pill"])
            .active(query.category.is_none())
            .build();
        self.market_pills.borrow_mut().push((None, all.clone()));
        self.market_categories.append(&all);
        for (id, title) in categories {
            let pill = gtk4::ToggleButton::builder()
                .label(&title)
                .css_classes(["flat", "dsh-market-pill"])
                .build();
            pill.set_group(Some(&all));
            if query.category.as_deref() == Some(id.as_str()) {
                pill.set_active(true);
            }
            self.market_pills.borrow_mut().push((Some(id.clone()), pill.clone()));
            self.market_categories.append(&pill);
        }
        let pills = self.market_pills.borrow().clone();
        for (id, pill) in pills {
            let page = self.clone();
            let id = id.clone();
            pill.connect_clicked(move |pill| {
                if page.market_pills_updating.get() {
                    return;
                }
                let target = if pill.is_active() { id.clone() } else { None };
                page.market_pills_updating.set(true);
                for (pid, button) in page.market_pills.borrow().iter() {
                    let active = match &target {
                        None => pid.is_none(),
                        Some(target) => pid.as_deref() == Some(target.as_str()),
                    };
                    button.set_active(active);
                }
                page.market_pills_updating.set(false);
                page.market_set_category(target);
            });
        }
    }

    fn market_entry_card(
        &self,
        entry: &MarketListingEntry,
        declared_labels: &[(String, String)],
    ) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::builder().activatable(true).build();
        let line = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        line.append(&package_icon(entry.icon.as_deref(), 36));
        let text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();
        let title_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        title_row.append(
            &gtk4::Label::builder()
                .label(&market::entry_title(entry))
                .css_classes(["heading"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        for (id, title) in declared_labels {
            if entry.metadata.labels.contains(id) {
                title_row.append(&tag_label(title, "neutral"));
            }
        }
        text.append(&title_row);
        if let Some(description) = market::entry_description(entry) {
            text.append(&dim_label(&description));
        }
        let meta = market::presented_version(entry)
            .map(|version| market::entry_meta(entry, version))
            .unwrap_or_default();
        text.append(&dim_label(&meta));
        line.append(&text);
        line.append(&self.market_entry_action(entry));
        row.set_child(Some(&line));
        row.set_action_name(Some("win.open-plugin-target"));
        row.set_action_target_value(Some(
            &format!("market:{}", entry.name).to_variant(),
        ));
        row
    }

    fn market_entry_action(&self, entry: &MarketListingEntry) -> gtk4::Widget {
        match market::entry_action(entry) {
            market::MarketEntryAction::Status(status) => {
                let button = gtk4::Button::builder()
                    .label(status.text())
                    .css_classes(["flat"])
                    .tooltip_text(format!("在已安装中查看 {}", market::entry_title(entry)))
                    .valign(gtk4::Align::Center)
                    .build();
                let page = self.clone();
                let name = entry.name.clone();
                button.connect_clicked(move |_| page.open_package(&name, false));
                button.upcast()
            }
            market::MarketEntryAction::Install => {
                let blocked = market::entry_install_blocked_reason(entry);
                let button = gtk4::Button::builder()
                    .label("安装")
                    .css_classes(["outline"])
                    .valign(gtk4::Align::Center)
                    .sensitive(blocked.is_none())
                    .build();
                if let Some(reason) = blocked {
                    button.set_tooltip_text(Some(&reason));
                }
                let page = self.clone();
                let entry = entry.clone();
                button.connect_clicked(move |_| {
                    if let Some(version) = market::presented_version(&entry) {
                        page.confirm_market_install(&entry, version);
                    }
                });
                button.upcast()
            }
        }
    }

    fn market_set_category(&self, category: Option<String>) {
        let query = {
            let mut state = self.state.market_state.borrow_mut();
            if state.query.category == category {
                return;
            }
            state.query.category = category;
            state.query.clone()
        };
        self.market_search(query);
    }

    fn market_toggle_label(&self, id: &str, active: bool) {
        let query = {
            let mut state = self.state.market_state.borrow_mut();
            let contains = state.query.labels.iter().any(|label| label == id);
            if active == contains {
                return;
            }
            if active {
                state.query.labels.push(id.to_string());
            } else {
                state.query.labels.retain(|label| label != id);
            }
            state.query.clone()
        };
        self.market_search(query);
    }

    fn market_select(&self, url: &str) {
        {
            let mut state = self.state.market_state.borrow_mut();
            if state.selected.as_deref() == Some(url) {
                return;
            }
            state.selected = Some(url.to_string());
            state.query = market::initial_query();
            state.answered = None;
            state.listing = None;
            state.listing_status = Some(MarketListingStatus::Loading);
            state.failure = None;
            state.open_entry = None;
        }
        self.market_search.set_text("");
        self.render_market();
        self.market_load(market::MARKET_PAGE_SIZE, false);
    }

    fn market_search(&self, query: market::DiscoverQuery) {
        {
            let mut state = self.state.market_state.borrow_mut();
            if state.selected.is_none() {
                state.query = query;
                return;
            }
            let busy = matches!(
                state.listing_status,
                Some(MarketListingStatus::Loading) | Some(MarketListingStatus::More)
            );
            if query == state.query && busy {
                return;
            }
            state.query = query;
            state.answered = None;
            state.listing = None;
            state.listing_status = Some(MarketListingStatus::Loading);
            state.failure = None;
            state.open_entry = None;
        }
        self.render_market();
        self.market_load(market::MARKET_PAGE_SIZE, false);
    }

    fn market_refresh(&self) {
        let has_listing = self.state.market_state.borrow().listing.is_some();
        if has_listing {
            self.market_load(market::MARKET_PAGE_SIZE, true);
        }
    }

    fn market_more(&self) {
        let (url, query, pages) = {
            let state = self.state.market_state.borrow();
            (
                state.selected.clone(),
                state.query.clone(),
                state.listing.clone(),
            )
        };
        let (Some(url), Some(pages)) = (url, pages) else {
            return;
        };
        if pages.page.as_ref().and_then(|page| page.next_offset).is_none() {
            return;
        }
        {
            let mut state = self.state.market_state.borrow_mut();
            if state.answered.as_ref() != Some(&state.query)
                || !matches!(
                    state.listing_status,
                    Some(MarketListingStatus::Ready) | Some(MarketListingStatus::Error)
                )
            {
                return;
            }
            state.listing_status = Some(MarketListingStatus::More);
            state.failure = None;
        }
        self.render_market();
        let conn = match self.controller_weak() {
            Some(weak) => weak.upgrade().map(|ctrl| ctrl.connection_handle()),
            None => None,
        };
        let Some(conn) = conn else {
            self.market_fail(&url, "Not connected", market::ListingFailure::Fetch);
            return;
        };
        let generation = self.next_market_gen();
        let offset = pages.entries.len();
        let params = PluginMarketSearchParams {
            url: url.clone(),
            query: market::market_query_params(
                &query,
                offset,
                market::MARKET_PAGE_SIZE,
                pages.fetched_at,
                false,
            ),
        };
        let rx = rpc_receiver::<PluginMarketSearchParams, PluginMarketSearchResult>(
            conn,
            METHOD_PLUGIN_MARKET_SEARCH,
            params,
            Duration::from_secs(30),
        );
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            let answer = rx.recv().await;
            if page.market_gen() != generation {
                return;
            }
            match answer {
                Some(Ok(result)) => {
                    if Some(result.result.fetched_at) != pages.fetched_at {
                        let count = pages.entries.len() + market::MARKET_PAGE_SIZE;
                        page.market_load(count, false);
                        return;
                    }
                    let mut merged = pages;
                    merged.merge(result.result, usize::MAX);
                    {
                        let mut state = page.state.market_state.borrow_mut();
                        state.listing = Some(merged);
                        state.listing_status = Some(MarketListingStatus::Ready);
                        state.failure = None;
                    }
                    page.render_market();
                }
                Some(Err(err)) => page.market_fail(&url, &err, market::listing_failure(&err)),
                None => page.market_fail(&url, "closed", market::ListingFailure::Fetch),
            }
        });
    }

    fn next_market_gen(&self) -> u64 {
        let mut state = self.state.market_state.borrow_mut();
        state.search_gen += 1;
        state.search_gen
    }

    fn market_gen(&self) -> u64 {
        self.state.market_state.borrow().search_gen
    }

    fn market_load(&self, count: usize, refresh: bool) {
        let (url, query) = {
            let state = self.state.market_state.borrow();
            (state.selected.clone(), state.query.clone())
        };
        let Some(url) = url else {
            return;
        };
        {
            let mut state = self.state.market_state.borrow_mut();
            state.listing_status = Some(MarketListingStatus::Loading);
            state.failure = None;
        }
        self.render_market();
        let conn = match self.controller_weak() {
            Some(weak) => weak.upgrade().map(|ctrl| ctrl.connection_handle()),
            None => None,
        };
        let Some(conn) = conn else {
            self.market_fail(&url, "Not connected", market::ListingFailure::Fetch);
            return;
        };
        let generation = self.next_market_gen();
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut pages = market::MarketPages::default();
            loop {
                let Some((offset, limit)) = pages.next_request(count) else {
                    break;
                };
                let fetched_at = pages.fetched_at;
                let use_refresh = refresh && pages.page.is_none() && !pages.restarted;
                let params = PluginMarketSearchParams {
                    url: url.clone(),
                    query: market::market_query_params(&query, offset, limit, fetched_at, use_refresh),
                };
                let rx = rpc_receiver::<PluginMarketSearchParams, PluginMarketSearchResult>(
                    conn.clone(),
                    METHOD_PLUGIN_MARKET_SEARCH,
                    params,
                    Duration::from_secs(30),
                );
                let mut rx = rx;
                let answer = rx.recv().await;
                if page.market_gen() != generation {
                    return;
                }
                match answer {
                    Some(Ok(result)) => match pages.merge(result.result, count) {
                        market::PageMerge::Continue { done: true } => break,
                        market::PageMerge::Continue { done: false } => {}
                        market::PageMerge::Restart => {}
                        market::PageMerge::Changed => {
                            page.market_fail(&url, "changed", market::ListingFailure::Changed);
                            return;
                        }
                    },
                    Some(Err(err)) => {
                        page.market_fail(&url, &err, market::listing_failure(&err));
                        return;
                    }
                    None => {
                        page.market_fail(&url, "closed", market::ListingFailure::Fetch);
                        return;
                    }
                }
            }
            let Some(facts) = pages.page.clone() else {
                return;
            };
            let declared = market::declared_query(&query, &facts.metadata);
            if declared != query {
                page.state.market_state.borrow_mut().query = declared;
                page.market_load(count, false);
                return;
            }
            {
                let mut state = page.state.market_state.borrow_mut();
                state.listing = Some(pages);
                state.answered = Some(query);
                state.listing_status = Some(MarketListingStatus::Ready);
                state.failure = None;
                for index in &mut state.indexes {
                    if index.url == facts.url {
                        index.title = facts.title.clone();
                        index.description = facts.description.clone();
                        index.error = None;
                    }
                }
            }
            page.render_market();
        });
    }

    fn market_fail(&self, url: &str, message: &str, failure: market::ListingFailure) {
        {
            let mut state = self.state.market_state.borrow_mut();
            state.listing_status = Some(MarketListingStatus::Error);
            state.failure = Some(failure);
            for index in &mut state.indexes {
                if index.url == url {
                    index.error = Some(message.to_string());
                }
            }
        }
        self.render_market();
    }

    fn market_open_entry(&self, name: &str) {
        self.state.market_state.borrow_mut().open_entry = Some(name.to_string());
        self.render_market();
        self.market_load_detail(name, false);
    }

    fn market_close_entry(&self) {
        self.state.market_state.borrow_mut().open_entry = None;
        self.render_market();
    }

    fn render_market_entry(&self, entry: &MarketListingEntry, facts: &MarketSearchResult) {
        clear_children(&self.market_entry_box);
        let crumb = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        let back = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(["flat"])
            .tooltip_text("发现")
            .build();
        let page = self.clone();
        back.connect_clicked(move |_| page.market_close_entry());
        crumb.append(&back);
        crumb.append(&gtk4::Label::new(Some("发现")));
        self.market_entry_box.append(&crumb);

        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        head.append(&package_icon(entry.icon.as_deref(), 48));
        let title_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();
        let title = market::entry_title(entry);
        let title_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        title_row.append(
            &gtk4::Label::builder()
                .label(&title)
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        if let Some(version) = market::presented_version(entry) {
            title_row.append(&tag_label(&version.version, "neutral"));
        }
        title_box.append(&title_row);
        if title != entry.name {
            title_box.append(
                &gtk4::Label::builder()
                    .label(&entry.name)
                    .css_classes(["monospace", "caption", "dim-label"])
                    .halign(gtk4::Align::Start)
                    .build(),
            );
        }
        head.append(&title_box);
        head.append(&self.market_entry_action(entry));
        self.market_entry_box.append(&head);

        if let Some(description) = market::entry_description(entry) {
            self.market_entry_box.append(&dim_label(&description));
        }

        if let Some(version) = market::presented_version(entry) {
            if !version.compatibility.compatible {
                self.market_entry_box
                    .append(&self.market_notice(&self.market_incompatibility_text(version, false)));
            }
        }

        self.market_entry_box.append(&self.market_detail_section(entry));

        self.market_entry_box.append(&self.market_versions_section(entry));

        self.market_entry_box.append(&self.market_facts_section(entry, facts));
    }

    fn market_incompatibility_text(
        &self,
        version: &MarketListingVersion,
        exempted_only: bool,
    ) -> String {
        if exempted_only || version.compatibility.exempted {
            return "此版本与当前 DSH 不兼容，但你已为它设置了兼容性豁免".to_string();
        }
        let runtime = version
            .compatibility
            .runtime_version
            .as_deref()
            .unwrap_or("未知");
        let range = version
            .compatibility
            .requirements
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join(" ");
        format!("此版本与当前的 DSH {runtime} 不兼容，它要求 DSH {range}")
    }

    fn market_notice(&self, text: &str) -> gtk4::Box {
        let notice = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["dsh-market-notice"])
            .build();
        notice.append(
            &gtk4::Image::builder()
                .icon_name("dialog-warning-symbolic")
                .valign(gtk4::Align::Center)
                .build(),
        );
        notice.append(
            &gtk4::Label::builder()
                .label(text)
                .wrap(true)
                .xalign(0.0)
                .hexpand(true)
                .build(),
        );
        notice
    }

    fn market_detail_key(entry: &MarketListingEntry) -> Option<String> {
        market::presented_version(entry).map(|version| {
            format!("{}@{}", entry.name, version.version)
        })
    }

    fn market_detail_section(&self, entry: &MarketListingEntry) -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        head.append(
            &gtk4::Label::builder()
                .label("详细介绍")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
        let refresh = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .css_classes(["flat"])
            .tooltip_text("重新读取")
            .build();
        let page = self.clone();
        let name = entry.name.clone();
        refresh.connect_clicked(move |_| page.market_load_detail(&name, true));
        head.append(&refresh);
        section.append(&head);

        let key = Self::market_detail_key(entry);
        let detail = key
            .as_deref()
            .and_then(|key| self.state.market_state.borrow().details.get(key).cloned());
        match detail {
            None | Some(MarketDetailState::Loading) => {
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .build();
                row.append(&gtk4::Spinner::builder().spinning(true).build());
                row.append(&gtk4::Label::new(Some("正在读取详细介绍")));
                section.append(&row);
            }
            Some(MarketDetailState::Error(text)) => {
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .build();
                row.append(&dot_widget(DotState::Error));
                row.append(&gtk4::Label::new(Some(text)));
                let retry = gtk4::Button::builder()
                    .label("重试")
                    .css_classes(["flat"])
                    .build();
                let page = self.clone();
                let name = entry.name.clone();
                retry.connect_clicked(move |_| page.market_load_detail(&name, true));
                row.append(&retry);
                section.append(&row);
            }
            Some(MarketDetailState::Ready(detail)) => match detail.status.as_str() {
                "found" => {
                    let markdown = detail.markdown.unwrap_or_default();
                    if markdown.trim().is_empty() {
                        section.append(&dim_label("暂无详细介绍"));
                    } else {
                        section.append(&crate::ui::message_renderer::render_markdown(&markdown));
                        section.append(&dim_label(
                            "图片在此客户端中不显示：详情文档中的图片依赖 Host 的文件路由",
                        ));
                    }
                }
                "none" => section.append(&dim_label("暂无详细介绍")),
                _ => section.append(&dim_label("无法读取详细介绍")),
            },
        }
        section
    }

    fn market_versions_section(&self, entry: &MarketListingEntry) -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        section.append(
            &gtk4::Label::builder()
                .label("版本")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        for version in &entry.versions {
            let row = gtk4::ListBoxRow::builder().build();
            let line = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(12)
                .margin_end(12)
                .build();
            let text = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .hexpand(true)
                .valign(gtk4::Align::Center)
                .build();
            let title_row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(6)
                .build();
            title_row.append(
                &gtk4::Label::builder()
                    .label(&version.version)
                    .css_classes(["heading"])
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            for tag in market::version_tags(entry, version) {
                title_row.append(&tag_label(tag.text(), tag.tone()));
            }
            text.append(&title_row);
            let meta = if version.compatibility.compatible {
                market::format_publish_date(version.publish_timestamp)
            } else {
                self.market_incompatibility_text(version, false)
            };
            text.append(&dim_label(&meta));
            line.append(&text);
            if market::version_install_offered(entry, version) {
                let install = gtk4::Button::builder()
                    .label("安装")
                    .css_classes(["flat"])
                    .valign(gtk4::Align::Center)
                    .build();
                let page = self.clone();
                let entry = entry.clone();
                let version = version.clone();
                install.connect_clicked(move |_| {
                    page.confirm_market_install(&entry, &version);
                });
                line.append(&install);
            }
            row.set_child(Some(&line));
            list.append(&row);
        }
        section.append(&list);
        section
    }

    fn market_facts_section(&self, entry: &MarketListingEntry, facts: &MarketSearchResult) -> gtk4::Box {
        let section = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        section.append(
            &gtk4::Label::builder()
                .label("来源")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        if let Some(author) = entry.author.as_deref() {
            list.append(&self.fact_row("作者", author));
        }
        if let Some(homepage) = entry.homepage.as_deref() {
            list.append(&self.fact_row("主页", homepage));
        }
        if let Some(repository) = entry.repository.as_ref() {
            let value = match repository.directory.as_deref() {
                Some(directory) => format!("{} {directory}", repository.url),
                None => repository.url.clone(),
            };
            list.append(&self.fact_row("源代码", &value));
        }
        if let Some(version) = market::presented_version(entry) {
            for fact in market::source_facts(&entry.name, version) {
                list.append(&self.fact_row(fact.label(), fact.value()));
            }
            list.append(&self.fact_row(
                "发布时间",
                &market::format_publish_date(version.publish_timestamp),
            ));
        }
        if let Some(category) = entry
            .metadata
            .category
            .as_deref()
            .and_then(|id| facts.metadata.categories.get(id))
            .and_then(|declaration| market::resolve_localized(Some(&declaration.title)))
        {
            list.append(&self.fact_row("分类", &category));
        }
        let labels = {
            let titles: Vec<String> = entry
                .metadata
                .labels
                .iter()
                .filter_map(|id| facts.metadata.labels.get(id))
                .filter_map(|declaration| market::resolve_localized(Some(&declaration.title)))
                .collect();
            if titles.is_empty() {
                None
            } else {
                Some(titles.join("、"))
            }
        };
        if let Some(labels) = labels {
            list.append(&self.fact_row("标签", &labels));
        }
        list.append(&self.fact_row("插件索引", &market::listing_title(facts)));
        section.append(&list);
        section
    }

    fn market_load_detail(&self, name: &str, refresh: bool) {
        let (url, entry) = {
            let state = self.state.market_state.borrow();
            (
                state.selected.clone(),
                state.listing.as_ref().and_then(|pages| {
                    pages
                        .entries
                        .iter()
                        .find(|entry| entry.name == name)
                        .cloned()
                }),
            )
        };
        let (Some(url), Some(entry)) = (url, entry) else {
            return;
        };
        let Some(version) = market::presented_version(&entry) else {
            return;
        };
        let version = version.clone();
        let key = format!("{}@{}", entry.name, version.version);
        {
            let mut state = self.state.market_state.borrow_mut();
            if !refresh && state.details.contains_key(&key) {
                return;
            }
            state.details.insert(key.clone(), MarketDetailState::Loading);
        }
        if refresh && self.state.market_state.borrow().open_entry.as_deref() == Some(name) {
            self.render_market();
        }
        let conn = match self.controller_weak() {
            Some(weak) => weak.upgrade().map(|ctrl| ctrl.connection_handle()),
            None => None,
        };
        let Some(conn) = conn else {
            self.state
                .market_state
                .borrow_mut()
                .details
                .insert(key, MarketDetailState::Error(market::detail_failure_text("closed")));
            return;
        };
        let params = PluginMarketReadEntryDetailParams {
            url: url.clone(),
            name: entry.name.clone(),
            version: version.version.clone(),
            language: Some("zh".to_string()),
            refresh: refresh.then_some(true),
            npm_file_sources: Some(NPM_FILE_SOURCES.iter().map(|s| s.to_string()).collect()),
        };
        let rx = rpc_receiver::<PluginMarketReadEntryDetailParams, PluginMarketReadEntryDetailResult>(
            conn,
            METHOD_PLUGIN_MARKET_READ_ENTRY_DETAIL,
            params,
            Duration::from_secs(60),
        );
        let page = self.clone();
        let name = name.to_string();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            let outcome = match rx.recv().await {
                Some(Ok(result)) => MarketDetailState::Ready(result.result),
                Some(Err(err)) => MarketDetailState::Error(market::detail_failure_text(&err)),
                None => MarketDetailState::Error(market::detail_failure_text("closed")),
            };
            {
                let mut state = page.state.market_state.borrow_mut();
                state.details.insert(key, outcome);
            }
            if page.state.market_state.borrow().open_entry.as_deref() == Some(name.as_str()) {
                page.render_market();
            }
        });
    }

    fn confirm_market_install(&self, entry: &MarketListingEntry, version: &MarketListingVersion) {
        let title = market::entry_title(entry);
        let mut body = vec![version.version.clone()];
        if let Some(author) = entry.author.as_deref() {
            body.push(format!("作者 {author}"));
        }
        for fact in market::source_facts(&entry.name, version) {
            body.push(format!("{} {}", fact.label(), fact.value()));
        }
        if !version.compatibility.compatible {
            body.push(self.market_incompatibility_text(version, true));
        }
        body.push("插件在本机以你的权限运行。请确认你信任它的来源".to_string());
        let dialog = adw::AlertDialog::builder()
            .heading(format!("安装「{title}」？"))
            .body(body.join("\n"))
            .build();
        dialog.add_response("cancel", "取消");
        dialog.add_response("install", "安装");
        dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);
        dialog.set_close_response("cancel");
        let page = self.clone();
        let entry = entry.clone();
        let version = version.clone();
        dialog.choose(
            Some(&self.overlay),
            None::<&gtk4::gio::Cancellable>,
            move |response| {
                if response == "install" {
                    page.start_market_install(&entry, &version);
                }
            },
        );
    }

    fn start_market_install(&self, entry: &MarketListingEntry, version: &MarketListingVersion) {
        let title = market::entry_title(entry);
        let spec = version.install.spec.clone();
        let registry = version.install.registry.clone();
        self.open_install_dialog_confirmed(&spec, &title, &version.version, registry);
    }

    fn open_market_sources(&self) {
        let dialog = adw::Dialog::builder()
            .title("索引源")
            .content_width(480)
            .content_height(460)
            .build();
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        content.append(&dim_label(
            "插件索引以一个索引文件的 URL 提供，「发现」从这里列出的索引浏览插件",
        ));
        let list = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .height_request(160)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&list)
            .build();
        content.append(&scroll);
        let draft = adw::EntryRow::builder().title("索引 URL").build();
        content.append(&draft);
        let failure_label = gtk4::Label::builder()
            .css_classes(["error", "caption"])
            .wrap(true)
            .xalign(0.0)
            .halign(gtk4::Align::Start)
            .visible(false)
            .build();
        content.append(&failure_label);
        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let close = gtk4::Button::builder().label("关闭").css_classes(["flat"]).build();
        let dialog_close = dialog.clone();
        close.connect_clicked(move |_| {
            dialog_close.close();
        });
        buttons.append(&close);
        let add = gtk4::Button::builder()
            .label("添加")
            .css_classes(["suggested-action"])
            .build();
        buttons.append(&add);
        content.append(&buttons);
        content.append(&self.market_notice(
            "索引由第三方提供。只添加你信任的来源：其中的插件安装后以你的权限运行",
        ));

        let page = self.clone();
        let failure_label_for_add = failure_label.clone();
        let draft_for_add = draft.clone();
        let list_for_add = list.clone();
        add.connect_clicked(move |_| {
            let text = draft_for_add.text().trim().to_string();
            if text.is_empty() {
                return;
            }
            let urls = {
                let state = page.state.market_state.borrow();
                state
                    .indexes
                    .iter()
                    .map(|index| index.url.clone())
                    .chain(std::iter::once(text))
                    .collect()
            };
            page.market_set_sources(
                urls,
                true,
                &failure_label_for_add,
                &draft_for_add,
                &list_for_add,
            );
        });

        self.market_sources_repopulate(&list, &failure_label, &draft);
        dialog.set_child(Some(&content));
        dialog.present(Some(&self.overlay));
    }

    fn market_sources_repopulate(
        &self,
        list: &gtk4::Box,
        failure_label: &gtk4::Label,
        draft: &adw::EntryRow,
    ) {
        clear_children(list);
        let indexes = self.state.market_state.borrow().indexes.clone();
        if indexes.is_empty() {
            list.append(&dim_label("还没有索引源。"));
        }
        for index in indexes {
            let row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();
            let text = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .hexpand(true)
                .valign(gtk4::Align::Center)
                .build();
            text.append(
                &gtk4::Label::builder()
                    .label(market::index_label(&index))
                    .css_classes(["heading"])
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            text.append(
                &gtk4::Label::builder()
                    .label(&index.url)
                    .css_classes(["monospace", "caption", "dim-label"])
                    .ellipsize(gtk4::pango::EllipsizeMode::End)
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            if let Some(error) = index.error.as_deref() {
                text.append(
                    &gtk4::Label::builder()
                        .label(error)
                        .css_classes(["error", "caption"])
                        .wrap(true)
                        .xalign(0.0)
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            row.append(&text);
            let remove = gtk4::Button::builder()
                .icon_name("user-trash-symbolic")
                .css_classes(["flat"])
                .valign(gtk4::Align::Center)
                .tooltip_text("移除")
                .build();
            let page = self.clone();
            let url = index.url.clone();
            let list_cb = list.clone();
            let failure_label = failure_label.clone();
            let draft = draft.clone();
            remove.connect_clicked(move |_| {
                let urls = {
                    let state = page.state.market_state.borrow();
                    state
                        .indexes
                        .iter()
                        .map(|index| index.url.clone())
                        .filter(|item| item != &url)
                        .collect()
                };
                page.market_set_sources(urls, false, &failure_label, &draft, &list_cb);
            });
            row.append(&remove);
            list.append(&row);
        }
    }

    fn market_set_sources(
        &self,
        urls: Vec<String>,
        adding: bool,
        failure_label: &gtk4::Label,
        draft: &adw::EntryRow,
        list: &gtk4::Box,
    ) {
        {
            let mut state = self.state.market_state.borrow_mut();
            if state.saving_sources {
                return;
            }
            state.saving_sources = true;
            state.source_failure = None;
        }
        failure_label.set_visible(false);
        let conn = match self.controller_weak() {
            Some(weak) => weak.upgrade().map(|ctrl| ctrl.connection_handle()),
            None => None,
        };
        let Some(conn) = conn else {
            self.state.market_state.borrow_mut().saving_sources = false;
            failure_label.set_label("未连接");
            failure_label.set_visible(true);
            return;
        };
        let params = PluginMarketSetIndexesParams { urls: urls.clone() };
        let rx = rpc_receiver::<PluginMarketSetIndexesParams, serde_json::Value>(
            conn,
            METHOD_PLUGIN_MARKET_SET_INDEXES,
            params,
            Duration::from_secs(15),
        );
        let page = self.clone();
        let failure_label = failure_label.clone();
        let draft = draft.clone();
        let list = list.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            match rx.recv().await {
                Some(Ok(_)) => {
                    {
                        let mut state = page.state.market_state.borrow_mut();
                        state.saving_sources = false;
                        state.indexes = urls
                            .iter()
                            .map(|url| {
                                let previous = state
                                    .indexes
                                    .iter()
                                    .find(|index| &index.url == url);
                                MarketIndexSummary {
                                    url: url.clone(),
                                    title: previous.and_then(|index| index.title.clone()),
                                    description: previous.and_then(|index| index.description.clone()),
                                    error: None,
                                }
                            })
                            .collect();
                    }
                    if adding {
                        draft.set_text("");
                    }
                    page.probe_market();
                    page.market_sources_repopulate(&list, &failure_label, &draft);
                }
                Some(Err(err)) => {
                    page.state.market_state.borrow_mut().saving_sources = false;
                    let failure = market::source_failure(&err);
                    failure_label.set_label(failure.text());
                    failure_label.set_visible(true);
                }
                None => {
                    page.state.market_state.borrow_mut().saving_sources = false;
                    failure_label.set_label("未连接");
                    failure_label.set_visible(true);
                }
            }
        });
    }

    fn refresh_current_view(&self) {
        match self.state.view.borrow().clone() {
            View::List => self.render_list(),
            View::More => self.render_more_page(),
            View::Package { name, from_more } => {
                let pkg = self
                    .state
                    .packages
                    .borrow()
                    .iter()
                    .find(|pkg| pkg.name == name)
                    .cloned();
                match pkg {
                    Some(pkg) => self.render_package_page(&pkg, from_more),
                    None => self.show_list(),
                }
            }
            View::Item(item) => self.render_item_page(item),
        }
    }

    pub fn handle_toggle_done(&self, key: String, result: Result<PluginChangeResult, String>) {
        let action = self.state.busy.borrow_mut().remove(&key);
        match result {
            Ok(change) => {
                let name = change.target.clone();
                if let Some(text) = change_notice_text(&change, &name) {
                    self.toast(&text);
                } else if change.application == "failed" {
                    self.toast(&change_failure_text(
                        &change,
                        action.unwrap_or(FailedAction::Enable),
                    ));
                }
                if change.application == "applied" || change.application == "restart-required" {
                    self.load_inventory();
                }
            }
            Err(err) => {
                let action = action.unwrap_or(FailedAction::Enable);
                self.toast(&format!("{}：{}", action.label(), err));
            }
        }
        self.refresh_current_view();
    }

    pub fn handle_remove_done(&self, name: String, result: Result<PluginChangeResult, String>) {
        match result {
            Ok(change) => {
                if let Some(text) = change_notice_text(&change, &name) {
                    self.toast(&text);
                } else if change.application == "failed" {
                    self.toast(&change_failure_text(&change, FailedAction::Uninstall));
                } else {
                    self.state.details.borrow_mut().remove(&name);
                }
                if change.application != "failed" {
                    self.load_inventory();
                }
            }
            Err(err) => self.toast(&format!("{}：{}", FailedAction::Uninstall.label(), err)),
        }
        self.refresh_current_view();
    }

    pub fn handle_install_state(&self, request_id: &str, phase: &str) {
        if self.state.install_model.borrow().request_id.as_deref() != Some(request_id) {
            return;
        }
        *self.state.install_state.borrow_mut() = Some((request_id.to_string(), phase.to_string()));
        reduce_install(
            &mut self.state.install_model.borrow_mut(),
            InstallAction::ProgressPhase {
                phase: phase.to_string(),
            },
        );
        self.render_install_dialog_content();
        self.update_add_button();
    }

    pub fn handle_install_log(&self, _request_id: Option<&str>, text: &str) {
        self.state.install_log.borrow_mut().push_str(text);
    }

    pub fn handle_install_log_chunk(
        &self,
        request_id: Option<&str>,
        job_id: &str,
        argv: &[String],
        cwd: &str,
        text: &str,
        exit_code: Option<i64>,
    ) {
        let tracked = self.state.install_model.borrow().request_id.clone();
        if tracked.is_none() || tracked.as_deref() != request_id {
            return;
        }
        reduce_install(
            &mut self.state.install_model.borrow_mut(),
            InstallAction::AppendLog {
                job_id: job_id.to_string(),
                argv: argv.to_vec(),
                cwd: cwd.to_string(),
                text: text.to_string(),
                exit_code,
            },
        );
        self.render_install_dialog_content();
    }

    pub fn handle_update_state(&self, request_id: &str, step: &str) {
        if self.state.update_request_id.borrow().as_deref() != Some(request_id) {
            return;
        }
        *self.state.update_state.borrow_mut() = Some((request_id.to_string(), step.to_string()));
        if !self.state.update_steps.borrow().contains(&step.to_string()) {
            self.state.update_steps.borrow_mut().push(step.to_string());
        }
        self.render_update_dialog_content();
    }

    pub fn handle_inspect_done(&self, result: Result<PluginSpecInspection, String>) {
        let mut model = self.state.install_model.borrow_mut();
        match result {
            Ok(PluginSpecInspection::Accepted {
                kind,
                name,
                version,
                description,
                bundle,
                registry,
                host,
            }) => {
                let subject = InstallSubjectInfo {
                    spec: model.spec.clone(),
                    kind,
                    name,
                    version,
                    description,
                    bundle,
                    registry,
                    host,
                    bundled: false,
                };
                reduce_install(&mut model, InstallAction::InspectAccepted { subject });
            }
            Ok(PluginSpecInspection::Refused {
                problem,
                reason,
                registries,
            }) => {
                reduce_install(
                    &mut model,
                    InstallAction::InspectRefused {
                        problem,
                        reason,
                        registries: registries.unwrap_or_default(),
                    },
                );
            }
            Err(err) => {
                reduce_install(
                    &mut model,
                    InstallAction::InspectRefused {
                        problem: "unknown".to_string(),
                        reason: err,
                        registries: vec![],
                    },
                );
            }
        }
        drop(model);
        self.render_install_dialog_content();
        self.update_add_button();
    }

    pub fn handle_install_done(
        &self,
        request_id: Option<String>,
        result: Result<PluginChangeResult, String>,
    ) {
        if let Some(id) = request_id.as_deref() {
            let matches_market = {
                let state = self.state.market_state.borrow();
                state
                    .install_request
                    .as_ref()
                    .map(|(req_id, _)| req_id == id)
                    .unwrap_or(false)
            };
            if matches_market {
                let title = self
                    .state
                    .market_state
                    .borrow_mut()
                    .install_request
                    .take()
                    .map(|(_, t)| t)
                    .unwrap_or_default();
                match &result {
                    Ok(change) => {
                        if let Some(text) = change_notice_text(change, &title) {
                            self.toast(&text);
                        } else if change.application == "failed" {
                            let reason = change
                                .error
                                .as_ref()
                                .and_then(|e| e.diagnostic.clone())
                                .filter(|d| !d.is_empty())
                                .unwrap_or_else(|| "Host 报告了一个错误".to_string());
                            self.toast(&format!("安装失败：{reason}"));
                        } else {
                            self.toast(&format!("已安装 {title}"));
                        }
                        if change.application != "failed" {
                            self.load_inventory();
                            self.market_reload_install_status();
                        }
                    }
                    Err(err) => self.toast(&format!("安装失败：{err}")),
                }
            }
        }

        let mut model = self.state.install_model.borrow_mut();
        match result {
            Ok(change) => {
                reduce_install(
                    &mut model,
                    InstallAction::InstallResult {
                        result: change.clone(),
                    },
                );
                if model.phase == InstallPhase::Done {
                    self.load_inventory();
                }
            }
            Err(err) => {
                let failure_change = PluginChangeResult {
                    changed: false,
                    application: "failed".to_string(),
                    restart_reason: None,
                    stage: "install".to_string(),
                    target: model.spec.clone(),
                    enabled: Some(true),
                    error: Some(PluginChangeError {
                        code: "operation-error".to_string(),
                        diagnostic: Some(err),
                        incompatible: None,
                    }),
                    package_result: None,
                    bundle: None,
                    version: None,
                    pending_builds: None,
                    approved_builds: None,
                    registries: None,
                    failed_at: None,
                };
                reduce_install(
                    &mut model,
                    InstallAction::InstallResult {
                        result: failure_change,
                    },
                );
            }
        }
        model.market_title = None;
        drop(model);
        self.render_install_dialog_content();
        self.update_add_button();
    }

    pub fn handle_cancel_install_done(&self, result: Result<PluginCancellationResult, String>) {
        let mut model = self.state.install_model.borrow_mut();
        match result {
            Ok(cancel) => {
                reduce_install(
                    &mut model,
                    InstallAction::CancelResult {
                        status: cancel.status.clone(),
                    },
                );
                if cancel.status == "cancelled" {
                    self.toast("已取消安装");
                }
            }
            Err(_) => {
                reduce_install(
                    &mut model,
                    InstallAction::CancelResult {
                        status: "unknown".to_string(),
                    },
                );
            }
        }
        drop(model);
        self.render_install_dialog_content();
        self.update_add_button();
    }

    pub fn handle_wait_install_done(&self, result: Result<Option<PluginChangeResult>, String>) {
        let mut model = self.state.install_model.borrow_mut();
        match result {
            Ok(change) => {
                reduce_install(&mut model, InstallAction::WaitInstallResult { result: change });
            }
            Err(_) => {
                reduce_install(&mut model, InstallAction::WaitInstallResult { result: None });
            }
        }
        drop(model);
        self.render_install_dialog_content();
        self.update_add_button();
    }

    pub fn handle_registries_loaded(&self, result: Result<PluginRegistries, String>) {
        if let Ok(registries) = result {
            *self.state.registries.borrow_mut() = Some(registries);
            self.render_install_dialog_content();
        }
    }

    pub fn handle_registry_probe_done(&self, result: Result<Option<String>, String>) {
        if let Ok(fastest) = result {
            *self.state.probe_fastest.borrow_mut() = fastest;
            self.render_install_dialog_content();
        }
    }

    pub fn handle_versions_loaded(&self, name: String, result: Result<PluginVersionList, String>) {
        self.state.versions_checking.borrow_mut().remove(&name);
        match result {
            Ok(list) => {
                self.state.versions_cache.borrow_mut().insert(name.clone(), list);
                let time_str = gtk4::glib::DateTime::now_local()
                    .ok()
                    .and_then(|dt| dt.format("%H:%M").ok().map(|s| s.to_string()))
                    .unwrap_or_else(|| "--:--".to_string());
                self.state.versions_checked_at.borrow_mut().insert(name, time_str);
            }
            Err(err) => {
                let refused = PluginVersionList::Refused {
                    problem: "unknown".to_string(),
                    reason: err,
                    attempts: Vec::new(),
                };
                self.state.versions_cache.borrow_mut().insert(name, refused);
            }
        }
        self.refresh_current_view();
    }

    pub fn handle_update_done(&self, result: Result<PluginUpdateResult, String>) {
        self.state.update_running.set(false);
        *self.state.update_request_id.borrow_mut() = None;
        match result {
            Ok(update_res) => {
                *self.state.update_result.borrow_mut() = Some(update_res);
                self.load_inventory();
            }
            Err(err) => {
                let fake = PluginUpdateResult {
                    from: String::new(),
                    to: String::new(),
                    step: "check".to_string(),
                    result: PluginChangeResult {
                        changed: false,
                        application: "failed".to_string(),
                        restart_reason: None,
                        stage: "update".to_string(),
                        target: String::new(),
                        enabled: None,
                        error: Some(PluginChangeError {
                            code: "operation-error".to_string(),
                            diagnostic: Some(err),
                            incompatible: None,
                        }),
                        package_result: None,
                        bundle: None,
                        version: None,
                        pending_builds: None,
                        approved_builds: None,
                        registries: None,
                        failed_at: None,
                    },
                    restore: None,
                };
                *self.state.update_result.borrow_mut() = Some(fake);
            }
        }
        self.render_update_dialog_content();
    }

    pub fn reload_forms(&self) {
        self.state.forms.borrow_mut().clear();
        if let View::Item(item) = self.state.view.borrow().clone() {
            self.load_forms(item);
        }
    }

    fn show_list(&self) {
        *self.state.view.borrow_mut() = View::List;
        self.stack.set_visible_child_name("list");
    }

    fn open_more(&self) {
        *self.state.view.borrow_mut() = View::More;
        self.render_more_page();
        self.stack.set_visible_child_name("more");
    }

    fn open_package(&self, name: &str, from_more: bool) {
        let pkg = self
            .state
            .packages
            .borrow()
            .iter()
            .find(|pkg| pkg.name == name)
            .cloned();
        let Some(pkg) = pkg else {
            return;
        };
        *self.state.view.borrow_mut() = View::Package {
            name: name.to_string(),
            from_more,
        };
        self.render_package_page(&pkg, from_more);
        self.stack.set_visible_child_name("package");
        self.load_detail(&pkg.name);
    }

    pub fn open_plugin_target(&self, target: &str) {
        if let Some(id) = target.strip_prefix("item:") {
            if let Some(item) = model::builtin_item(id) {
                self.open_item(item);
            }
        } else if let Some(name) = target.strip_prefix("package-more:") {
            self.open_package(name, true);
        } else if let Some(name) = target.strip_prefix("package:") {
            self.open_package(name, false);
        } else if let Some(name) = target.strip_prefix("market:") {
            self.market_open_entry(name);
        }
    }

    fn open_item(&self, item: &'static OfficialItem) {
        *self.state.view.borrow_mut() = View::Item(item);
        self.render_item_page(item);
        self.stack.set_visible_child_name("item");
        self.load_forms(item);
        if item.id == "web-search" {
            self.load_providers();
        }
    }

    fn crumb_row(&self, text: &str, back: View) -> gtk4::Box {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(4)
            .build();
        let button = gtk4::Button::builder()
            .icon_name("go-previous-symbolic")
            .css_classes(["flat"])
            .tooltip_text(text)
            .build();
        let page = self.clone();
        button.connect_clicked(move |_| match back.clone() {
            View::More => page.open_more(),
            View::Package { name, from_more } => page.open_package(&name, from_more),
            View::Item(item) => page.open_item(item),
            View::List => page.show_list(),
        });
        row.append(&button);
        row.append(&gtk4::Label::new(Some(text)));
        row
    }

    fn render_more_page(&self) {
        clear_children(&self.more_list);
        let packages = self.state.packages.borrow();
        let groups = group_packages(&packages, &[]);
        if groups.more.is_empty() {
            self.more_list.append(&dim_label("还没有安装任何插件。"));
            return;
        }
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        for pkg in &groups.more {
            list.append(&self.package_card(pkg, true));
        }
        self.more_list.append(&list);
    }

    fn render_list(&self) {
        clear_children(&self.status_area);
        clear_children(&self.groups_area);
        match self.state.status.get() {
            LoadStatus::Idle | LoadStatus::Loading => {
                self.status_area.append(&self.skeleton());
            }
            LoadStatus::Unavailable => {
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .build();
                row.append(&dot_widget(DotState::Idle));
                row.append(&gtk4::Label::new(Some(
                    "本部署没有可管理的 profile，无法安装或启停插件。",
                )));
                self.status_area.append(&row);
            }
            LoadStatus::Error => {
                let column = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Vertical)
                    .spacing(8)
                    .build();
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .build();
                row.append(&dot_widget(DotState::Error));
                row.append(&gtk4::Label::new(Some(
                    "可能由于网络问题，无法读取全部插件",
                )));
                let retry = gtk4::Button::builder()
                    .label("重试")
                    .css_classes(["flat"])
                    .build();
                let page = self.clone();
                retry.connect_clicked(move |_| page.user_refresh());
                column.append(&row);
                column.append(&retry);
                self.status_area.append(&column);
            }
            LoadStatus::Ready => self.render_groups(),
        }
        self.update_refresh_button();
    }

    fn skeleton(&self) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .build();
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        for _ in 0..4 {
            let row = gtk4::ListBoxRow::builder().build();
            let line = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .margin_top(10)
                .margin_bottom(10)
                .margin_start(12)
                .margin_end(12)
                .build();
            let icon = gtk4::Box::builder()
                .css_classes(["dsh-plugins-skeleton-icon"])
                .valign(gtk4::Align::Center)
                .build();
            let text = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(6)
                .hexpand(true)
                .valign(gtk4::Align::Center)
                .build();
            text.append(
                &gtk4::Box::builder()
                    .css_classes(["dsh-plugins-skeleton-bar"])
                    .halign(gtk4::Align::Start)
                    .width_request(160)
                    .build(),
            );
            text.append(
                &gtk4::Box::builder()
                    .css_classes(["dsh-plugins-skeleton-bar"])
                    .halign(gtk4::Align::Start)
                    .width_request(240)
                    .build(),
            );
            line.append(&icon);
            line.append(&text);
            row.set_child(Some(&line));
            list.append(&row);
        }
        box_.append(&list);
        box_
    }

    fn render_groups(&self) {
        let packages = self.state.packages.borrow();
        let served = {
            let forms = self.state.forms.borrow();
            let namespaces: Vec<String> = forms.iter().map(|ns| ns.ns.clone()).collect();
            served_builtin_items(&namespaces)
        };
        let groups = group_packages(&packages, &served);
        if groups.items.is_empty() && groups.extensions.is_empty() && groups.installed.is_empty() {
            self.status_area.append(
                &gtk4::Label::builder()
                    .label("还没有安装任何插件。")
                    .halign(gtk4::Align::Start)
                    .css_classes(["dim-label"])
                    .build(),
            );
            return;
        }
        if !groups.items.is_empty() {
            let cards: Vec<gtk4::ListBoxRow> = groups
                .items
                .iter()
                .map(|item| self.item_card(item))
                .collect();
            self.groups_area
                .append(&self.group("内置插件配置", None, cards));
        }
        if !groups.extensions.is_empty() || !groups.more.is_empty() {
            let cards: Vec<gtk4::ListBoxRow> = groups
                .extensions
                .iter()
                .map(|pkg| self.package_card(pkg, false))
                .collect();
            let more_action = if groups.more.is_empty() {
                None
            } else {
                let more_button = gtk4::Button::builder()
                    .label("更多")
                    .css_classes(["flat"])
                    .valign(gtk4::Align::Center)
                    .build();
                let page = self.clone();
                more_button.connect_clicked(move |_| page.open_more());
                Some(more_button)
            };
            self.groups_area
                .append(&self.group("实验性插件", more_action, cards));
        }
        if !groups.installed.is_empty() {
            let cards: Vec<gtk4::ListBoxRow> = groups
                .installed
                .iter()
                .map(|pkg| self.package_card(pkg, false))
                .collect();
            self.groups_area.append(&self.group("已安装插件", None, cards));
        }
    }

    fn group(
        &self,
        title: &str,
        action: Option<gtk4::Button>,
        rows: Vec<gtk4::ListBoxRow>,
    ) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        head.append(
            &gtk4::Label::builder()
                .label(title)
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
        if let Some(button) = action {
            head.append(&button);
        }
        box_.append(&head);
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        for row in &rows {
            list.append(row);
        }
        box_.append(&list);
        box_
    }

    fn item_card(&self, item: &'static OfficialItem) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::builder()
            .activatable(true)
            .build();
        let line = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        line.append(&item_icon(item.id, 32));
        let text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();
        text.append(
            &gtk4::Label::builder()
                .label(item.label)
                .css_classes(["heading"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        text.append(&dim_label(item.description));
        line.append(&text);
        line.append(
            &gtk4::Image::builder()
                .icon_name("go-next-symbolic")
                .css_classes(["dim-label"])
                .build(),
        );
        row.set_child(Some(&line));
        row.set_action_name(Some("win.open-plugin-target"));
        row.set_action_target_value(Some(&format!("item:{}", item.id).to_variant()));
        row
    }

    fn package_card(&self, pkg: &PackageView, from_more: bool) -> gtk4::ListBoxRow {
        let row = gtk4::ListBoxRow::builder()
            .activatable(true)
            .build();
        let line = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(12)
            .margin_end(12)
            .build();
        line.append(&package_icon(pkg.icon.as_deref(), 32));
        let text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .valign(gtk4::Align::Center)
            .build();
        let title_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        title_row.append(
            &gtk4::Label::builder()
                .label(pkg.title.clone().unwrap_or_else(|| pkg.name.clone()))
                .css_classes(["heading"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        for tag in package_tags(pkg) {
            title_row.append(&tag_label(tag.text(), tag.tone()));
        }
        text.append(&title_row);
        if let Some(description) = pkg.meta_description.as_deref().or(pkg.description.as_deref()) {
            text.append(&dim_label(description));
        }
        line.append(&text);

        let busy = self.state.busy.borrow().contains_key(&pkg.name);
        if busy {
            line.append(&gtk4::Spinner::builder().spinning(true).build());
        }
        let switch = gtk4::Switch::builder()
            .active(pkg.enabled)
            .valign(gtk4::Align::Center)
            .sensitive(self.bundle_switch_sensitive(pkg, busy))
            .tooltip_text(
                pkg.read_only_reason
                    .as_deref()
                    .map(management_reason_text)
                    .unwrap_or_default(),
            )
            .build();
        let page = self.clone();
        let name = pkg.name.clone();
        switch.connect_active_notify(move |switch| {
            page.toggle_bundle(&name, switch.is_active());
        });
        line.append(&switch);

        row.set_child(Some(&line));
        row.set_action_name(Some("win.open-plugin-target"));
        let target = if from_more {
            format!("package-more:{}", pkg.name)
        } else {
            format!("package:{}", pkg.name)
        };
        row.set_action_target_value(Some(&target.to_variant()));
        row
    }

    fn bundle_switch_sensitive(&self, pkg: &PackageView, busy: bool) -> bool {
        if busy || pkg.read_only_reason.is_some() {
            return false;
        }
        if pkg.enabled {
            return true;
        }
        match &pkg.error {
            None => true,
            Some(error) => error.code != "unsupported-platform"
                && !(pkg.availability == "missing" && pkg.install_target.is_none()),
        }
    }

    fn toggle_bundle(&self, name: &str, enable: bool) {
        let action = if enable {
            FailedAction::Enable
        } else {
            FailedAction::Disable
        };
        if !self.mark_busy(name.to_string(), action) {
            return;
        }
        self.with_controller(|ctrl| ctrl.toggle_plugin_bundle(name, enable));
        self.refresh_current_view();
    }

    fn toggle_row(&self, entry_id: &str, enable: bool) {
        let action = if enable {
            FailedAction::RowEnable
        } else {
            FailedAction::RowDisable
        };
        let key = format!("row:{entry_id}");
        if !self.mark_busy(key, action) {
            return;
        }
        self.with_controller(|ctrl| ctrl.toggle_plugin_row(entry_id, enable));
        self.refresh_current_view();
    }

    fn mark_busy(&self, key: String, action: FailedAction) -> bool {
        let mut busy = self.state.busy.borrow_mut();
        if busy.contains_key(&key) {
            return false;
        }
        busy.insert(key, action);
        true
    }

    fn load_detail(&self, name: &str) {
        if self.state.details.borrow().contains_key(name) {
            return;
        }
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            return;
        };
        let params = PluginDetailParams {
            name: name.to_string(),
            language: Some("zh".to_string()),
            npm_file_sources: Some(NPM_FILE_SOURCES.iter().map(|s| s.to_string()).collect()),
        };
        let rx = rpc_receiver::<PluginDetailParams, PluginOperationResult>(
            conn_arc,
            METHOD_PLUGIN_DETAIL,
            params,
            Duration::from_secs(20),
        );
        let page = self.clone();
        let name = name.to_string();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            let outcome = match rx.recv().await {
                Some(Ok(result)) => {
                    serde_json::from_value::<PluginBundleDetail>(result.result).ok()
                }
                _ => None,
            };
            let detail = outcome.unwrap_or(PluginBundleDetail {
                status: "failed".to_string(),
                origin: None,
                markdown: None,
            });
            page.state.details.borrow_mut().insert(name.clone(), detail);
            if let View::Package {
                name: open_name,
                from_more,
            } = page.state.view.borrow().clone()
            {
                if open_name == name {
                    if let Some(pkg) = page
                        .state
                        .packages
                        .borrow()
                        .iter()
                        .find(|pkg| pkg.name == name)
                        .cloned()
                    {
                        page.render_package_page(&pkg, from_more);
                    }
                }
            }
        });
    }

    fn detail_section(&self, name: &str) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        box_.append(
            &gtk4::Label::builder()
                .label("详细介绍")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        let detail = self.state.details.borrow().get(name).cloned();
        match detail {
            None => {
                let row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .build();
                row.append(&gtk4::Spinner::builder().spinning(true).build());
                row.append(&gtk4::Label::new(Some("正在读取详细介绍")));
                box_.append(&row);
            }
            Some(detail) => match detail.status.as_str() {
                "found" => {
                    let markdown = detail.markdown.unwrap_or_default();
                    if markdown.trim().is_empty() {
                        box_.append(&dim_label("暂无详细介绍"));
                    } else {
                        box_.append(&crate::ui::message_renderer::render_markdown(&markdown));
                    }
                }
                "none" => box_.append(&dim_label("暂无详细介绍")),
                _ => box_.append(&dim_label("无法读取详细介绍")),
            },
        }
        box_
    }

    fn render_package_page(&self, pkg: &PackageView, from_more: bool) {
        clear_children(&self.package_box);
        let back_label = if from_more { "返回实验性插件" } else { "返回插件" };
        let back_view = if from_more { View::More } else { View::List };
        self.package_box.append(&self.crumb_row(back_label, back_view));

        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        head.append(&package_icon(pkg.icon.as_deref(), 48));
        let actions = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .valign(gtk4::Align::Center)
            .hexpand(true)
            .halign(gtk4::Align::End)
            .build();
        let busy = self.state.busy.borrow().contains_key(&pkg.name);
        if busy {
            actions.append(&gtk4::Spinner::builder().spinning(true).build());
        }
        if pkg.updatable {
            let cur_ver = pkg.version.clone().unwrap_or_default();
            if let Some(candidate) = self
                .state
                .versions_cache
                .borrow()
                .get(&pkg.name)
                .and_then(|list| {
                    if let PluginVersionList::Listed { versions, .. } = list {
                        update_candidate(versions).cloned()
                    } else {
                        None
                    }
                })
            {
                let to_ver = candidate.version.clone();
                let up_btn = gtk4::Button::builder()
                    .label(format!("更新到 v{to_ver}"))
                    .css_classes(["suggested-action"])
                    .sensitive(!busy)
                    .build();
                let page = self.clone();
                let name = pkg.name.clone();
                let reg = match self.state.versions_cache.borrow().get(&pkg.name) {
                    Some(PluginVersionList::Listed { registry, .. }) => registry.clone(),
                    _ => None,
                };
                up_btn.connect_clicked(move |_| {
                    page.open_update_dialog(&name, &cur_ver, &to_ver, reg.clone());
                });
                actions.append(&up_btn);
            } else if update_available(pkg) {
                let up_btn = gtk4::Button::builder()
                    .label("更新")
                    .sensitive(!busy)
                    .build();
                let page = self.clone();
                let name = pkg.name.clone();
                let target_ver = pkg
                    .install_target
                    .as_ref()
                    .map(|t| t.version.clone())
                    .unwrap_or_default();
                up_btn.connect_clicked(move |_| {
                    page.open_update_dialog(&name, &cur_ver, &target_ver, None);
                });
                actions.append(&up_btn);
            }
        }
        if pkg.removable && pkg.read_only_reason.is_none() {
            let remove = gtk4::Button::builder()
                .label("卸载")
                .css_classes(["destructive-action"])
                .sensitive(!busy)
                .build();
            let page = self.clone();
            let name = pkg.name.clone();
            remove.connect_clicked(move |_| page.confirm_uninstall(&name));
            actions.append(&remove);
        }
        let switch = gtk4::Switch::builder()
            .active(pkg.enabled)
            .valign(gtk4::Align::Center)
            .sensitive(self.bundle_switch_sensitive(pkg, busy))
            .build();
        let page = self.clone();
        let name = pkg.name.clone();
        switch.connect_active_notify(move |switch| {
            page.toggle_bundle(&name, switch.is_active());
        });
        actions.append(&switch);
        head.append(&actions);
        self.package_box.append(&head);

        let title_row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        title_row.append(
            &gtk4::Label::builder()
                .label(pkg.title.clone().unwrap_or_else(|| pkg.name.clone()))
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        if let Some(version) = pkg.version.as_deref() {
            title_row.append(&tag_label(&format!("v{version}"), "neutral"));
        }
        for tag in package_tags(pkg) {
            title_row.append(&tag_label(tag.text(), tag.tone()));
        }
        self.package_box.append(&title_row);
        self.package_box.append(
            &gtk4::Label::builder()
                .label(&pkg.name)
                .css_classes(["monospace", "caption", "dim-label"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        if let Some(description) = pkg.meta_description.as_deref().or(pkg.description.as_deref()) {
            self.package_box.append(&dim_label(description));
        }
        if let Some(meta_error) = pkg.meta_error.as_deref() {
            self.package_box.append(&dim_label(&format!("包元信息错误：{meta_error}")));
        }
        if let Some(error) = pkg.error.as_ref() {
            let reason = error
                .diagnostic
                .clone()
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| management_reason_text(&error.code).to_string());
            self.package_box.append(
                &gtk4::Label::builder()
                    .label(format!("原因：{reason}"))
                    .wrap(true)
                    .xalign(0.0)
                    .css_classes(["error", "caption"])
                    .halign(gtk4::Align::Start)
                    .build(),
            );
        }
        if let Some(reason) = pkg.read_only_reason.as_deref() {
            self.package_box
                .append(&dim_label(management_reason_text(reason)));
        }

        if pkg.availability != "missing" {
            self.package_box.append(&self.detail_section(&pkg.name));
            self.package_box.append(&self.rows_section(pkg));
        }
        if pkg.updatable {
            self.package_box.append(&self.version_section(pkg));
        }
        self.package_box.append(&self.source_section(pkg));
    }

    fn confirm_uninstall(&self, name: &str) {
        let dialog = adw::AlertDialog::builder()
            .heading(format!("卸载「{name}」？"))
            .body("卸载后它提供的功能会消失。")
            .build();
        dialog.add_response("cancel", "取消");
        dialog.add_response("remove", "卸载");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_close_response("cancel");
        let page = self.clone();
        let name = name.to_string();
        dialog.choose(
            Some(&self.overlay),
            None::<&gtk4::gio::Cancellable>,
            move |response| {
                if response == "remove" {
                    page.mark_busy(name.clone(), FailedAction::Uninstall);
                    page.with_controller(|ctrl| ctrl.remove_plugin(&name));
                    page.refresh_current_view();
                }
            },
        );
    }

    fn rows_section(&self, pkg: &PackageView) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        head.append(
            &gtk4::Label::builder()
                .label("包含的组件")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
        let failed = pkg
            .rows
            .iter()
            .filter(|row| row.phase == Some(RowPhase::Failed))
            .count();
        let off = pkg.rows.iter().filter(|row| !row.enabled).count();
        let running = pkg
            .rows
            .iter()
            .filter(|row| row.enabled && row.phase == Some(RowPhase::Active))
            .count();
        let mut summary = format!("共 {} 个", pkg.rows.len());
        if running > 0 {
            summary.push_str(&format!(" · {running} 运行中"));
        }
        if off > 0 {
            summary.push_str(&format!(" · {off} 已停用"));
        }
        if failed > 0 {
            summary.push_str(&format!(" · {failed} 异常"));
        }
        head.append(&dim_label(&summary));
        box_.append(&head);
        if pkg.rows.is_empty() {
            box_.append(&dim_label("这个插件包不包含任何组件。"));
            return box_;
        }

        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let mut searchable: Vec<(gtk4::ListBoxRow, String)> = Vec::new();
        for row in &pkg.rows {
            let title = row.title.clone().unwrap_or_else(|| row.module_name.clone());
            let list_row = gtk4::ListBoxRow::builder().build();
            let line = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .margin_top(8)
                .margin_bottom(8)
                .margin_start(12)
                .margin_end(12)
                .build();
            line.append(&dot_widget(row_dot(row)));
            let text = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(2)
                .hexpand(true)
                .valign(gtk4::Align::Center)
                .build();
            text.append(
                &gtk4::Label::builder()
                    .label(&title)
                    .halign(gtk4::Align::Start)
                    .build(),
            );
            text.append(&dim_label(row_state_text(row)));
            if row.title.is_some() && row.title.as_deref() != Some(row.row_id.as_str()) {
                text.append(
                    &gtk4::Label::builder()
                        .label(&row.row_id)
                        .css_classes(["monospace", "caption", "dim-label"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            if row.title.as_deref() != Some(row.module_name.as_str()) {
                text.append(
                    &gtk4::Label::builder()
                        .label(&row.module_name)
                        .css_classes(["monospace", "caption", "dim-label"])
                        .halign(gtk4::Align::Start)
                        .build(),
                );
            }
            line.append(&text);
            let row_busy = row
                .entry_id
                .as_deref()
                .map(|entry_id| self.state.busy.borrow().contains_key(&format!("row:{entry_id}")))
                .unwrap_or(false);
            if row_busy {
                line.append(&gtk4::Spinner::builder().spinning(true).build());
            }
            if pkg.enabled {
                if let Some(entry_id) = row.entry_id.as_deref() {
                    let switch = gtk4::Switch::builder()
                        .active(row.enabled)
                        .valign(gtk4::Align::Center)
                        .sensitive(!row.read_only_reason.is_some() && !row_busy)
                        .tooltip_text(
                            row.read_only_reason
                                .as_deref()
                                .map(management_reason_text)
                                .unwrap_or_default(),
                        )
                        .build();
                    let page = self.clone();
                    let entry = entry_id.to_string();
                    switch.connect_active_notify(move |switch| {
                        page.toggle_row(&entry, switch.is_active());
                    });
                    line.append(&switch);
                }
            }
            list_row.set_child(Some(&line));
            let hay = format!("{title} {} {}", row.row_id, row.module_name).to_lowercase();
            searchable.push((list_row.clone(), hay));
            list.append(&list_row);
        }
        if pkg.rows.len() > ROW_FILTER_THRESHOLD {
            let search = gtk4::SearchEntry::builder()
                .placeholder_text("筛选组件")
                .build();
            let entries = Rc::new(RefCell::new(searchable));
            search.connect_search_changed(move |entry| {
                let query = entry.text().to_string().trim().to_lowercase();
                for (row, hay) in entries.borrow().iter() {
                    row.set_visible(query.is_empty() || hay.contains(&query));
                }
            });
            box_.append(&search);
        }
        box_.append(&list);
        box_
    }

    fn version_section(&self, pkg: &PackageView) -> gtk4::Box {
        self.ensure_versions_checked(&pkg.name);
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        head.append(
            &gtk4::Label::builder()
                .label("版本")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
        let checking = self.state.versions_checking.borrow().contains(&pkg.name);
        let recheck_btn = gtk4::Button::builder()
            .icon_name("view-refresh-symbolic")
            .css_classes(["flat"])
            .tooltip_text("重新检查版本")
            .sensitive(!checking)
            .build();
        let page = self.clone();
        let name = pkg.name.clone();
        recheck_btn.connect_clicked(move |_| page.recheck_versions(&name));
        head.append(&recheck_btn);
        box_.append(&head);

        let list = self.state.versions_cache.borrow().get(&pkg.name).cloned();
        let checked_time = self.state.versions_checked_at.borrow().get(&pkg.name).cloned();
        let summary = evaluate_version_summary(list.as_ref(), checking, checked_time.as_deref());

        let summary_group = adw::PreferencesGroup::new();
        match summary {
            VersionSummaryKind::Checking => {
                let row = adw::ActionRow::builder()
                    .title("正在检查更新")
                    .build();
                let spinner = gtk4::Spinner::builder().spinning(true).build();
                row.add_suffix(&spinner);
                summary_group.add(&row);
            }
            VersionSummaryKind::Refused { title, sub } => {
                let row = adw::ActionRow::builder()
                    .title(&title)
                    .subtitle(&sub)
                    .build();
                row.add_prefix(&dot_widget(DotState::Error));
                summary_group.add(&row);
                if let Some(PluginVersionList::Refused { attempts, .. }) = &list {
                    for attempt in attempts {
                        let reg_name = attempt.registry.as_deref().unwrap_or("默认安装源");
                        let prob = match attempt.problem.as_deref() {
                            Some("network") => "无法连接",
                            Some("not-found") => "未找到",
                            _ => "失败",
                        };
                        let att_row = adw::ActionRow::builder()
                            .title(reg_name)
                            .subtitle(format!("{prob} {}", attempt.reason.as_deref().unwrap_or("")))
                            .build();
                        summary_group.add(&att_row);
                    }
                }
            }
            VersionSummaryKind::Available {
                version,
                published_at,
            } => {
                let row = adw::ActionRow::builder()
                    .title(format!("可更新到 v{version}"))
                    .subtitle(published_at.map(|d| format!("发布于 {d}")).unwrap_or_default())
                    .build();
                row.add_prefix(&tag_label("可更新", "accent"));
                let update_btn = gtk4::Button::builder()
                    .label("更新")
                    .css_classes(["suggested-action"])
                    .valign(gtk4::Align::Center)
                    .build();
                let page = self.clone();
                let name = pkg.name.clone();
                let cur_ver = pkg.version.clone().unwrap_or_default();
                let target_ver = version.clone();
                let reg = match &list {
                    Some(PluginVersionList::Listed { registry, .. }) => registry.clone(),
                    _ => None,
                };
                update_btn.connect_clicked(move |_| {
                    page.open_update_dialog(&name, &cur_ver, &target_ver, reg.clone());
                });
                row.add_suffix(&update_btn);
                summary_group.add(&row);
            }
            VersionSummaryKind::Latest { checked_at } => {
                let row = adw::ActionRow::builder()
                    .title("已是最新版本")
                    .subtitle(checked_at.map(|t| format!("上次检查于 {t}")).unwrap_or_default())
                    .build();
                row.add_prefix(&dot_widget(DotState::Done));
                summary_group.add(&row);
            }
            VersionSummaryKind::NoneInstallable => {
                let row = adw::ActionRow::builder()
                    .title("没有可直接安装的新版本")
                    .subtitle("新版本已弃用、为预发布版本或没有声明组合包")
                    .build();
                row.add_prefix(&dot_widget(DotState::Idle));
                summary_group.add(&row);
            }
            VersionSummaryKind::BlockedDsh {
                version,
                peers,
                runtime,
            } => {
                let row = adw::ActionRow::builder()
                    .title("新版本需要更新 DSH")
                    .subtitle(format!("{version} 需要 {peers}，当前 DSH 为 {runtime}"))
                    .build();
                row.add_prefix(&dot_widget(DotState::Error));
                summary_group.add(&row);
            }
        }
        box_.append(&summary_group);

        if let Some(PluginVersionList::Listed {
            versions,
            omitted,
            registry,
            ..
        }) = list
        {
            let show_prerelease = self
                .state
                .versions_show_prerelease
                .borrow()
                .get(&pkg.name)
                .copied()
                .unwrap_or(false);
            let filtered = filter_version_items(&versions, show_prerelease);

            let versions_listbox = gtk4::ListBox::builder()
                .selection_mode(gtk4::SelectionMode::None)
                .css_classes(["boxed-list"])
                .build();
            let cur_version = pkg.version.as_deref().unwrap_or("");
            for item in filtered {
                let row = gtk4::ListBoxRow::builder().build();
                let line = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(8)
                    .margin_top(8)
                    .margin_bottom(8)
                    .margin_start(12)
                    .margin_end(12)
                    .build();
                let text_box = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Vertical)
                    .spacing(2)
                    .hexpand(true)
                    .build();
                let title_row = gtk4::Box::builder()
                    .orientation(gtk4::Orientation::Horizontal)
                    .spacing(6)
                    .build();
                title_row.append(
                    &gtk4::Label::builder()
                        .label(format!("v{}", item.version))
                        .css_classes(["heading"])
                        .build(),
                );
                if item.version == cur_version {
                    title_row.append(&tag_label("已安装", "neutral"));
                }
                if item.prerelease {
                    title_row.append(&tag_label("预发布", "neutral"));
                }
                if item.deprecated.is_some() {
                    title_row.append(&tag_label("已弃用", "error"));
                }
                text_box.append(&title_row);
                if let Some(dep) = &item.deprecated {
                    text_box.append(&dim_label(dep));
                } else if item.incompatible.is_some() {
                    text_box.append(&dim_label("需要较新版本的 DSH"));
                } else if !item.bundle {
                    text_box.append(&dim_label("这个版本没有声明组合包，无法安装"));
                }
                if let Some(pub_date) = &item.published_at {
                    text_box.append(&dim_label(pub_date));
                }
                line.append(&text_box);

                if installable_version(item) && item.version != cur_version {
                    let update_btn = gtk4::Button::builder()
                        .label("更新")
                        .css_classes(["flat"])
                        .valign(gtk4::Align::Center)
                        .build();
                    let page = self.clone();
                    let name = pkg.name.clone();
                    let from_ver = cur_version.to_string();
                    let to_ver = item.version.clone();
                    let reg = registry.clone();
                    update_btn.connect_clicked(move |_| {
                        page.open_update_dialog(&name, &from_ver, &to_ver, reg.clone());
                    });
                    line.append(&update_btn);
                }
                row.set_child(Some(&line));
                versions_listbox.append(&row);
            }
            box_.append(&versions_listbox);

            let footer_row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(12)
                .build();
            let check = gtk4::CheckButton::builder()
                .label("显示预发布版本")
                .active(show_prerelease)
                .build();
            let page = self.clone();
            let name = pkg.name.clone();
            check.connect_toggled(move |btn| {
                page.state
                    .versions_show_prerelease
                    .borrow_mut()
                    .insert(name.clone(), btn.is_active());
                page.refresh_current_view();
            });
            footer_row.append(&check);
            if omitted > 0 {
                footer_row.append(&dim_label(&format!("另有 {omitted} 个版本未列出")));
            }
            box_.append(&footer_row);
        }
        box_
    }

    fn source_section(&self, pkg: &PackageView) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .build();
        box_.append(
            &gtk4::Label::builder()
                .label("来源信息")
                .css_classes(["title-4"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["boxed-list"])
            .build();
        let spec = match pkg.source.as_deref() {
            Some(source) => source.to_string(),
            None => {
                if pkg.availability == "missing" {
                    if pkg.installed {
                        "包不可用".to_string()
                    } else {
                        "未安装".to_string()
                    }
                } else {
                    "内置".to_string()
                }
            }
        };
        list.append(&self.fact_row("代码来源", &spec));
        if let Some(version) = pkg.version.as_deref() {
            list.append(&self.fact_row("当前版本", version));
        }
        if let Some(target) = pkg.install_target.as_ref() {
            list.append(&self.fact_row("当前 DSH 对应来源", &target.spec));
            list.append(&self.fact_row("当前 DSH 对应版本", &target.version));
        }
        box_.append(&list);
        box_
    }

    fn fact_row(&self, title: &str, value: &str) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(title).build();
        let label = gtk4::Label::builder()
            .label(value)
            .css_classes(["monospace", "caption"])
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .valign(gtk4::Align::Center)
            .build();
        row.add_suffix(&label);
        row
    }

    fn item_namespaces(&self, item: &OfficialItem) -> Vec<String> {
        let forms = self.state.forms.borrow();
        item.namespaces
            .iter()
            .filter(|ns| forms.iter().any(|form| &form.ns == *ns))
            .map(|ns| ns.to_string())
            .collect()
    }

    fn load_forms(&self, item: &'static OfficialItem) {
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            return;
        };
        let rx = rpc_receiver::<serde_json::Value, SettingsDescribeResult>(
            conn_arc,
            METHOD_SETTINGS_DESCRIBE,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            if let Some(Ok(result)) = rx.recv().await {
                *page.state.forms.borrow_mut() = result.namespaces;
                if let View::Item(open_item) = page.state.view.borrow().clone() {
                    if open_item.id == item.id {
                        page.render_item_page(item);
                    }
                }
            }
        });
    }

    fn load_providers(&self) {
        if self.state.providers.borrow().is_some() {
            return;
        }
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            return;
        };
        let rx = rpc_receiver::<serde_json::Value, serde_json::Value>(
            conn_arc,
            METHOD_WEB_SEARCH_PROVIDERS,
            serde_json::json!({}),
            Duration::from_secs(10),
        );
        let page = self.clone();
        gtk4::glib::MainContext::default().spawn_local(async move {
            let mut rx = rx;
            match rx.recv().await {
                Some(Ok(value)) => {
                    let parse = |list: &serde_json::Value| -> Vec<ProviderOption> {
                        list.as_array()
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(|item| {
                                        let id = item["id"].as_str()?.to_string();
                                        let title = item["title"].as_str().map(str::to_string);
                                        Some(ProviderOption { id, title })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    };
                    let search = parse(&value["search"]);
                    let fetch = parse(&value["fetch"]);
                    *page.state.providers.borrow_mut() = Some((search, fetch));
                    page.state.providers_failed.set(false);
                }
                _ => {
                    page.state.providers_failed.set(true);
                }
            }
            if let View::Item(item) = page.state.view.borrow().clone() {
                if item.id == "web-search" {
                    page.render_item_page(item);
                }
            }
        });
    }

    fn render_item_page(&self, item: &'static OfficialItem) {
        clear_children(&self.item_box);
        self.item_box
            .append(&self.crumb_row("返回插件", View::List));
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(12)
            .build();
        head.append(&item_icon(item.id, 36));
        let text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        text.append(
            &gtk4::Label::builder()
                .label(item.label)
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        text.append(&dim_label(item.description));
        head.append(&text);
        self.item_box.append(&head);

        let web_served = self
            .state
            .forms
            .borrow()
            .iter()
            .any(|form| form.ns == "web")
            || self.state.providers.borrow().is_some();
        let served = self.item_namespaces(item);
        if served.is_empty() && !(item.id == "web-search" && web_served) {
            self.item_box
                .append(&dim_label("该插件当前未加载，暂时无法配置。"));
            return;
        }
        match item.id {
            "agent-loop" => self.render_agent_loop_form(),
            "shell" => self.render_shell_form(item),
            "subagent" => self.render_subagent_form(),
            "web-search" => self.render_web_search_form(item),
            _ => {}
        }
        self.item_box.append(&self.form_footer(item));
    }

    fn form_footer(&self, item: &'static OfficialItem) -> gtk4::Box {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .build();
        let status = gtk4::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .visible(false)
            .css_classes(["dim-label", "caption"])
            .build();
        box_.append(&status);
        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let discard = gtk4::Button::builder()
            .label("放弃")
            .css_classes(["flat"])
            .build();
        let save = gtk4::Button::builder()
            .label("保存")
            .css_classes(["suggested-action"])
            .build();
        buttons.append(&discard);
        buttons.append(&save);
        box_.append(&buttons);
        let page = self.clone();
        discard.connect_clicked(move |_| {
            page.state.drafts.borrow_mut().clear();
            if item.id == "web-search" {
                *page.state.providers.borrow_mut() = None;
                page.state.providers_failed.set(false);
                page.load_providers();
            }
            page.load_forms(item);
            page.render_item_page(item);
        });
        let page = self.clone();
        save.connect_clicked(move |_| page.save_forms(item, &status));
        box_
    }

    fn save_forms(&self, item: &'static OfficialItem, status: &gtk4::Label) {
        let mut targets: Vec<String> = self.item_namespaces(item);
        if item.id == "web-search" {
            for ns in ["web-deepseek-api", "web-search-deepseek"] {
                if !targets.iter().any(|entry| entry == ns)
                    && self.namespace_revision(ns).is_some()
                {
                    targets.push(ns.to_string());
                }
            }
        }
        let mut pending: Vec<(String, u64, serde_json::Value)> = Vec::new();
        {
            let drafts = self.state.drafts.borrow();
            let forms = self.state.forms.borrow();
            for ns in targets {
                let Some(patch) = drafts.get(&ns) else {
                    continue;
                };
                if patch.is_empty() {
                    continue;
                }
                let Some(revision) = forms
                    .iter()
                    .find(|form| form.ns == ns)
                    .map(|form| form.revision)
                else {
                    continue;
                };
                pending.push((
                    ns.clone(),
                    revision,
                    serde_json::Value::Object(patch.clone()),
                ));
            }
        }
        let credential_draft = self
            .state
            .drafts
            .borrow()
            .get("credentials")
            .and_then(|patch| patch.get("apiKey"))
            .and_then(|value| value.as_str())
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if pending.is_empty() && credential_draft.is_none() {
            return;
        }
        let conn_arc = self
            .controller_weak()
            .and_then(|weak| weak.upgrade())
            .map(|ctrl| ctrl.connection_handle());
        let Some(conn_arc) = conn_arc else {
            status.set_label("未连接");
            status.set_visible(true);
            return;
        };

        let total = pending.len() + usize::from(credential_draft.is_some());
        let remaining = Rc::new(Cell::new(total));
        let conflict = Rc::new(Cell::new(false));
        let other_failure = Rc::new(Cell::new(false));
        status.set_label("保存中…");
        status.set_visible(true);

        for (ns, revision, patch) in pending {
            let params = SettingsMutateParams {
                namespace: Some(ns.clone()),
                patch,
                expected_revision: Some(revision),
            };
            let rx = rpc_receiver::<SettingsMutateParams, serde_json::Value>(
                conn_arc.clone(),
                METHOD_SETTINGS_MUTATE,
                params,
                Duration::from_secs(15),
            );
            let page = self.clone();
            let remaining = remaining.clone();
            let conflict = conflict.clone();
            let other_failure = other_failure.clone();
            let status = status.clone();
            gtk4::glib::MainContext::default().spawn_local(async move {
                let mut rx = rx;
                if let Some(Err(err)) = rx.recv().await {
                    if err.contains("-32020") || err.to_lowercase().contains("conflict") {
                        conflict.set(true);
                    } else {
                        other_failure.set(true);
                    }
                }
                remaining.set(remaining.get() - 1);
                if remaining.get() == 0 {
                    page.finish_save(item, &status, conflict.get(), other_failure.get());
                }
            });
        }
        if let Some(key) = credential_draft {
            self.with_controller(|ctrl| ctrl.inject_credential("DEEPSEEK_API_KEY", &key));
            remaining.set(remaining.get() - 1);
            if remaining.get() == 0 {
                self.finish_save(item, status, conflict.get(), other_failure.get());
            }
        }
    }

    fn finish_save(
        &self,
        item: &'static OfficialItem,
        status: &gtk4::Label,
        conflict: bool,
        other_failure: bool,
    ) {
        if conflict {
            status.set_label("设置已在其他位置更新。请放弃修改后重试。");
            status.set_visible(true);
            return;
        }
        if other_failure {
            status.set_label("本部署没有接受这些值，已保留供你修改。");
            status.set_visible(true);
            return;
        }
        self.state.drafts.borrow_mut().clear();
        status.set_visible(false);
        self.load_forms(item);
    }

    fn namespace_value(&self, ns: &str, field: &str) -> Option<serde_json::Value> {
        self.state
            .forms
            .borrow()
            .iter()
            .find(|form| form.ns == ns)
            .and_then(|form| form.value.get(field).cloned())
    }

    fn namespace_revision(&self, ns: &str) -> Option<u64> {
        self.state
            .forms
            .borrow()
            .iter()
            .find(|form| form.ns == ns)
            .map(|form| form.revision)
    }

    fn draft_value(&self, ns: &str, field: &str) -> Option<serde_json::Value> {
        self.state
            .drafts
            .borrow()
            .get(ns)
            .and_then(|patch| patch.get(field).cloned())
    }

    fn set_draft(&self, ns: &str, field: &str, value: serde_json::Value) {
        self.state
            .drafts
            .borrow_mut()
            .entry(ns.to_string())
            .or_default()
            .insert(field.to_string(), value);
    }

    fn clear_draft(&self, ns: &str, field: &str) {
        if let Some(patch) = self.state.drafts.borrow_mut().get_mut(ns) {
            patch.remove(field);
        }
    }

    fn spin_field(
        &self,
        ns: &str,
        field: &str,
        title: &str,
        subtitle: &str,
        min: f64,
        max: f64,
    ) -> adw::SpinRow {
        let raw = self
            .draft_value(ns, field)
            .or_else(|| self.namespace_value(ns, field))
            .filter(|value| !value.is_null());
        let row = adw::SpinRow::with_range(min, max, 1.0);
        row.set_title(title);
        if !subtitle.is_empty() {
            row.set_subtitle(subtitle);
        }
        if let Some(number) = raw.and_then(|value| value.as_f64()) {
            row.set_value(number.clamp(min, max));
        }
        let page = self.clone();
        let ns = ns.to_string();
        let field = field.to_string();
        row.connect_changed(move |row| {
            let text = row.text().trim().to_string();
            if text.is_empty() {
                page.clear_draft(&ns, &field);
                return;
            }
            if let Ok(number) = text.parse::<f64>() {
                if let Some(value) = serde_json::Number::from_f64(number) {
                    page.set_draft(&ns, &field, serde_json::Value::Number(value));
                }
            }
        });
        row
    }

    fn render_agent_loop_form(&self) {
        if self.namespace_revision("agent-loop").is_none() {
            self.item_box
                .append(&dim_label("该插件当前未加载，暂时无法配置。"));
            return;
        }
        let group = adw::PreferencesGroup::new();
        group.add(&self.spin_field(
            "agent-loop",
            "maxParallelToolCalls",
            "并行工具调用数",
            "同一步内最多同时运行多少个可并行的调用。",
            0.0,
            1_048_576.0,
        ));
        self.item_box.append(&group);
    }

    fn render_shell_form(&self, item: &'static OfficialItem) {
        let ns = item
            .namespaces
            .iter()
            .find(|ns| self.namespace_revision(ns).is_some())
            .copied();
        let Some(ns) = ns else {
            self.item_box
                .append(&dim_label("该插件当前未加载，暂时无法配置。"));
            return;
        };
        let group = adw::PreferencesGroup::new();
        group.add(&self.spin_field(
            ns,
            "timeoutMs",
            "命令超时（毫秒）",
            "单条命令允许运行多久，超时即终止。",
            0.0,
            2_147_483_647.0,
        ));
        group.add(&self.spin_field(
            ns,
            "maxOutputBytes",
            "单流输出上限（字节）",
            "超出部分会转存到临时文件，而不是被丢弃。",
            0.0,
            2_147_483_647.0,
        ));
        self.item_box.append(&group);
    }

    fn render_subagent_form(&self) {
        if self.namespace_revision("subagent").is_none() {
            self.item_box
                .append(&dim_label("该插件当前未加载，暂时无法配置。"));
            return;
        }
        let group = adw::PreferencesGroup::builder().title("运行限制").build();
        group.add(&self.spin_field(
            "subagent",
            "maxDepth",
            "最大递归深度",
            "限制 Agent 创建子智能体的递归层级。",
            0.0,
            1_048_576.0,
        ));
        group.add(&self.spin_field(
            "subagent",
            "maxActiveSubagents",
            "子智能体并行数量上限",
            "同一主 Agent 下，所有递归层级同时存活的子智能体总数，主 Agent 不计入。达到上限时，新的启动请求会被拒绝。",
            1.0,
            1_048_576.0,
        ));
        self.item_box.append(&group);
        self.item_box.append(&dim_label(
            "如果某个工具单独设置了最大递归深度，以该工具的设置为准。",
        ));
    }

    fn provider_combo(
        &self,
        capability: &str,
        field: &str,
        item: &'static OfficialItem,
    ) -> adw::ComboRow {
        let (search, fetch) = self
            .state
            .providers
            .borrow()
            .clone()
            .unwrap_or_default();
        let mut options: Vec<ProviderOption> = if capability == "search" { search } else { fetch };
        let official_order: [&str; 2] = if capability == "search" {
            ["deepseek-search-api", "deepseek-official"]
        } else {
            ["http", "deepseek-fetch-api"]
        };
        options.sort_by_key(|option| {
            official_order
                .iter()
                .position(|id| *id == option.id)
                .unwrap_or(official_order.len())
        });

        let current = self
            .draft_value("web", field)
            .or_else(|| self.namespace_value("web", field))
            .and_then(|value| value.as_str().map(str::to_string));
        let mut ids: Vec<Option<String>> = vec![None];
        let model = gtk4::StringList::new(&["自动选择"]);
        for option in &options {
            model.append(&option.label(false));
            ids.push(Some(option.id.clone()));
        }
        let selected = current
            .as_deref()
            .and_then(|id| ids.iter().position(|entry| entry.as_deref() == Some(id)))
            .unwrap_or_else(|| match current {
                None => 0,
                Some(id) => {
                    model.append(
                        &ProviderOption {
                            id: id.clone(),
                            title: None,
                        }
                        .label(true),
                    );
                    ids.push(Some(id));
                    ids.len() - 1
                }
            });
        let combo = adw::ComboRow::builder()
            .model(&model)
            .selected(selected as u32)
            .build();
        let page = self.clone();
        let field = field.to_string();
        combo.connect_selected_notify(move |combo| {
            let value = ids
                .get(combo.selected() as usize)
                .cloned()
                .flatten()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null);
            page.set_draft("web", &field, value);
            let page = page.clone();
            gtk4::glib::idle_add_local_once(move || page.render_item_page(item));
        });
        combo
    }

    fn render_web_search_form(&self, item: &'static OfficialItem) {
        if self.state.providers_failed.get() && self.state.providers.borrow().is_none() {
            let failed = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();
            failed.append(&dim_label("无法读取提供方列表，此菜单可能不完整"));
            let retry = gtk4::Button::builder()
                .label("重试")
                .css_classes(["flat"])
                .build();
            let page = self.clone();
            retry.connect_clicked(move |_| {
                *page.state.providers.borrow_mut() = None;
                page.state.providers_failed.set(false);
                page.load_providers();
            });
            failed.append(&retry);
            self.item_box.append(&failed);
        }

        let search_group = adw::PreferencesGroup::builder()
            .title("网页搜索提供方")
            .build();
        search_group.add(&self.provider_combo("search", "searchProvider", item));
        self.item_box.append(&search_group);

        let search_kind = self
            .draft_value("web", "searchProvider")
            .or_else(|| self.namespace_value("web", "searchProvider"))
            .and_then(|value| value.as_str().map(str::to_string));
        match search_kind.as_deref() {
            Some("deepseek-search-api") => {
                if self.namespace_revision("web-deepseek-api").is_some() {
                    let panel = adw::PreferencesGroup::new();
                    panel.add(&self.spin_field(
                        "web-deepseek-api",
                        "count",
                        "每次搜索结果数",
                        "每个查询返回的结果条数，须为 1–50 的整数，默认 10；不控制搜索调用次数。web_search 保留的来源数不超过其自身的结果上限。",
                        1.0,
                        50.0,
                    ));
                    self.item_box.append(&panel);
                }
            }
            Some("deepseek-official") => {
                if self.namespace_revision("web-search-deepseek").is_some() {
                    self.item_box.append(&self.legacy_panel());
                }
            }
            Some(_) => {
                self.item_box
                    .append(&dim_label("请在该插件自己的设置中配置"));
            }
            None => {}
        }

        let fetch_group = adw::PreferencesGroup::builder()
            .title("网页获取提供方")
            .build();
        fetch_group.add(&self.provider_combo("fetch", "fetchProvider", item));
        self.item_box.append(&fetch_group);
    }

    fn legacy_panel(&self) -> adw::PreferencesGroup {
        let group = adw::PreferencesGroup::new();
        let key_ref = self
            .namespace_value("web-search-deepseek", "apiKeyEnv")
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "DEEPSEEK_API_KEY".to_string());

        let key_row = adw::PasswordEntryRow::builder().title("API Key").build();
        if let Some(draft) = self
            .draft_value("credentials", "apiKey")
            .and_then(|value| value.as_str().map(str::to_string))
        {
            key_row.set_text(&draft);
        }
        let page = self.clone();
        key_row.connect_changed(move |row| {
            page.set_draft(
                "credentials",
                "apiKey",
                serde_json::Value::String(row.text().to_string()),
            );
        });
        group.add(&key_row);
        let key_status = dim_label("未配置密钥。");
        {
            let conn_arc = self
                .controller_weak()
                .and_then(|weak| weak.upgrade())
                .map(|ctrl| ctrl.connection_handle());
            if let Some(conn_arc) = conn_arc {
                let rx = rpc_receiver::<CredentialRecordReadParams, CredentialRecordReadResult>(
                    conn_arc,
                    METHOD_CREDENTIALS_RECORDS_READ,
                    CredentialRecordReadParams {
                        key: key_ref.clone(),
                    },
                    Duration::from_secs(10),
                );
                let status_label = key_status.clone();
                gtk4::glib::MainContext::default().spawn_local(async move {
                    let mut rx = rx;
                    if let Some(Ok(result)) = rx.recv().await {
                        status_label.set_label(if result.configured {
                            "已配置密钥。"
                        } else {
                            "未配置密钥。"
                        });
                    }
                });
            }
        }
        group.add(&key_status);
        group.add(&dim_label(&format!(
            "账号模型发起的请求使用已登录账号；其他请求使用此 API Key（{key_ref}），默认与「模型」页共用"
        )));

        let base_row = adw::EntryRow::builder().title("Base URL").build();
        let base_value = self
            .draft_value("web-search-deepseek", "baseURL")
            .or_else(|| self.namespace_value("web-search-deepseek", "baseURL"))
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default();
        base_row.set_text(&base_value);
        let page = self.clone();
        base_row.connect_changed(move |row| {
            let text = row.text().to_string();
            if text.is_empty() {
                page.clear_draft("web-search-deepseek", "baseURL");
            } else {
                page.set_draft(
                    "web-search-deepseek",
                    "baseURL",
                    serde_json::Value::String(text),
                );
            }
        });
        group.add(&base_row);
        group.add(&self.spin_field(
            "web-search-deepseek",
            "maxUses",
            "单次请求最多搜索次数",
            "每次 Messages 请求允许的服务端搜索次数上限，须为正整数，默认 5",
            1.0,
            1_048_576.0,
        ));
        group.add(&dim_label(
            "填写含 API 前缀的地址；留空时使用官方地址；程序追加 /messages",
        ));
        group
    }

    fn ensure_versions_checked(&self, name: &str) {
        if self.state.versions_cache.borrow().contains_key(name)
            || self.state.versions_checking.borrow().contains(name)
        {
            return;
        }
        self.recheck_versions(name);
    }

    fn recheck_versions(&self, name: &str) {
        self.state
            .versions_checking
            .borrow_mut()
            .insert(name.to_string());
        let name_clone = name.to_string();
        self.with_controller(move |ctrl| ctrl.plugin_versions(&name_clone, None));
        self.refresh_current_view();
    }

    pub fn open_install_dialog(&self) {
        if let Some(dialog) = self.state.install_dialog.borrow().as_ref() {
            dialog.present(Some(&self.overlay));
            self.render_install_dialog_content();
            return;
        }
        let dialog = adw::Dialog::builder()
            .title("添加插件")
            .content_width(520)
            .content_height(580)
            .build();
        let content_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        let clamp = adw::Clamp::builder()
            .maximum_size(480)
            .child(&content_box)
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&clamp)
            .build();
        let view = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        view.add_top_bar(&header);
        view.set_content(Some(&scroll));
        dialog.set_child(Some(&view));

        let page = self.clone();
        dialog.connect_closed(move |_| {
            page.state.install_dialog.borrow_mut().take();
            page.state.install_dialog_content.borrow_mut().take();
            if !is_install_pending(page.state.install_model.borrow().phase) {
                reduce_install(
                    &mut page.state.install_model.borrow_mut(),
                    InstallAction::Reset,
                );
            }
            page.update_add_button();
        });

        *self.state.install_dialog.borrow_mut() = Some(dialog.clone());
        *self.state.install_dialog_content.borrow_mut() = Some(content_box);
        self.render_install_dialog_content();
        dialog.present(Some(&self.overlay));
        self.update_add_button();
    }

    pub fn open_install_dialog_confirmed(
        &self,
        spec: &str,
        title: &str,
        version: &str,
        registry: Option<String>,
    ) {
        if is_install_pending(self.state.install_model.borrow().phase) {
            self.open_install_dialog();
            return;
        }
        {
            let mut model = self.state.install_model.borrow_mut();
            model.spec = spec.to_string();
            model.subject = Some(InstallSubjectInfo {
                spec: spec.to_string(),
                kind: "registry".to_string(),
                name: Some(title.to_string()),
                version: Some(version.to_string()),
                description: None,
                bundle: Some(true),
                registry,
                host: None,
                bundled: false,
            });
            model.phase = InstallPhase::Confirmed;
            model.input_error = None;
            model.market_title = Some(title.to_string());
        }
        self.open_install_dialog();
    }

    fn submit_spec_inspect(&self, spec: &str) {
        let trimmed = spec.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        self.state.install_model.borrow_mut().market_title = None;
        {
            let packages = self.state.packages.borrow();
            if packages.iter().any(|p| p.name == trimmed && p.installed) {
                self.state.install_model.borrow_mut().input_error =
                    Some("该插件已安装。如需升级，请卸载后重新安装".to_string());
                self.render_install_dialog_content();
                return;
            }
            if let Some(pkg) = packages
                .iter()
                .find(|p| p.name == trimmed && p.official && !p.installed)
            {
                let subject = InstallSubjectInfo {
                    spec: trimmed.clone(),
                    kind: "registry".to_string(),
                    name: Some(pkg.name.clone()),
                    version: pkg.version.clone(),
                    description: pkg.description.clone(),
                    bundle: Some(true),
                    registry: None,
                    host: None,
                    bundled: true,
                };
                reduce_install(
                    &mut self.state.install_model.borrow_mut(),
                    InstallAction::InspectBundled { subject },
                );
                self.render_install_dialog_content();
                return;
            }
        }
        let choice = self.state.install_model.borrow().registry_choice.clone();
        let registry_url = {
            let reg_opt = self.state.registries.borrow();
            let empty = PluginRegistries {
                registry: None,
                fallback_registries: vec![],
                resolved: None,
            };
            choice
                .resolved_url(reg_opt.as_ref().unwrap_or(&empty))
                .map(str::to_string)
        };
        reduce_install(
            &mut self.state.install_model.borrow_mut(),
            InstallAction::StartCheck {
                spec: trimmed.clone(),
                choice,
            },
        );
        self.render_install_dialog_content();
        self.update_add_button();
        self.with_controller(|ctrl| ctrl.plugin_inspect(&trimmed, registry_url));
    }

    fn install_registry_url(&self) -> Option<String> {
        let model = self.state.install_model.borrow();
        if model.market_title.is_some() {
            return model
                .subject
                .as_ref()
                .and_then(|subject| subject.registry.clone());
        }
        let reg_opt = self.state.registries.borrow();
        let empty = PluginRegistries {
            registry: None,
            fallback_registries: vec![],
            resolved: None,
        };
        model
            .registry_choice
            .resolved_url(reg_opt.as_ref().unwrap_or(&empty))
            .map(str::to_string)
    }

    fn adopt_market_install_request(&self, request_id: &str) {
        let title = self.state.install_model.borrow().market_title.clone();
        if let Some(title) = title {
            self.state.market_state.borrow_mut().install_request =
                Some((request_id.to_string(), title));
        }
    }

    fn start_install_bundle(&self) {
        let (spec, approved_builds) = {
            let model = self.state.install_model.borrow();
            (model.spec.clone(), model.approved_builds.clone())
        };
        if spec.is_empty() {
            return;
        }
        let request_id = format!("req-{}", gtk4::glib::uuid_string_random());
        let registry_url = self.install_registry_url();
        self.adopt_market_install_request(&request_id);
        reduce_install(
            &mut self.state.install_model.borrow_mut(),
            InstallAction::StartInstall {
                request_id: request_id.clone(),
                approved_builds: approved_builds.clone(),
            },
        );
        self.render_install_dialog_content();
        self.update_add_button();
        self.with_controller(|ctrl| {
            ctrl.plugin_install(
                &spec,
                Some(true),
                None,
                Some(request_id),
                if approved_builds.is_empty() {
                    None
                } else {
                    Some(approved_builds)
                },
                registry_url,
            );
        });
    }

    fn approve_builds_and_retry(&self) {
        let new_req_id = format!("req-{}", gtk4::glib::uuid_string_random());
        let (spec, approved_builds) = {
            let mut model = self.state.install_model.borrow_mut();
            reduce_install(
                &mut model,
                InstallAction::ApproveBuildsAndRetry {
                    request_id: new_req_id.clone(),
                },
            );
            (model.spec.clone(), model.approved_builds.clone())
        };
        let registry_url = self.install_registry_url();
        self.adopt_market_install_request(&new_req_id);
        self.render_install_dialog_content();
        self.update_add_button();
        self.with_controller(|ctrl| {
            ctrl.plugin_install(
                &spec,
                Some(true),
                None,
                Some(new_req_id),
                Some(approved_builds),
                registry_url,
            );
        });
    }

    fn build_terminal_block(&self, runs: &[InstallRunJob]) -> gtk4::Box {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(6)
            .css_classes(["dsh-install-terminal"])
            .build();
        if runs.is_empty() {
            container.append(&dim_label("正在准备运行…"));
            return container;
        }
        let mut all_lines: Vec<String> = Vec::new();
        for run in runs {
            let header_row = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Horizontal)
                .spacing(8)
                .build();
            let cmd_text = format!("$ {}", run.argv.join(" "));
            let cmd_label = gtk4::Label::builder()
                .label(&cmd_text)
                .css_classes(["dsh-install-terminal-header"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .build();
            header_row.append(&cmd_label);
            if let Some(code) = run.exit_code {
                let code_label = tag_label(
                    &format!("exit {code}"),
                    if code == 0 { "neutral" } else { "error" },
                );
                header_row.append(&code_label);
            }
            container.append(&header_row);
            all_lines.extend(run.lines.clone());
        }

        let output_open = self.state.install_output_open.get();
        if output_open || all_lines.len() <= 12 {
            for line in &all_lines {
                let lbl = gtk4::Label::builder()
                    .label(line)
                    .css_classes(["dsh-install-terminal-output"])
                    .halign(gtk4::Align::Start)
                    .wrap(true)
                    .xalign(0.0)
                    .build();
                container.append(&lbl);
            }
            if all_lines.len() > 12 {
                let toggle_btn = gtk4::Button::builder()
                    .label("收起输出")
                    .css_classes(["flat", "caption"])
                    .halign(gtk4::Align::Start)
                    .build();
                let page = self.clone();
                toggle_btn.connect_clicked(move |_| {
                    page.state.install_output_open.set(false);
                    page.render_install_dialog_content();
                });
                container.append(&toggle_btn);
            }
        } else {
            let (head, folded, tail) = fold_terminal_lines(&all_lines, 12);
            for line in &head {
                let lbl = gtk4::Label::builder()
                    .label(line)
                    .css_classes(["dsh-install-terminal-output"])
                    .halign(gtk4::Align::Start)
                    .wrap(true)
                    .xalign(0.0)
                    .build();
                container.append(&lbl);
            }
            if folded > 0 {
                let toggle_btn = gtk4::Button::builder()
                    .label(format!("… 展开其余 {folded} 行 …"))
                    .css_classes(["flat", "caption", "dsh-install-terminal-folded"])
                    .halign(gtk4::Align::Start)
                    .build();
                let page = self.clone();
                toggle_btn.connect_clicked(move |_| {
                    page.state.install_output_open.set(true);
                    page.render_install_dialog_content();
                });
                container.append(&toggle_btn);
            }
            for line in &tail {
                let lbl = gtk4::Label::builder()
                    .label(line)
                    .css_classes(["dsh-install-terminal-output"])
                    .halign(gtk4::Align::Start)
                    .wrap(true)
                    .xalign(0.0)
                    .build();
                container.append(&lbl);
            }
        }

        let copy_btn = gtk4::Button::builder()
            .label("复制输出")
            .css_classes(["flat", "caption"])
            .halign(gtk4::Align::End)
            .build();
        let full_text = all_lines.join("\n");
        let page = self.clone();
        copy_btn.connect_clicked(move |_| {
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&full_text);
                page.toast("已复制输出到剪贴板");
            }
        });
        container.append(&copy_btn);
        container
    }

    fn render_install_dialog_content(&self) {
        let content_box = self.state.install_dialog_content.borrow().clone();
        let Some(content) = content_box else {
            return;
        };
        clear_children(&content);
        let phase = self.state.install_model.borrow().phase;
        match phase {
            InstallPhase::Idle => self.render_install_spec_screen(&content),
            InstallPhase::Checking => self.render_install_checking_screen(&content),
            InstallPhase::Confirmed => self.render_install_confirmed_screen(&content),
            InstallPhase::Running | InstallPhase::Cancelling | InstallPhase::Applying => {
                self.render_install_running_screen(&content, phase);
            }
            InstallPhase::Unconfirmed => self.render_install_unconfirmed_screen(&content),
            InstallPhase::Failed => self.render_install_failed_screen(&content),
            InstallPhase::Done => self.render_install_done_screen(&content),
        }
    }

    fn render_install_spec_screen(&self, content: &gtk4::Box) {
        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        header.append(
            &gtk4::Label::builder()
                .label("添加第三方插件")
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        header.append(&dim_label(
            "支持 npm 包名、Git 仓库地址、.tgz 直链以及本机绝对路径",
        ));
        content.append(&header);

        let group = adw::PreferencesGroup::new();
        let entry_row = adw::EntryRow::builder().title("包名或地址").build();
        entry_row.set_text(&self.state.install_model.borrow().spec);
        let page = self.clone();
        entry_row.connect_entry_activated(move |row| {
            page.submit_spec_inspect(&row.text());
        });
        let page = self.clone();
        entry_row.connect_changed(move |row| {
            page.state.install_model.borrow_mut().spec = row.text().to_string();
        });
        group.add(&entry_row);
        if let Some(ref err) = self.state.install_model.borrow().input_error {
            let err_lbl = gtk4::Label::builder()
                .label(err)
                .css_classes(["error", "caption"])
                .wrap(true)
                .xalign(0.0)
                .halign(gtk4::Align::Start)
                .build();
            group.add(&err_lbl);
        }
        content.append(&group);

        let guide = adw::ExpanderRow::builder()
            .title("插件安装引导和示例")
            .subtitle("填入插件 npm 包名")
            .build();
        let ex_row = adw::ActionRow::builder()
            .title("dsh-plugin-whale-pet")
            .subtitle("社区插件示例")
            .build();
        let fill_btn = gtk4::Button::builder()
            .label("填入示例")
            .css_classes(["flat"])
            .valign(gtk4::Align::Center)
            .build();
        let entry_row_clone = entry_row.clone();
        fill_btn.connect_clicked(move |_| {
            entry_row_clone.set_text("dsh-plugin-whale-pet");
        });
        ex_row.add_suffix(&fill_btn);
        guide.add_row(&ex_row);
        let hint_lbl = dim_label(
            "插件包名即 npm 包名（如 dsh-xxx 或 @作者/插件名），社区插件的 README 安装命令中 dsh plugin add 或 pnpm add 之后的部分。",
        );
        guide.add_row(&adw::ActionRow::builder().child(&hint_lbl).build());
        let spec_text = self.state.install_model.borrow().spec.clone();
        if spec_text.contains("github.com") || spec_text.contains("example.com") {
            let tmpl_hint = dim_label("提示：请将模板地址替换为具体的仓库或分支");
            guide.add_row(&adw::ActionRow::builder().child(&tmpl_hint).build());
        }
        content.append(&guide);

        let reg_expander = adw::ExpanderRow::builder().title("安装源").build();
        let current_choice = self.state.install_model.borrow().registry_choice.clone();
        reg_expander.set_subtitle(&current_choice.display_name());

        let default_row = adw::ActionRow::builder().title("默认安装源").build();
        if let Some(reg) = self.state.registries.borrow().as_ref() {
            if let Some(ref res) = reg.resolved {
                default_row.set_subtitle(res);
            }
        }
        if current_choice == RegistryChoice::Default {
            default_row.add_suffix(&dot_widget(DotState::Done));
        }
        let page = self.clone();
        default_row.set_activatable(true);
        default_row.connect_activated(move |_| {
            page.state.install_model.borrow_mut().registry_choice = RegistryChoice::Default;
            page.render_install_dialog_content();
        });
        reg_expander.add_row(&default_row);

        if let Some(reg) = self.state.registries.borrow().as_ref() {
            if let Some(ref primary) = reg.registry {
                let row = adw::ActionRow::builder().title(primary).build();
                if current_choice == RegistryChoice::Offered(primary.clone()) {
                    row.add_suffix(&dot_widget(DotState::Done));
                }
                let page = self.clone();
                let prim = primary.clone();
                row.set_activatable(true);
                row.connect_activated(move |_| {
                    page.state.install_model.borrow_mut().registry_choice =
                        RegistryChoice::Offered(prim.clone());
                    page.render_install_dialog_content();
                });
                reg_expander.add_row(&row);
            }
            for fb in &reg.fallback_registries {
                let row = adw::ActionRow::builder().title(fb).build();
                if current_choice == RegistryChoice::Offered(fb.clone()) {
                    row.add_suffix(&dot_widget(DotState::Done));
                }
                let page = self.clone();
                let fb_clone = fb.clone();
                row.set_activatable(true);
                row.connect_activated(move |_| {
                    page.state.install_model.borrow_mut().registry_choice =
                        RegistryChoice::Offered(fb_clone.clone());
                    page.render_install_dialog_content();
                });
                reg_expander.add_row(&row);
            }
        }

        let custom_entry = adw::EntryRow::builder()
            .title("自定义地址")
            .text(match &current_choice {
                RegistryChoice::Custom(url) => url,
                _ => "",
            })
            .build();
        let page = self.clone();
        custom_entry.connect_changed(move |row| {
            let text = row.text().to_string();
            if let Ok(url) = validate_custom_registry_url(&text) {
                page.state.install_model.borrow_mut().registry_choice =
                    RegistryChoice::Custom(url);
            }
        });
        reg_expander.add_row(&custom_entry);
        content.append(&reg_expander);

        let warn_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        warn_box.append(
            &gtk4::Image::builder()
                .icon_name("dialog-warning-symbolic")
                .valign(gtk4::Align::Center)
                .build(),
        );
        warn_box.append(&dim_label(
            "请确认插件来源可信。插件在本机以你的权限运行，来源不明的插件可能损坏 DeepSeek Harness，或读取和泄露你的数据。",
        ));
        content.append(&warn_box);

        let install_btn = gtk4::Button::builder()
            .label("检查并安装")
            .css_classes(["suggested-action"])
            .hexpand(true)
            .build();
        let page = self.clone();
        install_btn.connect_clicked(move |_| {
            page.submit_spec_inspect(&entry_row.text());
        });
        content.append(&install_btn);
    }

    fn render_install_checking_screen(&self, content: &gtk4::Box) {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .vexpand(true)
            .build();
        box_.append(
            &gtk4::Spinner::builder()
                .spinning(true)
                .width_request(36)
                .height_request(36)
                .build(),
        );
        box_.append(
            &gtk4::Label::builder()
                .label("正在检查插件规格…")
                .css_classes(["title-4"])
                .build(),
        );
        content.append(&box_);
    }

    fn render_install_confirmed_screen(&self, content: &gtk4::Box) {
        let model = self.state.install_model.borrow();
        let subject = model.subject.clone();
        drop(model);
        let Some(subject) = subject else {
            return;
        };

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        header.append(
            &gtk4::Label::builder()
                .label("确认插件信息")
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        content.append(&header);

        let group = adw::PreferencesGroup::new();
        group.add(
            &adw::ActionRow::builder()
                .title("名称")
                .subtitle(subject.name.as_deref().unwrap_or(&subject.spec))
                .build(),
        );
        if let Some(ref ver) = subject.version {
            group.add(
                &adw::ActionRow::builder()
                    .title("版本")
                    .subtitle(format!("v{ver}"))
                    .build(),
            );
        }
        if let Some(ref desc) = subject.description {
            group.add(
                &adw::ActionRow::builder()
                    .title("描述")
                    .subtitle(desc)
                    .build(),
            );
        }
        let kind_str = match subject.kind.as_str() {
            "registry" => "npm 包",
            "git" => "Git 仓库",
            "path" => "本地目录",
            "tarball" => "压缩包",
            other => other,
        };
        group.add(
            &adw::ActionRow::builder()
                .title("来源类型")
                .subtitle(kind_str)
                .build(),
        );
        content.append(&group);

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let back_btn = gtk4::Button::builder().label("返回修改").css_classes(["flat"]).build();
        let page = self.clone();
        back_btn.connect_clicked(move |_| {
            reduce_install(
                &mut page.state.install_model.borrow_mut(),
                InstallAction::Reset,
            );
            page.render_install_dialog_content();
        });
        buttons.append(&back_btn);

        let start_btn = gtk4::Button::builder()
            .label("开始安装")
            .css_classes(["suggested-action"])
            .build();
        let page = self.clone();
        start_btn.connect_clicked(move |_| {
            page.start_install_bundle();
        });
        buttons.append(&start_btn);
        content.append(&buttons);
    }

    fn render_install_running_screen(&self, content: &gtk4::Box, phase: InstallPhase) {
        let model = self.state.install_model.borrow();
        let runs = model.runs.clone();
        let req_id = model.request_id.clone().unwrap_or_default();
        let spec = model.spec.clone();
        drop(model);

        let top = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .build();
        top.append(
            &gtk4::Spinner::builder()
                .spinning(true)
                .width_request(32)
                .height_request(32)
                .build(),
        );
        let phase_text = match phase {
            InstallPhase::Cancelling => "正在停止安装…",
            InstallPhase::Applying => "正在应用配置，请稍候…",
            _ => "正在安装插件…",
        };
        top.append(
            &gtk4::Label::builder()
                .label(format!("{phase_text} ({spec})"))
                .css_classes(["title-4"])
                .build(),
        );
        content.append(&top);

        content.append(&self.build_terminal_block(&runs));

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let cancel_btn = gtk4::Button::builder()
            .label(if phase == InstallPhase::Cancelling {
                "正在停止安装…"
            } else if phase == InstallPhase::Applying {
                "正在应用配置…"
            } else {
                "取消安装"
            })
            .sensitive(phase == InstallPhase::Running)
            .build();
        let page = self.clone();
        cancel_btn.connect_clicked(move |_| {
            reduce_install(
                &mut page.state.install_model.borrow_mut(),
                InstallAction::RequestCancel,
            );
            page.render_install_dialog_content();
            page.with_controller(|ctrl| ctrl.plugin_cancel_install(&req_id));
        });
        buttons.append(&cancel_btn);
        content.append(&buttons);
    }

    fn render_install_unconfirmed_screen(&self, content: &gtk4::Box) {
        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .halign(gtk4::Align::Center)
            .build();
        box_.append(
            &gtk4::Image::builder()
                .icon_name("dialog-warning-symbolic")
                .pixel_size(48)
                .build(),
        );
        box_.append(
            &gtk4::Label::builder()
                .label("安装状态尚未确认")
                .css_classes(["title-3"])
                .build(),
        );
        box_.append(&dim_label("与 Host 通信中断，请核对安装状态。"));
        content.append(&box_);

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let close_btn = gtk4::Button::builder().label("关闭").css_classes(["flat"]).build();
        let page = self.clone();
        close_btn.connect_clicked(move |_| {
            if let Some(dialog) = page.state.install_dialog.borrow().as_ref() {
                dialog.close();
            }
        });
        buttons.append(&close_btn);

        let wait_btn = gtk4::Button::builder()
            .label("核对安装状态")
            .css_classes(["suggested-action"])
            .build();
        let page = self.clone();
        let req_id = self
            .state
            .install_model
            .borrow()
            .request_id
            .clone()
            .unwrap_or_default();
        wait_btn.connect_clicked(move |_| {
            page.with_controller(|ctrl| ctrl.plugin_wait_install(&req_id));
        });
        buttons.append(&wait_btn);
        content.append(&buttons);
    }

    fn render_install_failed_screen(&self, content: &gtk4::Box) {
        let model = self.state.install_model.borrow();
        let failure = model.failure.clone();
        let runs = model.runs.clone();
        drop(model);

        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .build();
        head.append(
            &gtk4::Image::builder()
                .icon_name("dialog-error-symbolic")
                .pixel_size(48)
                .css_classes(["error"])
                .build(),
        );
        head.append(
            &gtk4::Label::builder()
                .label("安装失败")
                .css_classes(["title-3"])
                .build(),
        );
        if let Some(ref f) = failure {
            head.append(
                &gtk4::Label::builder()
                    .label(&f.reason)
                    .wrap(true)
                    .xalign(0.5)
                    .css_classes(["error"])
                    .build(),
            );
        }
        content.append(&head);

        if let Some(ref f) = failure {
            if !f.pending_builds.is_empty() {
                let p_group = adw::PreferencesGroup::builder()
                    .title("待允许的安装脚本")
                    .description("以下依赖包含安装脚本，需要你允许后才能继续安装：")
                    .build();
                for pkg in &f.pending_builds {
                    p_group.add(&adw::ActionRow::builder().title(pkg).build());
                }
                content.append(&p_group);

                content.append(&dim_label(
                    "允许这些脚本运行可能执行任意代码。请确认插件来源可信。",
                ));
                let retry_btn = gtk4::Button::builder()
                    .label("允许这些脚本并重试")
                    .css_classes(["suggested-action"])
                    .build();
                let page = self.clone();
                retry_btn.connect_clicked(move |_| page.approve_builds_and_retry());
                content.append(&retry_btn);
            }
        }

        content.append(&self.build_terminal_block(&runs));

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let back_btn = gtk4::Button::builder().label("返回修改").css_classes(["flat"]).build();
        let page = self.clone();
        back_btn.connect_clicked(move |_| {
            reduce_install(
                &mut page.state.install_model.borrow_mut(),
                InstallAction::Reset,
            );
            page.render_install_dialog_content();
        });
        buttons.append(&back_btn);

        let close_btn = gtk4::Button::builder().label("关闭").build();
        let page = self.clone();
        close_btn.connect_clicked(move |_| {
            if let Some(dialog) = page.state.install_dialog.borrow().as_ref() {
                dialog.close();
            }
        });
        buttons.append(&close_btn);
        content.append(&buttons);
    }

    fn render_install_done_screen(&self, content: &gtk4::Box) {
        let model = self.state.install_model.borrow();
        let name = model.installed_name.clone().unwrap_or_else(|| model.spec.clone());
        let version = model.installed_version.clone().unwrap_or_default();
        let restart_req = model.restart_required;
        let restart_reason = model.restart_reason.clone();
        drop(model);

        let box_ = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .build();
        box_.append(
            &gtk4::Image::builder()
                .icon_name("emblem-ok-symbolic")
                .pixel_size(48)
                .css_classes(["success"])
                .build(),
        );
        box_.append(
            &gtk4::Label::builder()
                .label(format!("已安装 {name} v{version}"))
                .css_classes(["title-3"])
                .build(),
        );
        content.append(&box_);

        if restart_req {
            let banner = adw::Banner::builder()
                .title("请重启 DSH 使更改生效。")
                .button_label("立即重启")
                .revealed(true)
                .build();
            let page = self.clone();
            banner.connect_button_clicked(move |_| {
                page.with_controller(|ctrl| ctrl.restart_supervisor());
            });
            content.append(&banner);
            let reason_text = restart_reason_text(restart_reason.as_deref());
            if !reason_text.is_empty() {
                content.append(&dim_label(reason_text));
            }
        }

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let is_enabled = self
            .state
            .packages
            .borrow()
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.enabled)
            .unwrap_or(false);
        if !is_enabled {
            let enable_btn = gtk4::Button::builder()
                .label("立即启用")
                .css_classes(["suggested-action"])
                .build();
            let page = self.clone();
            let name_c = name.clone();
            enable_btn.connect_clicked(move |_| {
                page.toggle_bundle(&name_c, true);
                *page.state.highlight_card.borrow_mut() = Some(name_c.clone());
                if let Some(dialog) = page.state.install_dialog.borrow().as_ref() {
                    dialog.close();
                }
            });
            buttons.append(&enable_btn);
        }

        let done_btn = gtk4::Button::builder().label("完成").build();
        let page = self.clone();
        done_btn.connect_clicked(move |_| {
            if let Some(dialog) = page.state.install_dialog.borrow().as_ref() {
                dialog.close();
            }
        });
        buttons.append(&done_btn);
        content.append(&buttons);
    }

    pub fn open_update_dialog(
        &self,
        name: &str,
        from_version: &str,
        to_version: &str,
        registry: Option<String>,
    ) {
        *self.state.update_pkg_info.borrow_mut() = Some((
            name.to_string(),
            from_version.to_string(),
            to_version.to_string(),
            registry,
        ));
        self.state.update_running.set(false);
        *self.state.update_request_id.borrow_mut() = None;
        *self.state.update_result.borrow_mut() = None;
        self.state.update_steps.borrow_mut().clear();

        let dialog = adw::Dialog::builder()
            .title(format!("更新 {name}"))
            .content_width(480)
            .content_height(420)
            .build();
        let content_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(14)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        let clamp = adw::Clamp::builder()
            .maximum_size(440)
            .child(&content_box)
            .build();
        let scroll = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&clamp)
            .build();
        let view = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        view.add_top_bar(&header);
        view.set_content(Some(&scroll));
        dialog.set_child(Some(&view));

        let page = self.clone();
        dialog.connect_closed(move |_| {
            page.state.update_dialog.borrow_mut().take();
            page.state.update_dialog_content.borrow_mut().take();
        });

        *self.state.update_dialog.borrow_mut() = Some(dialog.clone());
        *self.state.update_dialog_content.borrow_mut() = Some(content_box);
        self.render_update_dialog_content();
        dialog.present(Some(&self.overlay));
    }

    fn render_update_dialog_content(&self) {
        let content_box = self.state.update_dialog_content.borrow().clone();
        let Some(content) = content_box else {
            return;
        };
        clear_children(&content);
        let pkg_info = self.state.update_pkg_info.borrow().clone();
        let Some((name, from_version, to_version, registry)) = pkg_info else {
            return;
        };

        if self.state.update_running.get() {
            let header = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(8)
                .halign(gtk4::Align::Center)
                .build();
            header.append(
                &gtk4::Spinner::builder()
                    .spinning(true)
                    .width_request(32)
                    .height_request(32)
                    .build(),
            );
            header.append(
                &gtk4::Label::builder()
                    .label(format!("正在更新 {name}…"))
                    .css_classes(["title-3"])
                    .build(),
            );
            content.append(&header);

            let steps_group = adw::PreferencesGroup::new();
            let announced = self.state.update_steps.borrow().clone();
            let ordered = ordered_update_steps(&announced, None, false);
            for step in ordered {
                let row = adw::ActionRow::new();
                let label = match step.as_str() {
                    "checking" => format!("检查 {to_version}"),
                    "removing" => format!("停止并移除 {from_version}"),
                    "installing" => format!("安装 {to_version}"),
                    "restoring" => format!("重新安装 {from_version}"),
                    _ => step.clone(),
                };
                row.set_title(&label);
                let is_last = announced.last().as_deref() == Some(&step);
                if is_last {
                    row.add_suffix(&gtk4::Spinner::builder().spinning(true).build());
                } else {
                    row.add_prefix(&dot_widget(DotState::Done));
                }
                steps_group.add(&row);
            }
            content.append(&steps_group);
            return;
        }

        if let Some(ref update_res) = *self.state.update_result.borrow() {
            let outcome = evaluate_update_outcome(Some(update_res), None);
            let outcome_box = gtk4::Box::builder()
                .orientation(gtk4::Orientation::Vertical)
                .spacing(12)
                .halign(gtk4::Align::Center)
                .build();
            match outcome {
                UpdateOutcomeKind::Done { version } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("emblem-ok-symbolic")
                            .pixel_size(48)
                            .css_classes(["success"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("已更新到 v{version}"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label("新版本已经开始运行。"));
                }
                UpdateOutcomeKind::RestartRequired { version, reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-warning-symbolic")
                            .pixel_size(48)
                            .css_classes(["warning"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("已安装 v{version}，重启后生效"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    let restart_text = restart_reason_text(reason.as_deref());
                    if !restart_text.is_empty() {
                        outcome_box.append(&dim_label(restart_text));
                    }
                    let restart_btn = gtk4::Button::builder()
                        .label("立即重启 DSH")
                        .css_classes(["suggested-action"])
                        .build();
                    let page = self.clone();
                    restart_btn.connect_clicked(move |_| {
                        page.with_controller(|ctrl| ctrl.restart_supervisor())
                    });
                    outcome_box.append(&restart_btn);
                }
                UpdateOutcomeKind::Refused { version, reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-error-symbolic")
                            .pixel_size(48)
                            .css_classes(["error"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("无法更新到 v{version}"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&reason));
                }
                UpdateOutcomeKind::RemoveFailed { version, reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-error-symbolic")
                            .pixel_size(48)
                            .css_classes(["error"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("无法移除 v{version}"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&reason));
                }
                UpdateOutcomeKind::Restored { from, to, reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-error-symbolic")
                            .pixel_size(48)
                            .css_classes(["error"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("更新失败，已恢复到 v{from}"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&format!(
                        "v{to} 安装失败，插件已重新安装 v{from}：{reason}"
                    )));
                }
                UpdateOutcomeKind::Kept { from, to, reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-error-symbolic")
                            .pixel_size(48)
                            .css_classes(["error"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label(format!("无法更新到 v{to}"))
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&format!(
                        "v{from} 未被移除，仍在使用：{reason}"
                    )));
                }
                UpdateOutcomeKind::Broken { from, to } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-error-symbolic")
                            .pixel_size(48)
                            .css_classes(["error"])
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label("更新失败，插件未安装")
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&format!(
                        "v{to} 安装失败，重新安装 v{from} 也未成功。请从“添加插件”重新安装 {name}。"
                    )));
                }
                UpdateOutcomeKind::Lost { reason } => {
                    outcome_box.append(
                        &gtk4::Image::builder()
                            .icon_name("dialog-warning-symbolic")
                            .pixel_size(48)
                            .build(),
                    );
                    outcome_box.append(
                        &gtk4::Label::builder()
                            .label("无法确认更新结果")
                            .css_classes(["title-3"])
                            .build(),
                    );
                    outcome_box.append(&dim_label(&reason));
                }
            }
            content.append(&outcome_box);
            let finish_btn = gtk4::Button::builder()
                .label("完成")
                .css_classes(["suggested-action"])
                .build();
            let page = self.clone();
            finish_btn.connect_clicked(move |_| {
                if let Some(dialog) = page.state.update_dialog.borrow().as_ref() {
                    dialog.close();
                }
            });
            content.append(&finish_btn);
            return;
        }

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .build();
        header.append(
            &gtk4::Label::builder()
                .label(format!("更新 {name}"))
                .css_classes(["title-3"])
                .halign(gtk4::Align::Start)
                .build(),
        );
        header.append(&dim_label(&format!(
            "将从 v{from_version} 更新到 v{to_version}，期间插件可能短暂停止。安装失败时会保留或重新安装 v{from_version}。"
        )));
        content.append(&header);

        let group = adw::PreferencesGroup::new();
        group.add(
            &adw::ActionRow::builder()
                .title("当前版本")
                .subtitle(format!("v{from_version}"))
                .build(),
        );
        group.add(
            &adw::ActionRow::builder()
                .title("目标版本")
                .subtitle(format!("v{to_version}"))
                .build(),
        );
        let reg_name = registry.as_deref().unwrap_or("默认安装源");
        group.add(
            &adw::ActionRow::builder()
                .title("安装来源")
                .subtitle(reg_name)
                .build(),
        );
        content.append(&group);

        let buttons = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::End)
            .build();
        let cancel_btn = gtk4::Button::builder().label("取消").css_classes(["flat"]).build();
        let page = self.clone();
        cancel_btn.connect_clicked(move |_| {
            if let Some(dialog) = page.state.update_dialog.borrow().as_ref() {
                dialog.close();
            }
        });
        buttons.append(&cancel_btn);

        let confirm_btn = gtk4::Button::builder()
            .label("确认更新")
            .css_classes(["suggested-action"])
            .build();
        let page = self.clone();
        let name_c = name.clone();
        let from_c = from_version.clone();
        let to_c = to_version.clone();
        let reg_c = registry.clone();
        confirm_btn.connect_clicked(move |_| {
            page.start_bundle_update(&name_c, &from_c, &to_c, reg_c.clone());
        });
        buttons.append(&confirm_btn);
        content.append(&buttons);
    }

    fn start_bundle_update(
        &self,
        name: &str,
        _from_version: &str,
        to_version: &str,
        registry: Option<String>,
    ) {
        let request_id = format!("req-{}", gtk4::glib::uuid_string_random());
        *self.state.update_request_id.borrow_mut() = Some(request_id.clone());
        self.state.update_running.set(true);
        self.state.update_steps.borrow_mut().clear();
        *self.state.update_result.borrow_mut() = None;
        self.render_update_dialog_content();

        let name_c = name.to_string();
        let to_c = to_version.to_string();
        self.with_controller(move |ctrl| {
            ctrl.plugin_update(&name_c, &to_c, Some(request_id), registry);
        });
    }
}

impl Default for PluginsPage {
    fn default() -> Self {
        Self::new()
    }
}
