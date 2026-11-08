use crate::schedule_model::*;
use crate::ui::controller::SessionController;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub mod model {
    use crate::connection::ConnectionError;
    use crate::protocol::WorkspaceView;
    use crate::schedule_model::*;
    use serde_json::Value;

    pub const HISTORY_PAGE_LIMIT: u32 = 20;
    pub const CLOCK_PERIOD_SECS: u64 = 15;
    const TITLE_MAX_UTF16: usize = 120;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum StatusFilter {
        Active,
        Paused,
        Ended,
        All,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EmptyKey {
        NoMatches,
        Empty,
        EmptyActive,
        EmptyPaused,
        EmptyInactive,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ScheduleFailure {
        Conflict,
        Missing,
        NotFuture,
        InvalidRule,
        ExecutionPending,
        SaveError,
        RunError,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ScheduleHistoryMiss {
        NotFound,
        CursorInvalid,
        Failed,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ScheduleMutationOp {
        Create,
        Edit { id: String },
        Run { id: String },
        Delete { id: String },
        ClearEnded,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LinkedState {
        Available,
        Archived,
        Unavailable,
    }

    const EN_MONTHS_SHORT: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    pub fn fill(template: &str, params: &[(&str, String)]) -> String {
        let mut out = template.to_string();
        for (key, value) in params {
            out = out.replace(&format!("{{{key}}}"), value);
        }
        out
    }

    pub fn copy(locale: Locale, key: &str) -> &'static str {
        match (locale, key) {
            (Locale::En, "new.action") => "New",
            (Locale::Zh, "new.action") => "新建",
            (Locale::En, "host.chatCreate") => "Create in a session",
            (Locale::Zh, "host.chatCreate") => "在会话中创建",
            (Locale::En, "host.manualCreate") => "Create manually",
            (Locale::Zh, "host.manualCreate") => "手动创建",
            (Locale::En, "host.createPrompt") => {
                "Please create an automation task based on the requirements."
            }
            (Locale::Zh, "host.createPrompt") => "请根据需求创建一个自动化任务。",
            (Locale::En, "host.creating") => "Creating…",
            (Locale::Zh, "host.creating") => "创建中…",
            (Locale::En, "host.notFuture") => {
                "Choose a future schedule; edit the task timing before resuming"
            }
            (Locale::Zh, "host.notFuture") => "请选择未来的运行时间；恢复前请先修改任务时间",
            (Locale::En, "host.invalidRule") => "Check the rule and time zone",
            (Locale::Zh, "host.invalidRule") => "请检查规则内容和时区",
            (Locale::En, "host.target") => "Run in",
            (Locale::Zh, "host.target") => "执行位置",
            (Locale::En, "host.targetNewOnce") => "Create a session and reuse it",
            (Locale::Zh, "host.targetNewOnce") => "新建会话并复用",
            (Locale::En, "host.targetNew") => "New session for every run",
            (Locale::Zh, "host.targetNew") => "每次新建会话",
            (Locale::En, "host.targetNewOnceHelp") => {
                "Create it on the first run and keep using it"
            }
            (Locale::Zh, "host.targetNewOnceHelp") => "首次运行时创建，之后沿用这个会话",
            (Locale::En, "host.targetNewHelp") => "Create a separate session for each run",
            (Locale::Zh, "host.targetNewHelp") => "每次运行都创建独立会话",
            (Locale::En, "host.latestConversation") => "Latest run session",
            (Locale::Zh, "host.latestConversation") => "最近运行会话",
            (Locale::En, "host.currentConversation") => "Current linked session",
            (Locale::Zh, "host.currentConversation") => "当前关联会话",
            (Locale::En, "host.project") => "Linked workspace",
            (Locale::Zh, "host.project") => "关联工作区",
            (Locale::En, "host.ungrouped") => "Ungrouped",
            (Locale::Zh, "host.ungrouped") => "未分组",
            (Locale::En, "host.interval") => "Interval",
            (Locale::Zh, "host.interval") => "重复间隔",
            (Locale::En, "host.monthDay") => "Day of the month",
            (Locale::Zh, "host.monthDay") => "每月日期",
            (Locale::En, "host.customRule") => "Customize",
            (Locale::Zh, "host.customRule") => "自定义",
            (Locale::En, "host.newSessionInterval") => "Minimum run interval: {interval}",
            (Locale::Zh, "host.newSessionInterval") => "运行间隔至少 {interval}",
            (Locale::En, "host.limitsError") => {
                "Could not load the run interval limit. Try again before saving."
            }
            (Locale::Zh, "host.limitsError") => "无法读取运行间隔限制，请在保存前重试",
            (Locale::En, "host.pause") => "Pause",
            (Locale::Zh, "host.pause") => "暂停任务",
            (Locale::En, "host.resume") => "Resume",
            (Locale::Zh, "host.resume") => "恢复任务",
            (Locale::En, "host.run") => "Run now",
            (Locale::Zh, "host.run") => "立即执行",
            (Locale::En, "host.runAfterInterval") => {
                "The minimum run interval has not passed. Run now is available at {time}."
            }
            (Locale::Zh, "host.runAfterInterval") => "未满最小运行间隔，{time} 后可立即执行",
            (Locale::En, "host.saveError") => {
                "Could not save. Your changes are preserved; try again."
            }
            (Locale::Zh, "host.saveError") => "无法保存，已保留修改，请重试",
            (Locale::En, "host.createError") => {
                "Could not create the task. Your input is preserved; try again."
            }
            (Locale::Zh, "host.createError") => "无法创建任务，已保留填写内容，请重试",
            (Locale::En, "host.conflict") => {
                "This task changed elsewhere. Review the latest values with your edits, or discard your changes."
            }
            (Locale::Zh, "host.conflict") => {
                "任务已在其他位置更新。可保留修改并复核最新值，或放弃更改。"
            }
            (Locale::En, "host.reapply") => "Review with my edits",
            (Locale::Zh, "host.reapply") => "保留修改并复核",
            (Locale::En, "host.runError") => "Could not queue the task. Try again.",
            (Locale::Zh, "host.runError") => "无法将任务加入队列，请重试",
            (Locale::En, "host.executionPending") => {
                "This task is queued, running, or was just delivered. Try again later."
            }
            (Locale::Zh, "host.executionPending") => "此任务已排队、执行中或刚执行过，请稍后再试",
            (Locale::En, "host.executionActive") => {
                "This task is queued or running. Manage it in the session."
            }
            (Locale::Zh, "host.executionActive") => "此任务已排队或执行中，请在会话中管理任务",
            (Locale::En, "host.sessionAfterFirstRun") => "Pending creation",
            (Locale::Zh, "host.sessionAfterFirstRun") => "待创建",
            (Locale::En, "host.openDeliverySession") => "Open session",
            (Locale::Zh, "host.openDeliverySession") => "查看会话",
            (Locale::En, "list.nextPrefix") => "Next scheduled time: ",
            (Locale::Zh, "list.nextPrefix") => "下次计划时间：",
            (Locale::En, "list.loading") => "Loading tasks…",
            (Locale::Zh, "list.loading") => "正在加载任务…",
            (Locale::En, "list.error") => "Could not load tasks.",
            (Locale::Zh, "list.error") => "无法加载任务",
            (Locale::En, "list.retry") => "Retry",
            (Locale::Zh, "list.retry") => "重试",
            (Locale::En, "list.empty") => {
                "No tasks yet. Tasks created in your sessions appear here."
            }
            (Locale::Zh, "list.empty") => "还没有自动化任务，在会话中创建的任务会显示在这里",
            (Locale::En, "list.emptyActive") => "No active automation tasks",
            (Locale::Zh, "list.emptyActive") => "没有已开启的自动化任务",
            (Locale::En, "list.emptyPaused") => "No paused automation tasks",
            (Locale::Zh, "list.emptyPaused") => "没有已暂停的自动化任务",
            (Locale::En, "list.emptyInactive") => "No ended automation tasks",
            (Locale::Zh, "list.emptyInactive") => "没有已结束的自动化任务",
            (Locale::En, "list.noMatches") => "No matching tasks",
            (Locale::Zh, "list.noMatches") => "没有匹配的自动化任务",
            (Locale::En, "search.placeholder") => "Search automation tasks",
            (Locale::Zh, "search.placeholder") => "搜索自动化任务",
            (Locale::En, "empty.action") => "New automation task",
            (Locale::Zh, "empty.action") => "新建自动化任务",
            (Locale::En, "status.active") => "Enabled",
            (Locale::Zh, "status.active") => "已开启",
            (Locale::En, "status.paused") => "Paused",
            (Locale::Zh, "status.paused") => "已暂停",
            (Locale::En, "status.ended") => "Ended",
            (Locale::Zh, "status.ended") => "已结束",
            (Locale::En, "statusFilter.all") => "All",
            (Locale::Zh, "statusFilter.all") => "全部",
            (Locale::En, "detail.close") => "Close details",
            (Locale::Zh, "detail.close") => "关闭详情",
            (Locale::En, "detail.nextRun") => "Next run",
            (Locale::Zh, "detail.nextRun") => "下次运行",
            (Locale::En, "detail.rule") => "Rules",
            (Locale::Zh, "detail.rule") => "规则",
            (Locale::En, "detail.records") => "Records",
            (Locale::Zh, "detail.records") => "任务运行记录",
            (Locale::En, "detail.name") => "Automation task name",
            (Locale::Zh, "detail.name") => "自动化任务名称",
            (Locale::En, "detail.namePlaceholder") => "E.g. Daily reading digest",
            (Locale::Zh, "detail.namePlaceholder") => "例如：每日阅读摘要",
            (Locale::En, "detail.instruction") => "Automation task instruction",
            (Locale::Zh, "detail.instruction") => "自动化任务内容",
            (Locale::En, "detail.instructionPlaceholder") => {
                "Describe the task and result.\nE.g. Summarize my reading list with source links."
            }
            (Locale::Zh, "detail.instructionPlaceholder") => {
                "说明任务内容和预期结果\n例如：汇总阅读清单中的新文章，并附原文链接"
            }
            (Locale::En, "detail.session") => "Linked session",
            (Locale::Zh, "detail.session") => "关联会话",
            (Locale::En, "detail.sessionArchived") => "The linked session is archived.",
            (Locale::Zh, "detail.sessionArchived") => "关联会话已归档",
            (Locale::En, "detail.sessionUnavailable") => "The linked session is unavailable.",
            (Locale::Zh, "detail.sessionUnavailable") => "关联会话当前不可用",
            (Locale::En, "detail.missing") => "Task unavailable. It may have been deleted.",
            (Locale::Zh, "detail.missing") => "任务已不可用，可能已被删除",
            (Locale::En, "delivery.empty") => "No delivery record available",
            (Locale::Zh, "delivery.empty") => "暂无任务运行记录",
            (Locale::En, "delivery.loading") => "Loading delivery records…",
            (Locale::Zh, "delivery.loading") => "正在加载任务运行记录…",
            (Locale::En, "delivery.error") => "Could not load delivery records.",
            (Locale::Zh, "delivery.error") => "无法加载任务运行记录",
            (Locale::En, "delivery.notFound") => "This task is no longer available.",
            (Locale::Zh, "delivery.notFound") => "任务已不可用",
            (Locale::En, "delivery.retry") => "Retry",
            (Locale::Zh, "delivery.retry") => "重试",
            (Locale::En, "delivery.loadMore") => "Load more",
            (Locale::Zh, "delivery.loadMore") => "加载更多",
            (Locale::En, "delivery.expand") => "Expand",
            (Locale::Zh, "delivery.expand") => "展开",
            (Locale::En, "delivery.collapse") => "Collapse",
            (Locale::Zh, "delivery.collapse") => "收起",
            (Locale::En, "delivery.pruned") => "Earlier delivery records have been cleared",
            (Locale::Zh, "delivery.pruned") => "更早的运行记录已清理",
            (Locale::En, "delivery.retention") => "Retention rules",
            (Locale::Zh, "delivery.retention") => "保留规则",
            (Locale::En, "delivery.retentionBounds") => {
                "Each task keeps up to {records} delivery records from the last {days} days."
            }
            (Locale::Zh, "delivery.retentionBounds") => {
                "每个任务最多保留近 {days} 天内的 {records} 条运行记录。"
            }
            (Locale::En, "delivery.retentionExplanation") => {
                "When a new record is saved, older records outside these limits are cleared automatically. Clearing records does not stop the task."
            }
            (Locale::Zh, "delivery.retentionExplanation") => {
                "产生新记录时，超出范围的旧记录会自动清理。清理记录不影响任务继续运行。"
            }
            (Locale::En, "rule.title") => "Run time",
            (Locale::Zh, "rule.title") => "运行时间",
            (Locale::En, "rule.repeat") => "Repeat",
            (Locale::Zh, "rule.repeat") => "重复",
            (Locale::En, "rule.weekday") => "Weekday",
            (Locale::Zh, "rule.weekday") => "星期",
            (Locale::En, "rule.weekly") => "Weekly",
            (Locale::Zh, "rule.weekly") => "每周",
            (Locale::En, "rule.weekdays") => "Monday to Friday",
            (Locale::Zh, "rule.weekdays") => "周一至周五",
            (Locale::En, "rule.daily") => "Every day",
            (Locale::Zh, "rule.daily") => "每天",
            (Locale::En, "rule.everyHours") => "Every N hours",
            (Locale::Zh, "rule.everyHours") => "每 N 小时",
            (Locale::En, "rule.everyMinutes") => "Every N minutes",
            (Locale::Zh, "rule.everyMinutes") => "每 N 分钟",
            (Locale::En, "host.frequency.monthly") => "Every month",
            (Locale::Zh, "host.frequency.monthly") => "每月",
            (Locale::En, "rule.once") => "Once",
            (Locale::Zh, "rule.once") => "仅一次",
            (Locale::En, "host.frequency.advanced") => "Custom",
            (Locale::Zh, "host.frequency.advanced") => "自定义",
            (Locale::En, "rule.unsaved") => "Unsaved changes",
            (Locale::Zh, "rule.unsaved") => "有未保存的修改",
            (Locale::En, "rule.save") => "Save changes",
            (Locale::Zh, "rule.save") => "保存修改",
            (Locale::En, "rule.saving") => "Saving…",
            (Locale::Zh, "rule.saving") => "保存中…",
            (Locale::En, "rule.cancel") => "Discard",
            (Locale::Zh, "rule.cancel") => "放弃",
            (Locale::En, "rule.invalidTitle") => "Use a task name of 1–120 characters.",
            (Locale::Zh, "rule.invalidTitle") => "请输入1至120个字符的任务名称",
            (Locale::En, "rule.invalidInterval") => {
                "Enter a whole number of at least 1 for the interval."
            }
            (Locale::Zh, "rule.invalidInterval") => "重复间隔需为不小于1的整数",
            (Locale::En, "rule.requiredWeekday") => "Select at least one weekday.",
            (Locale::Zh, "rule.requiredWeekday") => "请至少选择一天",
            (Locale::En, "rule.invalidMonthDay") => "Enter a day of the month from 1 to 31.",
            (Locale::Zh, "rule.invalidMonthDay") => "每月日期需为1至31之间的整数",
            (Locale::En, "rule.invalidTime") => {
                "Choose a valid date and time in the selected time zone."
            }
            (Locale::Zh, "rule.invalidTime") => "请在所选时区内选择有效的日期和时间",
            (Locale::En, "rule.invalidZone") => "Choose a valid time zone.",
            (Locale::Zh, "rule.invalidZone") => "请选择有效的时区",
            (Locale::En, "rule.tooComplex") => {
                "This rule is too frequent or complex. Choose a repeat option or ask the agent to simplify it."
            }
            (Locale::Zh, "rule.tooComplex") => {
                "此规则的运行频率过高或过于复杂，请选择其他重复方式，或让智能体简化规则"
            }
            (Locale::En, "rule.invalid") => {
                "Edit the custom rule, choose another repeat option, or ask the agent to correct it."
            }
            (Locale::Zh, "rule.invalid") => {
                "请修改自定义规则、选择其他重复方式，或让智能体协助修改"
            }
            (Locale::En, "delete.action") => "Delete task",
            (Locale::Zh, "delete.action") => "删除任务",
            (Locale::En, "delete.title") => "Delete this task?",
            (Locale::Zh, "delete.title") => "删除此任务？",
            (Locale::En, "delete.description") => {
                "This stops the task from triggering and deletes it together with its saved delivery records. The original session and its messages remain; queued messages are not withdrawn."
            }
            (Locale::Zh, "delete.description") => {
                "任务将停止触发，并连同其已保存的任务运行记录一并删除。原会话及其消息仍然保留；已排队的消息不会被撤回。"
            }
            (Locale::En, "delete.confirm") => "Confirm deletion",
            (Locale::Zh, "delete.confirm") => "确认删除",
            (Locale::En, "delete.cancel") => "Cancel",
            (Locale::Zh, "delete.cancel") => "取消",
            (Locale::En, "clear.action") => "Clear all ended tasks",
            (Locale::Zh, "clear.action") => "清除全部已结束任务",
            (Locale::En, "clear.title") => "Clear ended tasks ({count})?",
            (Locale::Zh, "clear.title") => "清除 {count} 个已结束任务？",
            (Locale::En, "clear.description") => {
                "These tasks and their saved delivery records will be permanently deleted. Active and paused tasks, original sessions, and queued messages are not affected."
            }
            (Locale::Zh, "clear.description") => {
                "这些任务及其运行记录将被永久删除，已开启或已暂停的任务、原会话和已排队消息等不受影响。"
            }
            (Locale::En, "clear.confirm") => "Clear tasks",
            (Locale::Zh, "clear.confirm") => "确认清除",
            (Locale::En, "toast.deleted") => "Task deleted.",
            (Locale::Zh, "toast.deleted") => "任务已删除",
            (Locale::En, "toast.deleteFailed") => "Could not delete the task.",
            (Locale::Zh, "toast.deleteFailed") => "无法删除任务",
            (Locale::En, "toast.cleared") => "Cleared {count} ended tasks.",
            (Locale::Zh, "toast.cleared") => "已清除 {count} 个已结束任务",
            (Locale::En, "toast.clearedNone") => "No ended tasks to clear.",
            (Locale::Zh, "toast.clearedNone") => "没有可清理的已结束任务",
            (Locale::En, "toast.clearFailed") => "Could not clear tasks.",
            (Locale::Zh, "toast.clearFailed") => "清除任务失败",
            (Locale::En, "timing.date") => "Date",
            (Locale::Zh, "timing.date") => "日期",
            (Locale::En, "timing.time") => "Time",
            (Locale::Zh, "timing.time") => "时间",
            (Locale::En, "timing.zone") => "Time zone",
            (Locale::Zh, "timing.zone") => "时区",
            (Locale::En, "timing.zoneSearch") => "Search by UTC offset, IANA ID, or city",
            (Locale::Zh, "timing.zoneSearch") => "搜索 UTC 偏移、IANA 标识或城市",
            (Locale::En, "weekday.1") => "Mon",
            (Locale::Zh, "weekday.1") => "一",
            (Locale::En, "weekday.2") => "Tue",
            (Locale::Zh, "weekday.2") => "二",
            (Locale::En, "weekday.3") => "Wed",
            (Locale::Zh, "weekday.3") => "三",
            (Locale::En, "weekday.4") => "Thu",
            (Locale::Zh, "weekday.4") => "四",
            (Locale::En, "weekday.5") => "Fri",
            (Locale::Zh, "weekday.5") => "五",
            (Locale::En, "weekday.6") => "Sat",
            (Locale::Zh, "weekday.6") => "六",
            (Locale::En, "weekday.7") => "Sun",
            (Locale::Zh, "weekday.7") => "日",
            _ => "",
        }
    }

    pub fn status_text(status: ScheduleStatus, locale: Locale) -> &'static str {
        match status {
            ScheduleStatus::Active => copy(locale, "status.active"),
            ScheduleStatus::Paused => copy(locale, "status.paused"),
            ScheduleStatus::Ended => copy(locale, "status.ended"),
        }
    }

    fn epoch_ms(text: &str) -> i64 {
        text.parse::<jiff::Timestamp>()
            .map(|ts| ts.as_millisecond())
            .unwrap_or(i64::MAX)
    }

    pub fn filter_entries(
        entries: &[ScheduleCatalogEntry],
        filter: StatusFilter,
        search: &str,
    ) -> Vec<ScheduleCatalogEntry> {
        let query = search.trim().to_lowercase();
        let mut rows: Vec<(i64, ScheduleCatalogEntry)> = entries
            .iter()
            .filter(|record| {
                let status_ok = match filter {
                    StatusFilter::All => true,
                    StatusFilter::Active => record.status == ScheduleStatus::Active,
                    StatusFilter::Paused => record.status == ScheduleStatus::Paused,
                    StatusFilter::Ended => record.status == ScheduleStatus::Ended,
                };
                let text_ok = query.is_empty()
                    || record.prompt.to_lowercase().contains(&query)
                    || task_session_id(record)
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query)
                    || record.title.to_lowercase().contains(&query);
                status_ok && text_ok
            })
            .map(|record| (epoch_ms(&task_next_run_at(record)), record.clone()))
            .collect();
        rows.sort_by_key(|(key, _)| *key);
        rows.into_iter().map(|(_, record)| record).collect()
    }

    pub fn ended_count(entries: &[ScheduleCatalogEntry]) -> usize {
        entries
            .iter()
            .filter(|record| record.status == ScheduleStatus::Ended)
            .count()
    }

    pub fn clear_ended_visible(filter: StatusFilter, ended: usize) -> bool {
        filter != StatusFilter::Active && ended > 0
    }

    pub fn empty_key(search: &str, filter: StatusFilter) -> EmptyKey {
        if !search.trim().is_empty() {
            return EmptyKey::NoMatches;
        }
        match filter {
            StatusFilter::Active => EmptyKey::EmptyActive,
            StatusFilter::Paused => EmptyKey::EmptyPaused,
            StatusFilter::Ended => EmptyKey::EmptyInactive,
            StatusFilter::All => EmptyKey::Empty,
        }
    }

    pub fn empty_text(key: EmptyKey, locale: Locale) -> &'static str {
        match key {
            EmptyKey::NoMatches => copy(locale, "list.noMatches"),
            EmptyKey::Empty => copy(locale, "list.empty"),
            EmptyKey::EmptyActive => copy(locale, "list.emptyActive"),
            EmptyKey::EmptyPaused => copy(locale, "list.emptyPaused"),
            EmptyKey::EmptyInactive => copy(locale, "list.emptyInactive"),
        }
    }

    pub fn row_summary(
        record: &ScheduleCatalogEntry,
        locale: Locale,
        system_zone: &str,
        now_ms: i64,
    ) -> Vec<String> {
        let zone = Some(FrequencyZone {
            system: system_zone,
        });
        let mut parts = Vec::new();
        if record.status != ScheduleStatus::Active {
            parts.push(status_text(record.status, locale).to_string());
        }
        parts.push(format_schedule_frequency(record, locale, zone, now_ms));
        if record.status == ScheduleStatus::Active {
            let zone_tz = jiff::tz::TimeZone::get(system_zone).unwrap_or(jiff::tz::TimeZone::UTC);
            let next = next_run_parts(&task_next_run_at(record), locale, now_ms, &zone_tz);
            parts.push(format!(
                "{}{} {}",
                copy(locale, "list.nextPrefix"),
                next.absolute,
                next.relative
            ));
        }
        parts
    }

    pub fn detail_next_line(
        record: &ScheduleCatalogEntry,
        locale: Locale,
        system_zone: &str,
        now_ms: i64,
    ) -> String {
        if record.status != ScheduleStatus::Active {
            return status_text(record.status, locale).to_string();
        }
        let zone_tz = jiff::tz::TimeZone::get(system_zone).unwrap_or(jiff::tz::TimeZone::UTC);
        let next = next_run_parts(&task_next_run_at(record), locale, now_ms, &zone_tz);
        format!(
            "{} {} {}",
            copy(locale, "detail.nextRun"),
            next.absolute,
            next.relative
        )
    }

    pub fn delivery_floor_active(record: &ScheduleCatalogEntry, now_ms: i64) -> bool {
        record
            .delivery_floor_ends_at
            .as_deref()
            .and_then(|text| text.parse::<jiff::Timestamp>().ok())
            .map(|ts| ts.as_millisecond() > now_ms)
            .unwrap_or(false)
    }

    fn floor_time_text(at_ms: i64, now_ms: i64, locale: Locale) -> String {
        let tz = jiff::tz::TimeZone::system();
        let at = jiff::Timestamp::from_millisecond(at_ms).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        let now = jiff::Timestamp::from_millisecond(now_ms).unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        let dt = tz.to_datetime(at);
        let time = format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second());
        if dt.date() == tz.to_datetime(now).date() {
            time
        } else {
            let month = i8::from(dt.month()) as usize;
            match locale {
                Locale::Zh => format!("{}月{}日 {}", month, dt.day(), time),
                Locale::En => format!("{} {}, {}", EN_MONTHS_SHORT[month - 1], dt.day(), time),
            }
        }
    }

    pub fn floor_hold_text(
        record: &ScheduleCatalogEntry,
        locale: Locale,
        now_ms: i64,
    ) -> Option<String> {
        let at = record
            .delivery_floor_ends_at
            .as_deref()
            .and_then(|text| text.parse::<jiff::Timestamp>().ok())
            .map(|ts| ts.as_millisecond())?;
        (at > now_ms).then(|| {
            fill(
                copy(locale, "host.runAfterInterval"),
                &[("time", floor_time_text(at, now_ms, locale))],
            )
        })
    }

    pub fn failure_from_error(err: &ConnectionError) -> ScheduleFailure {
        let ConnectionError::Remote {
            data: Some(data), ..
        } = err
        else {
            return ScheduleFailure::SaveError;
        };
        match data.get("code").and_then(Value::as_str) {
            Some("not_future") => ScheduleFailure::NotFuture,
            Some(
                "invalid_rule" | "invalid_time_zone" | "time_out_of_range" | "frequency_too_high",
            ) => ScheduleFailure::InvalidRule,
            _ => ScheduleFailure::SaveError,
        }
    }

    pub fn failure_from_result(value: &Value) -> Option<ScheduleFailure> {
        match value.get("code").and_then(Value::as_str) {
            Some("schedule_conflict") => Some(ScheduleFailure::Conflict),
            Some("schedule_not_found") => Some(ScheduleFailure::Missing),
            Some("schedule_execution_pending") => Some(ScheduleFailure::ExecutionPending),
            _ => None,
        }
    }

    pub fn run_failure(failure: ScheduleFailure) -> ScheduleFailure {
        if failure == ScheduleFailure::SaveError {
            ScheduleFailure::RunError
        } else {
            failure
        }
    }

    pub fn failure_text(failure: ScheduleFailure, created: bool, locale: Locale) -> &'static str {
        match failure {
            ScheduleFailure::Conflict => copy(locale, "host.conflict"),
            ScheduleFailure::NotFuture => copy(locale, "host.notFuture"),
            ScheduleFailure::InvalidRule => copy(locale, "host.invalidRule"),
            ScheduleFailure::ExecutionPending => copy(locale, "host.executionPending"),
            ScheduleFailure::Missing => copy(locale, "detail.missing"),
            ScheduleFailure::SaveError => {
                if created {
                    copy(locale, "host.createError")
                } else {
                    copy(locale, "host.saveError")
                }
            }
            ScheduleFailure::RunError => copy(locale, "host.runError"),
        }
    }

    pub fn miss_text(miss: ScheduleHistoryMiss, locale: Locale) -> &'static str {
        match miss {
            ScheduleHistoryMiss::NotFound => copy(locale, "delivery.notFound"),
            ScheduleHistoryMiss::CursorInvalid | ScheduleHistoryMiss::Failed => {
                copy(locale, "delivery.error")
            }
        }
    }

    pub fn issue_text(issue: DraftIssue, locale: Locale) -> &'static str {
        match issue {
            DraftIssue::Incomplete => "",
            DraftIssue::InvalidTitle => copy(locale, "rule.invalidTitle"),
            DraftIssue::InvalidInterval => copy(locale, "rule.invalidInterval"),
            DraftIssue::RequiredWeekday => copy(locale, "rule.requiredWeekday"),
            DraftIssue::InvalidMonthDay => copy(locale, "rule.invalidMonthDay"),
            DraftIssue::InvalidTime => copy(locale, "rule.invalidTime"),
            DraftIssue::InvalidZone => copy(locale, "rule.invalidZone"),
            DraftIssue::TooComplex => copy(locale, "rule.tooComplex"),
            DraftIssue::Invalid => copy(locale, "rule.invalid"),
        }
    }

    pub fn frequency_labels(locale: Locale) -> [&'static str; 8] {
        [
            copy(locale, "rule.weekly"),
            copy(locale, "rule.weekdays"),
            copy(locale, "rule.daily"),
            copy(locale, "rule.everyHours"),
            copy(locale, "rule.everyMinutes"),
            copy(locale, "host.frequency.monthly"),
            copy(locale, "rule.once"),
            copy(locale, "host.frequency.advanced"),
        ]
    }

    pub fn frequency_row_of(timing: &TimingDraft) -> usize {
        if timing.frequency == FrequencyPreset::Weekly && timing.weekdays == [1, 2, 3, 4, 5] {
            return 1;
        }
        match timing.frequency {
            FrequencyPreset::Weekly => 0,
            FrequencyPreset::Daily => 2,
            FrequencyPreset::Hourly => 3,
            FrequencyPreset::Minutely => 4,
            FrequencyPreset::Monthly => 5,
            FrequencyPreset::Once => 6,
            FrequencyPreset::Advanced => 7,
        }
    }

    pub fn apply_frequency_row(
        draft: &mut TaskEditorDraft,
        row: usize,
        has_original: bool,
        now_ms: i64,
    ) {
        let interval_from_now = !has_original;
        match row {
            1 => {
                draft.timing.frequency = FrequencyPreset::Weekly;
                draft.timing.weekdays = vec![1, 2, 3, 4, 5];
            }
            0 => draft.timing.frequency = FrequencyPreset::Weekly,
            2 => draft.timing.frequency = FrequencyPreset::Daily,
            3 => draft.timing.frequency = FrequencyPreset::Hourly,
            4 => draft.timing.frequency = FrequencyPreset::Minutely,
            5 => draft.timing.frequency = FrequencyPreset::Monthly,
            6 => draft.timing.frequency = FrequencyPreset::Once,
            _ => {
                let seeded = rrule_from_draft(&draft.timing, interval_from_now, now_ms);
                draft.timing.frequency = FrequencyPreset::Advanced;
                draft.timing.rrule = seeded;
            }
        }
    }

    pub fn target_labels(choice: TargetChoice, locale: Locale) -> &'static str {
        match choice {
            TargetChoice::Session => copy(locale, "host.currentConversation"),
            TargetChoice::NewOnce => copy(locale, "host.targetNewOnce"),
            TargetChoice::New => copy(locale, "host.targetNew"),
        }
    }

    pub fn target_help(choice: TargetChoice, locale: Locale) -> &'static str {
        match choice {
            TargetChoice::Session => "",
            TargetChoice::NewOnce => copy(locale, "host.targetNewOnceHelp"),
            TargetChoice::New => copy(locale, "host.targetNewHelp"),
        }
    }

    pub fn target_row_of(choice: TargetChoice, has_session: bool) -> usize {
        if has_session {
            match choice {
                TargetChoice::Session => 0,
                TargetChoice::NewOnce => 1,
                TargetChoice::New => 2,
            }
        } else {
            match choice {
                TargetChoice::Session | TargetChoice::NewOnce => 0,
                TargetChoice::New => 1,
            }
        }
    }

    pub fn target_choice_of_row(row: usize, has_session: bool) -> TargetChoice {
        if has_session {
            match row {
                0 => TargetChoice::Session,
                1 => TargetChoice::NewOnce,
                _ => TargetChoice::New,
            }
        } else if row == 0 {
            TargetChoice::NewOnce
        } else {
            TargetChoice::New
        }
    }

    pub fn patch_is_empty(patch: &SchedulePatch) -> bool {
        patch.title.is_none()
            && patch.prompt.is_none()
            && patch.timing.is_none()
            && patch.target.is_none()
            && patch.active.is_none()
    }

    pub fn reapply_task_draft(
        draft: &TaskEditorDraft,
        latest: &ScheduleCatalogEntry,
        changes: &SchedulePatch,
        now_ms: i64,
        system_zone: &str,
    ) -> TaskEditorDraft {
        let mut reapplied = task_draft(latest, now_ms, system_zone);
        if changes.timing.is_some() {
            reapplied.timing = draft.timing.clone();
        }
        if changes.title.is_some() {
            reapplied.title = draft.title.clone();
        }
        if changes.prompt.is_some() {
            reapplied.prompt = draft.prompt.clone();
        }
        if changes.target.is_some() {
            reapplied.target = draft.target;
            reapplied.session_id = draft.session_id.clone();
            reapplied.cwd = draft.cwd.clone();
        }
        reapplied
    }

    pub fn available_zones() -> Vec<String> {
        let mut zones: Vec<String> = jiff::tz::db()
            .available()
            .map(|name| name.as_str().to_string())
            .collect();
        zones.sort();
        zones
    }

    pub fn zone_matches(name: &str, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty() || name.to_lowercase().contains(&query)
    }

    pub fn merge_history_page(
        previous: &[ScheduleDeliveryRecord],
        page: &ScheduleDeliveryHistoryPage,
    ) -> Vec<ScheduleDeliveryRecord> {
        let mut records = previous.to_vec();
        for record in &page.records {
            if !records
                .iter()
                .any(|item| item.message_id == record.message_id)
            {
                records.push(record.clone());
            }
        }
        records
    }

    pub fn clear_ended_toast(count: i64, locale: Locale) -> String {
        if count <= 0 {
            copy(locale, "toast.clearedNone").to_string()
        } else {
            fill(
                copy(locale, "toast.cleared"),
                &[("count", count.to_string())],
            )
        }
    }

    pub fn delete_toast(locale: Locale) -> &'static str {
        copy(locale, "toast.deleted")
    }

    pub fn create_prompt(locale: Locale) -> &'static str {
        copy(locale, "host.createPrompt")
    }

    pub fn linked_state(archived: Option<bool>) -> LinkedState {
        match archived {
            Some(false) => LinkedState::Available,
            Some(true) => LinkedState::Archived,
            None => LinkedState::Unavailable,
        }
    }

    pub fn linked_notice(state: LinkedState, locale: Locale) -> Option<&'static str> {
        match state {
            LinkedState::Available => None,
            LinkedState::Archived => Some(copy(locale, "detail.sessionArchived")),
            LinkedState::Unavailable => Some(copy(locale, "detail.sessionUnavailable")),
        }
    }

    pub fn default_cwd(workspaces: &[WorkspaceView]) -> String {
        workspaces
            .first()
            .map(|workspace| workspace.path.clone())
            .unwrap_or_default()
    }

    pub fn clamp_title(text: &str) -> String {
        let mut out = String::new();
        let mut units = 0usize;
        for ch in text.chars() {
            let len = ch.len_utf16();
            if units + len > TITLE_MAX_UTF16 {
                break;
            }
            out.push(ch);
            units += len;
        }
        out
    }

    pub fn history_miss(code: &str) -> ScheduleHistoryMiss {
        match code {
            "delivery_cursor_not_found" => ScheduleHistoryMiss::CursorInvalid,
            _ => ScheduleHistoryMiss::NotFound,
        }
    }
}

