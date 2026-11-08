use dsh_libadwaita::connection::ConnectionError;
use dsh_libadwaita::schedule_model::*;
use dsh_libadwaita::ui::schedules::model::*;
use serde_json::json;

const NOW: i64 = 1_789_801_200_000;
const SCHEDULED: &str = "2026-09-19T15:51:00Z";
const LATER: &str = "2026-09-20T15:51:00Z";

fn entry(
    kind: ScheduleRecordKind,
    status: ScheduleStatus,
    scheduled_at: &str,
) -> ScheduleCatalogEntry {
    ScheduleCatalogEntry {
        id: "task-1".to_string(),
        kind,
        title: "Nightly digest".to_string(),
        prompt: "Summarize the reading list".to_string(),
        scheduled_at: scheduled_at.to_string(),
        after_seconds: None,
        every_seconds: None,
        time: None,
        time_zone: None,
        weekdays: None,
        expression: None,
        rrule: None,
        target: ScheduleTarget::Session {
            session_id: "sess-1".to_string(),
        },
        involved_session_ids: vec!["sess-1".to_string()],
        status,
        created_at: None,
        updated_at: None,
        execution_active: None,
        delivery_floor_ends_at: None,
        last_delivery: None,
    }
}

fn rrule_task(status: ScheduleStatus) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Rrule, status, SCHEDULED);
    record.rrule = Some(
        "DTSTART;TZID=Asia/Shanghai:20260918T013000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=1;BYMINUTE=30"
            .to_string(),
    );
    record.time_zone = Some("Asia/Shanghai".to_string());
    record
}

#[test]
fn test_filter_entries_by_status() {
    let active = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    let paused = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Paused, SCHEDULED);
    let ended = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Ended, SCHEDULED);
    let records = vec![active, paused, ended];
    let active_rows = filter_entries(&records, StatusFilter::Active, "");
    assert_eq!(active_rows.len(), 1);
    assert_eq!(active_rows[0].status, ScheduleStatus::Active);
    assert_eq!(filter_entries(&records, StatusFilter::Paused, "").len(), 1);
    assert_eq!(filter_entries(&records, StatusFilter::Ended, "").len(), 1);
    assert_eq!(filter_entries(&records, StatusFilter::All, "").len(), 3);
}

#[test]
fn test_filter_search_matches_title_prompt_and_session() {
    let mut by_title = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    by_title.id = "a".to_string();
    by_title.title = "Daily Standup".to_string();
    by_title.prompt = "morning notes".to_string();
    let mut by_prompt = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    by_prompt.id = "b".to_string();
    by_prompt.prompt = "Send the weekly REPORT".to_string();
    let mut by_session = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    by_session.id = "c".to_string();
    by_session.target = ScheduleTarget::Session {
        session_id: "sess-abc-42".to_string(),
    };
    let records = vec![by_title, by_prompt, by_session];
    assert_eq!(
        filter_entries(&records, StatusFilter::All, "standup").len(),
        1
    );
    assert_eq!(
        filter_entries(&records, StatusFilter::All, "report").len(),
        1
    );
    assert_eq!(
        filter_entries(&records, StatusFilter::All, "ABC-42").len(),
        1
    );
    assert_eq!(
        filter_entries(&records, StatusFilter::All, "missing").len(),
        0
    );
}

#[test]
fn test_filter_sorts_by_next_run_ascending() {
    let later = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, LATER);
    let earlier = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    let records = vec![later, earlier];
    let rows = filter_entries(&records, StatusFilter::All, "");
    assert_eq!(rows[0].scheduled_at, SCHEDULED);
    assert_eq!(rows[1].scheduled_at, LATER);
}

#[test]
fn test_ended_count_and_clear_visibility() {
    let active = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Active, SCHEDULED);
    let ended = entry(ScheduleRecordKind::Rrule, ScheduleStatus::Ended, SCHEDULED);
    let records = vec![active.clone(), ended.clone(), ended.clone()];
    assert_eq!(ended_count(&records), 2);
    assert!(!clear_ended_visible(StatusFilter::Active, 2));
    assert!(clear_ended_visible(StatusFilter::Ended, 2));
    assert!(clear_ended_visible(StatusFilter::All, 2));
    assert!(!clear_ended_visible(StatusFilter::All, 0));
}

