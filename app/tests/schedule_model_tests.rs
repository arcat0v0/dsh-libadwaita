mod schedule_model {
use dsh_libadwaita::schedule_model::*;
use std::collections::HashSet;

const NOW: i64 = 1789801200000;
const SCHED_1551: &str = "2026-09-19T15:51:00Z";
const SCHED_1551_MS: i64 = 1789833060000;
const SCHED_WEEKLY: &str = "2026-09-18T00:00:00Z";
const SCHED_DAILY: &str = "2026-09-18T01:30:00Z";

fn zone_utc() -> jiff::tz::TimeZone {
    jiff::tz::TimeZone::UTC
}

fn zone_shanghai() -> jiff::tz::TimeZone {
    jiff::tz::TimeZone::get("Asia/Shanghai").unwrap()
}

fn entry(kind: ScheduleRecordKind, scheduled_at: &str) -> ScheduleCatalogEntry {
    ScheduleCatalogEntry {
        id: "task-1".to_string(),
        kind,
        title: "T".to_string(),
        prompt: "P".to_string(),
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
        status: ScheduleStatus::Active,
        created_at: None,
        updated_at: None,
        execution_active: None,
        delivery_floor_ends_at: None,
        last_delivery: None,
    }
}

fn rrule_entry(rrule: &str, scheduled_at: &str) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Rrule, scheduled_at);
    record.rrule = Some(rrule.to_string());
    record
}

fn daily_record(time: &str, time_zone: &str, scheduled_at: &str) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Daily, scheduled_at);
    record.time = Some(time.to_string());
    record.time_zone = Some(time_zone.to_string());
    record
}

fn weekly_record(
    time: &str,
    weekdays: &[i64],
    time_zone: &str,
    scheduled_at: &str,
) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Weekly, scheduled_at);
    record.time = Some(time.to_string());
    record.time_zone = Some(time_zone.to_string());
    record.weekdays = Some(weekdays.to_vec());
    record
}

fn every_record(every_seconds: i64, scheduled_at: &str) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Every, scheduled_at);
    record.every_seconds = Some(every_seconds);
    record
}

fn cron_record(expression: &str, time_zone: &str, scheduled_at: &str) -> ScheduleCatalogEntry {
    let mut record = entry(ScheduleRecordKind::Cron, scheduled_at);
    record.expression = Some(expression.to_string());
    record.time_zone = Some(time_zone.to_string());
    record
}

fn guided_once(start: &str, time_zone: &str) -> TimingDraft {
    TimingDraft {
        frequency: FrequencyPreset::Once,
        interval: "1".to_string(),
        start: start.to_string(),
        time_zone: time_zone.to_string(),
        rrule: String::new(),
        retained_timing: None,
        weekdays: vec![6],
        month_day: 19,
        daily_time: "23:51".to_string(),
    }
}

#[test]
fn test_decodes_realistic_catalog_entry() {
    let raw = serde_json::json!({
        "id": "sch-9f2c",
        "kind": "rrule",
        "title": "Morning digest",
        "prompt": "Summarize the reading list.",
        "rrule": "DTSTART;TZID=Asia/Shanghai:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR",
        "scheduledAt": "2026-09-21T00:00:00.000Z",
        "target": {
            "kind": "new_once",
            "sessionId": "sess-reserved",
            "create": { "cwd": "/work/demo", "agentPreset": "planner" }
        },
        "involvedSessionIds": ["sess-owner", "sess-reserved"],
        "status": "active",
        "createdAt": "2026-09-18T02:00:00.000Z",
        "updatedAt": "2026-09-19T10:00:00.000Z",
        "executionActive": true,
        "deliveryFloorEndsAt": "2026-09-19T10:05:00.000Z",
        "lastDelivery": {
            "scheduledAt": "2026-09-18T00:00:00.000Z",
            "deliveredAt": "2026-09-18T00:00:01.200Z",
            "messageId": "msg-77",
            "targetSessionId": "sess-owner"
        }
    });
    let record: ScheduleCatalogEntry = serde_json::from_value(raw).unwrap();
    assert_eq!(record.id, "sch-9f2c");
    assert_eq!(record.kind, ScheduleRecordKind::Rrule);
    assert_eq!(record.title, "Morning digest");
    assert_eq!(
        record.rrule.as_deref(),
        Some("DTSTART;TZID=Asia/Shanghai:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR")
    );
    assert_eq!(
        record.target,
        ScheduleTarget::NewOnce {
            session_id: "sess-reserved".to_string(),
            create: ScheduleSessionCreateSpec {
                cwd: Some("/work/demo".to_string()),
                agent_preset: Some("planner".to_string()),
            },
        }
    );
    assert_eq!(
        record.involved_session_ids,
        vec!["sess-owner", "sess-reserved"]
    );
    assert_eq!(record.status, ScheduleStatus::Active);
    assert_eq!(record.updated_at.as_deref(), Some("2026-09-19T10:00:00.000Z"));
    assert_eq!(record.execution_active, Some(true));
    assert_eq!(
        record.delivery_floor_ends_at.as_deref(),
        Some("2026-09-19T10:05:00.000Z")
    );
    let receipt = record.last_delivery.unwrap();
    assert_eq!(receipt.message_id, "msg-77");
    assert_eq!(receipt.target_session_id, "sess-owner");

    let mut after = entry(ScheduleRecordKind::After, SCHED_1551);
    after.after_seconds = Some(300);
    let encoded = serde_json::to_value(&after).unwrap();
    assert_eq!(encoded["kind"], "after");
    assert_eq!(encoded["afterSeconds"], 300);
    assert_eq!(encoded["scheduledAt"], SCHED_1551);
    assert!(encoded.get("everySeconds").is_none());
}