use model::*;

pub fn ui_locale() -> Locale {
    let preference = crate::ui_prefs::UiPrefs::load(&crate::ui_prefs::default_ui_prefs_path())
        .locale
        .unwrap_or_default();
    Locale::from_tag(&crate::ui::copy::resolve_locale(
        Some(&preference),
        &serde_json::Value::Null,
    ))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as i64)
        .unwrap_or(0)
}

fn system_zone() -> String {
    jiff::tz::TimeZone::system()
        .iana_name()
        .unwrap_or("UTC")
        .to_string()
}

fn zone_tz(zone: &str) -> jiff::tz::TimeZone {
    jiff::tz::TimeZone::get(zone).unwrap_or(jiff::tz::TimeZone::UTC)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatalogState {
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetailTab {
    Rule,
    Records,
}

#[derive(Clone)]
enum DetailMode {
    Closed,
    Create { initial: TaskEditorDraft },
    Task { id: String },
}

struct HistoryState {
    last_request_before: Option<String>,
    records: Vec<ScheduleDeliveryRecord>,
    next_before: Option<String>,
    pruned: bool,
    retention: Option<DeliveryRetentionBounds>,
    miss: Option<ScheduleHistoryMiss>,
    loading: bool,
}

impl HistoryState {
    fn new() -> Self {
        Self {
            last_request_before: None,
            records: Vec::new(),
            next_before: None,
            pruned: false,
            retention: None,
            miss: None,
            loading: false,
        }
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

struct DetailState {
    mode: DetailMode,
    tab: DetailTab,
    draft: Rc<RefCell<TaskEditorDraft>>,
    base: Option<ScheduleCatalogEntry>,
    changes: Rc<RefCell<Option<SchedulePatch>>>,
    pending: Rc<Cell<bool>>,
    failure: Rc<RefCell<Option<ScheduleFailure>>>,
    limits: Rc<Cell<Option<i64>>>,
    history: HistoryState,
    widgets: Option<DetailWidgets>,
}

impl DetailState {
    fn closed() -> Self {
        Self {
            mode: DetailMode::Closed,
            tab: DetailTab::Rule,
            draft: Rc::new(RefCell::new(new_task_draft(0, "UTC"))),
            base: None,
            changes: Rc::new(RefCell::new(None)),
            pending: Rc::new(Cell::new(false)),
            failure: Rc::new(RefCell::new(None)),
            limits: Rc::new(Cell::new(None)),
            history: HistoryState::new(),
            widgets: None,
        }
    }
}

struct RowWidgets {
    id: String,
    button: gtk4::Button,
    summary: gtk4::Label,
    entry: ScheduleCatalogEntry,
}

struct RuleRows {
    interval: adw::SpinRow,
    weekday_row: adw::ActionRow,
    weekday_toggles: Vec<gtk4::ToggleButton>,
    month_day: adw::SpinRow,
    date_row: adw::ActionRow,
    date_label: gtk4::Label,
    time_row: adw::ActionRow,
    time_label: gtk4::Label,
    zone_row: adw::ActionRow,
    zone_label: gtk4::Label,
    custom_box: gtk4::Box,
    custom_buffer: gtk4::TextBuffer,
}

struct DetailWidgets {
    header_title: gtk4::Label,
    subtitle: gtk4::Label,
    notices: gtk4::Box,
    conflict: gtk4::Box,
    issue_label: gtk4::Label,
    failure_label: gtk4::Label,
    limits_box: gtk4::Box,
    limits_label: gtk4::Label,
    limits_retry: gtk4::Button,
    pause_btn: gtk4::Button,
    run_btn: gtk4::Button,
    run_reason: gtk4::Label,
    delete_btn: gtk4::Button,
    close_btn: gtk4::Button,
    more_button: gtk4::MenuButton,
    save_btn: gtk4::Button,
    discard_btn: gtk4::Button,
    unsaved_label: gtk4::Label,
    save_footer: gtk4::Box,
    session_footer: gtk4::Box,
    session_title_label: gtk4::Label,
    session_open_btn: gtk4::Button,
    session_pending_label: gtk4::Label,
    title_entry: adw::EntryRow,
    prompt_buffer: gtk4::TextBuffer,
    target_combo: adw::ComboRow,
    workspace_combo: adw::ComboRow,
    repeat_combo: adw::ComboRow,
    stack: gtk4::Stack,
    records_box: gtk4::Box,
    rules: RuleRows,
}

struct PageState {
    controller: RefCell<Option<std::rc::Weak<SessionController>>>,
    entries: RefCell<Vec<ScheduleCatalogEntry>>,
    rows: RefCell<Vec<RowWidgets>>,
    filter: Cell<StatusFilter>,
    search: RefCell<String>,
    catalog: Cell<CatalogState>,
    detail: RefCell<DetailState>,
    locale: Locale,
    zone: String,
    widget: gtk4::Paned,
    list: gtk4::ListBox,
    loading: gtk4::Box,
    error_box: gtk4::Box,
    empty_box: gtk4::Box,
    empty_label: gtk4::Label,
    search_entry: gtk4::SearchEntry,
    filter_buttons: [gtk4::ToggleButton; 4],
    clear_action: gtk4::gio::SimpleAction,
    clear_button: gtk4::MenuButton,
    detail_root: gtk4::Box,
}

pub struct SchedulesPage {
    state: Rc<PageState>,
}

impl SchedulesPage {
    pub fn new() -> Self {
        let locale = ui_locale();
        let list_pane = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .hexpand(true)
            .build();
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(12)
            .margin_start(24)
            .margin_end(24)
            .margin_top(40)
            .margin_bottom(18)
            .build();
        let clamp = adw::Clamp::builder()
            .maximum_size(960)
            .tightening_threshold(720)
            .child(&content)
            .build();
        list_pane.set_child(Some(&clamp));

        let header = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let title = gtk4::Label::builder()
            .label(crate::ui::copy::Copybook::for_code("system").schedules)
            .css_classes(["title-2"])
            .hexpand(true)
            .halign(gtk4::Align::Start)
            .build();
        header.append(&title);
        let create_menu = gtk4::gio::Menu::new();
        create_menu.append(
            Some(copy(locale, "host.chatCreate")),
            Some("schedule-page.chat-create"),
        );
        create_menu.append(
            Some(copy(locale, "host.manualCreate")),
            Some("schedule-page.manual-create"),
        );
        let split = adw::SplitButton::builder()
            .label(copy(locale, "new.action"))
            .menu_model(&create_menu)
            .css_classes(["suggested-action"])
            .build();
        header.append(&split);
        content.append(&header);

        let filters = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(6)
            .build();
        let filter_labels = [
            copy(locale, "status.active"),
            copy(locale, "status.paused"),
            copy(locale, "status.ended"),
            copy(locale, "statusFilter.all"),
        ];
        let mut filter_buttons: Vec<gtk4::ToggleButton> = Vec::new();
        for (index, label) in filter_labels.iter().enumerate() {
            let button = gtk4::ToggleButton::builder().label(*label).build();
            if let Some(first) = filter_buttons.first() {
                button.set_group(Some(first));
            }
            button.set_active(index == 0);
            filters.append(&button);
            filter_buttons.push(button);
        }
        content.append(&filters);

        let tools = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        let search_entry = gtk4::SearchEntry::builder()
            .placeholder_text(copy(locale, "search.placeholder"))
            .hexpand(true)
            .build();
        tools.append(&search_entry);
        let clear_menu = gtk4::gio::Menu::new();
        clear_menu.append(
            Some(copy(locale, "clear.action")),
            Some("schedule-page.clear-ended"),
        );
        let clear_button = gtk4::MenuButton::builder()
            .icon_name("view-more-symbolic")
            .menu_model(&clear_menu)
            .css_classes(["flat"])
            .valign(gtk4::Align::Center)
            .visible(false)
            .build();
        tools.append(&clear_button);
        content.append(&tools);

        let loading = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .margin_top(48)
            .margin_bottom(48)
            .build();
        loading.append(&gtk4::Spinner::builder().spinning(true).build());
        loading.append(
            &gtk4::Label::builder()
                .label(copy(locale, "list.loading"))
                .build(),
        );
        content.append(&loading);

        let error_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .visible(false)
            .build();
        error_box.append(
            &gtk4::Label::builder()
                .label(copy(locale, "list.error"))
                .css_classes(["dim-label"])
                .build(),
        );
        let retry = gtk4::Button::builder()
            .label(copy(locale, "list.retry"))
            .css_classes(["flat"])
            .build();
        error_box.append(&retry);
        content.append(&error_box);

        let empty_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(10)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .margin_top(48)
            .margin_bottom(48)
            .visible(false)
            .build();
        empty_box.append(
            &gtk4::Image::builder()
                .icon_name("alarm-symbolic")
                .pixel_size(48)
                .css_classes(["dim-label"])
                .build(),
        );
        let empty_label = gtk4::Label::builder()
            .wrap(true)
            .justify(gtk4::Justification::Center)
            .build();
        empty_box.append(&empty_label);
        let empty_create = gtk4::Button::builder()
            .label(copy(locale, "empty.action"))
            .css_classes(["flat"])
            .halign(gtk4::Align::Center)
            .build();
        empty_box.append(&empty_create);
        content.append(&empty_box);

        let list = gtk4::ListBox::builder()
            .selection_mode(gtk4::SelectionMode::None)
            .css_classes(["dsh-schedule-list"])
            .build();
        content.append(&list);

        let detail_root = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .width_request(440)
            .css_classes(["dsh-schedule-detail"])
            .visible(false)
            .build();

        let widget = gtk4::Paned::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .position(720)
            .shrink_start_child(false)
            .shrink_end_child(false)
            .build();
        widget.set_start_child(Some(&list_pane));
        widget.set_end_child(Some(&detail_root));

        let state = Rc::new(PageState {
            controller: RefCell::new(None),
            entries: RefCell::new(Vec::new()),
            rows: RefCell::new(Vec::new()),
            filter: Cell::new(StatusFilter::Active),
            search: RefCell::new(String::new()),
            catalog: Cell::new(CatalogState::Loading),
            detail: RefCell::new(DetailState::closed()),
            locale,
            zone: system_zone(),
            widget: widget.clone(),
            list: list.clone(),
            loading,
            error_box,
            empty_box,
            empty_label,
            search_entry,
            filter_buttons: filter_buttons.try_into().unwrap(),
            clear_action: gtk4::gio::SimpleAction::new("clear-ended", None),
            clear_button,
            detail_root,
        });

        let actions = gtk4::gio::SimpleActionGroup::new();
        let state_for_chat = state.clone();
        let chat_create = gtk4::gio::SimpleAction::new("chat-create", None);
        chat_create.connect_activate(move |_, _| {
            if let Some(ctrl) = controller_of(&state_for_chat) {
                ctrl.start_schedule_session();
            }
        });
        actions.add_action(&chat_create);
        let state_for_manual = state.clone();
        let manual_create = gtk4::gio::SimpleAction::new("manual-create", None);
        manual_create.connect_activate(move |_, _| {
            open_create(&state_for_manual);
        });
        actions.add_action(&manual_create);
        let state_for_clear = state.clone();
        let clear_action = state.clear_action.clone();
        clear_action.connect_activate(move |_, _| {
            confirm_clear_ended(&state_for_clear);
        });
        actions.add_action(&clear_action);
        widget.insert_action_group("schedule-page", Some(&actions));

        let state_for_split = state.clone();
        split.connect_clicked(move |_| {
            if let Some(ctrl) = controller_of(&state_for_split) {
                ctrl.start_schedule_session();
            }
        });

        for (index, button) in state.filter_buttons.iter().enumerate() {
            let state_for_filter = state.clone();
            button.connect_toggled(move |toggle| {
                if !toggle.is_active() {
                    return;
                }
                let filter = match index {
                    0 => StatusFilter::Active,
                    1 => StatusFilter::Paused,
                    2 => StatusFilter::Ended,
                    _ => StatusFilter::All,
                };
                if state_for_filter.filter.get() == filter {
                    return;
                }
                state_for_filter.filter.set(filter);
                rebuild_rows(&state_for_filter);
            });
        }

        let state_for_search = state.clone();
        state.search_entry.connect_search_changed(move |entry| {
            *state_for_search.search.borrow_mut() = entry.text().to_string();
            rebuild_rows(&state_for_search);
        });

        let state_for_empty = state.clone();
        empty_create.connect_clicked(move |_| {
            open_create(&state_for_empty);
        });
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            state_for_retry.catalog.set(CatalogState::Loading);
            refresh_feedback(&state_for_retry);
            if let Some(ctrl) = controller_of(&state_for_retry) {
                ctrl.load_schedules();
            }
        });

        let state_for_escape = state.clone();
        let escape_controller = gtk4::EventControllerKey::new();
        escape_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::Escape {
                let open = !matches!(state_for_escape.detail.borrow().mode, DetailMode::Closed);
                if open {
                    close_detail(&state_for_escape);
                    return glib::Propagation::Stop;
                }
            }
            glib::Propagation::Proceed
        });
        widget.add_controller(escape_controller);

        let state_for_clock = state.clone();
        glib::timeout_add_local(
            std::time::Duration::from_secs(CLOCK_PERIOD_SECS),
            move || {
                tick_clock(&state_for_clock);
                glib::ControlFlow::Continue
            },
        );

        refresh_feedback(&state);
        Self { state }
    }

    pub fn widget(&self) -> &gtk4::Paned {
        &self.state.widget
    }

    pub fn set_controller(&self, controller: Rc<SessionController>) {
        *self.state.controller.borrow_mut() = Some(Rc::downgrade(&controller));
        self.load();
    }

    fn with_controller(&self, f: impl FnOnce(&SessionController)) {
        if let Some(controller) = self
            .state
            .controller
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
        {
            f(&controller);
        }
    }

    pub fn load(&self) {
        self.state.catalog.set(CatalogState::Loading);
        refresh_feedback(&self.state);
        self.with_controller(|ctrl| ctrl.load_schedules());
    }

    pub fn handle_catalog(&self, entries: &[ScheduleCatalogEntry]) {
        *self.state.entries.borrow_mut() = entries.to_vec();
        self.state.catalog.set(CatalogState::Ready);
        let mut reseeded = false;
        let mut closed = false;
        {
            let mut detail = self.state.detail.borrow_mut();
            let current = match &detail.mode {
                DetailMode::Task { id } => Some(id.clone()),
                _ => None,
            };
            if let Some(id) = current {
                match entries.iter().find(|entry| entry.id == id) {
                    None => {
                        detail.mode = DetailMode::Closed;
                        detail.widgets = None;
                        detail.pending.set(false);
                        closed = true;
                    }
                    Some(latest) => {
                        let dirty = {
                            let draft = detail.draft.borrow();
                            let base = detail.base.as_ref().expect("task detail base");
                            !patch_is_empty(&task_draft_patch(
                                &draft,
                                base,
                                false,
                                now_ms(),
                                &self.state.zone,
                            ))
                        };
                        if !dirty {
                            detail.base = Some(latest.clone());
                            *detail.draft.borrow_mut() =
                                task_draft(latest, now_ms(), &self.state.zone);
                            *detail.failure.borrow_mut() = None;
                            detail.pending.set(false);
                            reseeded = true;
                        }
                    }
                }
            }
        }
        if closed {
            self.state.detail_root.set_visible(false);
        }
        if reseeded {
            rebuild_detail(&self.state);
        } else {
            refresh_form(&self.state);
            render_records(&self.state);
        }
        rebuild_rows(&self.state);
    }

    pub fn handle_catalog_failed(&self) {
        self.state.catalog.set(CatalogState::Failed);
        refresh_feedback(&self.state);
    }

    pub fn handle_mutation_settled(&self) {
        let detail = self.state.detail.borrow();
        detail.pending.set(false);
        *detail.failure.borrow_mut() = None;
        drop(detail);
        refresh_form(&self.state);
    }

    pub fn handle_created(&self, entry: &ScheduleCatalogEntry) {
        {
            let mut detail = self.state.detail.borrow_mut();
            detail.pending.set(false);
            detail.mode = DetailMode::Task {
                id: entry.id.clone(),
            };
            detail.base = Some(entry.clone());
            *detail.draft.borrow_mut() = task_draft(entry, now_ms(), &self.state.zone);
            *detail.failure.borrow_mut() = None;
            detail.history.reset();
            detail.tab = DetailTab::Rule;
        }
        self.state.filter.set(StatusFilter::All);
        for (index, button) in self.state.filter_buttons.iter().enumerate() {
            button.set_active(index == 3);
        }
        *self.state.search.borrow_mut() = String::new();
        self.state.search_entry.set_text("");
        rebuild_rows(&self.state);
        rebuild_detail(&self.state);
    }

    pub fn handle_mutation_failed(&self, op: &ScheduleMutationOp, failure: ScheduleFailure) {
        let applies = match op {
            ScheduleMutationOp::Create => {
                matches!(self.state.detail.borrow().mode, DetailMode::Create { .. })
            }
            ScheduleMutationOp::Edit { id } | ScheduleMutationOp::Run { id } => matches!(
                &self.state.detail.borrow().mode,
                DetailMode::Task { id: current } if current == id
            ),
            ScheduleMutationOp::Delete { .. } | ScheduleMutationOp::ClearEnded => false,
        };
        if applies {
            let detail = self.state.detail.borrow();
            let failure = if matches!(op, ScheduleMutationOp::Run { .. }) {
                run_failure(failure)
            } else {
                failure
            };
            detail.pending.set(false);
            *detail.failure.borrow_mut() = Some(failure);
            drop(detail);
            refresh_form(&self.state);
        } else {
            self.state.detail.borrow().pending.set(false);
        }
    }

    pub fn handle_history(
        &self,
        id: &str,
        outcome: Result<ScheduleDeliveryHistoryPage, ScheduleHistoryMiss>,
    ) {
        {
            let mut detail = self.state.detail.borrow_mut();
            let current = match &detail.mode {
                DetailMode::Task { id: current } => Some(current.clone()),
                _ => None,
            };
            if current.as_deref() != Some(id) {
                return;
            }
            detail.history.loading = false;
            let replace = detail.history.last_request_before.is_none();
            match outcome {
                Ok(page) => {
                    let records = if replace {
                        page.records.clone()
                    } else {
                        merge_history_page(&detail.history.records, &page)
                    };
                    detail.history.records = records;
                    detail.history.next_before = page.next_before.clone();
                    detail.history.pruned = page.earlier_records_pruned;
                    detail.history.retention = Some(page.retention.clone());
                    detail.history.miss = None;
                }
                Err(miss) => {
                    if replace {
                        detail.history.records.clear();
                        detail.history.next_before = None;
                    }
                    detail.history.miss = Some(miss);
                }
            }
        }
        render_records(&self.state);
    }

    pub fn handle_limits(&self, value: Option<i64>) {
        self.state.detail.borrow().limits.set(value);
        refresh_form(&self.state);
    }
}