#[test]
fn test_empty_key_selection() {
    assert_eq!(empty_key("digest", StatusFilter::All), EmptyKey::NoMatches);
    assert_eq!(empty_key("", StatusFilter::Active), EmptyKey::EmptyActive);
    assert_eq!(empty_key("", StatusFilter::Paused), EmptyKey::EmptyPaused);
    assert_eq!(empty_key("", StatusFilter::Ended), EmptyKey::EmptyInactive);
    assert_eq!(empty_key("", StatusFilter::All), EmptyKey::Empty);
    assert_eq!(
        empty_text(EmptyKey::EmptyPaused, Locale::Zh),
        "没有已暂停的自动化任务"
    );
    assert_eq!(
        empty_text(EmptyKey::NoMatches, Locale::En),
        "No matching tasks"
    );
}

#[test]
fn test_row_summary_parts_for_paused_task() {
    let record = rrule_task(ScheduleStatus::Paused);
    let parts = row_summary(&record, Locale::Zh, "Asia/Shanghai", NOW);
    assert_eq!(parts[0], "已暂停");
    assert!(parts[1].contains("每天 01:30"));
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_row_summary_parts_for_active_task() {
    let record = rrule_task(ScheduleStatus::Active);
    let parts = row_summary(&record, Locale::Zh, "Asia/Shanghai", NOW);
    assert_eq!(parts.len(), 2);
    assert!(parts[0].starts_with("每天 01:30"));
    let next = parts.last().unwrap();
    assert!(next.starts_with("下次计划时间："), "got {next}");
    assert!(next.contains("9月19日 23:51"), "got {next}");
    assert!(next.contains("后"), "got {next}");
}

#[test]
fn test_row_summary_countdown_follows_delivery_floor() {
    let mut record = rrule_task(ScheduleStatus::Active);
    record.delivery_floor_ends_at = Some(LATER.to_string());
    let parts = row_summary(&record, Locale::En, "Asia/Shanghai", NOW);
    let next = parts.last().unwrap();
    assert!(next.contains("Sep 20, 2026, 23:51"), "got {next}");
}

#[test]
fn test_detail_next_line_for_statuses() {
    let active = rrule_task(ScheduleStatus::Active);
    let line = detail_next_line(&active, Locale::Zh, "Asia/Shanghai", NOW);
    assert!(line.starts_with("下次运行 "));
    let paused = rrule_task(ScheduleStatus::Paused);
    assert_eq!(
        detail_next_line(&paused, Locale::Zh, "Asia/Shanghai", NOW),
        "已暂停"
    );
}

#[test]
fn test_delivery_floor_active_and_hold_text() {
    let mut record = rrule_task(ScheduleStatus::Active);
    record.delivery_floor_ends_at = Some(LATER.to_string());
    assert!(delivery_floor_active(&record, NOW));
    let text = floor_hold_text(&record, Locale::Zh, NOW).unwrap();
    assert!(text.starts_with("未满最小运行间隔，"), "got {text}");
    assert!(floor_hold_text(&record, Locale::Zh, 1_898_801_200_000).is_none());
    record.delivery_floor_ends_at = None;
    assert!(!delivery_floor_active(&record, NOW));
    assert!(floor_hold_text(&record, Locale::Zh, NOW).is_none());
}

#[test]
fn test_failure_from_error_maps_bridge_codes() {
    let err = ConnectionError::Remote {
        code: -32045,
        message: "schedule refused".to_string(),
        data: Some(json!({"code": "not_future"})),
    };
    assert_eq!(failure_from_error(&err), ScheduleFailure::NotFuture);
    let err = ConnectionError::Remote {
        code: -32045,
        message: "schedule refused".to_string(),
        data: Some(json!({"code": "invalid_time_zone"})),
    };
    assert_eq!(failure_from_error(&err), ScheduleFailure::InvalidRule);
    let err = ConnectionError::Remote {
        code: -32045,
        message: "boom".to_string(),
        data: Some(json!({"code": "invalid_prompt"})),
    };
    assert_eq!(failure_from_error(&err), ScheduleFailure::SaveError);
    assert_eq!(
        failure_from_error(&ConnectionError::Timeout),
        ScheduleFailure::SaveError
    );
}

#[test]
fn test_failure_from_result_maps_operation_codes() {
    assert_eq!(
        failure_from_result(&json!({"id": "t1", "code": "schedule_conflict"})),
        Some(ScheduleFailure::Conflict)
    );
    assert_eq!(
        failure_from_result(&json!({"id": "t1", "code": "schedule_not_found"})),
        Some(ScheduleFailure::Missing)
    );
    assert_eq!(
        failure_from_result(&json!({"id": "t1", "code": "schedule_execution_pending"})),
        Some(ScheduleFailure::ExecutionPending)
    );
    assert_eq!(
        failure_from_result(&json!({"id": "t1", "title": "T"})),
        None
    );
}

#[test]
fn test_failure_text_localized() {
    assert_eq!(
        failure_text(ScheduleFailure::Conflict, false, Locale::Zh),
        "任务已在其他位置更新。可保留修改并复核最新值，或放弃更改。"
    );
    assert_eq!(
        failure_text(ScheduleFailure::NotFuture, false, Locale::En),
        "Choose a future schedule; edit the task timing before resuming"
    );
    assert_eq!(
        failure_text(ScheduleFailure::SaveError, true, Locale::Zh),
        "无法创建任务，已保留填写内容，请重试"
    );
    assert_eq!(
        failure_text(ScheduleFailure::SaveError, false, Locale::Zh),
        "无法保存，已保留修改，请重试"
    );
    assert_eq!(
        failure_text(ScheduleFailure::ExecutionPending, false, Locale::En),
        "This task is queued, running, or was just delivered. Try again later."
    );
}

#[test]
fn test_frequency_row_round_trip() {
    let mut draft = new_task_draft(NOW, "Asia/Shanghai");
    draft.timing.frequency = FrequencyPreset::Weekly;
    draft.timing.weekdays = vec![1, 2, 3, 4, 5];
    assert_eq!(frequency_row_of(&draft.timing), 1);
    draft.timing.weekdays = vec![1, 6];
    assert_eq!(frequency_row_of(&draft.timing), 0);

    apply_frequency_row(&mut draft, 1, false, NOW);
    assert_eq!(draft.timing.weekdays, vec![1, 2, 3, 4, 5]);
    apply_frequency_row(&mut draft, 3, false, NOW);
    assert_eq!(draft.timing.frequency, FrequencyPreset::Hourly);
    apply_frequency_row(&mut draft, 7, false, NOW);
    assert_eq!(draft.timing.frequency, FrequencyPreset::Advanced);
    assert!(draft.timing.rrule.contains("RRULE:"));
}

#[test]
fn test_frequency_labels_are_localized() {
    let labels = frequency_labels(Locale::Zh);
    assert_eq!(labels[0], "每周");
    assert_eq!(labels[1], "周一至周五");
    assert_eq!(labels[5], "每月");
    assert_eq!(labels[7], "自定义");
    let labels = frequency_labels(Locale::En);
    assert_eq!(labels[3], "Every N hours");
    assert_eq!(labels[6], "Once");
}

#[test]
fn test_target_row_mapping() {
    assert_eq!(target_row_of(TargetChoice::NewOnce, false), 0);
    assert_eq!(target_row_of(TargetChoice::New, false), 1);
    assert_eq!(target_choice_of_row(0, false), TargetChoice::NewOnce);
    assert_eq!(target_choice_of_row(1, false), TargetChoice::New);
    assert_eq!(target_row_of(TargetChoice::Session, true), 0);
    assert_eq!(target_row_of(TargetChoice::NewOnce, true), 1);
    assert_eq!(target_choice_of_row(0, true), TargetChoice::Session);
    assert_eq!(target_choice_of_row(1, true), TargetChoice::NewOnce);
}

#[test]
fn test_patch_is_empty() {
    let mut patch = SchedulePatch::default();
    assert!(patch_is_empty(&patch));
    patch.title = Some("Renamed".to_string());
    assert!(!patch_is_empty(&patch));
    let empty_timing = SchedulePatch {
        timing: Some(ScheduleTiming::After { after_seconds: 60 }),
        ..Default::default()
    };
    assert!(!patch_is_empty(&empty_timing));
}

#[test]
fn test_reapply_task_draft_keeps_changed_groups_only() {
    let mut original = rrule_task(ScheduleStatus::Active);
    original.updated_at = Some("v1".to_string());
    let mut latest = rrule_task(ScheduleStatus::Active);
    latest.title = "Renamed elsewhere".to_string();
    latest.prompt = "Replaced prompt".to_string();

    let mut draft = task_draft(&original, NOW, "Asia/Shanghai");
    draft.prompt = "My new prompt".to_string();
    let changes = task_draft_patch(&draft, &original, false, NOW, "Asia/Shanghai");
    assert!(changes.prompt.is_some());

    let reapplied = reapply_task_draft(&draft, &latest, &changes, NOW, "Asia/Shanghai");
    assert_eq!(reapplied.prompt, "My new prompt");
    assert_eq!(reapplied.title, "Renamed elsewhere");

    let mut latest_newer = rrule_task(ScheduleStatus::Active);
    latest_newer.rrule = Some(
        "DTSTART;TZID=Asia/Shanghai:20260918T023000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=2;BYMINUTE=30"
            .to_string(),
    );
    let reapplied = reapply_task_draft(&draft, &latest_newer, &changes, NOW, "Asia/Shanghai");
    let seeded_latest = timing_from_record(&latest_newer, NOW, "Asia/Shanghai");
    assert_eq!(reapplied.timing, seeded_latest);
}

#[test]
fn test_available_zones_and_search() {
    let zones = available_zones();
    assert!(zones.contains(&"UTC".to_string()));
    assert!(zones.iter().any(|zone| zone == "Asia/Shanghai"));
    assert!(zone_matches("Asia/Shanghai", "shang"));
    assert!(zone_matches("Asia/Shanghai", "asia"));
    assert!(!zone_matches("Asia/Shanghai", "paris"));
}

#[test]
fn test_merge_history_page_dedupes_by_message_id() {
    let record = |id: &str| ScheduleDeliveryRecord {
        scheduled_at: SCHEDULED.to_string(),
        delivered_at: SCHEDULED.to_string(),
        message_id: id.to_string(),
        target_session_id: "sess-1".to_string(),
        prompt: None,
    };
    let previous = vec![record("m1"), record("m2")];
    let page = ScheduleDeliveryHistoryPage {
        id: "task-1".to_string(),
        records: vec![record("m2"), record("m3")],
        earlier_records_unavailable: false,
        earlier_records_pruned: false,
        retention: DeliveryRetentionBounds {
            days: 30,
            records: 200,
        },
        next_before: Some("cursor".to_string()),
    };
    let merged = merge_history_page(&previous, &page);
    let ids: Vec<&str> = merged.iter().map(|item| item.message_id.as_str()).collect();
    assert_eq!(ids, vec!["m1", "m2", "m3"]);
}

#[test]
fn test_toasts_and_create_prompt() {
    assert_eq!(create_prompt(Locale::Zh), "请根据需求创建一个自动化任务。");
    assert_eq!(
        create_prompt(Locale::En),
        "Please create an automation task based on the requirements."
    );
    assert_eq!(clear_ended_toast(3, Locale::Zh), "已清除 3 个已结束任务");
    assert_eq!(clear_ended_toast(0, Locale::En), "No ended tasks to clear.");
    assert_eq!(delete_toast(Locale::Zh), "任务已删除");
}

#[test]
fn test_linked_state_from_cache() {
    assert_eq!(linked_state(Some(false)), LinkedState::Available);
    assert_eq!(linked_state(Some(true)), LinkedState::Archived);
    assert_eq!(linked_state(None), LinkedState::Unavailable);
}

#[test]
fn test_draft_issue_text_localized() {
    assert_eq!(
        issue_text(DraftIssue::InvalidTitle, Locale::Zh),
        "请输入1至120个字符的任务名称"
    );
    assert_eq!(
        issue_text(DraftIssue::RequiredWeekday, Locale::En),
        "Select at least one weekday."
    );
    assert_eq!(
        issue_text(DraftIssue::TooComplex, Locale::Zh),
        "此规则的运行频率过高或过于复杂，请选择其他重复方式，或让智能体简化规则"
    );
}

#[test]
fn test_zone_label_prefers_system_zone_match() {
    let record = rrule_task(ScheduleStatus::Active);
    let frequency = format_schedule_frequency(
        &record,
        Locale::Zh,
        Some(FrequencyZone {
            system: "Asia/Shanghai",
        }),
        NOW,
    );
    assert_eq!(frequency, "每天 01:30");
    let frequency = format_schedule_frequency(
        &record,
        Locale::Zh,
        Some(FrequencyZone { system: "UTC" }),
        NOW,
    );
    assert!(frequency.contains("Asia/Shanghai"), "got {frequency}");
}

#[test]
fn test_clamp_title_counts_utf16_units() {
    assert_eq!(clamp_title("abc"), "abc");
    let limit_title: String = "🦀".repeat(60);
    assert_eq!(limit_title.chars().count(), 60);
    assert_eq!(limit_title.encode_utf16().count(), 120);
    assert_eq!(clamp_title(&limit_title), limit_title);
    let overflow = format!("{limit_title}x");
    let clamped = clamp_title(&overflow);
    assert_eq!(clamped, limit_title);
    assert!(clamped.encode_utf16().count() <= 120);
    let mixed = format!("a{}", "😀".repeat(60));
    let clamped = clamp_title(&mixed);
    assert_eq!(clamped.chars().count(), 60);
    assert_eq!(clamped.encode_utf16().count(), 119);
    assert!(clamped.encode_utf16().count() <= 120);
}

#[test]
fn test_history_miss_maps_host_codes() {
    assert_eq!(
        history_miss("delivery_cursor_not_found"),
        ScheduleHistoryMiss::CursorInvalid
    );
    assert_eq!(
        history_miss("schedule_not_found"),
        ScheduleHistoryMiss::NotFound
    );
    assert_eq!(history_miss("something-else"), ScheduleHistoryMiss::NotFound);
}