#[test]
fn test_decodes_delivery_history_page_with_retention() {
    let raw = serde_json::json!({
        "id": "sch-9f2c",
        "records": [
            {
                "scheduledAt": "2026-09-18T00:00:00.000Z",
                "deliveredAt": "2026-09-18T00:00:01.200Z",
                "messageId": "msg-77",
                "targetSessionId": "sess-owner",
                "prompt": "Summarize."
            }
        ],
        "earlierRecordsUnavailable": false,
        "earlierRecordsPruned": true,
        "retention": { "days": 30, "records": 200 },
        "nextBefore": "msg-77"
    });
    let page: ScheduleDeliveryHistoryPage = serde_json::from_value(raw).unwrap();
    assert_eq!(page.id, "sch-9f2c");
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].prompt.as_deref(), Some("Summarize."));
    assert!(page.earlier_records_pruned);
    assert!(!page.earlier_records_unavailable);
    assert_eq!(
        page.retention,
        DeliveryRetentionBounds {
            days: 30,
            records: 200
        }
    );
    assert_eq!(page.next_before.as_deref(), Some("msg-77"));
}

#[test]
fn test_task_next_run_at_prefers_the_later_instant() {
    let mut record = entry(ScheduleRecordKind::At, SCHED_1551);
    assert_eq!(task_next_run_at(&record), SCHED_1551);
    record.delivery_floor_ends_at = Some("2026-09-19T16:20:00Z".to_string());
    assert_eq!(task_next_run_at(&record), "2026-09-19T16:20:00Z");
    record.delivery_floor_ends_at = Some("2026-09-19T15:00:00Z".to_string());
    assert_eq!(task_next_run_at(&record), SCHED_1551);
    record.delivery_floor_ends_at = Some("not-a-date".to_string());
    assert_eq!(task_next_run_at(&record), SCHED_1551);
    record.delivery_floor_ends_at = Some(SCHED_1551.to_string());
    assert_eq!(task_next_run_at(&record), SCHED_1551);
}

#[test]
fn test_task_session_id_follows_the_target_kind() {
    let mut record = entry(ScheduleRecordKind::At, SCHED_1551);
    record.target = ScheduleTarget::Session {
        session_id: "sess-fixed".to_string(),
    };
    assert_eq!(task_session_id(&record).as_deref(), Some("sess-fixed"));
    record.target = ScheduleTarget::NewOnce {
        session_id: "sess-reserved".to_string(),
        create: ScheduleSessionCreateSpec::default(),
    };
    assert_eq!(task_session_id(&record).as_deref(), Some("sess-reserved"));
    record.target = ScheduleTarget::New {
        create: ScheduleSessionCreateSpec::default(),
    };
    assert_eq!(task_session_id(&record), None);
    record.last_delivery = Some(ScheduleDeliveryReceipt {
        scheduled_at: SCHED_1551.to_string(),
        delivered_at: "2026-09-19T15:52:00Z".to_string(),
        message_id: "msg-1".to_string(),
        target_session_id: "sess-latest".to_string(),
    });
    assert_eq!(task_session_id(&record).as_deref(), Some("sess-latest"));
}

#[test]
fn test_rule_fields_reads_the_first_rrule_line() {
    let fields =
        rule_fields("DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;INTERVAL=2;BYHOUR=9");
    assert_eq!(fields.get("FREQ").map(String::as_str), Some("DAILY"));
    assert_eq!(fields.get("INTERVAL").map(String::as_str), Some("2"));
    assert_eq!(fields.get("BYHOUR").map(String::as_str), Some("9"));
    assert_eq!(fields.get("COUNT"), None);
    assert_eq!(rule_fields("RRULE:FREQ=WEEKLY").len(), 1);
}

#[test]
fn test_host_frequency_kind_detects_guided_rules() {
    let cases: &[(&str, FrequencyKind)] = &[
        (
            "DTSTART;TZID=Asia/Shanghai:20260918T093000\nRRULE:FREQ=DAILY;COUNT=1",
            FrequencyKind::Once,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MINUTELY;INTERVAL=5",
            FrequencyKind::Minutely,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=HOURLY;INTERVAL=2",
            FrequencyKind::Hourly,
        ),
        (
            "DTSTART;TZID=Asia/Shanghai:20260918T093000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=9;BYMINUTE=30",
            FrequencyKind::Daily,
        ),
        (
            "DTSTART;TZID=Asia/Shanghai:20260918T093000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=9;BYMINUTE=30;BYSETPOS=1",
            FrequencyKind::Daily,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;INTERVAL=2",
            FrequencyKind::Daily,
        ),
        (
            "DTSTART:20260919T155100Z\nRRULE:FREQ=DAILY;INTERVAL=2",
            FrequencyKind::Daily,
        ),
        (
            "DTSTART;TZID=UTC:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR",
            FrequencyKind::Weekly,
        ),
        (
            "DTSTART;TZID=UTC:20260919T235100\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=19,-1;BYSETPOS=1",
            FrequencyKind::Monthly,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=5",
            FrequencyKind::Custom,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;UNTIL=20261231T235900Z",
            FrequencyKind::Custom,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;BYSECOND=30",
            FrequencyKind::Custom,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;INTERVAL=1;BYSETPOS=1",
            FrequencyKind::Custom,
        ),
        (
            "DTSTART;TZID=UTC:20260919T155101\nRRULE:FREQ=DAILY;INTERVAL=1",
            FrequencyKind::Custom,
        ),
        ("RRULE:FREQ=YEARLY", FrequencyKind::Custom),
        (
            "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=YEARLY;BYMONTHDAY=1",
            FrequencyKind::Custom,
        ),
    ];
    for (text, expected) in cases {
        let record = rrule_entry(text, SCHED_1551);
        assert_eq!(host_frequency_kind(&record), *expected, "rule {text:?}");
    }
}