impl Default for SchedulesPage {
    fn default() -> Self {
        Self::new()
    }
}

fn controller_of(state: &Rc<PageState>) -> Option<Rc<SessionController>> {
    state
        .controller
        .borrow()
        .as_ref()
        .and_then(|weak| weak.upgrade())
}

fn rebuild_rows(state: &Rc<PageState>) {
    while let Some(child) = state.list.first_child() {
        state.list.remove(&child);
    }
    let locale = state.locale;
    let zone = state.zone.clone();
    let now = now_ms();
    let filtered = filter_entries(
        &state.entries.borrow(),
        state.filter.get(),
        &state.search.borrow(),
    );
    let selected = match &state.detail.borrow().mode {
        DetailMode::Task { id } => Some(id.clone()),
        _ => None,
    };
    let mut rows = Vec::new();
    for entry in filtered {
        let button = gtk4::Button::builder()
            .css_classes(["dsh-schedule-row"])
            .build();
        let content = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(10)
            .build();
        content.append(
            &gtk4::Image::builder()
                .icon_name("alarm-symbolic")
                .css_classes(["dim-label"])
                .build(),
        );
        let text = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(2)
            .hexpand(true)
            .build();
        let title = gtk4::Label::builder()
            .label(entry.title.clone())
            .ellipsize(gtk4::pango::EllipsizeMode::End)
            .xalign(0.0)
            .css_classes(["dsh-schedule-row-title"])
            .build();
        if entry.status != ScheduleStatus::Active {
            title.add_css_class("dsh-schedule-row-ended");
        }
        text.append(&title);
        let summary = gtk4::Label::builder()
            .label(row_summary(&entry, locale, &zone, now).join(" · "))
            .wrap(true)
            .xalign(0.0)
            .css_classes(["dsh-schedule-row-summary"])
            .build();
        text.append(&summary);
        content.append(&text);
        button.set_child(Some(&content));
        if selected.as_deref() == Some(entry.id.as_str()) {
            button.add_css_class("dsh-schedule-row-selected");
        }
        let row = gtk4::ListBoxRow::builder()
            .child(&button)
            .activatable(true)
            .build();
        state.list.append(&row);
        let id = entry.id.clone();
        let state_for_row = state.clone();
        button.connect_clicked(move |_| {
            open_task(&state_for_row, &id);
        });
        rows.push(RowWidgets {
            id: entry.id.clone(),
            button,
            summary,
            entry,
        });
    }
    *state.rows.borrow_mut() = rows;
    refresh_feedback(state);
}

