use jiff::civil::{Date, DateTime, Time};
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{Timestamp, ToSpan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
const MAX_RRULE_COUNT: i64 = 1_000;
const MAX_PERIOD_CANDIDATES: i64 = 10_080;
const MAX_SET_POSITIONS: usize = 31;
const MIN_RRULE_YEAR: i64 = 100;
const CRON_TIME_LIST_LIMIT: usize = 6;
const WEEKDAY_CODES: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
const SUPPORTED_PARTS: [&str; 14] = [
    "FREQ", "INTERVAL", "COUNT", "UNTIL", "WKST", "BYSECOND", "BYMINUTE", "BYHOUR", "BYDAY",
    "BYMONTHDAY", "BYYEARDAY", "BYWEEKNO", "BYMONTH", "BYSETPOS",
];
const LIST_RANGES: [(&str, i64, i64); 8] = [
    ("BYSECOND", 0, 59),
    ("BYMINUTE", 0, 59),
    ("BYHOUR", 0, 23),
    ("BYMONTHDAY", -31, 31),
    ("BYYEARDAY", -366, 366),
    ("BYWEEKNO", -53, 53),
    ("BYMONTH", 1, 12),
    ("BYSETPOS", -366, 366),
];
const UNIT_SECONDS: [(TimeUnit, i64); 4] = [
    (TimeUnit::Day, 86_400),
    (TimeUnit::Hour, 3_600),
    (TimeUnit::Minute, 60),
    (TimeUnit::Second, 1),
];
const EN_MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const EN_MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
const ZH_MONTHS_LONG: [&str; 12] = [
    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleRecordKind {
    After,
    At,
    Every,
    Daily,
    Weekly,
    Cron,
    Rrule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleStatus {
    Active,
    Paused,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSessionCreateSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_preset: Option<String>,
}

impl Default for ScheduleSessionCreateSpec {
    fn default() -> Self {
        Self {
            cwd: None,
            agent_preset: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ScheduleTarget {
    #[serde(rename = "session")]
    Session {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "new_once")]
    NewOnce {
        #[serde(rename = "sessionId")]
        session_id: String,
        #[serde(default)]
        create: ScheduleSessionCreateSpec,
    },
    #[serde(rename = "new")]
    New {
        #[serde(default)]
        create: ScheduleSessionCreateSpec,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDeliveryReceipt {
    pub scheduled_at: String,
    pub delivered_at: String,
    pub message_id: String,
    pub target_session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDeliveryRecord {
    pub scheduled_at: String,
    pub delivered_at: String,
    pub message_id: String,
    pub target_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryRetentionBounds {
    pub days: i64,
    pub records: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleDeliveryHistoryPage {
    pub id: String,
    #[serde(default)]
    pub records: Vec<ScheduleDeliveryRecord>,
    #[serde(default)]
    pub earlier_records_unavailable: bool,
    #[serde(default)]
    pub earlier_records_pruned: bool,
    pub retention: DeliveryRetentionBounds,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_before: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleCatalogEntry {
    pub id: String,
    pub kind: ScheduleRecordKind,
    pub title: String,
    pub prompt: String,
    pub scheduled_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_seconds: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekdays: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rrule: Option<String>,
    pub target: ScheduleTarget,
    #[serde(default)]
    pub involved_session_ids: Vec<String>,
    pub status: ScheduleStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_floor_ends_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_delivery: Option<ScheduleDeliveryReceipt>,
}

pub fn scheduled_session_ids(entries: &[ScheduleCatalogEntry]) -> HashSet<String> {
    let mut sessions = HashSet::new();
    for entry in entries {
        if entry.status != ScheduleStatus::Active {
            continue;
        }
        match &entry.target {
            ScheduleTarget::Session { session_id }
            | ScheduleTarget::NewOnce { session_id, .. } => {
                sessions.insert(session_id.clone());
            }
            ScheduleTarget::New { .. } => {}
        }
        sessions.extend(entry.involved_session_ids.iter().cloned());
    }
    sessions
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleLocalAtInput {
    pub date: String,
    pub time: String,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleDailyInput {
    pub time: String,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleWeeklyInput {
    pub time: String,
    pub time_zone: String,
    pub weekdays: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleCronInput {
    pub expression: String,
    pub time_zone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScheduleAtInput {
    Instant(String),
    Local(ScheduleLocalAtInput),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ScheduleTiming {
    #[serde(rename = "after")]
    After { after_seconds: i64 },
    #[serde(rename = "at")]
    At { at: ScheduleAtInput },
    #[serde(rename = "every")]
    Every { every_seconds: i64 },
    #[serde(rename = "daily")]
    Daily { daily: ScheduleDailyInput },
    #[serde(rename = "weekly")]
    Weekly { weekly: ScheduleWeeklyInput },
    #[serde(rename = "cron")]
    Cron { cron: ScheduleCronInput },
    #[serde(rename = "rrule")]
    Rrule { rrule: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ScheduleTargetRequest {
    #[serde(rename = "session")]
    Session {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    #[serde(rename = "new_once")]
    NewOnce {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        create: Option<ScheduleSessionCreateSpec>,
    },
    #[serde(rename = "new")]
    New {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        create: Option<ScheduleSessionCreateSpec>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<ScheduleTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ScheduleTargetRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyPreset {
    Once,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyKind {
    Once,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom,
}

impl FrequencyKind {
    fn preset(self) -> FrequencyPreset {
        match self {
            FrequencyKind::Once => FrequencyPreset::Once,
            FrequencyKind::Minutely => FrequencyPreset::Minutely,
            FrequencyKind::Hourly => FrequencyPreset::Hourly,
            FrequencyKind::Daily => FrequencyPreset::Daily,
            FrequencyKind::Weekly => FrequencyPreset::Weekly,
            FrequencyKind::Monthly => FrequencyPreset::Monthly,
            FrequencyKind::Custom => FrequencyPreset::Advanced,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetChoice {
    Session,
    NewOnce,
    New,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftIssue {
    Incomplete,
    InvalidTitle,
    InvalidInterval,
    RequiredWeekday,
    InvalidMonthDay,
    InvalidTime,
    InvalidZone,
    TooComplex,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Day,
    Hour,
    Minute,
    Second,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    Zh,
}

impl Locale {
    pub fn from_tag(tag: &str) -> Self {
        let language = tag.trim().to_lowercase();
        let language = language.split(['.', '@']).next().unwrap_or("");
        let language = language.replace('_', "-");
        let language = language.split('-').next().unwrap_or("");
        if language == "zh" {
            Locale::Zh
        } else {
            Locale::En
        }
    }
}

#[derive(Clone, Copy)]
pub struct FrequencyZone<'a> {
    pub system: &'a str,
}

pub struct NextRunParts {
    pub absolute: String,
    pub relative: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimingDraft {
    pub frequency: FrequencyPreset,
    pub interval: String,
    pub start: String,
    pub time_zone: String,
    pub rrule: String,
    pub retained_timing: Option<ScheduleTiming>,
    pub weekdays: Vec<i64>,
    pub month_day: i64,
    pub daily_time: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskEditorDraft {
    pub timing: TimingDraft,
    pub title: String,
    pub prompt: String,
    pub target: TargetChoice,
    pub session_id: String,
    pub cwd: String,
}

fn fill(template: &str, params: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (key, value) in params {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

fn copy(locale: Locale, key: &str) -> &'static str {
    match (locale, key) {
        (Locale::En, "frequency.once") => "Once",
        (Locale::Zh, "frequency.once") => "仅一次",
        (Locale::En, "frequency.every") => "Every {value} {unit}",
        (Locale::Zh, "frequency.every") => "每 {value} {unit}",
        (Locale::En, "frequency.daily") => "Daily at {time} ({timeZone})",
        (Locale::Zh, "frequency.daily") => "每天 {time}（{timeZone}）",
        (Locale::En, "frequency.dailyLocal") => "Daily at {time}",
        (Locale::Zh, "frequency.dailyLocal") => "每天 {time}",
        (Locale::En, "frequency.weekly") => "Weekly on {weekdays} at {time} ({timeZone})",
        (Locale::Zh, "frequency.weekly") => "每周{weekdays} {time}（{timeZone}）",
        (Locale::En, "frequency.weeklyLocal") => "Weekly on {weekdays} at {time}",
        (Locale::Zh, "frequency.weeklyLocal") => "每周{weekdays} {time}",
        (Locale::En, "frequency.cron") => "Custom ({timeZone})",
        (Locale::Zh, "frequency.cron") => "自定义（{timeZone}）",
        (Locale::En, "frequency.cronLocal") => "Custom",
        (Locale::Zh, "frequency.cronLocal") => "自定义",
        (Locale::En, "frequency.cronRule") => "{rule} ({timeZone})",
        (Locale::Zh, "frequency.cronRule") => "{rule}（{timeZone}）",
        (Locale::En, "frequency.hostMonthly") => "Monthly on day {days} at {time}, or month end",
        (Locale::Zh, "frequency.hostMonthly") => "每月 {days} 日 {time}，不足该日则月末",
        (Locale::En, "frequency.hostDailyInterval") => "Every {interval} days at {time}",
        (Locale::Zh, "frequency.hostDailyInterval") => "每 {interval} 天 {time}",
        (Locale::En, "frequency.hostWeeklyInterval") => {
            "Every {interval} weeks on {weekdays} at {time}"
        }
        (Locale::Zh, "frequency.hostWeeklyInterval") => "每 {interval} 周的周{weekdays} {time}",
        (Locale::En, "frequency.hostMonthlyInterval") => {
            "Every {interval} months on day {days} at {time}, or month end"
        }
        (Locale::Zh, "frequency.hostMonthlyInterval") => {
            "每 {interval} 个月的 {days} 日 {time}，不足该日则月末"
        }
        (Locale::En, "frequency.hostZone") => "{rule} ({timeZone})",
        (Locale::Zh, "frequency.hostZone") => "{rule}（{timeZone}）",
        (Locale::En, "frequency.hostCustom") => "Custom",
        (Locale::Zh, "frequency.hostCustom") => "自定义",
        (Locale::En, "frequency.weekday.join") => ", ",
        (Locale::Zh, "frequency.weekday.join") => "、",
        (Locale::En, "frequency.weekday.1") => "Mon",
        (Locale::Zh, "frequency.weekday.1") => "一",
        (Locale::En, "frequency.weekday.2") => "Tue",
        (Locale::Zh, "frequency.weekday.2") => "二",
        (Locale::En, "frequency.weekday.3") => "Wed",
        (Locale::Zh, "frequency.weekday.3") => "三",
        (Locale::En, "frequency.weekday.4") => "Thu",
        (Locale::Zh, "frequency.weekday.4") => "四",
        (Locale::En, "frequency.weekday.5") => "Fri",
        (Locale::Zh, "frequency.weekday.5") => "五",
        (Locale::En, "frequency.weekday.6") => "Sat",
        (Locale::Zh, "frequency.weekday.6") => "六",
        (Locale::En, "frequency.weekday.7") => "Sun",
        (Locale::Zh, "frequency.weekday.7") => "日",
        (Locale::En, "unit.day.one") => "day",
        (Locale::Zh, "unit.day.one") => "天",
        (Locale::En, "unit.day.other") => "days",
        (Locale::Zh, "unit.day.other") => "天",
        (Locale::En, "unit.hour.one") => "hour",
        (Locale::Zh, "unit.hour.one") => "小时",
        (Locale::En, "unit.hour.other") => "hours",
        (Locale::Zh, "unit.hour.other") => "小时",
        (Locale::En, "unit.minute.one") => "minute",
        (Locale::Zh, "unit.minute.one") => "分钟",
        (Locale::En, "unit.minute.other") => "minutes",
        (Locale::Zh, "unit.minute.other") => "分钟",
        (Locale::En, "unit.second.one") => "second",
        (Locale::Zh, "unit.second.one") => "秒",
        (Locale::En, "unit.second.other") => "seconds",
        (Locale::Zh, "unit.second.other") => "秒",
        (Locale::En, "relative.now") => "Due now",
        (Locale::Zh, "relative.now") => "现在到期",
        (Locale::En, "relative.future") => "in {value} {unit}",
        (Locale::Zh, "relative.future") => "{value}{unit}后",
        (Locale::En, "relative.overdue") => "{value} {unit} overdue",
        (Locale::Zh, "relative.overdue") => "已逾期 {value}{unit}",
        (Locale::En, "time.utcPrefix") => "UTC",
        (Locale::Zh, "time.utcPrefix") => "UTC",
        (Locale::En, "cron.list.join") => ", ",
        (Locale::Zh, "cron.list.join") => "、",
        (Locale::En, "cron.part.join") => " ",
        (Locale::Zh, "cron.part.join") => " ",
        (Locale::En, "cron.weekday.name") => "{weekday}",
        (Locale::Zh, "cron.weekday.name") => "周{weekday}",
        (Locale::En, "cron.weekday.range") => "{from}–{to}",
        (Locale::Zh, "cron.weekday.range") => "{from}至{to}",
        (Locale::En, "cron.months") => " in {months}",
        (Locale::Zh, "cron.months") => "（{months}）",
        (Locale::En, "cron.day.every") => "Every day{months}",
        (Locale::Zh, "cron.day.every") => "每天{months}",
        (Locale::En, "cron.day.weekdays") => "{weekdays}{months}",
        (Locale::Zh, "cron.day.weekdays") => "{weekdays}{months}",
        (Locale::En, "cron.day.monthDays") => "Day {days} of every month{months}",
        (Locale::Zh, "cron.day.monthDays") => "每月 {days} 日{months}",
        (Locale::En, "cron.day.both") => "Day {days} of every month or {weekdays}{months}",
        (Locale::Zh, "cron.day.both") => "每月 {days} 日或{weekdays}{months}",
        (Locale::En, "cron.day.bothStarred") => "Day {days} of every month and {weekdays}{months}",
        (Locale::Zh, "cron.day.bothStarred") => "每月 {days} 日且{weekdays}{months}",
        (Locale::En, "cron.hours.range") => "{from}–{to}",
        (Locale::Zh, "cron.hours.range") => "{from} 至 {to}",
        (Locale::En, "cron.hours.list") => "{hours}",
        (Locale::Zh, "cron.hours.list") => "{hours}",
        (Locale::En, "cron.time.everyMinute") => "Every minute",
        (Locale::Zh, "cron.time.everyMinute") => "每分钟",
        (Locale::En, "cron.time.everyMinutes") => "Every {step} minutes",
        (Locale::Zh, "cron.time.everyMinutes") => "每 {step} 分钟",
        (Locale::En, "cron.time.joinedEveryMinute") => "every minute",
        (Locale::Zh, "cron.time.joinedEveryMinute") => "每分钟",
        (Locale::En, "cron.time.joinedEveryMinutes") => "every {step} minutes",
        (Locale::Zh, "cron.time.joinedEveryMinutes") => "每 {step} 分钟",
        (Locale::En, "cron.time.everyHour") => "Every hour",
        (Locale::Zh, "cron.time.everyHour") => "每小时",
        (Locale::En, "cron.time.joinedEveryHour") => "every hour",
        (Locale::Zh, "cron.time.joinedEveryHour") => "每小时",
        (Locale::En, "cron.time.everyNHours") => "Every {count} hours",
        (Locale::Zh, "cron.time.everyNHours") => "每 {count} 小时",
        (Locale::En, "cron.time.joinedEveryNHours") => "every {count} hours",
        (Locale::Zh, "cron.time.joinedEveryNHours") => "每 {count} 小时",
        (Locale::En, "cron.time.hourlyAt") => "Every hour at minute {minutes}",
        (Locale::Zh, "cron.time.hourlyAt") => "每小时的第 {minutes} 分钟",
        (Locale::En, "cron.time.joinedHourlyAt") => "every hour at minute {minutes}",
        (Locale::Zh, "cron.time.joinedHourlyAt") => "每小时的第 {minutes} 分钟",
        (Locale::En, "cron.time.hoursEveryMinute") => "every minute during hours {hours}",
        (Locale::Zh, "cron.time.hoursEveryMinute") => "{hours} 点的每分钟",
        (Locale::En, "cron.time.hoursEveryMinutes") => "every {step} minutes during hours {hours}",
        (Locale::Zh, "cron.time.hoursEveryMinutes") => "{hours} 点内每 {step} 分钟",
        (Locale::En, "cron.time.at") => "at {times}",
        (Locale::Zh, "cron.time.at") => "{times}",
        (Locale::En, "cron.time.hoursAt") => "at minute {minutes} of hours {hours}",
        (Locale::Zh, "cron.time.hoursAt") => "{hours} 点的第 {minutes} 分钟",
        _ => "",
    }
}

fn unit_label(locale: Locale, unit: TimeUnit, value: i64) -> String {
    let plural = if value == 1 { "one" } else { "other" };
    let key = match unit {
        TimeUnit::Day => "day",
        TimeUnit::Hour => "hour",
        TimeUnit::Minute => "minute",
        TimeUnit::Second => "second",
    };
    copy(locale, &format!("unit.{key}.{plural}")).to_string()
}

fn weekday_name(locale: Locale, weekday: i64) -> &'static str {
    copy(locale, &format!("frequency.weekday.{}", weekday.clamp(1, 7)))
}

fn month_long_name(locale: Locale, month: i64) -> &'static str {
    let index = month.clamp(1, 12) as usize - 1;
    match locale {
        Locale::En => EN_MONTHS_LONG[index],
        Locale::Zh => ZH_MONTHS_LONG[index],
    }
}

fn parse_epoch_ms(text: &str) -> Option<i64> {
    text.parse::<Timestamp>().ok().map(|ts| ts.as_millisecond())
}

fn resolve_zone(name: &str) -> TimeZone {
    TimeZone::get(name).unwrap_or(TimeZone::UTC)
}

fn canonical_zone_name(name: &str) -> Option<String> {
    TimeZone::get(name)
        .ok()
        .and_then(|tz| tz.iana_name().map(str::to_string))
}

fn civil_text(dt: DateTime) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        dt.year(),
        i8::from(dt.month()),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

fn start_in_zone(zone_name: &str, at_ms: i64) -> String {
    let tz = resolve_zone(zone_name);
    let ts = Timestamp::from_millisecond(at_ms).unwrap_or(Timestamp::UNIX_EPOCH);
    civil_text(tz.to_datetime(ts))
}

fn segment(text: &str, start: usize, end: usize) -> &str {
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) || start > end {
        return "";
    }
    let start = start.min(text.len());
    let end = end.clamp(start, text.len());
    &text[start..end]
}

fn parse_leading_int(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let start = match bytes.first() {
        Some(b'+') | Some(b'-') => 1,
        _ => 0,
    };
    let digits_end = start + text[start..].chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_end == start {
        return None;
    }
    text[..digits_end].parse().ok()
}

fn number_of(text: &str) -> i64 {
    parse_leading_int(text).unwrap_or(0)
}

fn clock_label(time: &str) -> String {
    let mut text = time.to_string();
    if let Some(dot) = text.rfind('.') {
        let fraction = &text[dot + 1..];
        if !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()) {
            text.truncate(dot);
        }
    }
    if text.ends_with(":00") {
        text.truncate(text.len() - 3);
    }
    text
}

fn utf16_length(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

fn is_hhmm(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    let hour = match bytes[0] {
        b'0' | b'1' => bytes[1].is_ascii_digit(),
        b'2' => (b'0'..=b'3').contains(&bytes[1]),
        _ => false,
    };
    hour && (b'0'..=b'5').contains(&bytes[3]) && bytes[4].is_ascii_digit()
}

fn is_safe_positive_integer(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes[0] == b'0' || !bytes.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let value: u128 = text.parse().unwrap_or(u128::MAX);
    value as i128 <= MAX_SAFE_INTEGER
}

struct LocalCalendar {
    year: i16,
    month: i8,
    day: i8,
    hour: i8,
    minute: i8,
    second: i8,
    millisecond: i16,
}

fn parse_local_calendar(date_text: &str, time_text: &str) -> Option<LocalCalendar> {
    let date_bytes = date_text.as_bytes();
    if date_text.len() != 10
        || date_bytes[4] != b'-'
        || date_bytes[7] != b'-'
        || !(0..10).all(|index| index == 4 || index == 7 || date_bytes[index].is_ascii_digit())
    {
        return None;
    }
    let time_bytes = time_text.as_bytes();
    if time_text.len() < 8
        || time_bytes[2] != b':'
        || time_bytes[5] != b':'
        || !(0..8).all(|index| index == 2 || index == 5 || time_bytes[index].is_ascii_digit())
    {
        return None;
    }
    let millisecond = if time_text.len() == 8 {
        0
    } else {
        let fraction = &time_text[9..];
        if time_bytes[8] != b'.' || fraction.is_empty() || fraction.len() > 3 || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        format!("{:0<3}", fraction).parse().ok()?
    };
    let year: i64 = date_text[0..4].parse().ok()?;
    let month: i64 = date_text[5..7].parse().ok()?;
    let day: i64 = date_text[8..10].parse().ok()?;
    let hour: i64 = time_text[0..2].parse().ok()?;
    let minute: i64 = time_text[3..5].parse().ok()?;
    let second: i64 = time_text[6..8].parse().ok()?;
    if year == 0 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Date::new(year as i16, month as i8, day as i8).ok()?;
    Some(LocalCalendar {
        year: year as i16,
        month: month as i8,
        day: day as i8,
        hour: hour as i8,
        minute: minute as i8,
        second: second as i8,
        millisecond: millisecond as i16,
    })
}

fn resolve_local_instant(tz: &TimeZone, calendar: &LocalCalendar) -> Option<i64> {
    let date = Date::new(calendar.year, calendar.month, calendar.day).ok()?;
    let time = Time::new(
        calendar.hour,
        calendar.minute,
        calendar.second,
        calendar.millisecond as i32 * 1_000_000,
    )
    .ok()?;
    let dt = DateTime::from_parts(date, time);
    let ambiguous = tz.to_ambiguous_zoned(dt);
    if matches!(ambiguous.offset(), AmbiguousOffset::Gap { .. }) {
        return None;
    }
    ambiguous.earlier().ok().map(|zoned| zoned.timestamp().as_millisecond())
}

pub fn task_next_run_at(task: &ScheduleCatalogEntry) -> String {
    let scheduled = parse_epoch_ms(&task.scheduled_at);
    let floor = task
        .delivery_floor_ends_at
        .as_deref()
        .and_then(parse_epoch_ms);
    match (scheduled, floor) {
        (Some(scheduled), Some(floor)) if floor > scheduled => {
            task.delivery_floor_ends_at.clone().unwrap_or_default()
        }
        _ => task.scheduled_at.clone(),
    }
}

pub fn task_session_id(task: &ScheduleCatalogEntry) -> Option<String> {
    match &task.target {
        ScheduleTarget::New { .. } => {
            task.last_delivery.as_ref().map(|receipt| receipt.target_session_id.clone())
        }
        ScheduleTarget::Session { session_id } | ScheduleTarget::NewOnce { session_id, .. } => {
            Some(session_id.clone())
        }
    }
}

pub fn rule_fields(rrule: &str) -> BTreeMap<String, String> {
    let mut fields = BTreeMap::new();
    let Some(line) = rrule.lines().find_map(|line| {
        line.strip_prefix("RRULE:").filter(|rest| !rest.is_empty())
    }) else {
        return fields;
    };
    for field in line.split(';') {
        match field.split_once('=') {
            Some((key, value)) => {
                fields.insert(key.to_string(), value.to_string());
            }
            None => {
                fields.insert(field.to_string(), String::new());
            }
        }
    }
    fields
}

fn dtstart_body(rrule: &str) -> Option<(&str, &str)> {
    for line in rrule.lines() {
        if let Some(rest) = line.strip_prefix("DTSTART;TZID=") {
            match rest.find(':') {
                Some(index) if index > 0 => return Some((&rest[..index], &rest[index + 1..])),
                _ => continue,
            }
        }
        if let Some(rest) = line.strip_prefix("DTSTART:") {
            return Some(("", rest));
        }
    }
    None
}

fn basic_shape(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 15
        && bytes[8] == b'T'
        && (0..8).all(|index| bytes[index].is_ascii_digit())
        && (9..15).all(|index| bytes[index].is_ascii_digit())
}

struct BasicWall {
    epoch_ms: i64,
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

fn basic_time(text: &str) -> Result<BasicWall, RruleRejection> {
    if !basic_shape(text) {
        return Err(RruleRejection::Syntax);
    }
    let year: i64 = text[0..4].parse().unwrap_or(0);
    let month: i64 = text[4..6].parse().unwrap_or(0);
    let day: i64 = text[6..8].parse().unwrap_or(0);
    let hour: i64 = text[9..11].parse().unwrap_or(0);
    let minute: i64 = text[11..13].parse().unwrap_or(0);
    let second: i64 = text[13..15].parse().unwrap_or(0);
    if year < MIN_RRULE_YEAR {
        return Err(RruleRejection::Syntax);
    }
    let date = Date::new(year as i16, month as i8, day as i8)
        .map_err(|_| RruleRejection::Syntax)?;
    let time = Time::new(hour as i8, minute as i8, second as i8, 0)
        .map_err(|_| RruleRejection::Syntax)?;
    let epoch_ms = DateTime::from_parts(date, time)
        .to_zoned(TimeZone::UTC)
        .map_err(|_| RruleRejection::Syntax)?
        .timestamp()
        .as_millisecond();
    Ok(BasicWall {
        epoch_ms,
        year,
        month,
        day,
        hour,
        minute,
        second,
    })
}

fn resolve_wall(zone_name: &str, wall: &BasicWall) -> Option<i64> {
    let tz = TimeZone::get(zone_name).ok()?;
    let date = Date::new(wall.year as i16, wall.month as i8, wall.day as i8).ok()?;
    let dt = date.at(wall.hour as i8, wall.minute as i8, wall.second as i8, 0);
    let ambiguous = tz.to_ambiguous_zoned(dt);
    if matches!(ambiguous.offset(), AmbiguousOffset::Gap { .. }) {
        return None;
    }
    ambiguous.earlier().ok().map(|zoned| zoned.timestamp().as_millisecond())
}

fn dtstart_clock_seconds(rrule: &str) -> Option<String> {
    let (_, body) = dtstart_body(rrule)?;
    let body = body.strip_suffix('Z').unwrap_or(body);
    if !basic_shape(body) {
        return None;
    }
    Some(body[13..15].to_string())
}

fn dtstart_parts(rrule: &str) -> Option<(Option<String>, String)> {
    let (tzid, body) = dtstart_body(rrule)?;
    let body = body.strip_suffix('Z').unwrap_or(body);
    if !basic_shape(body) {
        return None;
    }
    let start = format!(
        "{}-{}-{}T{}:{}:{}",
        &body[0..4],
        &body[4..6],
        &body[6..8],
        &body[9..11],
        &body[11..13],
        &body[13..15]
    );
    Some((if tzid.is_empty() { None } else { Some(tzid.to_string()) }, start))
}

fn rrule_clock(rrule: &str) -> String {
    let bytes = rrule.as_bytes();
    for index in 0..bytes.len() {
        if bytes[index] == b'T'
            && bytes.len() >= index + 7
            && bytes[index + 1..index + 7].iter().all(u8::is_ascii_digit)
        {
            return rrule[index + 1..index + 7].to_string();
        }
    }
    String::new()
}

fn iso_weekday_of_text(date_text: &str) -> i64 {
    segment(date_text, 0, 10)
        .parse::<Date>()
        .map(|date| date.weekday().to_monday_one_offset() as i64)
        .unwrap_or(1)
}

pub fn host_frequency_kind(task: &ScheduleCatalogEntry) -> FrequencyKind {
    match task.rrule.as_deref() {
        Some(rrule) => host_frequency_kind_of_rrule(rrule),
        None => FrequencyKind::Custom,
    }
}

fn host_frequency_kind_of_rrule(rrule: &str) -> FrequencyKind {
    let Some(seconds) = dtstart_clock_seconds(rrule) else {
        return FrequencyKind::Custom;
    };
    let fields = rule_fields(rrule);
    let freq = fields.get("FREQ").map(String::as_str);
    if fields.get("COUNT").map(String::as_str) == Some("1")
        && fields.len() == 2
        && freq == Some("DAILY")
    {
        return FrequencyKind::Once;
    }
    if fields.contains_key("COUNT") || fields.contains_key("UNTIL") || fields.contains_key("BYSECOND")
    {
        return FrequencyKind::Custom;
    }
    let allowed = |names: &[&str]| fields.keys().all(|key| names.contains(&key.as_str()));
    match freq {
        Some("MINUTELY") if allowed(&["FREQ", "INTERVAL"]) => return FrequencyKind::Minutely,
        Some("HOURLY") if allowed(&["FREQ", "INTERVAL"]) => return FrequencyKind::Hourly,
        Some("DAILY")
            if seconds == "00"
                && allowed(&["FREQ", "INTERVAL", "BYHOUR", "BYMINUTE", "BYSETPOS"])
                && (!fields.contains_key("BYHOUR")
                    && !fields.contains_key("BYMINUTE")
                    && !fields.contains_key("BYSETPOS")
                    || (is_short_number(fields.get("BYHOUR").map(String::as_str))
                        && is_short_number(fields.get("BYMINUTE").map(String::as_str))
                        && (!fields.contains_key("BYSETPOS")
                            || fields.get("BYSETPOS").map(String::as_str) == Some("1")))) =>
        {
            return FrequencyKind::Daily
        }
        Some("WEEKLY")
            if allowed(&["FREQ", "INTERVAL", "BYDAY"])
                && fields
                    .get("BYDAY")
                    .map(|days| {
                        days.split(',')
                            .all(|day| WEEKDAY_CODES.contains(&day))
                    })
                .unwrap_or(false) =>
        {
            return FrequencyKind::Weekly
        }
        Some("MONTHLY")
            if allowed(&["FREQ", "INTERVAL", "BYMONTHDAY", "BYSETPOS"])
                && fields.get("BYSETPOS").map(String::as_str) == Some("1")
                && fields
                    .get("BYMONTHDAY")
                    .map(|days| {
                        let parts: Vec<&str> = days.split(',').collect();
                        parts.len() == 2
                            && parts[1] == "-1"
                            && is_short_number(Some(parts[0]))
                    })
                .unwrap_or(false) =>
        {
            return FrequencyKind::Monthly
        }
        _ => {}
    }
    FrequencyKind::Custom
}

fn is_short_number(text: Option<&str>) -> bool {
    match text {
        Some(text) => {
            let bytes = text.as_bytes();
            (1..=2).contains(&bytes.len()) && bytes.iter().all(u8::is_ascii_digit)
        }
        None => false,
    }
}

pub fn record_time_zone(record: &ScheduleCatalogEntry) -> Option<String> {
    if record.kind == ScheduleRecordKind::Rrule {
        let text = record.rrule.as_deref()?;
        let (tzid, _) = dtstart_body(text)?;
        return if tzid.is_empty() {
            None
        } else {
            Some(tzid.to_string())
        };
    }
    match record.kind {
        ScheduleRecordKind::After | ScheduleRecordKind::At | ScheduleRecordKind::Every => None,
        ScheduleRecordKind::Daily | ScheduleRecordKind::Weekly | ScheduleRecordKind::Cron => {
            record.time_zone.clone()
        }
        ScheduleRecordKind::Rrule => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronRejection {
    Syntax,
    TimeZone,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CronFields {
    pub expression: String,
    pub minutes: Vec<i64>,
    pub hours: Vec<i64>,
    pub days_of_month: Vec<i64>,
    pub months: Vec<i64>,
    pub days_of_week: Vec<i64>,
    pub minute_star: bool,
    pub hour_star: bool,
    pub day_of_month_star: bool,
    pub day_of_week_star: bool,
}

#[derive(Clone, Copy)]
struct CronFieldSpec {
    min: i64,
    max: i64,
    canonical_max: i64,
}

const CRON_SPECS: [CronFieldSpec; 5] = [
    CronFieldSpec {
        min: 0,
        max: 59,
        canonical_max: 59,
    },
    CronFieldSpec {
        min: 0,
        max: 23,
        canonical_max: 23,
    },
    CronFieldSpec {
        min: 1,
        max: 31,
        canonical_max: 31,
    },
    CronFieldSpec {
        min: 1,
        max: 12,
        canonical_max: 12,
    },
    CronFieldSpec {
        min: 0,
        max: 7,
        canonical_max: 6,
    },
];

fn cron_value(text: &str, spec: &CronFieldSpec) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i128 = text.parse().ok()?;
    if value > i64::MAX as i128 {
        return None;
    }
    let value = value as i64;
    (value >= spec.min && value <= spec.max).then_some(value)
}

fn cron_step(text: &str) -> Option<i64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let step: i128 = text.parse().ok()?;
    if step < 1 || step > i64::MAX as i128 {
        return None;
    }
    Some(step as i64)
}

fn cron_field_values(raw: &str, spec: &CronFieldSpec) -> Option<Vec<i64>> {
    let mut matched = BTreeMap::new();
    for element in raw.split(',') {
        if element.is_empty() {
            return None;
        }
        if element == "*" {
            let mut value = spec.min;
            while value <= spec.max {
                matched.insert(value, ());
                value += 1;
            }
            continue;
        }
        if let Some(step_text) = element.strip_prefix("*/") {
            let step = cron_step(step_text)?;
            let mut value = spec.min;
            while value <= spec.max {
                matched.insert(value, ());
                value += step;
            }
            continue;
        }
        let (range, step_text) = match element.split_once('/') {
            Some((range, step)) => (range, Some(step)),
            None => (element, None),
        };
        let (start_text, end_text) = match range.split_once('-') {
            Some((start, end)) => (start, Some(end)),
            None => (range, None),
        };
        let start = cron_value(start_text, spec)?;
        match end_text {
            None => {
                if step_text.is_some() {
                    return None;
                }
                matched.insert(start, ());
            }
            Some(end_text) => {
                let last = cron_value(end_text, spec)?;
                if start > last {
                    return None;
                }
                let step = match step_text {
                    Some(text) => cron_step(text)?,
                    None => 1,
                };
                let mut value = start;
                while value <= last {
                    matched.insert(value, ());
                    value += step;
                }
            }
        }
    }
    let mut folded = BTreeMap::new();
    for value in matched.into_keys() {
        let folded_value = if spec.canonical_max != spec.max && value == spec.max {
            spec.min
        } else {
            value
        };
        folded.insert(folded_value, ());
    }
    Some(folded.into_keys().collect())
}

fn encode_cron_field(values: &[i64]) -> String {
    let first = values[0];
    if values.len() == 1 {
        return first.to_string();
    }
    let last = values[values.len() - 1];
    let step = values[1] - first;
    if values.windows(2).all(|pair| pair[1] - pair[0] == step) {
        return if step == 1 {
            format!("{first}-{last}")
        } else {
            format!("{first}-{last}/{step}")
        };
    }
    let mut parts: Vec<String> = Vec::new();
    let mut run_start = first;
    for index in 1..values.len() {
        let current = values[index];
        let previous = values[index - 1];
        if current == previous + 1 {
            continue;
        }
        parts.push(if run_start == previous {
            run_start.to_string()
        } else {
            format!("{run_start}-{previous}")
        });
        run_start = current;
    }
    parts.push(if run_start == last {
        run_start.to_string()
    } else {
        format!("{run_start}-{last}")
    });
    parts.join(",")
}

fn star_walk(spec: &CronFieldSpec, step: i64) -> Vec<i64> {
    let mut walked = BTreeMap::new();
    let mut value = spec.min;
    while value <= spec.max {
        let folded = if spec.canonical_max != spec.max && value == spec.max {
            spec.min
        } else {
            value
        };
        walked.insert(folded, ());
        value += step;
    }
    walked.into_keys().collect()
}

fn encode_star_cron_field(values: &[i64], spec: &CronFieldSpec) -> Option<String> {
    if values.len() == (spec.canonical_max - spec.min + 1) as usize {
        return Some("*".to_string());
    }
    let present: Vec<i64> = values.to_vec();
    let mut best_step: Option<i64> = None;
    let mut best_walk: Vec<i64> = Vec::new();
    for step in 1..=(spec.max - spec.min + 1) {
        let walk = star_walk(spec, step);
        if walk.len() > best_walk.len() && walk.iter().all(|value| present.contains(value)) {
            best_step = Some(step);
            best_walk = walk;
        }
    }
    let step = best_step?;
    let walked: Vec<i64> = best_walk;
    let remaining: Vec<i64> = values
        .iter()
        .copied()
        .filter(|value| !walked.contains(value))
        .collect();
    Some(if remaining.is_empty() {
        format!("*/{step}")
    } else {
        format!("*/{step},{}", encode_cron_field(&remaining))
    })
}

pub fn parse_cron(expression: &str, time_zone: &str) -> Result<CronFields, CronRejection> {
    if expression.is_empty() || expression.trim() != expression {
        return Err(CronRejection::Syntax);
    }
    let raw: Vec<&str> = expression.split_whitespace().collect();
    if raw.len() != 5 {
        return Err(CronRejection::Syntax);
    }
    let mut fields: Vec<(Vec<i64>, String, bool)> = Vec::new();
    for (index, field) in raw.iter().enumerate() {
        let spec = &CRON_SPECS[index];
        let values = cron_field_values(field, spec).ok_or(CronRejection::Syntax)?;
        let star = field.starts_with('*');
        let canonical = if star {
            encode_star_cron_field(&values, spec).ok_or(CronRejection::Syntax)?
        } else {
            encode_cron_field(&values)
        };
        fields.push((values, canonical, star));
    }
    canonical_zone_name(time_zone).ok_or(CronRejection::TimeZone)?;
    Ok(CronFields {
        expression: fields.iter().map(|(_, canonical, _)| canonical.clone()).collect::<Vec<_>>().join(" "),
        minutes: fields[0].0.clone(),
        hours: fields[1].0.clone(),
        days_of_month: fields[2].0.clone(),
        months: fields[3].0.clone(),
        days_of_week: fields[4].0.clone(),
        minute_star: fields[0].2,
        hour_star: fields[1].2,
        day_of_month_star: fields[2].2,
        day_of_week_star: fields[4].2,
    })
}

fn two_digits(value: i64) -> String {
    format!("{value:02}")
}

fn consecutive_runs(values: &[i64]) -> Vec<Vec<i64>> {
    let mut runs: Vec<Vec<i64>> = Vec::new();
    for value in values {
        match runs.last_mut() {
            Some(run) if run[run.len() - 1] + 1 == *value => run.push(*value),
            _ => runs.push(vec![*value]),
        }
    }
    runs
}

fn number_list(values: &[i64], locale: Locale) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(copy(locale, "cron.list.join"))
}

fn cron_weekday_text(values: &[i64], locale: Locale) -> String {
    let join = copy(locale, "cron.list.join");
    let name = |day: i64| -> String {
        fill(
            copy(locale, "cron.weekday.name"),
            &[("weekday", weekday_name(locale, day).to_string())],
        )
    };
    let mut iso: Vec<i64> = values.iter().map(|value| if *value == 0 { 7 } else { *value }).collect();
    iso.sort_unstable();
    consecutive_runs(&iso)
        .iter()
        .map(|run| {
            if run.len() >= 3 {
                fill(
                    copy(locale, "cron.weekday.range"),
                    &[("from", name(run[0])), ("to", name(run[run.len() - 1]))],
                )
            } else {
                run.iter().map(|day| name(*day)).collect::<Vec<_>>().join(join)
            }
        })
        .collect::<Vec<_>>()
        .join(join)
}

fn cron_month_suffix(values: &[i64], locale: Locale) -> String {
    if values.len() == 12 {
        return String::new();
    }
    let months = values
        .iter()
        .map(|month| month_long_name(locale, *month))
        .collect::<Vec<_>>()
        .join(copy(locale, "cron.list.join"));
    fill(copy(locale, "cron.months"), &[("months", months)])
}

fn cron_day_text(fields: &CronFields, locale: Locale) -> (bool, String) {
    let months = cron_month_suffix(&fields.months, locale);
    let weekdays = cron_weekday_text(&fields.days_of_week, locale);
    let days = number_list(&fields.days_of_month, locale);
    let params = [
        ("days", days.clone()),
        ("weekdays", weekdays.clone()),
        ("months", months.clone()),
    ];
    let day_of_month = (fields.day_of_month_star, fields.days_of_month.len() == 31);
    let day_of_week = (fields.day_of_week_star, fields.days_of_week.len() == 7);
    if day_of_month.0 || day_of_week.0 {
        if day_of_month.1 && day_of_week.1 {
            return (months.is_empty(), fill(copy(locale, "cron.day.every"), &params));
        }
        if day_of_month.1 {
            return (false, fill(copy(locale, "cron.day.weekdays"), &params));
        }
        if day_of_week.1 {
            return (false, fill(copy(locale, "cron.day.monthDays"), &params));
        }
        return (false, fill(copy(locale, "cron.day.bothStarred"), &params));
    }
    if day_of_month.1 || day_of_week.1 {
        return (months.is_empty(), fill(copy(locale, "cron.day.every"), &params));
    }
    (false, fill(copy(locale, "cron.day.both"), &params))
}

fn cron_hour_text(values: &[i64], locale: Locale) -> String {
    let hours: Vec<String> = values.iter().map(|value| two_digits(*value)).collect();
    let contiguous = hours.len() >= 2
        && values[values.len() - 1] - values[0] == (values.len() - 1) as i64;
    if contiguous {
        fill(
            copy(locale, "cron.hours.range"),
            &[("from", hours[0].clone()), ("to", hours[hours.len() - 1].clone())],
        )
    } else {
        fill(
            copy(locale, "cron.hours.list"),
            &[("hours", hours.join(copy(locale, "cron.list.join")))],
        )
    }
}

fn cron_full_range_step(values: &[i64], min: i64, max: i64) -> Option<i64> {
    if values.len() < 2 || values[0] != min {
        return None;
    }
    let step = values[1] - values[0];
    if !values.windows(2).all(|pair| pair[1] - pair[0] == step) {
        return None;
    }
    (values[values.len() - 1] + step > max).then_some(step)
}

fn cron_time_text(fields: &CronFields, locale: Locale) -> (bool, String, String) {
    let minutes_unrestricted = fields.minute_star && fields.minutes.len() == 60;
    let hours_unrestricted = fields.hour_star && fields.hours.len() == 24;
    let step = if minutes_unrestricted {
        None
    } else {
        cron_full_range_step(&fields.minutes, 0, 59)
    };
    if hours_unrestricted {
        if minutes_unrestricted {
            return (
                true,
                copy(locale, "cron.time.everyMinute").to_string(),
                copy(locale, "cron.time.joinedEveryMinute").to_string(),
            );
        }
        if let Some(step) = step.filter(|step| *step >= 2) {
            return (
                true,
                fill(copy(locale, "cron.time.everyMinutes"), &[("step", step.to_string())]),
                fill(
                    copy(locale, "cron.time.joinedEveryMinutes"),
                    &[("step", step.to_string())],
                ),
            );
        }
        if fields.minutes.len() == 1 && fields.minutes[0] == 0 {
            return (
                true,
                copy(locale, "cron.time.everyHour").to_string(),
                copy(locale, "cron.time.joinedEveryHour").to_string(),
            );
        }
        let minutes = number_list(&fields.minutes, locale);
        return (
            false,
            fill(copy(locale, "cron.time.hourlyAt"), &[("minutes", minutes.clone())]),
            fill(copy(locale, "cron.time.joinedHourlyAt"), &[("minutes", minutes)]),
        );
    }
    let hours = cron_hour_text(&fields.hours, locale);
    if minutes_unrestricted {
        let text = fill(copy(locale, "cron.time.hoursEveryMinute"), &[("hours", hours)]);
        return (false, text.clone(), text);
    }
    if let Some(step) = step.filter(|step| *step >= 2) {
        let text = fill(
            copy(locale, "cron.time.hoursEveryMinutes"),
            &[("hours", hours), ("step", step.to_string())],
        );
        return (false, text.clone(), text);
    }
    let hour_step = cron_full_range_step(&fields.hours, 0, 23);
    if fields.minutes.len() == 1
        && fields.minutes[0] == 0
        && hour_step.map(|step| step >= 2).unwrap_or(false)
    {
        let count = hour_step.unwrap().to_string();
        return (
            true,
            fill(copy(locale, "cron.time.everyNHours"), &[("count", count.clone())]),
            fill(copy(locale, "cron.time.joinedEveryNHours"), &[("count", count)]),
        );
    }
    let mut times: Vec<String> = Vec::new();
    for hour in &fields.hours {
        for minute in &fields.minutes {
            times.push(format!("{}:{}", two_digits(*hour), two_digits(*minute)));
        }
    }
    if times.len() <= CRON_TIME_LIST_LIMIT {
        let text = fill(
            copy(locale, "cron.time.at"),
            &[("times", times.join(copy(locale, "cron.list.join")))],
        );
        (false, text.clone(), text)
    } else {
        let text = fill(
            copy(locale, "cron.time.hoursAt"),
            &[("hours", hours), ("minutes", number_list(&fields.minutes, locale))],
        );
        (false, text.clone(), text)
    }
}

pub fn cron_preview(fields: &CronFields, locale: Locale) -> String {
    let day = cron_day_text(fields, locale);
    let time = cron_time_text(fields, locale);
    if day.0 && time.0 {
        time.1
    } else {
        format!("{}{}{}", day.1, copy(locale, "cron.part.join"), time.2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RruleRejection {
    Syntax,
    TimeZone,
    Density,
}

pub struct RruleParsed {
    pub expression: String,
    pub time_zone: String,
}

struct RuleValues {
    frequency: String,
    count: Option<i64>,
    until: Option<i64>,
    lists: BTreeMap<String, Vec<i64>>,
    by_day_entries: usize,
    by_day_ordinals: usize,
}

fn rule_parts(line: &str) -> Result<BTreeMap<String, String>, RruleRejection> {
    let mut parts = BTreeMap::new();
    for component in line["RRULE:".len()..].split(';') {
        let Some(separator) = component.find('=') else {
            return Err(RruleRejection::Syntax);
        };
        let key = &component[..separator];
        let value = &component[separator + 1..];
        if separator == 0 || value.is_empty() || parts.contains_key(key) {
            return Err(RruleRejection::Syntax);
        }
        if !SUPPORTED_PARTS.contains(&key) {
            return Err(RruleRejection::Syntax);
        }
        parts.insert(key.to_string(), value.to_string());
    }
    Ok(parts)
}

fn is_unsigned(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

fn is_signed(text: &str) -> bool {
    let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
    is_unsigned(digits)
}

fn positive_value(text: &str, max: i128) -> Result<i64, RruleRejection> {
    if !is_unsigned(text) {
        return Err(RruleRejection::Syntax);
    }
    let value: i128 = text.parse().map_err(|_| RruleRejection::Syntax)?;
    if value < 1 || value > max {
        return Err(RruleRejection::Syntax);
    }
    Ok(value as i64)
}

fn frequency_of(text: Option<&str>) -> Result<String, RruleRejection> {
    let Some(text) = text else {
        return Err(RruleRejection::Syntax);
    };
    if text == "SECONDLY" {
        return Err(RruleRejection::Density);
    }
    if ["MINUTELY", "HOURLY", "DAILY", "WEEKLY", "MONTHLY", "YEARLY"].contains(&text) {
        Ok(text.to_string())
    } else {
        Err(RruleRejection::Syntax)
    }
}

fn until_of(text: Option<&str>) -> Result<Option<i64>, RruleRejection> {
    let Some(text) = text else {
        return Ok(None);
    };
    if text.len() != 16 || !text.ends_with('Z') || !basic_shape(&text[..15]) {
        return Err(RruleRejection::Syntax);
    }
    Ok(Some(basic_time(&text[..15])?.epoch_ms))
}

fn week_start_of(text: Option<&str>) -> Result<usize, RruleRejection> {
    let text = text.unwrap_or("MO");
    WEEKDAY_CODES
        .iter()
        .position(|code| *code == text)
        .ok_or(RruleRejection::Syntax)
}

fn list_values(text: &str, min: i64, max: i64) -> Result<Vec<i64>, RruleRejection> {
    let signed = min < 0;
    let mut values = Vec::new();
    for item in text.split(',') {
        if !(if signed { is_signed(item) } else { is_unsigned(item) }) {
            return Err(RruleRejection::Syntax);
        }
        let Ok(value) = item.parse::<i64>() else {
            return Err(RruleRejection::Syntax);
        };
        if value < min || value > max || (signed && value == 0) {
            return Err(RruleRejection::Syntax);
        }
        values.push(value);
    }
    let deduped: BTreeMap<i64, ()> = values.iter().map(|value| (*value, ())).collect();
    if deduped.len() != values.len() {
        return Err(RruleRejection::Syntax);
    }
    Ok(values)
}

fn is_weekday_ordinal(digits: &str) -> bool {
    match digits.len() {
        1 => {
            let first = digits.as_bytes()[0];
            (b'1'..=b'9').contains(&first)
        }
        2 => {
            let (first, second) = (digits.as_bytes()[0], digits.as_bytes()[1]);
            second.is_ascii_digit()
                && ((b'1'..=b'4').contains(&first) || (first == b'5' && (b'0'..=b'3').contains(&second)))
        }
        _ => false,
    }
}

fn by_day_of(text: Option<&str>) -> Result<(usize, usize), RruleRejection> {
    let Some(text) = text else {
        return Ok((0, 0));
    };
    let selectors: Vec<&str> = text.split(',').collect();
    let mut seen = BTreeMap::new();
    let mut ordinals = 0;
    for selector in &selectors {
        if selector.len() < 2 {
            return Err(RruleRejection::Syntax);
        }
        let (ordinal, code) = selector.split_at(selector.len() - 2);
        if !WEEKDAY_CODES.contains(&code) {
            return Err(RruleRejection::Syntax);
        }
        if !ordinal.is_empty() {
            let digits = ordinal.strip_prefix(['+', '-']).unwrap_or(ordinal);
            if !is_weekday_ordinal(digits) {
                return Err(RruleRejection::Syntax);
            }
            ordinals += 1;
        }
        let sign = if ordinal.starts_with('-') { "-" } else { "" };
        let digits = ordinal.trim_start_matches(['+', '-']);
        let value = if digits.is_empty() {
            0
        } else {
            digits.parse::<i64>().unwrap_or(0)
        };
        seen.insert(format!("{sign}{value}{code}"), ());
    }
    if seen.len() != selectors.len() {
        return Err(RruleRejection::Syntax);
    }
    Ok((selectors.len(), ordinals))
}

fn rule_values(parts: &BTreeMap<String, String>) -> Result<RuleValues, RruleRejection> {
    let frequency = frequency_of(parts.get("FREQ").map(String::as_str))?;
    if let Some(text) = parts.get("INTERVAL") {
        positive_value(text, MAX_SAFE_INTEGER)?;
    }
    let count = match parts.get("COUNT") {
        Some(text) => Some(positive_value(text, MAX_RRULE_COUNT as i128)?),
        None => None,
    };
    let until = until_of(parts.get("UNTIL").map(String::as_str))?;
    if let Some(text) = parts.get("WKST") {
        week_start_of(Some(text))?;
    }
    let mut lists = BTreeMap::new();
    for (key, min, max) in LIST_RANGES {
        if let Some(text) = parts.get(key) {
            lists.insert(key.to_string(), list_values(text, min, max)?);
        }
    }
    let (by_day_entries, by_day_ordinals) = by_day_of(parts.get("BYDAY").map(String::as_str))?;
    Ok(RuleValues {
        frequency,
        count,
        until,
        lists,
        by_day_entries,
        by_day_ordinals,
    })
}

fn check_combination(values: &RuleValues) -> Result<(), RruleRejection> {
    if values.count.is_some() && values.until.is_some() {
        return Err(RruleRejection::Syntax);
    }
    if values.lists.get("BYSECOND").map(|list| list.len()).unwrap_or(1) > 1 {
        return Err(RruleRejection::Density);
    }
    if (values.frequency == "MINUTELY" || values.frequency == "HOURLY")
        && (!values.lists.is_empty() || values.by_day_entries > 0)
    {
        return Err(RruleRejection::Syntax);
    }
    for key in ["BYWEEKNO", "BYYEARDAY"] {
        if values.lists.contains_key(key) && values.frequency != "YEARLY" {
            return Err(RruleRejection::Syntax);
        }
    }
    if values.lists.contains_key("BYMONTHDAY")
        && values.frequency != "MONTHLY"
        && values.frequency != "YEARLY"
    {
        return Err(RruleRejection::Syntax);
    }
    if values.by_day_ordinals > 0 && values.frequency != "MONTHLY" && values.frequency != "YEARLY" {
        return Err(RruleRejection::Syntax);
    }
    if values.by_day_ordinals > 0 && values.lists.contains_key("BYWEEKNO") {
        return Err(RruleRejection::Syntax);
    }
    if values.lists.contains_key("BYSETPOS")
        && values.lists.len() == 1
        && values.by_day_entries == 0
    {
        return Err(RruleRejection::Syntax);
    }
    Ok(())
}

fn apply_start_defaults(
    parts: &mut BTreeMap<String, String>,
    values: &RuleValues,
    wall: &BasicWall,
) {
    if values.frequency == "MINUTELY" || values.frequency == "HOURLY" {
        return;
    }
    if !parts.contains_key("BYHOUR") {
        parts.insert("BYHOUR".to_string(), wall.hour.to_string());
    }
    if !parts.contains_key("BYMINUTE") {
        parts.insert("BYMINUTE".to_string(), wall.minute.to_string());
    }
    if !parts.contains_key("BYSECOND") {
        parts.insert("BYSECOND".to_string(), wall.second.to_string());
    }
    if values.by_day_entries > 0
        || parts.contains_key("BYMONTHDAY")
        || parts.contains_key("BYYEARDAY")
        || parts.contains_key("BYWEEKNO")
    {
        return;
    }
    let date = Date::new(wall.year as i16, wall.month as i8, wall.day as i8).ok();
    if values.frequency == "WEEKLY" {
        if let Some(date) = date {
            parts.insert(
                "BYDAY".to_string(),
                WEEKDAY_CODES[date.weekday().to_monday_zero_offset() as usize].to_string(),
            );
        }
    }
    if values.frequency == "MONTHLY" || values.frequency == "YEARLY" {
        parts.insert("BYMONTHDAY".to_string(), wall.day.to_string());
    }
    if values.frequency == "YEARLY" && !parts.contains_key("BYMONTH") {
        parts.insert("BYMONTH".to_string(), wall.month.to_string());
    }
}

fn period_days(values: &RuleValues) -> i64 {
    let size = |key: &str| -> i64 {
        values
            .lists
            .get(key)
            .map(|list| list.len() as i64)
            .unwrap_or(i64::MAX)
    };
    let by_day_days = |per_weekday: i64| -> i64 {
        if values.by_day_entries == 0 {
            return i64::MAX;
        }
        (values.by_day_ordinals as i64).saturating_add(per_weekday.saturating_mul(
            values.by_day_entries as i64 - values.by_day_ordinals as i64,
        ))
    };
    match values.frequency.as_str() {
        "MINUTELY" | "HOURLY" | "DAILY" => 1,
        "WEEKLY" => 7.min(by_day_days(1)),
        "MONTHLY" => 31.min(size("BYMONTHDAY")).min(by_day_days(5)),
        _ => {
            let months = values.lists.get("BYMONTH").map(|list| list.len() as i64);
            match months {
                None => 366
                    .min(size("BYYEARDAY"))
                    .min(12i64.saturating_mul(size("BYMONTHDAY")))
                    .min(9i64.saturating_mul(size("BYWEEKNO")))
                    .min(by_day_days(53)),
                Some(months) => 31i64
                    .saturating_mul(months)
                    .min(size("BYYEARDAY"))
                    .min(months.saturating_mul(size("BYMONTHDAY")))
                    .min(9i64.saturating_mul(size("BYWEEKNO")))
                    .min(months.saturating_mul(by_day_days(5))),
            }
        }
    }
}

fn check_cost(
    values: &RuleValues,
    reject_ineffective_set_positions: bool,
) -> Result<(), RruleRejection> {
    let times = ["BYHOUR", "BYMINUTE", "BYSECOND"].iter().fold(1i64, |product, key| {
        product.saturating_mul(values.lists.get(*key).map(|list| list.len() as i64).unwrap_or(1))
    });
    let candidates = period_days(values).saturating_mul(times);
    if candidates > MAX_PERIOD_CANDIDATES {
        return Err(RruleRejection::Density);
    }
    let positions = values.lists.get("BYSETPOS").cloned().unwrap_or_default();
    if reject_ineffective_set_positions && candidates == 1 && !positions.is_empty() {
        return Err(RruleRejection::Syntax);
    }
    if positions.len() > MAX_SET_POSITIONS {
        return Err(RruleRejection::Syntax);
    }
    if positions.iter().any(|position| position.abs() > candidates) {
        return Err(RruleRejection::Syntax);
    }
    Ok(())
}

fn parse_dtstart(line: &str) -> Result<(String, BasicWall), RruleRejection> {
    if let Some(body) = line.strip_prefix("DTSTART:") {
        let body = body.strip_suffix('Z').ok_or(RruleRejection::Syntax)?;
        let wall = basic_time(body)?;
        return Ok(("UTC".to_string(), wall));
    }
    if let Some(rest) = line.strip_prefix("DTSTART;TZID=") {
        if rest.len() >= 17 && rest.as_bytes()[rest.len() - 16] == b':' {
            let tzid = &rest[..rest.len() - 16];
            let body = &rest[rest.len() - 15..];
            if !tzid.is_empty() && !tzid.contains(';') {
                let wall = basic_time(body)?;
                return Ok((tzid.to_string(), wall));
            }
        }
    }
    Err(RruleRejection::Syntax)
}

pub fn parse_rrule(
    text: &str,
    reject_ineffective_set_positions: bool,
) -> Result<RruleParsed, RruleRejection> {
    let normalized = text.replace("\r\n", "\n");
    let lines: Vec<&str> = normalized
        .trim()
        .split('\n')
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != 2 {
        return Err(RruleRejection::Syntax);
    }
    let (start_line, rule_line) = (lines[0], lines[1]);
    if !start_line.starts_with("DTSTART") || !rule_line.starts_with("RRULE:") {
        return Err(RruleRejection::Syntax);
    }
    let (zone_name, wall) = parse_dtstart(start_line)?;
    let zone = canonical_zone_name(&zone_name).ok_or(RruleRejection::TimeZone)?;
    if resolve_wall(&zone, &wall).is_none() {
        return Err(RruleRejection::Syntax);
    }
    let parts = rule_parts(rule_line)?;
    let values = rule_values(&parts)?;
    check_combination(&values)?;
    let mut recurrence_parts = parts.clone();
    apply_start_defaults(&mut recurrence_parts, &values, &wall);
    let recurrence_values = rule_values(&recurrence_parts)?;
    check_cost(&recurrence_values, reject_ineffective_set_positions)?;
    Ok(RruleParsed {
        expression: format!("{start_line}\n{rule_line}"),
        time_zone: zone,
    })
}

pub fn default_timing(now_ms: i64, system_zone: &str) -> TimingDraft {
    let tz = resolve_zone(system_zone);
    let ts = Timestamp::from_millisecond(now_ms + 3_600_000).unwrap_or(Timestamp::UNIX_EPOCH);
    let dt = tz.to_datetime(ts);
    let date = dt.date();
    let start = format!(
        "{}T{:02}:{:02}:00",
        date,
        dt.hour(),
        dt.minute()
    );
    TimingDraft {
        frequency: FrequencyPreset::Once,
        interval: "1".to_string(),
        start,
        time_zone: system_zone.to_string(),
        rrule: String::new(),
        retained_timing: None,
        weekdays: vec![date.weekday().to_monday_one_offset() as i64],
        month_day: date.day() as i64,
        daily_time: format!("{:02}:{:02}", dt.hour(), dt.minute()),
    }
}

pub fn timing_from_record(
    record: &ScheduleCatalogEntry,
    now_ms: i64,
    system_zone: &str,
) -> TimingDraft {
    let initial = default_timing(now_ms, system_zone);
    match record.kind {
        ScheduleRecordKind::Cron => TimingDraft {
            frequency: FrequencyPreset::Advanced,
            rrule: record.expression.clone().unwrap_or_default(),
            time_zone: record
                .time_zone
                .clone()
                .unwrap_or_else(|| initial.time_zone.clone()),
            ..initial
        },
        ScheduleRecordKind::Rrule => {
            let text = record.rrule.clone().unwrap_or_default();
            let start_match = dtstart_parts(&text);
            let fields = rule_fields(&text);
            let kind = host_frequency_kind_of_rrule(&text);
            let start = start_match
                .as_ref()
                .map(|(_, start)| start.clone())
                .unwrap_or_else(|| initial.start.clone());
            let by_day: Vec<&str> = fields
                .get("BYDAY")
                .map(|days| days.split(',').collect())
                .unwrap_or_default();
            let hour = fields
                .get("BYHOUR")
                .cloned()
                .unwrap_or_else(|| segment(&start, 11, 13).to_string());
            let minute = fields
                .get("BYMINUTE")
                .cloned()
                .unwrap_or_else(|| segment(&start, 14, 16).to_string());
            let month_day_field = (kind == FrequencyKind::Monthly)
                .then(|| fields.get("BYMONTHDAY"))
                .flatten()
                .map(String::as_str);
            let weekdays: Vec<i64> = if by_day.is_empty() {
                vec![iso_weekday_of_text(&start)]
            } else {
                by_day
                    .iter()
                    .map(|day| {
                        WEEKDAY_CODES
                            .iter()
                            .position(|code| code == day)
                            .map(|position| position as i64 + 1)
                            .unwrap_or(0)
                    })
                    .collect()
            };
            TimingDraft {
                frequency: kind.preset(),
                interval: fields
                    .get("INTERVAL")
                    .cloned()
                    .unwrap_or_else(|| "1".to_string()),
                month_day: parse_leading_int(
                    month_day_field.unwrap_or(segment(&start, 8, 10)),
                )
                .unwrap_or(initial.month_day),
                start: start.clone(),
                time_zone: start_match
                    .and_then(|(tzid, _)| tzid)
                    .unwrap_or_else(|| "UTC".to_string()),
                rrule: text,
                weekdays,
                daily_time: if kind == FrequencyKind::Daily {
                    format!("{:0>2}:{:0>2}", hour, minute)
                } else {
                    segment(&start, 11, 16).to_string()
                },
                retained_timing: None,
            }
        }
        _ => {
            let retained = match record.kind {
                ScheduleRecordKind::After | ScheduleRecordKind::At => ScheduleTiming::At {
                    at: ScheduleAtInput::Instant(record.scheduled_at.clone()),
                },
                ScheduleRecordKind::Every => ScheduleTiming::Every {
                    every_seconds: record.every_seconds.unwrap_or(0),
                },
                ScheduleRecordKind::Daily => ScheduleTiming::Daily {
                    daily: ScheduleDailyInput {
                        time: record.time.clone().unwrap_or_default(),
                        time_zone: record.time_zone.clone().unwrap_or_default(),
                    },
                },
                ScheduleRecordKind::Weekly => ScheduleTiming::Weekly {
                    weekly: ScheduleWeeklyInput {
                        time: record.time.clone().unwrap_or_default(),
                        time_zone: record.time_zone.clone().unwrap_or_default(),
                        weekdays: record.weekdays.clone().unwrap_or_default(),
                    },
                },
                _ => ScheduleTiming::At {
                    at: ScheduleAtInput::Instant(record.scheduled_at.clone()),
                },
            };
            let scheduled_at = parse_epoch_ms(&record.scheduled_at);
            let mut time_zone = record.time_zone.clone().unwrap_or_else(|| {
                if record.kind == ScheduleRecordKind::Every {
                    "UTC".to_string()
                } else {
                    initial.time_zone.clone()
                }
            });
            let mut start = scheduled_at
                .map(|ms| start_in_zone(&time_zone, ms))
                .unwrap_or_else(|| initial.start.clone());
            if matches!(
                record.kind,
                ScheduleRecordKind::After | ScheduleRecordKind::At
            ) {
                let local = parse_local_calendar(segment(&start, 0, 10), segment(&start, 11, 19));
                let floor = scheduled_at.map(|ms| (ms / 1_000) * 1_000);
                let resolved = match (&local, floor) {
                    (Some(calendar), Some(floor_ms)) => resolve_local_instant(
                        &resolve_zone(&time_zone),
                        calendar,
                    )
                    .map(|ms| ms == floor_ms)
                    .unwrap_or(false),
                    _ => false,
                };
                if !resolved {
                    time_zone = "UTC".to_string();
                    start = scheduled_at
                        .map(|ms| start_in_zone("UTC", ms))
                        .unwrap_or_else(|| initial.start.clone());
                }
            }
            let mut guided = TimingDraft {
                start: start.clone(),
                time_zone: time_zone.clone(),
                retained_timing: Some(retained),
                daily_time: segment(&start, 11, 16).to_string(),
                month_day: segment(&start, 8, 10).parse().unwrap_or(initial.month_day),
                weekdays: vec![iso_weekday_of_text(&start)],
                ..initial.clone()
            };
            let advanced_zone = record
                .time_zone
                .clone()
                .unwrap_or_else(|| initial.time_zone.clone());
            match record.kind {
                ScheduleRecordKind::After | ScheduleRecordKind::At => {
                    guided.frequency = FrequencyPreset::Once;
                    guided
                }
                ScheduleRecordKind::Every => {
                    let seconds = record.every_seconds.unwrap_or(0);
                    if seconds % 60 == 0 && (seconds / 60).abs() as i128 <= MAX_SAFE_INTEGER {
                        let hourly = seconds % 3_600 == 0;
                        guided.frequency = if hourly {
                            FrequencyPreset::Hourly
                        } else {
                            FrequencyPreset::Minutely
                        };
                        guided.interval = (seconds / if hourly { 3_600 } else { 60 }).to_string();
                        guided
                    } else {
                        advanced_typed(&initial, &advanced_zone, guided.retained_timing)
                    }
                }
                ScheduleRecordKind::Daily => {
                    let time = record.time.as_deref().unwrap_or("");
                    if matches_hhmmss_zero(time) {
                        guided.frequency = FrequencyPreset::Daily;
                        guided.daily_time = segment(time, 0, 5).to_string();
                        guided
                    } else {
                        advanced_typed(&initial, &advanced_zone, guided.retained_timing)
                    }
                }
                ScheduleRecordKind::Weekly => {
                    let time = record.time.as_deref().unwrap_or("");
                    if matches_hhmmss(time) {
                        guided.frequency = FrequencyPreset::Weekly;
                        guided.start = format!("{}T{}", segment(&start, 0, 10), segment(time, 0, 8));
                        guided.weekdays = record.weekdays.clone().unwrap_or_default();
                        guided
                    } else {
                        advanced_typed(&initial, &advanced_zone, guided.retained_timing)
                    }
                }
                _ => advanced_typed(&initial, &advanced_zone, guided.retained_timing),
            }
        }
    }
}

fn advanced_typed(
    initial: &TimingDraft,
    time_zone: &str,
    retained: Option<ScheduleTiming>,
) -> TimingDraft {
    TimingDraft {
        frequency: FrequencyPreset::Advanced,
        rrule: String::new(),
        time_zone: time_zone.to_string(),
        retained_timing: retained,
        ..initial.clone()
    }
}

fn matches_hhmmss_zero(time: &str) -> bool {
    let bytes = time.as_bytes();
    if bytes.len() < 8
        || !(0..8).all(|index| index == 2 || index == 5 || bytes[index].is_ascii_digit())
        || bytes[2] != b':'
        || bytes[5] != b':'
        || bytes[6] != b'0'
        || bytes[7] != b'0'
    {
        return false;
    }
    match bytes.len() {
        8 => true,
        _ => bytes[8] == b'.' && bytes.len() > 9 && bytes[9..].iter().all(|b| *b == b'0'),
    }
}

fn matches_hhmmss(time: &str) -> bool {
    let bytes = time.as_bytes();
    if bytes.len() < 8
        || !(0..8).all(|index| index == 2 || index == 5 || bytes[index].is_ascii_digit())
        || bytes[2] != b':'
        || bytes[5] != b':'
    {
        return false;
    }
    match bytes.len() {
        8 => true,
        _ => bytes[8] == b'.' && bytes.len() > 9 && bytes[9..].iter().all(u8::is_ascii_digit),
    }
}

pub fn rrule_from_draft(draft: &TimingDraft, interval_from_now: bool, now_ms: i64) -> String {
    if draft.frequency == FrequencyPreset::Advanced {
        return draft.rrule.clone();
    }
    let anchor =
        if interval_from_now && matches!(draft.frequency, FrequencyPreset::Minutely | FrequencyPreset::Hourly) {
            start_in_zone(&draft.time_zone, (now_ms / 60_000) * 60_000)
        } else {
            draft.start.clone()
        };
    let time = if draft.frequency == FrequencyPreset::Daily {
        format!("{}:00", draft.daily_time)
    } else {
        segment(&anchor, 11, 19).to_string()
    };
    let mut start = format!("{}T{}", segment(&anchor, 0, 10), time).replace(['-', ':'], "");
    let rule = match draft.frequency {
        FrequencyPreset::Once => "FREQ=DAILY;COUNT=1".to_string(),
        FrequencyPreset::Minutely => format!("FREQ=MINUTELY;INTERVAL={}", draft.interval),
        FrequencyPreset::Hourly => format!("FREQ=HOURLY;INTERVAL={}", draft.interval),
        FrequencyPreset::Daily => format!(
            "FREQ=DAILY;INTERVAL={};BYHOUR={};BYMINUTE={}",
            draft.interval,
            number_of(segment(&draft.daily_time, 0, 2)),
            number_of(segment(&draft.daily_time, 3, 5))
        ),
        FrequencyPreset::Weekly => {
            let days: Vec<&str> = draft
                .weekdays
                .iter()
                .map(|day| WEEKDAY_CODES.get((day - 1).max(0) as usize).copied().unwrap_or(""))
                .collect();
            format!("FREQ=WEEKLY;INTERVAL={};BYDAY={}", draft.interval, days.join(","))
        }
        FrequencyPreset::Monthly => format!(
            "FREQ=MONTHLY;INTERVAL={};BYMONTHDAY={},-1;BYSETPOS=1",
            draft.interval, draft.month_day
        ),
        FrequencyPreset::Advanced => String::new(),
    };
    if interval_from_now
        && draft.retained_timing.is_some()
        && !matches!(
            draft.frequency,
            FrequencyPreset::Once | FrequencyPreset::Minutely | FrequencyPreset::Hourly
        )
    {
        let clock = if draft.frequency == FrequencyPreset::Daily {
            String::new()
        } else {
            format!(
                ";BYHOUR={};BYMINUTE={}",
                number_of(segment(&time, 0, 2)),
                number_of(segment(&time, 3, 5))
            )
        };
        let probe = format!(
            "DTSTART;TZID={}:{}\nRRULE:{}{};BYSECOND={}",
            draft.time_zone,
            start_in_zone(&draft.time_zone, now_ms).replace(['-', ':'], ""),
            rule,
            clock,
            number_of(segment(&time, 6, 8))
        );
        match guided_probe_next(draft, now_ms) {
            None => return probe,
            Some(next_ms) => {
                start = start_in_zone(&draft.time_zone, next_ms).replace(['-', ':'], "")
            }
        }
    }
    format!("DTSTART;TZID={}:{}\nRRULE:{}", draft.time_zone, start, rule)
}

fn resolve_candidate(tz: &TimeZone, dt: DateTime) -> Option<i64> {
    let ambiguous = tz.to_ambiguous_zoned(dt);
    if matches!(ambiguous.offset(), AmbiguousOffset::Gap { .. }) {
        return None;
    }
    ambiguous.earlier().ok().map(|zoned| zoned.timestamp().as_millisecond())
}

fn guided_probe_next(draft: &TimingDraft, now_ms: i64) -> Option<i64> {
    let tz = TimeZone::get(&draft.time_zone).ok()?;
    let now = Timestamp::from_millisecond(now_ms).ok()?;
    let now_local = tz.to_datetime(now);
    let interval = parse_leading_int(&draft.interval).filter(|value| *value >= 1)?;
    let clock = |time: &str| -> Option<(i64, i64, i64)> {
        Some((
            parse_leading_int(segment(time, 0, 2))?,
            parse_leading_int(segment(time, 3, 5))?,
            parse_leading_int(segment(time, 6, 8)).unwrap_or(0),
        ))
    };
    match draft.frequency {
        FrequencyPreset::Daily => {
            let (h, m, _) = clock(&format!("{}:00", draft.daily_time))?;
            for step in 0..=400i64 {
                let date = now_local.date().checked_add((step * interval).days()).ok()?;
                let dt = date.at(h as i8, m as i8, 0, 0);
                if dt > now_local {
                    return resolve_candidate(&tz, dt);
                }
            }
            None
        }
        FrequencyPreset::Weekly => {
            if draft.weekdays.is_empty() {
                return None;
            }
            let time = segment(&draft.start, 11, 19);
            let (h, m, s) = clock(time)?;
            let mut weekdays = draft.weekdays.clone();
            weekdays.sort_unstable();
            let week_start = now_local
                .date()
                .checked_sub(now_local.date().weekday().to_monday_zero_offset().days())
                .ok()?;
            for week in 0..=400i64 {
                let monday = week_start.checked_add((week * interval).weeks()).ok()?;
                for day in &weekdays {
                    if *day < 1 || *day > 7 {
                        return None;
                    }
                    let date = monday.checked_add((day - 1).days()).ok()?;
                    let dt = date.at(h as i8, m as i8, s as i8, 0);
                    if week == 0 && (date < now_local.date() || dt <= now_local) {
                        continue;
                    }
                    return resolve_candidate(&tz, dt);
                }
            }
            None
        }
        FrequencyPreset::Monthly => {
            if !(1..=31).contains(&draft.month_day) {
                return None;
            }
            let time = segment(&draft.start, 11, 19);
            let (h, m, s) = clock(time)?;
            let month_anchor =
                Date::new(now_local.year(), now_local.month(), 1).ok()?;
            for step in 0..=400i64 {
                let first = month_anchor.checked_add((step * interval).months()).ok()?;
                let day = draft
                    .month_day
                    .min(first.days_in_month() as i64);
                let date = Date::new(first.year(), first.month(), day as i8).ok()?;
                let dt = date.at(h as i8, m as i8, s as i8, 0);
                if step == 0 && dt <= now_local {
                    continue;
                }
                return resolve_candidate(&tz, dt);
            }
            None
        }
        _ => None,
    }
}

pub fn timing_from_draft(
    draft: &TimingDraft,
    interval_from_now: bool,
    now_ms: i64,
) -> ScheduleTiming {
    if draft.frequency == FrequencyPreset::Advanced {
        if draft.rrule.trim().is_empty() {
            if let Some(retained) = &draft.retained_timing {
                return retained.clone();
            }
        }
        if !draft.rrule.trim_start().starts_with("DTSTART") {
            let expression = draft.rrule.trim().to_string();
            let canonical = parse_cron(&expression, &draft.time_zone)
                .ok()
                .map(|fields| fields.expression);
            return ScheduleTiming::Cron {
                cron: ScheduleCronInput {
                    expression: canonical.unwrap_or(expression),
                    time_zone: draft.time_zone.clone(),
                },
            };
        }
    }
    let rrule = rrule_from_draft(draft, interval_from_now, now_ms);
    let normalized = if draft.frequency == FrequencyPreset::Advanced {
        parse_rrule(&rrule, false).ok().map(|parsed| parsed.expression)
    } else {
        None
    };
    ScheduleTiming::Rrule {
        rrule: normalized.unwrap_or(rrule),
    }
}

pub fn new_task_draft(now_ms: i64, system_zone: &str) -> TaskEditorDraft {
    TaskEditorDraft {
        timing: default_timing(now_ms, system_zone),
        title: String::new(),
        prompt: String::new(),
        target: TargetChoice::NewOnce,
        session_id: String::new(),
        cwd: String::new(),
    }
}

pub fn task_draft(
    task: &ScheduleCatalogEntry,
    now_ms: i64,
    system_zone: &str,
) -> TaskEditorDraft {
    let target = match &task.target {
        ScheduleTarget::Session { .. } => TargetChoice::Session,
        ScheduleTarget::NewOnce { .. } => TargetChoice::NewOnce,
        ScheduleTarget::New { .. } => TargetChoice::New,
    };
    let (session_id, cwd) = match &task.target {
        ScheduleTarget::Session { session_id } => (session_id.clone(), String::new()),
        ScheduleTarget::NewOnce { create, .. } | ScheduleTarget::New { create } => {
            (String::new(), create.cwd.clone().unwrap_or_default())
        }
    };
    TaskEditorDraft {
        timing: timing_from_record(task, now_ms, system_zone),
        title: task.title.clone(),
        prompt: task.prompt.clone(),
        target,
        session_id,
        cwd,
    }
}

pub fn task_draft_issue(
    draft: &TaskEditorDraft,
    changes: Option<&SchedulePatch>,
    now_ms: i64,
) -> Option<DraftIssue> {
    if draft.title.trim().is_empty()
        || draft.prompt.trim().is_empty()
        || (draft.target == TargetChoice::Session && draft.session_id.is_empty())
    {
        return Some(DraftIssue::Incomplete);
    }
    if utf16_length(draft.title.trim()) > 120 {
        return Some(DraftIssue::InvalidTitle);
    }
    let frequency = draft.timing.frequency;
    if frequency != FrequencyPreset::Once
        && frequency != FrequencyPreset::Advanced
        && !is_safe_positive_integer(&draft.timing.interval)
    {
        return Some(DraftIssue::InvalidInterval);
    }
    if frequency == FrequencyPreset::Weekly && draft.timing.weekdays.is_empty() {
        return Some(DraftIssue::RequiredWeekday);
    }
    if frequency == FrequencyPreset::Monthly
        && (draft.timing.month_day < 1 || draft.timing.month_day > 31)
    {
        return Some(DraftIssue::InvalidMonthDay);
    }
    if frequency == FrequencyPreset::Daily && !is_hhmm(&draft.timing.daily_time) {
        return Some(DraftIssue::InvalidTime);
    }
    if changes.is_some_and(|changes| changes.timing.is_none()) {
        return None;
    }
    let timing = timing_from_draft(&draft.timing, true, now_ms);
    let rejection = match &timing {
        ScheduleTiming::Cron { cron } => match parse_cron(&cron.expression, &cron.time_zone) {
            Err(CronRejection::Syntax) => Some(RruleRejection::Syntax),
            Err(CronRejection::TimeZone) => Some(RruleRejection::TimeZone),
            Ok(_) => None,
        },
        ScheduleTiming::Rrule { rrule } => parse_rrule(rrule, true).err(),
        _ => None,
    };
    let Some(rejection) = rejection else {
        return None;
    };
    match rejection {
        RruleRejection::TimeZone => Some(DraftIssue::InvalidZone),
        RruleRejection::Density => Some(DraftIssue::TooComplex),
        RruleRejection::Syntax => {
            if frequency == FrequencyPreset::Advanced {
                Some(DraftIssue::Invalid)
            } else {
                Some(DraftIssue::InvalidTime)
            }
        }
    }
}

pub fn draft_target(
    draft: &TaskEditorDraft,
    original: Option<&ScheduleCatalogEntry>,
) -> ScheduleTargetRequest {
    if draft.target == TargetChoice::Session {
        return ScheduleTargetRequest::Session {
            session_id: draft.session_id.clone(),
        };
    }
    let previous = original.and_then(|task| match &task.target {
        ScheduleTarget::Session { .. } => None,
        ScheduleTarget::NewOnce { create, .. } | ScheduleTarget::New { create } => Some(create),
    });
    let mut create = ScheduleSessionCreateSpec::default();
    if !draft.cwd.is_empty() {
        create.cwd = Some(draft.cwd.clone());
    }
    if previous.map(|spec| spec.cwd.as_deref().unwrap_or("")) == Some(draft.cwd.as_str()) {
        if let Some(preset) = previous.and_then(|spec| spec.agent_preset.clone()) {
            create.agent_preset = Some(preset);
        }
    }
    match draft.target {
        TargetChoice::NewOnce => ScheduleTargetRequest::NewOnce {
            create: Some(create),
        },
        TargetChoice::New => ScheduleTargetRequest::New { create: Some(create) },
        TargetChoice::Session => ScheduleTargetRequest::Session {
            session_id: draft.session_id.clone(),
        },
    }
}

pub fn task_draft_patch(
    draft: &TaskEditorDraft,
    original: &ScheduleCatalogEntry,
    interval_from_now: bool,
    now_ms: i64,
    system_zone: &str,
) -> SchedulePatch {
    let seeded = task_draft(original, now_ms, system_zone);
    let timing_changed = timing_from_draft(&draft.timing, false, now_ms)
        != timing_from_draft(&seeded.timing, false, now_ms);
    let target_changed = draft.target != seeded.target
        || draft.session_id != seeded.session_id
        || draft.cwd != seeded.cwd;
    SchedulePatch {
        title: (draft.title.trim() != seeded.title.trim())
            .then(|| draft.title.trim().to_string()),
        prompt: (draft.prompt.trim() != seeded.prompt.trim())
            .then(|| draft.prompt.trim().to_string()),
        timing: timing_changed
            .then(|| timing_from_draft(&draft.timing, interval_from_now, now_ms)),
        target: target_changed.then(|| draft_target(draft, Some(original))),
        active: None,
    }
}

pub fn format_weekdays(weekdays: &[i64], locale: Locale) -> String {
    weekdays
        .iter()
        .map(|weekday| {
            weekday_name(locale, if (1..=7).contains(weekday) { *weekday } else { 1 })
        })
        .collect::<Vec<_>>()
        .join(copy(locale, "frequency.weekday.join"))
}

fn zone_offset_minutes(zone: &str, at_ms: i64) -> Option<i64> {
    let tz = TimeZone::get(zone).ok()?;
    let ts = Timestamp::from_millisecond(at_ms).ok()?;
    Some(tz.to_offset(ts).seconds() as i64 / 60)
}

fn offset_label(minutes: i64, locale: Locale) -> String {
    let absolute = minutes.abs();
    format!(
        "{}{}{:02}:{:02}",
        copy(locale, "time.utcPrefix"),
        if minutes < 0 { "-" } else { "+" },
        absolute / 60,
        absolute % 60
    )
}

pub fn zone_label(zone: &str, locale: Locale, at_ms: i64) -> String {
    match zone_offset_minutes(zone, at_ms) {
        None => canonical_zone_name(zone).unwrap_or_else(|| zone.to_string()),
        Some(minutes) => {
            let label = offset_label(minutes, locale);
            if zone == "UTC" {
                label
            } else {
                format!("{label} · {}", canonical_zone_name(zone).unwrap_or_else(|| zone.to_string()))
            }
        }
    }
}

fn rrule_frequency_text(
    record: &ScheduleCatalogEntry,
    locale: Locale,
    zone: Option<FrequencyZone<'_>>,
    at_ms: i64,
) -> String {
    let text = record.rrule.as_deref().unwrap_or("");
    let kind = host_frequency_kind_of_rrule(text);
    if kind == FrequencyKind::Custom {
        return copy(locale, "frequency.hostCustom").to_string();
    }
    if kind == FrequencyKind::Once {
        return copy(locale, "frequency.once").to_string();
    }
    let fields = rule_fields(text);
    let clock = rrule_clock(text);
    let hour = format!(
        "{:02}",
        fields
            .get("BYHOUR")
            .map(|value| number_of(value))
            .unwrap_or_else(|| number_of(segment(&clock, 0, 2)))
    );
    let minute = format!(
        "{:02}",
        fields
            .get("BYMINUTE")
            .map(|value| number_of(value))
            .unwrap_or_else(|| number_of(segment(&clock, 2, 4)))
    );
    let time = clock_label(&format!("{}:{}:{}", hour, minute, segment(&clock, 4, 6)));
    let time_zone = record_time_zone(record).unwrap_or_else(|| "UTC".to_string());
    let interval = fields.get("INTERVAL").map(|value| number_of(value)).unwrap_or(1);
    let with_zone = |text: String| -> String {
        match zone {
            Some(frequency_zone) if frequency_zone.system == time_zone => text,
            _ => fill(
                copy(locale, "frequency.hostZone"),
                &[
                    ("rule", text),
                    ("timeZone", zone_label(&time_zone, locale, at_ms)),
                ],
            ),
        }
    };
    if kind == FrequencyKind::Weekly {
        let weekdays: Vec<i64> = fields
            .get("BYDAY")
            .map(|days| {
                days.split(',')
                    .map(|day| {
                        WEEKDAY_CODES
                            .iter()
                            .position(|code| *code == day)
                            .map(|position| position as i64 + 1)
                            .unwrap_or(0)
                    })
                    .filter(|day| *day > 0)
                    .collect()
            })
            .unwrap_or_default();
        let template = if interval == 1 {
            "frequency.weeklyLocal"
        } else {
            "frequency.hostWeeklyInterval"
        };
        return with_zone(fill(
            copy(locale, template),
            &[
                ("weekdays", format_weekdays(&weekdays, locale)),
                ("time", time),
                ("interval", interval.to_string()),
            ],
        ));
    }
    if kind == FrequencyKind::Daily {
        let template = if interval == 1 {
            "frequency.dailyLocal"
        } else {
            "frequency.hostDailyInterval"
        };
        return with_zone(fill(
            copy(locale, template),
            &[("time", time), ("interval", interval.to_string())],
        ));
    }
    if kind == FrequencyKind::Minutely || kind == FrequencyKind::Hourly {
        let unit = if kind == FrequencyKind::Minutely {
            TimeUnit::Minute
        } else {
            TimeUnit::Hour
        };
        return fill(
            copy(locale, "frequency.every"),
            &[
                ("value", interval.to_string()),
                ("unit", unit_label(locale, unit, interval)),
            ],
        );
    }
    let days = fields
        .get("BYMONTHDAY")
        .and_then(|days| days.split(',').next())
        .unwrap_or("")
        .to_string();
    let template = if interval == 1 {
        "frequency.hostMonthly"
    } else {
        "frequency.hostMonthlyInterval"
    };
    with_zone(fill(
        copy(locale, template),
        &[
            ("days", days),
            ("time", time),
            ("interval", interval.to_string()),
        ],
    ))
}

pub fn format_schedule_frequency(
    record: &ScheduleCatalogEntry,
    locale: Locale,
    zone: Option<FrequencyZone<'_>>,
    at_ms: i64,
) -> String {
    if record.kind == ScheduleRecordKind::Rrule {
        return rrule_frequency_text(record, locale, zone, at_ms);
    }
    let system_match = zone.map(|frequency_zone| {
        frequency_zone.system == record.time_zone.as_deref().unwrap_or("")
    });
    let zone_name = record.time_zone.as_deref().unwrap_or("");
    match record.kind {
        ScheduleRecordKind::After | ScheduleRecordKind::At => {
            copy(locale, "frequency.once").to_string()
        }
        ScheduleRecordKind::Cron => {
            let parsed = record
                .expression
                .as_deref()
                .and_then(|expression| parse_cron(expression, "UTC").ok());
            if let Some(fields) = parsed {
                if system_match.unwrap_or(false) {
                    return cron_preview(&fields, locale);
                }
                return fill(
                    copy(locale, "frequency.cronRule"),
                    &[
                        ("rule", cron_preview(&fields, locale)),
                        ("timeZone", zone_label(zone_name, locale, at_ms)),
                    ],
                );
            }
            if zone.is_none() || !system_match.unwrap_or(false) {
                return fill(
                    copy(locale, "frequency.cron"),
                    &[("timeZone", zone_label(zone_name, locale, at_ms))],
                );
            }
            copy(locale, "frequency.cronLocal").to_string()
        }
        ScheduleRecordKind::Daily => {
            let time = clock_label(record.time.as_deref().unwrap_or(""));
            if system_match.unwrap_or(false) {
                fill(copy(locale, "frequency.dailyLocal"), &[("time", time)])
            } else {
                fill(
                    copy(locale, "frequency.daily"),
                    &[
                        ("time", time),
                        ("timeZone", zone_label(zone_name, locale, at_ms)),
                    ],
                )
            }
        }
        ScheduleRecordKind::Weekly => {
            let weekdays = format_weekdays(&record.weekdays.clone().unwrap_or_default(), locale);
            let time = clock_label(record.time.as_deref().unwrap_or(""));
            if system_match.unwrap_or(false) {
                fill(
                    copy(locale, "frequency.weeklyLocal"),
                    &[("weekdays", weekdays), ("time", time)],
                )
            } else {
                fill(
                    copy(locale, "frequency.weekly"),
                    &[
                        ("weekdays", weekdays),
                        ("time", time),
                        ("timeZone", zone_label(zone_name, locale, at_ms)),
                    ],
                )
            }
        }
        ScheduleRecordKind::Every => {
            let seconds = record.every_seconds.unwrap_or(0);
            let selected = UNIT_SECONDS
                .iter()
                .find(|unit| seconds % unit.1 == 0)
                .copied()
                .unwrap_or((TimeUnit::Second, 1));
            let value = seconds / selected.1;
            fill(
                copy(locale, "frequency.every"),
                &[
                    ("value", value.to_string()),
                    ("unit", unit_label(locale, selected.0, value)),
                ],
            )
        }
        ScheduleRecordKind::Rrule => rrule_frequency_text(record, locale, zone, at_ms),
    }
}

pub fn format_schedule_absolute(scheduled_at: &str, locale: Locale, zone: &TimeZone) -> String {
    let Ok(ts) = scheduled_at.parse::<Timestamp>() else {
        return scheduled_at.to_string();
    };
    let dt = zone.to_datetime(ts);
    let month = i8::from(dt.month()) as usize;
    match locale {
        Locale::En => format!(
            "{} {}, {}, {:02}:{:02}",
            EN_MONTHS_SHORT[month - 1],
            dt.day(),
            dt.year(),
            dt.hour(),
            dt.minute()
        ),
        Locale::Zh => format!(
            "{}月{}日 {:02}:{:02}",
            month,
            dt.day(),
            dt.hour(),
            dt.minute()
        ),
    }
}

pub fn format_schedule_relative(scheduled_at: &str, now_ms: i64, locale: Locale) -> String {
    let Some(at) = parse_epoch_ms(scheduled_at) else {
        return fill(
            copy(locale, "relative.overdue"),
            &[
                ("value", "1".to_string()),
                ("unit", unit_label(locale, TimeUnit::Second, 1)),
            ],
        );
    };
    let difference = at - now_ms;
    if difference == 0 {
        return copy(locale, "relative.now").to_string();
    }
    let absolute_seconds = difference.abs() as f64 / 1_000.0;
    let selected = UNIT_SECONDS
        .iter()
        .find(|unit| absolute_seconds >= unit.1 as f64)
        .copied()
        .unwrap_or((TimeUnit::Second, 1));
    let raw = if difference > 0 {
        (absolute_seconds / selected.1 as f64).ceil()
    } else {
        (absolute_seconds / selected.1 as f64).floor()
    };
    let value = raw.max(1.0) as i64;
    fill(
        copy(locale, if difference > 0 { "relative.future" } else { "relative.overdue" }),
        &[
            ("value", value.to_string()),
            ("unit", unit_label(locale, selected.0, value)),
        ],
    )
}

pub fn next_run_parts(
    scheduled_at: &str,
    locale: Locale,
    now_ms: i64,
    zone: &TimeZone,
) -> NextRunParts {
    NextRunParts {
        absolute: format_schedule_absolute(scheduled_at, locale, zone),
        relative: format!("({})", format_schedule_relative(scheduled_at, now_ms, locale)),
    }
}