#[test]
fn test_default_timing_seeds_one_hour_ahead() {
    let draft = default_timing(NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Once);
    assert_eq!(draft.start, "2026-09-19T16:00:00");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(draft.interval, "1");
    assert_eq!(draft.rrule, "");
    assert_eq!(draft.retained_timing, None);
    assert_eq!(draft.weekdays, vec![6]);
    assert_eq!(draft.month_day, 19);
    assert_eq!(draft.daily_time, "16:00");
}

#[test]
fn test_timing_from_record_round_trips_absolute_one_shot() {
    let mut record = entry(ScheduleRecordKind::At, SCHED_1551);
    record.target = ScheduleTarget::NewOnce {
        session_id: "sess-r".to_string(),
        create: ScheduleSessionCreateSpec::default(),
    };
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Once);
    assert_eq!(draft.start, "2026-09-19T23:51:00");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(draft.daily_time, "23:51");
    assert_eq!(draft.month_day, 19);
    assert_eq!(draft.weekdays, vec![6]);
    assert_eq!(
        draft.retained_timing,
        Some(ScheduleTiming::At {
            at: ScheduleAtInput::Instant(SCHED_1551.to_string()),
        })
    );
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=DAILY;COUNT=1".to_string(),
        }
    );

    let mut after = entry(ScheduleRecordKind::After, SCHED_1551);
    after.after_seconds = Some(300);
    let draft = timing_from_record(&after, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Once);
    assert_eq!(draft.start, "2026-09-19T23:51:00");
}

#[test]
fn test_timing_from_record_falls_back_to_utc_for_ambiguous_local_displays() {
    let record = entry(ScheduleRecordKind::At, "2026-11-01T06:30:00Z");
    let draft = timing_from_record(&record, NOW, "America/New_York");
    assert_eq!(draft.time_zone, "UTC");
    assert_eq!(draft.start, "2026-11-01T06:30:00");
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=UTC:20261101T063000\nRRULE:FREQ=DAILY;COUNT=1".to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_keeps_millisecond_targets_readable() {
    let record = entry(ScheduleRecordKind::At, "2026-09-19T15:51:00.500Z");
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(draft.start, "2026-09-19T23:51:00");
}

#[test]
fn test_timing_from_record_round_trips_fixed_intervals() {
    let record = every_record(300, SCHED_1551);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Minutely);
    assert_eq!(draft.interval, "5");
    assert_eq!(draft.time_zone, "UTC");
    assert_eq!(draft.start, "2026-09-19T15:51:00");
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MINUTELY;INTERVAL=5".to_string(),
        }
    );

    let record = every_record(7200, SCHED_1551);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Hourly);
    assert_eq!(draft.interval, "2");
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=HOURLY;INTERVAL=2".to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_round_trips_daily_records() {
    let record = daily_record("09:30:00", "Asia/Shanghai", SCHED_DAILY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Daily);
    assert_eq!(draft.daily_time, "09:30");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(
        draft.retained_timing,
        Some(ScheduleTiming::Daily {
            daily: ScheduleDailyInput {
                time: "09:30:00".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
            },
        })
    );
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=Asia/Shanghai:20260918T093000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=9;BYMINUTE=30"
                .to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_round_trips_weekly_records() {
    let record = weekly_record("08:00:00", &[1, 3, 5], "Asia/Shanghai", SCHED_WEEKLY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Weekly);
    assert_eq!(draft.start, "2026-09-18T08:00:00");
    assert_eq!(draft.weekdays, vec![1, 3, 5]);
    assert_eq!(draft.month_day, 18);
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=Asia/Shanghai:20260918T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR"
                .to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_keeps_the_monday_to_friday_preset() {
    let record = weekly_record("08:00:00", &[1, 2, 3, 4, 5], "Asia/Shanghai", SCHED_WEEKLY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Weekly);
    assert_eq!(draft.weekdays, vec![1, 2, 3, 4, 5]);
    let timing = timing_from_draft(&draft, false, NOW);
    assert_eq!(
        timing,
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=Asia/Shanghai:20260918T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,TU,WE,TH,FR"
                .to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_round_trips_guided_rrule_records() {
    let once_text = "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=DAILY;COUNT=1";
    let once = rrule_entry(once_text, SCHED_1551);
    let draft = timing_from_record(&once, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Once);
    assert_eq!(draft.start, "2026-09-19T23:51:00");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: once_text.to_string(),
        }
    );

    let minutely_text = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MINUTELY;INTERVAL=5";
    let minutely = rrule_entry(minutely_text, SCHED_1551);
    let draft = timing_from_record(&minutely, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Minutely);
    assert_eq!(draft.interval, "5");
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: minutely_text.to_string(),
        }
    );

    let monthly_text = "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=19,-1;BYSETPOS=1";
    let monthly = rrule_entry(monthly_text, SCHED_1551);
    let draft = timing_from_record(&monthly, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Monthly);
    assert_eq!(draft.month_day, 19);
    assert_eq!(draft.daily_time, "23:51");
    assert_eq!(draft.weekdays, vec![6]);
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: monthly_text.to_string(),
        }
    );
}

#[test]
fn test_timing_from_record_opens_unsupported_rules_in_advanced_mode() {
    let counted_text = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=5";
    let counted = rrule_entry(counted_text, SCHED_1551);
    let draft = timing_from_record(&counted, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.rrule, counted_text);
    assert_eq!(draft.time_zone, "UTC");
    assert_eq!(draft.retained_timing, None);
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: counted_text.to_string(),
        }
    );

    let bare = rrule_entry("RRULE:FREQ=YEARLY", SCHED_1551);
    let draft = timing_from_record(&bare, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.rrule, "RRULE:FREQ=YEARLY");
    assert_eq!(draft.time_zone, "UTC");
    assert_eq!(draft.start, "2026-09-19T16:00:00");

    let anchored = rrule_entry(
        "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=19,-1;BYSETPOS=1;BYSECOND=30",
        SCHED_1551,
    );
    let draft = timing_from_record(&anchored, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.month_day, 19);
}