fn refresh_rows_selected(state: &Rc<PageState>) {
    let selected = match &state.detail.borrow().mode {
        DetailMode::Task { id } => Some(id.clone()),
        _ => None,
    };
    for row in state.rows.borrow().iter() {
        if selected.as_deref() == Some(row.id.as_str()) {
            row.button.add_css_class("dsh-schedule-row-selected");
        } else {
            row.button.remove_css_class("dsh-schedule-row-selected");
        }
    }
}

fn refresh_feedback(state: &Rc<PageState>) {
    let entries_empty = state.entries.borrow().is_empty();
    let rows_empty = state.rows.borrow().is_empty();
    let catalog = state.catalog.get();
    state
        .loading
        .set_visible(catalog == CatalogState::Loading && entries_empty);
    state.error_box.set_visible(catalog == CatalogState::Failed);
    state
        .empty_box
        .set_visible(catalog == CatalogState::Ready && rows_empty);
    state
        .list
        .set_visible(catalog != CatalogState::Failed && !rows_empty);
    if catalog == CatalogState::Ready && rows_empty {
        let filter = state.filter.get();
        let search = state.search.borrow().clone();
        state
            .empty_label
            .set_text(empty_text(empty_key(&search, filter), state.locale));
    }
    let ended = ended_count(&state.entries.borrow());
    let visible = clear_ended_visible(state.filter.get(), ended);
    state.clear_action.set_enabled(visible);
    state.clear_button.set_visible(visible);
}

fn open_task(state: &Rc<PageState>, id: &str) {
    let Some(entry) = state
        .entries
        .borrow()
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
    else {
        return;
    };
    {
        let mut detail = state.detail.borrow_mut();
        detail.mode = DetailMode::Task { id: id.to_string() };
        detail.base = Some(entry.clone());
        *detail.draft.borrow_mut() = task_draft(&entry, now_ms(), &state.zone);
        *detail.changes.borrow_mut() = None;
        *detail.failure.borrow_mut() = None;
        detail.pending.set(false);
        detail.tab = DetailTab::Rule;
        detail.history.reset();
    }
    refresh_rows_selected(state);
    rebuild_detail(state);
}

fn open_create(state: &Rc<PageState>) {
    let mut initial = new_task_draft(now_ms(), &state.zone);
    initial.cwd = controller_of(state)
        .map(|ctrl| default_cwd(&ctrl.workspace_choices()))
        .unwrap_or_default();
    {
        let mut detail = state.detail.borrow_mut();
        detail.mode = DetailMode::Create {
            initial: initial.clone(),
        };
        detail.base = None;
        *detail.draft.borrow_mut() = initial;
        *detail.changes.borrow_mut() = None;
        *detail.failure.borrow_mut() = None;
        detail.pending.set(false);
        detail.tab = DetailTab::Rule;
        detail.history.reset();
    }
    refresh_rows_selected(state);
    rebuild_detail(state);
}

fn close_detail(state: &Rc<PageState>) {
    {
        let mut detail = state.detail.borrow_mut();
        detail.mode = DetailMode::Closed;
        detail.widgets = None;
        detail.pending.set(false);
    }
    state.detail_root.set_visible(false);
    refresh_rows_selected(state);
}

fn rebuild_detail(state: &Rc<PageState>) {
    while let Some(child) = state.detail_root.first_child() {
        state.detail_root.remove(&child);
    }
    if matches!(state.detail.borrow().mode, DetailMode::Closed) {
        state.detail_root.set_visible(false);
        return;
    }
    state.detail_root.set_visible(true);
    build_detail(state, &state.detail_root);
    refresh_form(state);
    render_records(state);
    if state.detail.borrow().limits.get().is_none() {
        if let Some(ctrl) = controller_of(state) {
            ctrl.schedule_delivery_limits();
        }
    }
}

fn tick_clock(state: &Rc<PageState>) {
    let now = now_ms();
    let locale = state.locale;
    let zone = state.zone.clone();
    for row in state.rows.borrow().iter() {
        row.summary
            .set_text(&row_summary(&row.entry, locale, &zone, now).join(" · "));
    }
    refresh_form(state);
}

fn fetch_records(state: &Rc<PageState>, before: Option<String>) {
    let id = {
        let detail = state.detail.borrow();
        match &detail.mode {
            DetailMode::Task { id } if !detail.history.loading => id.clone(),
            _ => return,
        }
    };
    {
        let mut detail = state.detail.borrow_mut();
        detail.history.loading = true;
        detail.history.last_request_before = before.clone();
        detail.history.miss = None;
    }
    render_records(state);
    if let Some(ctrl) = controller_of(state) {
        ctrl.schedule_history(&id, before);
    }
}

fn switch_tab(state: &Rc<PageState>, tab: DetailTab) {
    {
        let mut detail = state.detail.borrow_mut();
        detail.tab = tab;
        if let Some(widgets) = detail.widgets.as_ref() {
            widgets.stack.set_visible_child_name(match tab {
                DetailTab::Rule => "rule",
                DetailTab::Records => "records",
            });
        }
    }
    if tab == DetailTab::Records {
        fetch_records(state, None);
    }
    refresh_form(state);
}

fn save_detail(state: &Rc<PageState>) {
    let Some(ctrl) = controller_of(state) else {
        return;
    };
    let now = now_ms();
    let detail = state.detail.borrow();
    let draft = detail.draft.borrow().clone();
    let task_id = match &detail.mode {
        DetailMode::Create { .. } => None,
        DetailMode::Task { id } => Some(id.clone()),
        DetailMode::Closed => return,
    };
    match task_id {
        None => {
            let timing = timing_from_draft(&draft.timing, true, now);
            let target = draft_target(&draft, None);
            detail.pending.set(true);
            drop(detail);
            refresh_form(state);
            ctrl.schedule_create(
                draft.title.trim(),
                draft.prompt.trim(),
                timing,
                Some(target),
            );
        }
        Some(id) => {
            let (patch, expected) = {
                let base = detail.base.as_ref().expect("task detail base");
                (
                    task_draft_patch(&draft, base, true, now, &state.zone),
                    base.updated_at.clone(),
                )
            };
            detail.pending.set(true);
            drop(detail);
            refresh_form(state);
            ctrl.schedule_edit(&id, patch, expected);
        }
    }
}

fn discard_detail(state: &Rc<PageState>) {
    let create = matches!(state.detail.borrow().mode, DetailMode::Create { .. });
    if create {
        close_detail(state);
        return;
    }
    {
        let detail = state.detail.borrow();
        if let Some(base) = detail.base.as_ref() {
            *detail.draft.borrow_mut() = task_draft(base, now_ms(), &state.zone);
        }
        *detail.failure.borrow_mut() = None;
        detail.pending.set(false);
    }
    rebuild_detail(state);
}

fn toggle_pause(state: &Rc<PageState>) {
    let Some(ctrl) = controller_of(state) else {
        return;
    };
    let detail = state.detail.borrow();
    let id = match &detail.mode {
        DetailMode::Task { id } => id.clone(),
        _ => return,
    };
    let Some(latest) = state
        .entries
        .borrow()
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
    else {
        return;
    };
    let patch = SchedulePatch {
        active: Some(latest.status != ScheduleStatus::Active),
        ..Default::default()
    };
    detail.pending.set(true);
    drop(detail);
    refresh_form(state);
    ctrl.schedule_edit(&id, patch, latest.updated_at.clone());
}

fn run_task(state: &Rc<PageState>) {
    let Some(ctrl) = controller_of(state) else {
        return;
    };
    let detail = state.detail.borrow();
    let id = match &detail.mode {
        DetailMode::Task { id } => id.clone(),
        _ => return,
    };
    detail.pending.set(true);
    drop(detail);
    refresh_form(state);
    ctrl.schedule_run(&id);
}

fn confirm_delete(state: &Rc<PageState>) {
    let Some(ctrl) = controller_of(state) else {
        return;
    };
    let locale = state.locale;
    let (id, prompt, updated_at) = {
        let detail = state.detail.borrow();
        let base = match (&detail.mode, detail.base.as_ref()) {
            (DetailMode::Task { .. }, Some(base)) => base,
            _ => return,
        };
        (
            match &detail.mode {
                DetailMode::Task { id } => id.clone(),
                _ => return,
            },
            base.prompt.clone(),
            base.updated_at.clone(),
        )
    };
    let dialog = adw::AlertDialog::builder()
        .heading(copy(locale, "delete.title"))
        .body(copy(locale, "delete.description"))
        .build();
    let prompt_label = gtk4::Label::builder()
        .label(prompt)
        .wrap(true)
        .xalign(0.0)
        .build();
    dialog.set_extra_child(Some(&prompt_label));
    dialog.add_response("cancel", copy(locale, "delete.cancel"));
    dialog.add_response("delete", copy(locale, "delete.confirm"));
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    dialog.choose(
        Some(&state.widget),
        None::<&gtk4::gio::Cancellable>,
        move |response| {
            if response == "delete" {
                ctrl.schedule_delete(&id, updated_at);
            }
        },
    );
}

fn confirm_clear_ended(state: &Rc<PageState>) {
    let Some(ctrl) = controller_of(state) else {
        return;
    };
    let locale = state.locale;
    let count = ended_count(&state.entries.borrow());
    let dialog = adw::AlertDialog::builder()
        .heading(fill(
            copy(locale, "clear.title"),
            &[("count", count.to_string())],
        ))
        .body(copy(locale, "clear.description"))
        .build();
    dialog.add_response("cancel", copy(locale, "delete.cancel"));
    dialog.add_response("clear", copy(locale, "clear.confirm"));
    dialog.set_response_appearance("clear", adw::ResponseAppearance::Destructive);
    dialog.set_close_response("cancel");
    dialog.choose(
        Some(&state.widget),
        None::<&gtk4::gio::Cancellable>,
        move |response| {
            if response == "clear" {
                ctrl.schedule_delete_inactive();
            }
        },
    );
}

fn reapply_latest(state: &Rc<PageState>) {
    let id = match &state.detail.borrow().mode {
        DetailMode::Task { id } => id.clone(),
        _ => return,
    };
    let Some(latest) = state
        .entries
        .borrow()
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
    else {
        return;
    };
    let changes = state.detail.borrow().changes.borrow().clone();
    let Some(changes) = changes else {
        return;
    };
    let draft = state.detail.borrow().draft.borrow().clone();
    let reapplied = reapply_task_draft(&draft, &latest, &changes, now_ms(), &state.zone);
    {
        let detail = state.detail.borrow();
        *detail.draft.borrow_mut() = reapplied;
        *detail.failure.borrow_mut() = None;
    }
    rebuild_detail(state);
}