#[test]
fn test_timing_from_record_opens_unsupported_typed_rules_in_advanced_mode() {
    let record = every_record(90, SCHED_1551);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.rrule, "");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(draft.start, "2026-09-19T16:00:00");
    assert_eq!(
        draft.retained_timing,
        Some(ScheduleTiming::Every { every_seconds: 90 })
    );
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Every { every_seconds: 90 }
    );

    let record = daily_record("09:30:15", "Asia/Shanghai", SCHED_DAILY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(
        draft.retained_timing,
        Some(ScheduleTiming::Daily {
            daily: ScheduleDailyInput {
                time: "09:30:15".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
            },
        })
    );

    let record = weekly_record("08:00", &[1, 3], "Asia/Shanghai", SCHED_WEEKLY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(
        draft.retained_timing,
        Some(ScheduleTiming::Weekly {
            weekly: ScheduleWeeklyInput {
                time: "08:00".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
                weekdays: vec![1, 3],
            },
        })
    );
}

#[test]
fn test_timing_from_record_opens_cron_records_in_advanced_mode() {
    let record = cron_record("*/10 9-17 * * 1-5", "Asia/Shanghai", SCHED_1551);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(draft.frequency, FrequencyPreset::Advanced);
    assert_eq!(draft.rrule, "*/10 9-17 * * 1-5");
    assert_eq!(draft.time_zone, "Asia/Shanghai");
    assert_eq!(draft.retained_timing, None);
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Cron {
            cron: ScheduleCronInput {
                expression: "*/10 9-17 * * 1-5".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
            },
        }
    );
}

#[test]
fn test_rrule_from_draft_anchors_fixed_intervals_to_the_current_minute() {
    let mut draft = guided_once("2026-09-19T23:51:00", "Asia/Shanghai");
    draft.frequency = FrequencyPreset::Minutely;
    draft.interval = "5".to_string();
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T150000\nRRULE:FREQ=MINUTELY;INTERVAL=5"
    );
    assert_eq!(
        rrule_from_draft(&draft, false, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=MINUTELY;INTERVAL=5"
    );
    draft.frequency = FrequencyPreset::Hourly;
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T150000\nRRULE:FREQ=HOURLY;INTERVAL=5"
    );
}

#[test]
fn test_rrule_from_draft_compiles_guided_presets() {
    let mut draft = guided_once("2026-09-19T23:51:00", "Asia/Shanghai");
    assert_eq!(
        rrule_from_draft(&draft, false, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=DAILY;COUNT=1"
    );
    draft.frequency = FrequencyPreset::Daily;
    draft.daily_time = "09:30".to_string();
    assert_eq!(
        rrule_from_draft(&draft, false, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T093000\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=9;BYMINUTE=30"
    );
    draft.frequency = FrequencyPreset::Weekly;
    draft.weekdays = vec![1, 3, 5];
    draft.start = "2026-09-18T08:00:00".to_string();
    assert_eq!(
        rrule_from_draft(&draft, false, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260918T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR"
    );
    draft.month_day = 31;
    draft.start = "2026-09-19T08:00:00".to_string();
    draft.frequency = FrequencyPreset::Monthly;
    assert_eq!(
        rrule_from_draft(&draft, false, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T080000\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31,-1;BYSETPOS=1"
    );
    draft.frequency = FrequencyPreset::Advanced;
    draft.rrule = "*/5 * * * *".to_string();
    assert_eq!(rrule_from_draft(&draft, false, NOW), "*/5 * * * *");
    assert_eq!(rrule_from_draft(&draft, true, NOW), "*/5 * * * *");
}

#[test]
fn test_rrule_from_draft_anchors_edited_typed_recurrences_to_the_current_period() {
    let record = weekly_record("08:00:00", &[1, 5], "Asia/Shanghai", SCHED_WEEKLY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,FR"
    );

    let record = daily_record("23:59:00", "Asia/Shanghai", SCHED_DAILY);
    let draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T235900\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=23;BYMINUTE=59"
    );

    let mut draft = guided_once("2026-09-19T08:00:00", "Asia/Shanghai");
    draft.frequency = FrequencyPreset::Monthly;
    draft.month_day = 31;
    draft.retained_timing = Some(ScheduleTiming::Rrule {
        rrule: "DTSTART;TZID=Asia/Shanghai:20260919T080000\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31,-1;BYSETPOS=1"
            .to_string(),
    });
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260930T080000\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=31,-1;BYSETPOS=1"
    );

    let mut draft = guided_once("2026-09-19T23:51:00", "Asia/Shanghai");
    draft.retained_timing = Some(ScheduleTiming::At {
        at: ScheduleAtInput::Instant(SCHED_1551.to_string()),
    });
    assert_eq!(
        rrule_from_draft(&draft, true, NOW),
        "DTSTART;TZID=Asia/Shanghai:20260919T235100\nRRULE:FREQ=DAILY;COUNT=1"
    );
}

#[test]
fn test_timing_from_draft_parses_advanced_cron_input() {
    let mut draft = guided_once("2026-09-19T23:51:00", "UTC");
    draft.frequency = FrequencyPreset::Advanced;
    draft.rrule = "30 8 * * 1-5".to_string();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Cron {
            cron: ScheduleCronInput {
                expression: "30 8 * * 1-5".to_string(),
                time_zone: "UTC".to_string(),
            },
        }
    );
    draft.rrule = "  30 8 * * 1-5  ".to_string();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Cron {
            cron: ScheduleCronInput {
                expression: "30 8 * * 1-5".to_string(),
                time_zone: "UTC".to_string(),
            },
        }
    );
    draft.rrule = "definitely not a rule".to_string();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Cron {
            cron: ScheduleCronInput {
                expression: "definitely not a rule".to_string(),
                time_zone: "UTC".to_string(),
            },
        }
    );
    draft.rrule = String::new();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Cron {
            cron: ScheduleCronInput {
                expression: String::new(),
                time_zone: "UTC".to_string(),
            },
        }
    );
}