fn refresh_form(state: &Rc<PageState>) {
    let detail = state.detail.borrow();
    let Some(widgets) = detail.widgets.as_ref() else {
        return;
    };
    let now = now_ms();
    let draft = detail.draft.borrow().clone();
    let create_mode = matches!(detail.mode, DetailMode::Create { .. });
    let task_id = match &detail.mode {
        DetailMode::Task { id } => Some(id.clone()),
        _ => None,
    };
    let (dirty, changes) = if let Some(_id) = &task_id {
        let Some(base) = detail.base.as_ref() else {
            return;
        };
        let patch = task_draft_patch(&draft, base, false, now, &state.zone);
        let dirty = !patch_is_empty(&patch);
        (dirty, if dirty { Some(patch) } else { None })
    } else {
        let initial = match &detail.mode {
            DetailMode::Create { initial } => initial.clone(),
            _ => return,
        };
        (draft != initial, None)
    };
    let issue = if dirty {
        task_draft_issue(&draft, changes.as_ref(), now)
    } else {
        None
    };
    *detail.changes.borrow_mut() = changes;

    let entries = state.entries.borrow();
    let latest = task_id
        .as_ref()
        .and_then(|id| entries.iter().find(|entry| &entry.id == id).cloned());
    drop(entries);
    let conflict = latest
        .as_ref()
        .zip(detail.base.as_ref())
        .map(|(latest, base)| base.updated_at != latest.updated_at)
        .unwrap_or(false);
    let failure = if conflict {
        Some(ScheduleFailure::Conflict)
    } else {
        detail.failure.borrow().clone()
    };
    let pending = detail.pending.get();
    let authoritative = latest.is_some();
    let shown = latest.as_ref().or(detail.base.as_ref());
    let execution_active = shown
        .and_then(|entry| entry.execution_active)
        .unwrap_or(false);
    let status = shown.map(|entry| entry.status);
    let target_archived = shown
        .and_then(|entry| match &entry.target {
            ScheduleTarget::Session { .. } => task_session_id(entry),
            _ => None,
        })
        .map(|linked| {
            controller_of(state).map(|ctrl| linked_state(ctrl.session_archived(&linked)))
                == Some(LinkedState::Archived)
        })
        .unwrap_or(false);
    let floor_active = shown
        .map(|entry| delivery_floor_active(entry, now))
        .unwrap_or(false);
    let locale = state.locale;
    let zone = state.zone.clone();

    let valid = issue.is_none();
    widgets.save_btn.set_sensitive(if create_mode {
        !pending && dirty && valid
    } else {
        !pending && dirty && valid && !conflict && authoritative && !execution_active
    });
    widgets.save_btn.set_label(if pending {
        copy(
            locale,
            if create_mode {
                "host.creating"
            } else {
                "rule.saving"
            },
        )
    } else {
        copy(
            locale,
            if create_mode {
                "host.create"
            } else {
                "rule.save"
            },
        )
    });
    widgets.discard_btn.set_sensitive(!pending);
    widgets.unsaved_label.set_text(if dirty {
        copy(locale, "rule.unsaved")
    } else {
        ""
    });
    widgets.close_btn.set_sensitive(!pending);
    widgets
        .save_footer
        .set_visible(create_mode || dirty || detail.tab == DetailTab::Rule);

    let effective_failure = failure;
    let conflict_open = effective_failure == Some(ScheduleFailure::Conflict);
    widgets.conflict.set_visible(conflict_open);
    let non_conflict = effective_failure.filter(|failure| *failure != ScheduleFailure::Conflict);
    widgets.failure_label.set_text(
        non_conflict
            .map(|failure| failure_text(failure, create_mode, locale))
            .unwrap_or(""),
    );
    widgets.failure_label.set_visible(non_conflict.is_some());
    let issue_shown = effective_failure
        .is_none()
        .then_some(issue)
        .flatten()
        .filter(|issue| *issue != DraftIssue::Incomplete);
    widgets.issue_label.set_text(
        issue_shown
            .map(|issue| issue_text(issue, locale))
            .unwrap_or(""),
    );
    widgets.issue_label.set_visible(issue_shown.is_some());

    widgets.pause_btn.set_visible(!create_mode);
    widgets.more_button.set_visible(!create_mode);
    widgets.header_title.set_visible(create_mode);
    if create_mode {
        widgets.subtitle.set_visible(false);
        widgets.session_footer.set_visible(false);
    } else {
        let shown_entry = shown.expect("task detail entry");
        widgets.subtitle.set_visible(true);
        widgets
            .subtitle
            .set_text(&detail_next_line(shown_entry, locale, &zone, now));
        widgets.pause_btn.set_sensitive(
            !pending
                && !dirty
                && !execution_active
                && status != Some(ScheduleStatus::Ended)
                && authoritative,
        );
        widgets.pause_btn.set_icon_name(match status {
            Some(ScheduleStatus::Active) => "media-playback-pause-symbolic",
            _ => "media-playback-start-symbolic",
        });
        widgets.pause_btn.set_tooltip_text(Some(match status {
            Some(ScheduleStatus::Active) => copy(locale, "host.pause"),
            _ => copy(locale, "host.resume"),
        }));
        let run_held = pending || dirty || execution_active || target_archived || !authoritative;
        let floor_only = !run_held && floor_active;
        widgets.run_btn.set_sensitive(!run_held && !floor_active);
        let run_reason_text = if floor_only {
            floor_hold_text(shown_entry, locale, now).unwrap_or_default()
        } else {
            String::new()
        };
        widgets.run_reason.set_text(&run_reason_text);
        widgets.run_reason.set_visible(floor_only);
        widgets
            .delete_btn
            .set_sensitive(!pending && !dirty && !execution_active && authoritative);
        widgets.session_footer.set_visible(true);
        refresh_notices(
            state,
            widgets,
            shown_entry,
            effective_failure,
            dirty,
            authoritative,
        );
        refresh_session_footer(state, widgets, shown_entry);
    }
    refresh_rule_rows(widgets, &draft, locale, now);
    refresh_limits(widgets, &detail, &draft, locale);
}

fn refresh_notices(
    state: &Rc<PageState>,
    widgets: &DetailWidgets,
    entry: &ScheduleCatalogEntry,
    failure: Option<ScheduleFailure>,
    dirty: bool,
    authoritative: bool,
) {
    while let Some(child) = widgets.notices.first_child() {
        widgets.notices.remove(&child);
    }
    let locale = state.locale;
    if matches!(entry.target, ScheduleTarget::Session { .. }) {
        if let Some(linked) = task_session_id(entry) {
            let linked_state_now = controller_of(state)
                .map(|ctrl| linked_state(ctrl.session_archived(&linked)))
                .unwrap_or(LinkedState::Unavailable);
            if let Some(notice) = linked_notice(linked_state_now, locale) {
                widgets.notices.append(&notice_label(notice));
            }
        }
    }
    if entry.execution_active == Some(true) && failure != Some(ScheduleFailure::ExecutionPending) {
        widgets
            .notices
            .append(&notice_label(copy(locale, "host.executionActive")));
    }
    if dirty && !authoritative && failure != Some(ScheduleFailure::Missing) {
        widgets
            .notices
            .append(&notice_label(copy(locale, "detail.missing")));
    }
}

fn notice_label(text: &str) -> gtk4::Label {
    gtk4::Label::builder()
        .label(text)
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dsh-schedule-notice"])
        .build()
}

fn refresh_session_footer(
    state: &Rc<PageState>,
    widgets: &DetailWidgets,
    entry: &ScheduleCatalogEntry,
) {
    let linked = task_session_id(entry);
    let is_new = matches!(entry.target, ScheduleTarget::New { .. });
    widgets.session_title_label.set_text(if is_new {
        copy(state.locale, "host.latestConversation")
    } else {
        copy(state.locale, "detail.session")
    });
    let pending_creation = match (&entry.target, linked.as_deref()) {
        (_, None) => true,
        (ScheduleTarget::NewOnce { .. }, Some(id)) => {
            controller_of(state).map(|ctrl| linked_state(ctrl.session_archived(id)))
                == Some(LinkedState::Unavailable)
        }
        _ => false,
    };
    match linked {
        Some(id) if !pending_creation => {
            let title = controller_of(state)
                .and_then(|ctrl| ctrl.session_display_title(&id))
                .unwrap_or_else(|| id.clone());
            widgets.session_open_btn.set_label(&title);
            widgets.session_open_btn.set_sensitive(
                controller_of(state).map(|ctrl| linked_state(ctrl.session_archived(&id)))
                    == Some(LinkedState::Available),
            );
            widgets.session_open_btn.set_visible(true);
            widgets.session_pending_label.set_visible(false);
        }
        _ => {
            widgets.session_open_btn.set_visible(false);
            widgets.session_pending_label.set_visible(true);
        }
    }
}

fn refresh_rule_rows(widgets: &DetailWidgets, draft: &TaskEditorDraft, locale: Locale, now: i64) {
    let frequency = draft.timing.frequency;
    let advanced = frequency == FrequencyPreset::Advanced;
    widgets
        .rules
        .interval
        .set_visible(frequency != FrequencyPreset::Once && !advanced);
    widgets
        .rules
        .weekday_row
        .set_visible(frequency == FrequencyPreset::Weekly);
    widgets
        .rules
        .month_day
        .set_visible(frequency == FrequencyPreset::Monthly);
    widgets
        .rules
        .date_row
        .set_visible(frequency == FrequencyPreset::Once);
    widgets.rules.time_row.set_visible(
        !advanced && frequency != FrequencyPreset::Minutely && frequency != FrequencyPreset::Hourly,
    );
    widgets
        .rules
        .zone_row
        .set_visible(!advanced || !draft.timing.rrule.trim_start().starts_with("DTSTART"));
    widgets.rules.custom_box.set_visible(advanced);
    widgets
        .rules
        .date_label
        .set_text(&draft.timing.start[..10].replace('-', "/"));
    let clock_text = if frequency == FrequencyPreset::Daily {
        draft.timing.daily_time.clone()
    } else {
        draft
            .timing
            .start
            .get(11..19)
            .unwrap_or_default()
            .to_string()
    };
    widgets.rules.time_label.set_text(&clock_text);
    widgets
        .rules
        .zone_label
        .set_text(&zone_label(&draft.timing.time_zone, locale, now));
}

fn refresh_limits(
    widgets: &DetailWidgets,
    detail: &DetailState,
    draft: &TaskEditorDraft,
    locale: Locale,
) {
    let visible = draft.target == TargetChoice::New;
    widgets.limits_box.set_visible(visible);
    if !visible {
        return;
    }
    match detail.limits.get() {
        Some(interval) => {
            let text = fill(
                copy(locale, "host.newSessionInterval"),
                &[("interval", interval.to_string())],
            );
            widgets.limits_label.set_text(&text);
            widgets.limits_retry.set_visible(false);
        }
        None => {
            widgets
                .limits_label
                .set_text(copy(locale, "host.limitsError"));
            widgets.limits_retry.set_visible(true);
        }
    }
}

fn sync_form(
    widgets: &DetailWidgets,
    draft: &TaskEditorDraft,
    has_session: bool,
    workspace_paths: &[String],
) {
    widgets.title_entry.set_text(&draft.title);
    widgets.prompt_buffer.set_text(&draft.prompt);
    widgets
        .target_combo
        .set_selected(target_row_of(draft.target, has_session) as u32);
    if let Some(index) = workspace_paths.iter().position(|path| path == &draft.cwd) {
        widgets.workspace_combo.set_selected(index as u32);
    }
    widgets
        .repeat_combo
        .set_selected(frequency_row_of(&draft.timing) as u32);
    widgets.rules.interval.set_text(&draft.timing.interval);
    for (index, toggle) in widgets.rules.weekday_toggles.iter().enumerate() {
        toggle.set_active(draft.timing.weekdays.contains(&((index + 1) as i64)));
    }
    widgets
        .rules
        .month_day
        .set_value(draft.timing.month_day as f64);
    widgets.rules.custom_buffer.set_text(&draft.timing.rrule);
}

fn build_detail(state: &Rc<PageState>, root: &gtk4::Box) {
    let locale = state.locale;
    let (current_session_id, workspace_paths) = {
        let detail = state.detail.borrow();
        let session = detail.base.as_ref().and_then(|base| match &base.target {
            ScheduleTarget::Session { session_id } => Some(session_id.clone()),
            _ => None,
        });
        let paths = controller_of(state)
            .map(|ctrl| {
                ctrl.workspace_choices()
                    .iter()
                    .map(|workspace| workspace.path.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        (session, paths)
    };
    let has_session = current_session_id.is_some();
    let create_mode = matches!(state.detail.borrow().mode, DetailMode::Create { .. });

    let root_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .build();
    root.append(&root_box);

    let top_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .margin_start(12)
        .margin_end(12)
        .margin_top(10)
        .margin_bottom(6)
        .build();
    let left = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .hexpand(true)
        .build();
    let header_title = gtk4::Label::builder()
        .label(copy(locale, "host.manualCreate"))
        .css_classes(["title-3"])
        .visible(create_mode)
        .build();
    left.append(&header_title);
    let rule_tab = gtk4::ToggleButton::builder()
        .label(copy(locale, "detail.rule"))
        .css_classes(["flat"])
        .active(true)
        .visible(!create_mode)
        .build();
    let records_tab = gtk4::ToggleButton::builder()
        .label(copy(locale, "detail.records"))
        .css_classes(["flat"])
        .visible(!create_mode)
        .build();
    records_tab.set_group(Some(&rule_tab));
    left.append(&rule_tab);
    left.append(&records_tab);
    top_bar.append(&left);

    let pause_btn = gtk4::Button::builder()
        .icon_name("media-playback-pause-symbolic")
        .css_classes(["flat"])
        .visible(!create_mode)
        .build();
    top_bar.append(&pause_btn);

    let more_popover = gtk4::Popover::new();
    let more_content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(2)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();
    let run_btn = gtk4::Button::builder()
        .label(copy(locale, "host.run"))
        .css_classes(["flat"])
        .build();
    more_content.append(&run_btn);
    let run_reason = gtk4::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .css_classes(["dsh-schedule-caption"])
        .build();
    more_content.append(&run_reason);
    let delete_btn = gtk4::Button::builder()
        .label(copy(locale, "delete.action"))
        .css_classes(["flat", "destructive-action"])
        .build();
    more_content.append(&delete_btn);
    more_popover.set_child(Some(&more_content));
    let more_button = gtk4::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .css_classes(["flat"])
        .popover(&more_popover)
        .visible(!create_mode)
        .build();
    top_bar.append(&more_button);

    let close_btn = gtk4::Button::builder()
        .icon_name("window-close-symbolic")
        .tooltip_text(copy(locale, "detail.close"))
        .css_classes(["flat"])
        .build();
    top_bar.append(&close_btn);
    root_box.append(&top_bar);

    let stack = gtk4::Stack::builder()
        .vexpand(true)
        .hhomogeneous(false)
        .build();

    let rules_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    let rules_content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .margin_bottom(12)
        .build();

    let subtitle = gtk4::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .visible(!create_mode)
        .css_classes(["dsh-schedule-caption"])
        .build();
    rules_content.append(&subtitle);
    let notices = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .build();
    rules_content.append(&notices);
    let conflict = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .css_classes(["dsh-schedule-conflict"])
        .visible(false)
        .build();
    conflict.append(
        &gtk4::Label::builder()
            .label(copy(locale, "host.conflict"))
            .wrap(true)
            .xalign(0.0)
            .build(),
    );
    let reapply_btn = gtk4::Button::builder()
        .label(copy(locale, "host.reapply"))
        .css_classes(["flat"])
        .halign(gtk4::Align::Start)
        .build();
    conflict.append(&reapply_btn);
    rules_content.append(&conflict);

    let name_group = adw::PreferencesGroup::new();
    let title_entry = adw::EntryRow::builder()
        .title(copy(locale, "detail.name"))
        .build();
    name_group.add(&title_entry);
    rules_content.append(&name_group);

    rules_content.append(
        &gtk4::Label::builder()
            .label(copy(locale, "detail.instruction"))
            .css_classes(["dim-label", "caption"])
            .xalign(0.0)
            .build(),
    );
    let prompt_view = gtk4::TextView::builder()
        .wrap_mode(gtk4::WrapMode::WordChar)
        .accepts_tab(false)
        .build();
    let prompt_buffer = prompt_view.buffer();
    let prompt_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(112)
        .max_content_height(220)
        .child(&prompt_view)
        .build();
    rules_content.append(&prompt_scroll);

    let target_group = adw::PreferencesGroup::new();
    let mut target_labels_vec: Vec<&'static str> = vec![
        target_labels(TargetChoice::NewOnce, locale),
        target_labels(TargetChoice::New, locale),
    ];
    if has_session {
        target_labels_vec.insert(0, target_labels(TargetChoice::Session, locale));
    }
    let target_combo = adw::ComboRow::builder()
        .title(copy(locale, "host.target"))
        .model(&gtk4::StringList::new(&target_labels_vec))
        .build();
    target_group.add(&target_combo);
    let mut workspace_titles: Vec<String> = workspace_paths
        .iter()
        .map(|path| {
            controller_of(state)
                .and_then(|ctrl| {
                    ctrl.workspace_choices()
                        .into_iter()
                        .find(|workspace| &workspace.path == path)
                })
                .map(|workspace| workspace.title.clone())
                .unwrap_or_else(|| path.clone())
        })
        .collect();
    if workspace_titles.is_empty() {
        workspace_titles.push(copy(locale, "host.ungrouped").to_string());
    }
    let workspace_refs: Vec<&str> = workspace_titles.iter().map(String::as_str).collect();
    let workspace_combo = adw::ComboRow::builder()
        .title(copy(locale, "host.project"))
        .model(&gtk4::StringList::new(&workspace_refs))
        .build();
    target_group.add(&workspace_combo);
    rules_content.append(&target_group);

    let rule_group = adw::PreferencesGroup::builder()
        .title(copy(locale, "rule.title"))
        .build();
    let repeat_combo = adw::ComboRow::builder()
        .title(copy(locale, "rule.repeat"))
        .model(&gtk4::StringList::new(&frequency_labels(locale)))
        .build();
    rule_group.add(&repeat_combo);
    let interval = adw::SpinRow::with_range(1.0, 99_999.0, 1.0);
    interval.set_title(copy(locale, "host.interval"));
    rule_group.add(&interval);

    let weekday_row = adw::ActionRow::builder()
        .title(copy(locale, "rule.weekday"))
        .build();
    let weekday_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(4)
        .valign(gtk4::Align::Center)
        .build();
    let mut weekday_toggles = Vec::new();
    for day in 1..=7i64 {
        let toggle = gtk4::ToggleButton::builder()
            .label(copy(locale, &format!("weekday.{day}")))
            .css_classes(["flat", "pill"])
            .build();
        weekday_box.append(&toggle);
        weekday_toggles.push(toggle);
    }
    weekday_row.add_suffix(&weekday_box);
    rule_group.add(&weekday_row);

    let month_day = adw::SpinRow::with_range(1.0, 31.0, 1.0);
    month_day.set_title(copy(locale, "host.monthDay"));
    rule_group.add(&month_day);

    let date_popover = gtk4::Popover::new();
    let calendar = gtk4::Calendar::new();
    date_popover.set_child(Some(&calendar));
    let date_label = gtk4::Label::new(None);
    let date_row = adw::ActionRow::builder()
        .title(copy(locale, "timing.date"))
        .build();
    let date_button = gtk4::MenuButton::builder()
        .css_classes(["flat"])
        .popover(&date_popover)
        .valign(gtk4::Align::Center)
        .build();
    date_button.set_child(Some(&date_label));
    date_row.add_suffix(&date_button);
    rule_group.add(&date_row);

    let time_popover = gtk4::Popover::new();
    let time_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .build();
    let hour_spin = gtk4::SpinButton::with_range(0.0, 23.0, 1.0);
    let minute_spin = gtk4::SpinButton::with_range(0.0, 59.0, 1.0);
    let second_spin = gtk4::SpinButton::with_range(0.0, 59.0, 1.0);
    time_box.append(&hour_spin);
    time_box.append(&minute_spin);
    time_box.append(&second_spin);
    time_popover.set_child(Some(&time_box));
    let time_label = gtk4::Label::new(None);
    let time_row = adw::ActionRow::builder()
        .title(copy(locale, "timing.time"))
        .build();
    let time_button = gtk4::MenuButton::builder()
        .css_classes(["flat"])
        .popover(&time_popover)
        .valign(gtk4::Align::Center)
        .build();
    time_button.set_child(Some(&time_label));
    time_row.add_suffix(&time_button);
    rule_group.add(&time_row);

    let zone_popover = gtk4::Popover::new();
    let zone_content = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(8)
        .margin_bottom(8)
        .margin_start(8)
        .margin_end(8)
        .width_request(320)
        .height_request(360)
        .build();
    let zone_search = gtk4::SearchEntry::builder()
        .placeholder_text(copy(locale, "timing.zoneSearch"))
        .build();
    zone_content.append(&zone_search);
    let zone_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .build();
    let zone_list = gtk4::ListBox::builder()
        .selection_mode(gtk4::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    zone_scroll.set_child(Some(&zone_list));
    zone_content.append(&zone_scroll);
    zone_popover.set_child(Some(&zone_content));
    for zone_name in available_zones() {
        let row = adw::ActionRow::builder().title(zone_name.clone()).build();
        let zone_button = gtk4::Button::builder()
            .label(zone_label(&zone_name, locale, now_ms()))
            .css_classes(["flat", "caption"])
            .valign(gtk4::Align::Center)
            .build();
        row.add_suffix(&zone_button);
        zone_list.append(&row);
        let state_for_zone = state.clone();
        let zone_for_pick = zone_name.clone();
        let popover_for_zone = zone_popover.clone();
        zone_button.connect_clicked(move |_| {
            state_for_zone
                .detail
                .borrow()
                .draft
                .borrow_mut()
                .timing
                .time_zone = zone_for_pick.clone();
            popover_for_zone.popdown();
            refresh_form(&state_for_zone);
        });
        let row_for_search = row.clone();
        let name_for_search = zone_name.clone();
        zone_search.connect_search_changed(move |entry| {
            row_for_search.set_visible(zone_matches(&name_for_search, &entry.text()));
        });
    }
    let zone_label_widget = gtk4::Label::new(None);
    let zone_row = adw::ActionRow::builder()
        .title(copy(locale, "timing.zone"))
        .build();
    let zone_button = gtk4::MenuButton::builder()
        .css_classes(["flat"])
        .popover(&zone_popover)
        .valign(gtk4::Align::Center)
        .build();
    zone_button.set_child(Some(&zone_label_widget));
    zone_row.add_suffix(&zone_button);
    rule_group.add(&zone_row);

    rules_content.append(&rule_group);

    let custom_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .visible(false)
        .build();
    custom_box.append(
        &gtk4::Label::builder()
            .label(copy(locale, "host.customRule"))
            .css_classes(["dim-label", "caption"])
            .xalign(0.0)
            .build(),
    );
    let custom_view = gtk4::TextView::builder()
        .wrap_mode(gtk4::WrapMode::WordChar)
        .css_classes(["monospace"])
        .build();
    let custom_buffer = custom_view.buffer();
    let custom_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .min_content_height(96)
        .max_content_height(200)
        .child(&custom_view)
        .build();
    custom_box.append(&custom_scroll);
    rules_content.append(&custom_box);

    let limits_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .visible(false)
        .build();
    let limits_label = gtk4::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .css_classes(["dsh-schedule-caption"])
        .hexpand(true)
        .build();
    limits_box.append(&limits_label);
    let limits_retry = gtk4::Button::builder()
        .label(copy(locale, "list.retry"))
        .css_classes(["flat"])
        .visible(false)
        .build();
    limits_box.append(&limits_retry);
    rules_content.append(&limits_box);

    let issue_label = gtk4::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .css_classes(["dsh-schedule-failure"])
        .build();
    rules_content.append(&issue_label);
    let failure_label = gtk4::Label::builder()
        .wrap(true)
        .xalign(0.0)
        .visible(false)
        .css_classes(["dsh-schedule-failure"])
        .build();
    rules_content.append(&failure_label);

    rules_scroll.set_child(Some(&rules_content));
    stack.add_named(&rules_scroll, Some("rule"));

    let records_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    let records_box = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(10)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .margin_bottom(12)
        .build();
    records_scroll.set_child(Some(&records_box));
    stack.add_named(&records_scroll, Some("records"));
    root_box.append(&stack);

    let session_footer = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(8)
        .margin_bottom(4)
        .visible(!create_mode)
        .build();
    let session_title_label = gtk4::Label::builder()
        .label(copy(locale, "detail.session"))
        .css_classes(["dim-label"])
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .build();
    session_footer.append(&session_title_label);
    let session_open_btn = gtk4::Button::builder().css_classes(["flat"]).build();
    session_footer.append(&session_open_btn);
    let session_pending_label = gtk4::Label::builder()
        .label(copy(locale, "host.sessionAfterFirstRun"))
        .css_classes(["dim-label"])
        .visible(false)
        .build();
    session_footer.append(&session_pending_label);
    root_box.append(&session_footer);

    let save_footer = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .margin_start(16)
        .margin_end(16)
        .margin_top(4)
        .margin_bottom(12)
        .build();
    let unsaved_label = gtk4::Label::builder()
        .css_classes(["dim-label"])
        .hexpand(true)
        .halign(gtk4::Align::Start)
        .build();
    save_footer.append(&unsaved_label);
    let discard_btn = gtk4::Button::builder()
        .label(copy(locale, "rule.cancel"))
        .build();
    save_footer.append(&discard_btn);
    let save_btn = gtk4::Button::builder()
        .label(copy(
            locale,
            if create_mode {
                "host.create"
            } else {
                "rule.save"
            },
        ))
        .css_classes(["suggested-action"])
        .build();
    save_footer.append(&save_btn);
    root_box.append(&save_footer);

    let tab = state.detail.borrow().tab;
    stack.set_visible_child_name(match tab {
        DetailTab::Rule => "rule",
        DetailTab::Records => "records",
    });

    let widgets = DetailWidgets {
        header_title: header_title.clone(),
        subtitle: subtitle.clone(),
        notices: notices.clone(),
        conflict: conflict.clone(),
        issue_label: issue_label.clone(),
        failure_label: failure_label.clone(),
        limits_box: limits_box.clone(),
        limits_label: limits_label.clone(),
        limits_retry: limits_retry.clone(),
        pause_btn: pause_btn.clone(),
        run_btn: run_btn.clone(),
        run_reason: run_reason.clone(),
        delete_btn: delete_btn.clone(),
        close_btn: close_btn.clone(),
        more_button: more_button.clone(),
        save_btn: save_btn.clone(),
        discard_btn: discard_btn.clone(),
        unsaved_label: unsaved_label.clone(),
        save_footer: save_footer.clone(),
        session_footer: session_footer.clone(),
        session_title_label: session_title_label.clone(),
        session_open_btn: session_open_btn.clone(),
        session_pending_label: session_pending_label.clone(),
        title_entry: title_entry.clone(),
        prompt_buffer: prompt_buffer.clone(),
        target_combo: target_combo.clone(),
        workspace_combo: workspace_combo.clone(),
        repeat_combo: repeat_combo.clone(),
        stack: stack.clone(),
        records_box: records_box.clone(),
        rules: RuleRows {
            interval: interval.clone(),
            weekday_row: weekday_row.clone(),
            weekday_toggles: weekday_toggles.clone(),
            month_day: month_day.clone(),
            date_row: date_row.clone(),
            date_label: date_label.clone(),
            time_row: time_row.clone(),
            time_label: time_label.clone(),
            zone_row: zone_row.clone(),
            zone_label: zone_label_widget.clone(),
            custom_box: custom_box.clone(),
            custom_buffer: custom_buffer.clone(),
        },
    };

    {
        let draft = state.detail.borrow().draft.borrow().clone();
        sync_form(&widgets, &draft, has_session, &workspace_paths);
    }
    state.detail.borrow_mut().widgets = Some(widgets);

    let state_for_title = state.clone();
    title_entry.connect_changed(move |entry: &adw::EntryRow| {
        let text = entry.text().to_string();
        if text.chars().count() > 120 {
            entry.set_text(&clamp_title(&text));
            return;
        }
        state_for_title.detail.borrow().draft.borrow_mut().title = text;
        refresh_form(&state_for_title);
    });
    let state_for_prompt = state.clone();
    prompt_buffer.connect_changed(move |buffer| {
        state_for_prompt.detail.borrow().draft.borrow_mut().prompt = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string();
        refresh_form(&state_for_prompt);
    });
    let state_for_target = state.clone();
    target_combo.connect_selected_notify(move |combo| {
        let choice = target_choice_of_row(combo.selected() as usize, has_session);
        {
            let detail = state_for_target.detail.borrow();
            let mut draft = detail.draft.borrow_mut();
            if choice == TargetChoice::Session {
                draft.session_id = current_session_id.clone().unwrap_or_default();
                draft.cwd = String::new();
            } else if draft.target == TargetChoice::Session {
                draft.cwd = controller_of(&state_for_target)
                    .map(|ctrl| default_cwd(&ctrl.workspace_choices()))
                    .unwrap_or_default();
                draft.session_id = String::new();
            }
            draft.target = choice;
        }
        refresh_form(&state_for_target);
    });
    let state_for_workspace = state.clone();
    let workspace_paths_for_combo = workspace_paths.clone();
    workspace_combo.connect_selected_notify(move |combo| {
        if let Some(path) = workspace_paths_for_combo.get(combo.selected() as usize) {
            state_for_workspace.detail.borrow().draft.borrow_mut().cwd = path.clone();
            refresh_form(&state_for_workspace);
        }
    });
    let state_for_repeat = state.clone();
    let has_original = !create_mode;
    repeat_combo.connect_selected_notify(move |combo| {
        let detail = state_for_repeat.detail.borrow();
        let mut draft = detail.draft.borrow_mut();
        apply_frequency_row(
            &mut draft,
            combo.selected() as usize,
            has_original,
            now_ms(),
        );
        drop(draft);
        drop(detail);
        refresh_form(&state_for_repeat);
    });
    let state_for_interval = state.clone();
    interval.connect_changed(move |row| {
        state_for_interval
            .detail
            .borrow()
            .draft
            .borrow_mut()
            .timing
            .interval = row.text().to_string();
        refresh_form(&state_for_interval);
    });
    for (index, toggle) in weekday_toggles.into_iter().enumerate() {
        let state_for_day = state.clone();
        toggle.connect_toggled(move |toggle| {
            let day = (index + 1) as i64;
            {
                let detail = state_for_day.detail.borrow();
                let mut draft = detail.draft.borrow_mut();
                if toggle.is_active() {
                    if !draft.timing.weekdays.contains(&day) {
                        draft.timing.weekdays.push(day);
                        draft.timing.weekdays.sort_unstable();
                    }
                } else {
                    draft.timing.weekdays.retain(|value| *value != day);
                }
            }
            refresh_form(&state_for_day);
        });
    }
    let state_for_date = state.clone();
    calendar.connect_day_selected(move |calendar| {
        let date = calendar.date();
        let text = format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            date.month(),
            date.day_of_month()
        );
        state_for_date
            .detail
            .borrow()
            .draft
            .borrow_mut()
            .timing
            .start
            .replace_range(..10, &text);
        refresh_form(&state_for_date);
    });
    let state_for_time = state.clone();
    let write_time = {
        let hour = hour_spin.clone();
        let minute = minute_spin.clone();
        let second = second_spin.clone();
        move |state: &Rc<PageState>| {
            let clock = format!(
                "{:02}:{:02}:{:02}",
                hour.value() as i64,
                minute.value() as i64,
                second.value() as i64
            );
            let detail = state.detail.borrow();
            let mut draft = detail.draft.borrow_mut();
            if draft.timing.frequency == FrequencyPreset::Daily {
                draft.timing.daily_time = clock[..5].to_string();
            } else {
                draft.timing.start.replace_range(11..19, &clock);
            }
            drop(draft);
            drop(detail);
            refresh_form(state);
        }
    };
    let write_time_for_hour = write_time.clone();
    hour_spin.connect_value_changed(move |_| write_time_for_hour(&state_for_time));
    let state_for_time_minute = state.clone();
    let write_time_for_minute = write_time.clone();
    minute_spin.connect_value_changed(move |_| write_time_for_minute(&state_for_time_minute));
    let state_for_time_second = state.clone();
    let write_time_for_second = write_time.clone();
    second_spin.connect_value_changed(move |_| write_time_for_second(&state_for_time_second));
    let state_for_custom = state.clone();
    custom_buffer.connect_changed(move |buffer| {
        state_for_custom
            .detail
            .borrow()
            .draft
            .borrow_mut()
            .timing
            .rrule = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), true)
            .to_string();
        refresh_form(&state_for_custom);
    });

    let state_for_save = state.clone();
    save_btn.connect_clicked(move |_| {
        save_detail(&state_for_save);
    });
    let state_for_discard = state.clone();
    discard_btn.connect_clicked(move |_| {
        discard_detail(&state_for_discard);
    });
    let state_for_close = state.clone();
    close_btn.connect_clicked(move |_| {
        close_detail(&state_for_close);
    });
    let state_for_pause = state.clone();
    pause_btn.connect_clicked(move |_| {
        toggle_pause(&state_for_pause);
    });
    let state_for_run = state.clone();
    run_btn.connect_clicked(move |_| {
        run_task(&state_for_run);
    });
    let state_for_delete = state.clone();
    delete_btn.connect_clicked(move |_| {
        confirm_delete(&state_for_delete);
    });
    let state_for_reapply = state.clone();
    reapply_btn.connect_clicked(move |_| {
        reapply_latest(&state_for_reapply);
    });
    let state_for_limits = state.clone();
    limits_retry.connect_clicked(move |_| {
        if let Some(ctrl) = controller_of(&state_for_limits) {
            ctrl.schedule_delivery_limits();
        }
    });
    let state_for_rule_tab = state.clone();
    rule_tab.connect_toggled(move |toggle| {
        if toggle.is_active() {
            switch_tab(&state_for_rule_tab, DetailTab::Rule);
        }
    });
    let state_for_records_tab = state.clone();
    records_tab.connect_toggled(move |toggle| {
        if toggle.is_active() {
            switch_tab(&state_for_records_tab, DetailTab::Records);
        }
    });
    let state_for_session = state.clone();
    session_open_btn.connect_clicked(move |_| {
        let linked = {
            let detail = state_for_session.detail.borrow();
            detail.base.as_ref().and_then(task_session_id)
        };
        if let Some(id) = linked {
            if let Some(ctrl) = controller_of(&state_for_session) {
                ctrl.open_schedule_session(&id);
            }
        }
    });
}