#[test]
fn test_timing_from_draft_retains_stored_timing_in_advanced_mode() {
    let record = every_record(90, SCHED_1551);
    let mut draft = timing_from_record(&record, NOW, "Asia/Shanghai");
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Every { every_seconds: 90 }
    );
    draft.rrule = "   ".to_string();
    assert_eq!(
        timing_from_draft(&draft, true, NOW),
        ScheduleTiming::Every { every_seconds: 90 }
    );
    draft.rrule = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=1".to_string();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=1".to_string(),
        }
    );
    draft.rrule = "  DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=1  ".to_string();
    assert_eq!(
        timing_from_draft(&draft, false, NOW),
        ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=1".to_string(),
        }
    );
}

#[test]
fn test_task_draft_issue_reports_each_correction_key() {
    let mut draft = new_task_draft(NOW, "Asia/Shanghai");
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Incomplete));
    draft.title = "Name".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Incomplete));
    draft.prompt = "Do the thing".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
    draft.title = "x".repeat(121);
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidTitle));
    draft.title = "x".repeat(120);
    assert_eq!(task_draft_issue(&draft, None, NOW), None);

    draft.timing.frequency = FrequencyPreset::Minutely;
    for bad in ["0", "-1", "1.5", "01", " 2", "2 ", "9007199254740992", "abc"] {
        draft.timing.interval = bad.to_string();
        assert_eq!(
            task_draft_issue(&draft, None, NOW),
            Some(DraftIssue::InvalidInterval),
            "interval {bad:?}"
        );
    }
    draft.timing.interval = "9007199254740991".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);

    draft.timing.frequency = FrequencyPreset::Weekly;
    draft.timing.weekdays = vec![];
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::RequiredWeekday));
    draft.timing.weekdays = vec![1, 5];
    assert_eq!(task_draft_issue(&draft, None, NOW), None);

    draft.timing.frequency = FrequencyPreset::Monthly;
    draft.timing.month_day = 0;
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidMonthDay));
    draft.timing.month_day = 32;
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidMonthDay));
    draft.timing.month_day = 31;
    assert_eq!(task_draft_issue(&draft, None, NOW), None);

    draft.timing.frequency = FrequencyPreset::Daily;
    draft.timing.daily_time = "24:00".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidTime));
    draft.timing.daily_time = "09:60".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidTime));
    draft.timing.daily_time = "9:30".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidTime));
    draft.timing.daily_time = "09:30".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
}

#[test]
fn test_task_draft_issue_validates_advanced_rules() {
    let mut draft = new_task_draft(NOW, "Asia/Shanghai");
    draft.title = "Name".to_string();
    draft.prompt = "Do the thing".to_string();
    draft.timing.frequency = FrequencyPreset::Advanced;

    draft.timing.rrule = "61 * * * *".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Invalid));
    draft.timing.rrule = "30 8 * *".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Invalid));
    draft.timing.rrule = " 30 8 * * * ".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
    draft.timing.rrule = String::new();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Invalid));
    draft.timing.rrule = "30 8 * * *".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);

    draft.timing.time_zone = "Not/AZone".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidZone));
    draft.timing.time_zone = "Asia/Shanghai".to_string();
    draft.timing.rrule = "DTSTART;TZID=Not/AZone:20260919T155100\nRRULE:FREQ=DAILY".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidZone));

    draft.timing.time_zone = "UTC".to_string();
    draft.timing.rrule = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=SECONDLY".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::TooComplex));
    draft.timing.rrule = concat!(
        "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MONTHLY;BYMONTHDAY=",
        "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31;",
        "BYHOUR=0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23;",
        "BYMINUTE=0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,",
        "28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59"
    )
    .to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::TooComplex));
    draft.timing.rrule = "RRULE:FREQ=DAILY".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Invalid));
    draft.timing.rrule = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;BYSETPOS=1".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Invalid));
    draft.timing.rrule = "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=5".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
}

#[test]
fn test_task_draft_issue_reports_guided_time_failures() {
    let mut draft = new_task_draft(NOW, "Asia/Shanghai");
    draft.title = "Name".to_string();
    draft.prompt = "Do the thing".to_string();
    draft.timing.start = "2026-02-30T10:00:00".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidTime));
    draft.timing.start = "2026-09-19T10:00:00".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
    draft.timing.time_zone = "Not/AZone".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::InvalidZone));
}

#[test]
fn test_task_draft_issue_treats_untouched_stored_timing_as_authoritative() {
    let record = daily_record("09:30:00", "Asia/Shanghai", SCHED_DAILY);
    let mut draft = task_draft(&record, NOW, "Asia/Shanghai");
    draft.timing.frequency = FrequencyPreset::Once;
    draft.timing.start = "2026-02-30T09:30:00".to_string();
    let changes = SchedulePatch {
        title: Some("Renamed".to_string()),
        ..SchedulePatch::default()
    };
    assert_eq!(task_draft_issue(&draft, Some(&changes), NOW), None);
    let changes = SchedulePatch {
        timing: Some(ScheduleTiming::Daily {
            daily: ScheduleDailyInput {
                time: "09:30:00".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
            },
        }),
        ..SchedulePatch::default()
    };
    assert_eq!(
        task_draft_issue(&draft, Some(&changes), NOW),
        Some(DraftIssue::InvalidTime)
    );
}