fn render_records(state: &Rc<PageState>) {
    let detail = state.detail.borrow();
    let Some(widgets) = detail.widgets.as_ref() else {
        return;
    };
    while let Some(child) = widgets.records_box.first_child() {
        widgets.records_box.remove(&child);
    }
    let locale = state.locale;
    if detail.history.loading {
        widgets
            .records_box
            .append(&gtk4::Label::new(Some(copy(locale, "delivery.loading"))));
        return;
    }
    if let Some(miss) = detail.history.miss {
        let miss_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::Center)
            .margin_top(24)
            .build();
        miss_box.append(
            &gtk4::Label::builder()
                .label(miss_text(miss, locale))
                .wrap(true)
                .build(),
        );
        let retry = gtk4::Button::builder()
            .label(copy(locale, "delivery.retry"))
            .css_classes(["flat"])
            .halign(gtk4::Align::Center)
            .build();
        miss_box.append(&retry);
        widgets.records_box.append(&miss_box);
        let state_for_retry = state.clone();
        retry.connect_clicked(move |_| {
            fetch_records(&state_for_retry, None);
        });
        return;
    }
    if detail.history.records.is_empty() {
        widgets
            .records_box
            .append(&gtk4::Label::new(Some(copy(locale, "delivery.empty"))));
        return;
    }
    let task_zone = detail
        .base
        .as_ref()
        .and_then(record_time_zone)
        .unwrap_or_else(|| state.zone.clone());
    let tz = zone_tz(&task_zone);
    for record in &detail.history.records {
        let row = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        row.append(
            &gtk4::Image::builder()
                .icon_name("alarm-symbolic")
                .css_classes(["dim-label"])
                .build(),
        );
        let body = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .hexpand(true)
            .build();
        let head = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(8)
            .build();
        head.append(
            &gtk4::Label::builder()
                .label(format_schedule_absolute(&record.scheduled_at, locale, &tz))
                .css_classes(["dsh-schedule-caption"])
                .halign(gtk4::Align::Start)
                .hexpand(true)
                .build(),
        );
        let session_id = record.target_session_id.clone();
        let title = controller_of(state)
            .and_then(|ctrl| ctrl.session_display_title(&session_id))
            .unwrap_or_else(|| copy(locale, "host.openDeliverySession").to_string());
        let available = controller_of(state)
            .map(|ctrl| linked_state(ctrl.session_archived(&session_id)))
            == Some(LinkedState::Available);
        let open = gtk4::Button::builder()
            .label(title)
            .sensitive(available)
            .css_classes(["flat"])
            .build();
        let state_for_open = state.clone();
        let session_for_open = session_id.clone();
        open.connect_clicked(move |_| {
            if let Some(ctrl) = controller_of(&state_for_open) {
                ctrl.open_schedule_session(&session_for_open);
            }
        });
        head.append(&open);
        body.append(&head);
        if let Some(prompt) = record.prompt.as_deref().filter(|text| !text.is_empty()) {
            let prompt_label = gtk4::Label::builder()
                .label(prompt)
                .wrap(true)
                .wrap_mode(gtk4::pango::WrapMode::WordChar)
                .ellipsize(gtk4::pango::EllipsizeMode::End)
                .lines(2)
                .xalign(0.0)
                .css_classes(["dsh-schedule-caption"])
                .build();
            body.append(&prompt_label);
            let toggle = gtk4::Button::builder()
                .label(copy(locale, "delivery.expand"))
                .css_classes(["flat"])
                .halign(gtk4::Align::Start)
                .build();
            body.append(&toggle);
            let label_for_toggle = prompt_label.clone();
            let button_for_toggle = toggle.clone();
            let locale_for_toggle = locale;
            toggle.connect_clicked(move |_| {
                if label_for_toggle.lines() == 0 {
                    label_for_toggle.set_lines(2);
                    button_for_toggle.set_label(copy(locale_for_toggle, "delivery.expand"));
                } else {
                    label_for_toggle.set_lines(0);
                    button_for_toggle.set_label(copy(locale_for_toggle, "delivery.collapse"));
                }
            });
        }
        row.append(&body);
        widgets.records_box.append(&row);
    }
    if let Some(before) = detail.history.next_before.clone() {
        let load_more = gtk4::Button::builder()
            .label(copy(locale, "delivery.loadMore"))
            .css_classes(["flat"])
            .halign(gtk4::Align::Start)
            .build();
        widgets.records_box.append(&load_more);
        let state_for_more = state.clone();
        load_more.connect_clicked(move |_| {
            fetch_records(&state_for_more, Some(before.clone()));
        });
    }
    if detail.history.pruned && detail.history.next_before.is_none() {
        let footer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(4)
            .margin_top(12)
            .build();
        footer.append(
            &gtk4::Label::builder()
                .label(copy(locale, "delivery.pruned"))
                .css_classes(["dsh-schedule-caption"])
                .build(),
        );
        if let Some(bounds) = detail.history.retention.clone() {
            footer.append(
                &gtk4::Label::builder()
                    .label(format!(
                        "{} {}",
                        fill(
                            copy(locale, "delivery.retentionBounds"),
                            &[
                                ("days", bounds.days.to_string()),
                                ("records", bounds.records.to_string()),
                            ],
                        ),
                        copy(locale, "delivery.retentionExplanation"),
                    ))
                    .wrap(true)
                    .css_classes(["dsh-schedule-caption"])
                    .build(),
            );
        }
        widgets.records_box.append(&footer);
    }
}