#[test]
fn test_task_draft_issue_rejects_session_targets_without_a_session() {
    let mut draft = new_task_draft(NOW, "Asia/Shanghai");
    draft.title = "Name".to_string();
    draft.prompt = "Do the thing".to_string();
    draft.target = TargetChoice::Session;
    draft.session_id = String::new();
    assert_eq!(task_draft_issue(&draft, None, NOW), Some(DraftIssue::Incomplete));
    draft.session_id = "sess-9".to_string();
    assert_eq!(task_draft_issue(&draft, None, NOW), None);
}

#[test]
fn test_task_draft_patch_writes_only_meaningful_changes() {
    let mut record = entry(ScheduleRecordKind::At, SCHED_1551);
    record.title = "A".to_string();
    record.prompt = "P".to_string();
    record.updated_at = Some("2026-09-19T10:00:00Z".to_string());
    let seeded = task_draft(&record, NOW, "Asia/Shanghai");
    assert_eq!(
        task_draft_patch(&seeded, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch::default()
    );

    let mut draft = seeded.clone();
    draft.title = "  B  ".to_string();
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            title: Some("B".to_string()),
            ..SchedulePatch::default()
        }
    );
    draft.title = " A ".to_string();
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch::default()
    );

    let mut draft = seeded.clone();
    draft.prompt = "New prompt".to_string();
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            prompt: Some("New prompt".to_string()),
            ..SchedulePatch::default()
        }
    );

    let mut draft = seeded.clone();
    draft.timing.start = "2026-09-20T23:51:00".to_string();
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            timing: Some(ScheduleTiming::Rrule {
                rrule: "DTSTART;TZID=Asia/Shanghai:20260920T235100\nRRULE:FREQ=DAILY;COUNT=1"
                    .to_string(),
            }),
            ..SchedulePatch::default()
        }
    );
    let patch = task_draft_patch(&draft, &record, true, NOW, "Asia/Shanghai");
    assert_eq!(
        patch.timing,
        Some(ScheduleTiming::Rrule {
            rrule: "DTSTART;TZID=Asia/Shanghai:20260920T235100\nRRULE:FREQ=DAILY;COUNT=1"
                .to_string(),
        })
    );
}

#[test]
fn test_task_draft_patch_tracks_target_changes() {
    let mut record = entry(ScheduleRecordKind::At, SCHED_1551);
    record.title = "A".to_string();
    record.prompt = "P".to_string();
    record.target = ScheduleTarget::NewOnce {
        session_id: "sess-reserved".to_string(),
        create: ScheduleSessionCreateSpec {
            cwd: Some("/old".to_string()),
            agent_preset: Some("coder".to_string()),
        },
    };
    let seeded = task_draft(&record, NOW, "Asia/Shanghai");
    assert_eq!(seeded.cwd, "/old");
    assert_eq!(
        task_draft_patch(&seeded, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch::default()
    );
    assert_eq!(
        draft_target(&seeded, Some(&record)),
        ScheduleTargetRequest::NewOnce {
            create: Some(ScheduleSessionCreateSpec {
                cwd: Some("/old".to_string()),
                agent_preset: Some("coder".to_string()),
            }),
        }
    );

    let mut draft = seeded.clone();
    draft.cwd = "/new".to_string();
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            target: Some(ScheduleTargetRequest::NewOnce {
                create: Some(ScheduleSessionCreateSpec {
                    cwd: Some("/new".to_string()),
                    agent_preset: None,
                }),
            }),
            ..SchedulePatch::default()
        }
    );

    let mut draft = seeded.clone();
    draft.target = TargetChoice::New;
    assert_eq!(
        task_draft_patch(&draft, &record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            target: Some(ScheduleTargetRequest::New {
                create: Some(ScheduleSessionCreateSpec {
                    cwd: Some("/old".to_string()),
                    agent_preset: Some("coder".to_string()),
                }),
            }),
            ..SchedulePatch::default()
        }
    );

    let mut session_record = entry(ScheduleRecordKind::At, SCHED_1551);
    session_record.target = ScheduleTarget::Session {
        session_id: "sess-9".to_string(),
    };
    let seeded = task_draft(&session_record, NOW, "Asia/Shanghai");
    assert_eq!(seeded.target, TargetChoice::Session);
    assert_eq!(seeded.session_id, "sess-9");
    assert_eq!(seeded.cwd, "");
    let mut draft = seeded.clone();
    draft.target = TargetChoice::NewOnce;
    draft.cwd = "/new".to_string();
    assert_eq!(
        task_draft_patch(&draft, &session_record, false, NOW, "Asia/Shanghai"),
        SchedulePatch {
            target: Some(ScheduleTargetRequest::NewOnce {
                create: Some(ScheduleSessionCreateSpec {
                    cwd: Some("/new".to_string()),
                    agent_preset: None,
                }),
            }),
            ..SchedulePatch::default()
        }
    );
}

#[test]
fn test_schedule_patch_serializes_camel_case() {
    let patch = SchedulePatch {
        title: Some("Renamed".to_string()),
        timing: Some(ScheduleTiming::Every { every_seconds: 300 }),
        target: Some(ScheduleTargetRequest::Session {
            session_id: "sess-9".to_string(),
        }),
        active: Some(false),
        ..SchedulePatch::default()
    };
    let encoded = serde_json::to_value(&patch).unwrap();
    assert_eq!(encoded["title"], "Renamed");
    assert_eq!(encoded["timing"]["kind"], "every");
    assert_eq!(encoded["timing"]["every_seconds"], 300);
    assert_eq!(encoded["target"]["kind"], "session");
    assert_eq!(encoded["target"]["sessionId"], "sess-9");
    assert_eq!(encoded["active"], false);
    assert!(encoded.get("prompt").is_none());
    assert_eq!(serde_json::to_value(SchedulePatch::default()).unwrap(), serde_json::json!({}));

    let timing: ScheduleTiming =
        serde_json::from_value(serde_json::json!({ "kind": "at", "at": "2026-09-19T15:51:00Z" }))
            .unwrap();
    assert_eq!(
        timing,
        ScheduleTiming::At {
            at: ScheduleAtInput::Instant("2026-09-19T15:51:00Z".to_string()),
        }
    );
    let timing: ScheduleTiming = serde_json::from_value(serde_json::json!({
        "kind": "at",
        "at": { "date": "2026-09-19", "time": "23:51:00", "time_zone": "Asia/Shanghai" }
    }))
    .unwrap();
    assert_eq!(
        timing,
        ScheduleTiming::At {
            at: ScheduleAtInput::Local(ScheduleLocalAtInput {
                date: "2026-09-19".to_string(),
                time: "23:51:00".to_string(),
                time_zone: "Asia/Shanghai".to_string(),
            }),
        }
    );
}

#[test]
fn test_format_schedule_frequency_reads_stored_rules_in_both_locales() {
    let record = entry(ScheduleRecordKind::At, SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Once");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "仅一次");

    let record = every_record(300, SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Every 5 minutes");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "每 5 分钟");
    let record = every_record(7200, SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Every 2 hours");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "每 2 小时");
    let record = every_record(172800, SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Every 2 days");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "每 2 天");
    let record = every_record(86340, SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, None, NOW),
        "Every 1439 minutes"
    );
    let record = every_record(60, SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Every 1 minute");

    let record = daily_record("09:30:00", "Asia/Shanghai", SCHED_DAILY);
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "Daily at 09:30"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::Zh,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "每天 09:30"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "UTC" }),
            NOW
        ),
        "Daily at 09:30 (UTC+08:00 · Asia/Shanghai)"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::Zh,
            Some(FrequencyZone { system: "UTC" }),
            NOW
        ),
        "每天 09:30（UTC+08:00 · Asia/Shanghai）"
    );

    let record = weekly_record("08:00:00", &[1, 3, 5], "Asia/Shanghai", SCHED_WEEKLY);
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "Weekly on Mon, Wed, Fri at 08:00"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "Asia/Shanghai" }), NOW),
        "每周一、三、五 08:00"
    );
}

#[test]
fn test_format_schedule_frequency_reads_rrule_rules_in_both_locales() {
    let record = rrule_entry(
        "DTSTART;TZID=Asia/Shanghai:20260918T093000\nRRULE:FREQ=DAILY;INTERVAL=2;BYHOUR=9;BYMINUTE=30",
        SCHED_DAILY,
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "Every 2 days at 09:30"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::Zh,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "每 2 天 09:30"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every 2 days at 09:30 (UTC+08:00 · Asia/Shanghai)"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每 2 天 09:30（UTC+08:00 · Asia/Shanghai）"
    );

    let record = rrule_entry(
        "DTSTART;TZID=UTC:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=1;BYDAY=MO,WE,FR",
        SCHED_WEEKLY,
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Weekly on Mon, Wed, Fri at 08:00"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每周一、三、五 08:00"
    );
    let record = rrule_entry(
        "DTSTART;TZID=UTC:20260921T080000\nRRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TU",
        SCHED_WEEKLY,
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every 2 weeks on Mon, Tue at 08:00"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每 2 周的周一、二 08:00"
    );

    let record = rrule_entry(
        "DTSTART;TZID=UTC:20260919T235100\nRRULE:FREQ=MONTHLY;INTERVAL=1;BYMONTHDAY=19,-1;BYSETPOS=1",
        SCHED_1551,
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Monthly on day 19 at 23:51, or month end"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每月 19 日 23:51，不足该日则月末"
    );
    let record = rrule_entry(
        "DTSTART;TZID=UTC:20260919T235100\nRRULE:FREQ=MONTHLY;INTERVAL=2;BYMONTHDAY=19,-1;BYSETPOS=1",
        SCHED_1551,
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every 2 months on day 19 at 23:51, or month end"
    );

    let record = rrule_entry(
        "DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=MINUTELY;INTERVAL=30",
        SCHED_1551,
    );
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Every 30 minutes");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "每 30 分钟");

    let record = rrule_entry("DTSTART;TZID=UTC:20260919T155100\nRRULE:FREQ=DAILY;COUNT=5", SCHED_1551);
    assert_eq!(format_schedule_frequency(&record, Locale::En, None, NOW), "Custom");
    assert_eq!(format_schedule_frequency(&record, Locale::Zh, None, NOW), "自定义");

    let record = rrule_entry(
        "DTSTART:20260919T155100Z\nRRULE:FREQ=DAILY;INTERVAL=1;BYHOUR=15;BYMINUTE=51",
        SCHED_1551,
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "Daily at 15:51 (UTC+00:00)"
    );
}

#[test]
fn test_format_schedule_frequency_describes_cron_rules() {
    let record = cron_record("30 8 * * 1-5", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Mon–Fri at 08:30"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "周一至周五 08:30"
    );

    let record = cron_record("* * * * *", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every minute"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每分钟"
    );

    let record = cron_record("*/10 9-17 * * 1-5", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::En,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "Mon–Fri every 10 minutes during hours 09–17 (UTC+00:00)"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::Zh,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "周一至周五 09 至 17 点内每 10 分钟（UTC+00:00）"
    );

    let record = cron_record("0 12 * 6 *", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every day in June at 12:00"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每天（6月） 12:00"
    );

    let record = cron_record("0 */2 * * *", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Every 2 hours"
    );
    assert_eq!(
        format_schedule_frequency(&record, Locale::Zh, Some(FrequencyZone { system: "UTC" }), NOW),
        "每 2 小时"
    );

    let record = cron_record("not a cron", "UTC", SCHED_1551);
    assert_eq!(
        format_schedule_frequency(&record, Locale::En, Some(FrequencyZone { system: "UTC" }), NOW),
        "Custom"
    );
    assert_eq!(
        format_schedule_frequency(
            &record,
            Locale::Zh,
            Some(FrequencyZone { system: "Asia/Shanghai" }),
            NOW
        ),
        "自定义（UTC+00:00）"
    );
}

#[test]
fn test_format_schedule_absolute_and_relative_use_fixed_timestamps() {
    assert_eq!(
        format_schedule_absolute(SCHED_1551, Locale::En, &zone_utc()),
        "Sep 19, 2026, 15:51"
    );
    assert_eq!(
        format_schedule_absolute(SCHED_1551, Locale::Zh, &zone_utc()),
        "9月19日 15:51"
    );
    assert_eq!(
        format_schedule_absolute(SCHED_1551, Locale::En, &zone_shanghai()),
        "Sep 19, 2026, 23:51"
    );
    assert_eq!(
        format_schedule_absolute("2027-03-05T08:07:00Z", Locale::En, &zone_utc()),
        "Mar 5, 2027, 08:07"
    );
    assert_eq!(
        format_schedule_absolute("2027-03-05T08:07:00Z", Locale::Zh, &zone_utc()),
        "3月5日 08:07"
    );
    assert_eq!(format_schedule_absolute("garbage", Locale::En, &zone_utc()), "garbage");

    assert_eq!(format_schedule_relative(SCHED_1551, SCHED_1551_MS, Locale::En), "Due now");
    assert_eq!(format_schedule_relative(SCHED_1551, SCHED_1551_MS, Locale::Zh), "现在到期");
    assert_eq!(
        format_schedule_relative(SCHED_1551, SCHED_1551_MS - 60_000, Locale::En),
        "in 1 minute"
    );
    assert_eq!(
        format_schedule_relative(SCHED_1551, SCHED_1551_MS - 60_000, Locale::Zh),
        "1分钟后"
    );
    assert_eq!(
        format_schedule_relative(SCHED_1551, SCHED_1551_MS - 90_000, Locale::En),
        "in 2 minutes"
    );
    assert_eq!(
        format_schedule_relative(SCHED_1551, SCHED_1551_MS + 60_000, Locale::En),
        "1 minute overdue"
    );
    assert_eq!(
        format_schedule_relative(SCHED_1551, SCHED_1551_MS + 60_000, Locale::Zh),
        "已逾期 1分钟"
    );
    assert_eq!(
        format_schedule_relative("2026-09-21T15:51:00Z", SCHED_1551_MS, Locale::En),
        "in 2 days"
    );
    assert_eq!(
        format_schedule_relative("2026-09-19T17:21:00Z", SCHED_1551_MS, Locale::Zh),
        "2小时后"
    );
    assert_eq!(
        format_schedule_relative("2026-09-19T15:51:30Z", SCHED_1551_MS, Locale::En),
        "in 30 seconds"
    );

    let parts = next_run_parts(SCHED_1551, Locale::En, SCHED_1551_MS - 60_000, &zone_utc());
    assert_eq!(parts.absolute, "Sep 19, 2026, 15:51");
    assert_eq!(parts.relative, "(in 1 minute)");
    let parts = next_run_parts(SCHED_1551, Locale::Zh, SCHED_1551_MS, &zone_shanghai());
    assert_eq!(parts.absolute, "9月19日 23:51");
    assert_eq!(parts.relative, "(现在到期)");
}

#[test]
fn test_locale_from_tag_resolves_lang_env_values() {
    assert_eq!(Locale::from_tag("zh_CN.UTF-8"), Locale::Zh);
    assert_eq!(Locale::from_tag("zh-CN"), Locale::Zh);
    assert_eq!(Locale::from_tag("ZH"), Locale::Zh);
    assert_eq!(Locale::from_tag("en_US.UTF-8"), Locale::En);
    assert_eq!(Locale::from_tag("C"), Locale::En);
    assert_eq!(Locale::from_tag(""), Locale::En);
}

#[test]
fn test_scheduled_session_ids_collects_active_targets_and_involved_sessions() {
    let mut session = entry(ScheduleRecordKind::At, SCHED_1551);
    session.target = ScheduleTarget::Session {
        session_id: "sess-a".to_string(),
    };
    session.involved_session_ids = vec!["sess-a".to_string(), "sess-b".to_string()];

    let mut new_once = entry(ScheduleRecordKind::At, SCHED_1551);
    new_once.target = ScheduleTarget::NewOnce {
        session_id: "sess-c".to_string(),
        create: ScheduleSessionCreateSpec::default(),
    };
    new_once.involved_session_ids.clear();

    let mut new = entry(ScheduleRecordKind::Every, SCHED_1551);
    new.target = ScheduleTarget::New {
        create: ScheduleSessionCreateSpec::default(),
    };
    new.involved_session_ids = vec!["sess-d".to_string()];

    let mut paused = entry(ScheduleRecordKind::At, SCHED_1551);
    paused.status = ScheduleStatus::Paused;
    paused.target = ScheduleTarget::Session {
        session_id: "sess-e".to_string(),
    };
    paused.involved_session_ids = vec!["sess-f".to_string()];

    let sessions = scheduled_session_ids(&[session, new_once, new, paused]);
    let expected: HashSet<String> = ["sess-a", "sess-b", "sess-c", "sess-d"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(sessions, expected);
}
}
