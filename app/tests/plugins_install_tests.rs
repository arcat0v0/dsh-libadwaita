use dsh_libadwaita::protocol::*;
use dsh_libadwaita::ui::plugins_page::model::*;
use std::collections::BTreeMap;

#[test]
fn test_spec_input_classification() {
    assert_eq!(classify_spec_input("dsh-plugin-whale-pet"), SpecKind::Npm);
    assert_eq!(
        classify_spec_input("@deepseek-ai/dsh-base"),
        SpecKind::Npm
    );
    assert_eq!(
        classify_spec_input("dsh-plugin-whale-pet@^1.2.0"),
        SpecKind::Npm
    );
    assert_eq!(
        classify_spec_input("git+https://github.com/example/plugin.git"),
        SpecKind::Git
    );
    assert_eq!(
        classify_spec_input("github:example/plugin"),
        SpecKind::Git
    );
    assert_eq!(
        classify_spec_input("https://registry.example.com/plugin.tgz"),
        SpecKind::Url
    );
    assert_eq!(classify_spec_input("/home/user/my-plugin"), SpecKind::PathOrOther);
    assert_eq!(classify_spec_input("./relative/path"), SpecKind::PathOrOther);

    assert_eq!(
        sanitize_install_input("dsh-plugin-whale-pet"),
        "dsh-plugin-whale-pet"
    );
    assert_eq!(
        sanitize_install_input("git+https://token@github.com/secret/repo.git"),
        "[git]"
    );
    assert_eq!(
        sanitize_install_input("https://private-user:secret@example.com/pack.tgz"),
        "[url]"
    );
    assert_eq!(
        sanitize_install_input("/Users/alice/private/code"),
        "[path-or-other]"
    );
}

#[test]
fn test_input_problem_sentence_mapping() {
    let reg1 = Some("https://registry.npmjs.org/".to_string());
    let reg2 = Some("https://registry.npmmirror.com/".to_string());

    let invalid = input_problem_sentence("invalid-spec", "bad syntax", &[]);
    assert!(invalid.contains("无法识别这个包名或地址：bad syntax"));

    let installed = input_problem_sentence("already-installed", "", &[]);
    assert_eq!(installed, "该插件已安装。如需升级，请卸载后重新安装");

    let shipped = input_problem_sentence("shipped", "", &[]);
    assert_eq!(shipped, "该插件随 DSH 提供，升级 DSH 即可获得新版本");

    let not_found = input_problem_sentence("not-found", "", &[]);
    assert_eq!(not_found, "未找到相关插件");

    let not_package = input_problem_sentence("not-a-package", "", &[]);
    assert_eq!(not_package, "该路径不存在或不是有效的插件包");

    let not_bundle = input_problem_sentence("not-a-bundle", "no manifest", &[]);
    assert!(not_bundle.contains("这个包没有声明组合包，无法作为插件安装：no manifest"));

    let single_net = input_problem_sentence("network", "timeout", &[reg1.clone()]);
    assert_eq!(single_net, "无法连接插件源，请检查网络后重试");

    let multi_net = input_problem_sentence("network", "timeout", &[reg1, reg2]);
    assert!(multi_net.contains("所有安装源都无法连接"));
    assert!(multi_net.contains("https://registry.npmjs.org/"));
    assert!(multi_net.contains("https://registry.npmmirror.com/"));
}

#[test]
fn test_registry_choice_reconciliation_and_custom_url_validation() {
    assert_eq!(
        validate_custom_registry_url("   https://registry.npmmirror.com   ").unwrap(),
        "https://registry.npmmirror.com/"
    );
    assert_eq!(
        validate_custom_registry_url("http://localhost:4873/").unwrap(),
        "http://localhost:4873/"
    );
    assert!(validate_custom_registry_url("ftp://registry.example.com").is_err());
    assert!(validate_custom_registry_url("not-a-url").is_err());
    assert!(validate_custom_registry_url("https://").is_err());

    let registries = PluginRegistries {
        registry: Some("https://registry.npmjs.org/".to_string()),
        fallback_registries: vec!["https://registry.npmmirror.com/".to_string()],
        resolved: Some("https://registry.npmjs.org/".to_string()),
    };

    let choice = reconcile_registry_choice(
        Some("https://registry.npmmirror.com/"),
        &registries,
        None,
    );
    assert_eq!(
        choice,
        RegistryChoice::Offered("https://registry.npmmirror.com/".to_string())
    );

    let choice_custom = reconcile_registry_choice(
        Some("https://my-internal-registry.corp/"),
        &registries,
        None,
    );
    assert_eq!(
        choice_custom,
        RegistryChoice::Custom("https://my-internal-registry.corp/".to_string())
    );

    let choice_probe = reconcile_registry_choice(
        None,
        &registries,
        Some("https://registry.npmmirror.com/"),
    );
    assert_eq!(
        choice_probe,
        RegistryChoice::Offered("https://registry.npmmirror.com/".to_string())
    );

    let registries_no_primary = PluginRegistries {
        registry: None,
        fallback_registries: vec![],
        resolved: Some("https://registry.npmjs.org/".to_string()),
    };
    let choice_default = reconcile_registry_choice(None, &registries_no_primary, None);
    assert_eq!(choice_default, RegistryChoice::Default);
}

#[test]
fn test_install_state_machine_reducer() {
    let mut state = InstallStateModel::default();
    assert_eq!(state.phase, InstallPhase::Idle);

    reduce_install(
        &mut state,
        InstallAction::StartCheck {
            spec: "dsh-plugin-whale-pet".to_string(),
            choice: RegistryChoice::Default,
        },
    );
    assert_eq!(state.phase, InstallPhase::Checking);
    assert_eq!(state.spec, "dsh-plugin-whale-pet");

    reduce_install(
        &mut state,
        InstallAction::InspectRefused {
            problem: "not-found".to_string(),
            reason: "package not found".to_string(),
            registries: vec![],
        },
    );
    assert_eq!(state.phase, InstallPhase::Idle);
    assert_eq!(state.input_error.as_deref(), Some("未找到相关插件"));

    reduce_install(
        &mut state,
        InstallAction::StartCheck {
            spec: "dsh-plugin-whale-pet".to_string(),
            choice: RegistryChoice::Default,
        },
    );
    let subject = InstallSubjectInfo {
        spec: "dsh-plugin-whale-pet".to_string(),
        kind: "registry".to_string(),
        name: Some("dsh-plugin-whale-pet".to_string()),
        version: Some("1.0.0".to_string()),
        description: Some("Whale pet".to_string()),
        bundle: Some(true),
        registry: Some("https://registry.npmjs.org/".to_string()),
        host: None,
        bundled: false,
    };
    reduce_install(
        &mut state,
        InstallAction::InspectAccepted {
            subject: subject.clone(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Confirmed);

    reduce_install(
        &mut state,
        InstallAction::StartInstall {
            request_id: "req-123".to_string(),
            approved_builds: vec![],
        },
    );
    assert_eq!(state.phase, InstallPhase::Running);
    assert_eq!(state.request_id.as_deref(), Some("req-123"));

    reduce_install(
        &mut state,
        InstallAction::AppendLog {
            job_id: "job-1".to_string(),
            argv: vec!["pnpm".to_string(), "add".to_string()],
            cwd: "/home/user".to_string(),
            text: "Installing package...\n".to_string(),
            exit_code: None,
        },
    );
    assert_eq!(state.runs.len(), 1);
    assert_eq!(state.runs[0].lines, vec!["Installing package..."]);

    reduce_install(&mut state, InstallAction::RequestCancel);
    assert_eq!(state.phase, InstallPhase::Cancelling);

    reduce_install(
        &mut state,
        InstallAction::CancelResult {
            status: "too-late".to_string(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Applying);

    let change = PluginChangeResult {
        changed: true,
        application: "applied".to_string(),
        restart_reason: None,
        stage: "install".to_string(),
        target: "dsh-plugin-whale-pet".to_string(),
        enabled: Some(true),
        error: None,
        package_result: None,
        bundle: Some("dsh-plugin-whale-pet".to_string()),
        version: Some("1.0.0".to_string()),
        pending_builds: None,
        approved_builds: None,
        registries: None,
        failed_at: None,
    };
    reduce_install(
        &mut state,
        InstallAction::InstallResult {
            result: change.clone(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Done);
    assert_eq!(
        state.installed_name.as_deref(),
        Some("dsh-plugin-whale-pet")
    );
    assert_eq!(state.installed_version.as_deref(), Some("1.0.0"));
    assert!(!state.restart_required);

    let mut state_bundled = InstallStateModel::default();
    let bundled_subject = InstallSubjectInfo {
        bundled: true,
        ..subject
    };
    reduce_install(
        &mut state_bundled,
        InstallAction::InspectBundled {
            subject: bundled_subject,
        },
    );
    assert_eq!(state_bundled.phase, InstallPhase::Done);
}

#[test]
fn test_pending_builds_extraction_and_retry() {
    let change_with_builds = PluginChangeResult {
        changed: false,
        application: "failed".to_string(),
        restart_reason: None,
        stage: "install".to_string(),
        target: "custom-plugin".to_string(),
        enabled: Some(true),
        error: Some(PluginChangeError {
            code: "build-script".to_string(),
            diagnostic: None,
            incompatible: None,
        }),
        package_result: Some(PluginPackageResult {
            exit_code: 1,
            output: "Build script blocked".to_string(),
            truncated: false,
            log_path: "/tmp/pnpm.log".to_string(),
            kind: Some("build-blocked".to_string()),
        }),
        bundle: None,
        version: None,
        pending_builds: Some(vec![
            "esbuild".to_string(),
            "canvas".to_string(),
            "esbuild".to_string(),
        ]),
        approved_builds: None,
        registries: None,
        failed_at: None,
    };

    let builds = extract_pending_builds(&change_with_builds);
    assert_eq!(builds, vec!["esbuild".to_string(), "canvas".to_string()]);

    let mut state = InstallStateModel::default();
    reduce_install(
        &mut state,
        InstallAction::InstallResult {
            result: change_with_builds,
        },
    );
    assert_eq!(state.phase, InstallPhase::Failed);
    assert_eq!(
        state
            .failure
            .as_ref()
            .unwrap()
            .pending_builds,
        vec!["esbuild".to_string(), "canvas".to_string()]
    );

    reduce_install(
        &mut state,
        InstallAction::ApproveBuildsAndRetry {
            request_id: "req-retry-1".to_string(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Running);
    assert_eq!(
        state.approved_builds,
        vec!["esbuild".to_string(), "canvas".to_string()]
    );
}

#[test]
fn test_classify_install_failure() {
    let failure_change = PluginChangeResult {
        changed: false,
        application: "failed".to_string(),
        restart_reason: None,
        stage: "install".to_string(),
        target: "plugin-x".to_string(),
        enabled: Some(true),
        error: None,
        package_result: Some(PluginPackageResult {
            exit_code: 1,
            output: "timed out".to_string(),
            truncated: false,
            log_path: "/tmp/log".to_string(),
            kind: Some("timeout".to_string()),
        }),
        bundle: None,
        version: None,
        pending_builds: None,
        approved_builds: None,
        registries: None,
        failed_at: None,
    };
    let info = classify_install_failure(&failure_change);
    assert_eq!(info.kind.as_deref(), Some("timeout"));
    assert_eq!(info.reason, "安装超时，请检查网络后重试");

    let network_host_change = PluginChangeResult {
        failed_at: Some("spec-host".to_string()),
        package_result: Some(PluginPackageResult {
            exit_code: 1,
            output: "".to_string(),
            truncated: false,
            log_path: "".to_string(),
            kind: Some("network".to_string()),
        }),
        ..failure_change
    };
    let host_info = classify_install_failure(&network_host_change);
    assert!(host_info.reason.contains("无法连接目标主机"));
}

#[test]
fn test_version_summary_state_and_filter() {
    let versions = vec![
        PluginVersionItem {
            version: "1.2.0".to_string(),
            published_at: Some("2026-06-01".to_string()),
            tags: vec!["latest".to_string()],
            prerelease: false,
            deprecated: None,
            bundle: true,
            incompatible: None,
        },
        PluginVersionItem {
            version: "1.3.0-rc.1".to_string(),
            published_at: None,
            tags: vec![],
            prerelease: true,
            deprecated: None,
            bundle: true,
            incompatible: None,
        },
        PluginVersionItem {
            version: "2.0.0".to_string(),
            published_at: None,
            tags: vec![],
            prerelease: false,
            deprecated: None,
            bundle: false,
            incompatible: None,
        },
    ];

    assert!(installable_version(&versions[0]));
    assert!(installable_version(&versions[1]));
    assert!(!installable_version(&versions[2]));

    let candidate = update_candidate(&versions);
    assert_eq!(candidate.unwrap().version, "1.2.0");

    let stable_only = filter_version_items(&versions, false);
    assert_eq!(stable_only.len(), 2);
    let all = filter_version_items(&versions, true);
    assert_eq!(all.len(), 3);

    let listed = PluginVersionList::Listed {
        installed: "1.1.0".to_string(),
        registry: Some("https://registry.npmjs.org/".to_string()),
        attempts: vec![],
        versions: versions.clone(),
        omitted: 0,
    };
    let summary = evaluate_version_summary(Some(&listed), false, None);
    match summary {
        VersionSummaryKind::Available { version, .. } => assert_eq!(version, "1.2.0"),
        other => panic!("expected available: {:?}", other),
    }

    let empty_listed = PluginVersionList::Listed {
        installed: "1.2.0".to_string(),
        registry: None,
        attempts: vec![],
        versions: vec![],
        omitted: 0,
    };
    let latest_summary = evaluate_version_summary(Some(&empty_listed), false, Some("14:30"));
    match latest_summary {
        VersionSummaryKind::Latest { checked_at } => assert_eq!(checked_at.as_deref(), Some("14:30")),
        other => panic!("expected latest: {:?}", other),
    }

    let mut requirements = BTreeMap::new();
    requirements.insert("@deepseek-ai/dsh".to_string(), ">=2.0.0".to_string());
    let incompatible_versions = vec![PluginVersionItem {
        version: "2.0.0".to_string(),
        published_at: None,
        tags: vec![],
        prerelease: false,
        deprecated: None,
        bundle: true,
        incompatible: Some(IncompatiblePlugin {
            name: "pkg".to_string(),
            version: "2.0.0".to_string(),
            runtime_version: "1.6.0".to_string(),
            requirements,
        }),
    }];
    let blocked_list = PluginVersionList::Listed {
        installed: "1.0.0".to_string(),
        registry: None,
        attempts: vec![],
        versions: incompatible_versions,
        omitted: 0,
    };
    let blocked_summary = evaluate_version_summary(Some(&blocked_list), false, None);
    match blocked_summary {
        VersionSummaryKind::BlockedDsh { version, runtime, .. } => {
            assert_eq!(version, "2.0.0");
            assert_eq!(runtime, "1.6.0");
        }
        other => panic!("expected blocked: {:?}", other),
    }
}

#[test]
fn test_update_step_ordering_and_outcome_mapping() {
    let steps = ordered_update_steps(
        &["checking".to_string(), "removing".to_string()],
        Some("install"),
        true,
    );
    assert_eq!(
        steps,
        vec![
            "checking".to_string(),
            "removing".to_string(),
            "installing".to_string(),
            "restoring".to_string(),
        ]
    );

    let success_update = PluginUpdateResult {
        from: "1.0.0".to_string(),
        to: "1.1.0".to_string(),
        step: "install".to_string(),
        result: PluginChangeResult {
            changed: true,
            application: "applied".to_string(),
            restart_reason: None,
            stage: "install".to_string(),
            target: "pkg".to_string(),
            enabled: Some(true),
            error: None,
            package_result: None,
            bundle: Some("pkg".to_string()),
            version: Some("1.1.0".to_string()),
            pending_builds: None,
            approved_builds: None,
            registries: None,
            failed_at: None,
        },
        restore: None,
    };
    match evaluate_update_outcome(Some(&success_update), None) {
        UpdateOutcomeKind::Done { version } => assert_eq!(version, "1.1.0"),
        other => panic!("expected done: {:?}", other),
    }

    let kept_update = PluginUpdateResult {
        from: "1.0.0".to_string(),
        to: "1.1.0".to_string(),
        step: "install".to_string(),
        result: PluginChangeResult {
            changed: false,
            application: "failed".to_string(),
            restart_reason: None,
            stage: "install".to_string(),
            target: "pkg".to_string(),
            enabled: Some(true),
            error: Some(PluginChangeError {
                code: "download-failed".to_string(),
                diagnostic: Some("network error".to_string()),
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
    match evaluate_update_outcome(Some(&kept_update), None) {
        UpdateOutcomeKind::Kept { from, to, reason } => {
            assert_eq!(from, "1.0.0");
            assert_eq!(to, "1.1.0");
            assert_eq!(reason, "network error");
        }
        other => panic!("expected kept: {:?}", other),
    }
}

#[test]
fn test_terminal_folding() {
    let lines: Vec<String> = (1..=8).map(|i| format!("line {i}")).collect();
    let (head, folded, tail) = fold_terminal_lines(&lines, 12);
    assert_eq!(head.len(), 8);
    assert_eq!(folded, 0);
    assert!(tail.is_empty());

    let long_lines: Vec<String> = (1..=20).map(|i| format!("line {i}")).collect();
    let (head, folded, tail) = fold_terminal_lines(&long_lines, 12);
    assert_eq!(head.len(), 6);
    assert_eq!(head[0], "line 1");
    assert_eq!(head[5], "line 6");
    assert_eq!(folded, 8);
    assert_eq!(tail.len(), 6);
    assert_eq!(tail[0], "line 15");
    assert_eq!(tail[5], "line 20");
}

#[test]
fn test_install_result_lost_reconciles_through_wait() {
    let mut state = InstallStateModel::default();
    state.phase = InstallPhase::Confirmed;
    reduce_install(
        &mut state,
        InstallAction::StartInstall {
            request_id: "req-lost".to_string(),
            approved_builds: Vec::new(),
        },
    );
    reduce_install(
        &mut state,
        InstallAction::LostResult {
            reason: "与 DSH 的连接中断".to_string(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Unconfirmed);
    assert_eq!(state.request_id.as_deref(), Some("req-lost"));
    assert_eq!(
        state.failure.as_ref().and_then(|failure| failure.uncertainty.as_deref()),
        Some("与 DSH 的连接中断")
    );

    reduce_install(
        &mut state,
        InstallAction::WaitInstallResult { result: None },
    );
    assert_eq!(state.phase, InstallPhase::Unconfirmed);

    let applied = PluginChangeResult {
        changed: true,
        application: "applied".to_string(),
        restart_reason: None,
        stage: "install".to_string(),
        target: "pkg".to_string(),
        enabled: Some(true),
        error: None,
        package_result: None,
        bundle: Some("pkg".to_string()),
        version: Some("1.0.0".to_string()),
        pending_builds: None,
        approved_builds: None,
        registries: None,
        failed_at: None,
    };
    reduce_install(
        &mut state,
        InstallAction::WaitInstallResult {
            result: Some(applied),
        },
    );
    assert_eq!(state.phase, InstallPhase::Done);
    assert_eq!(state.installed_name.as_deref(), Some("pkg"));
    assert_eq!(state.installed_version.as_deref(), Some("1.0.0"));

    reduce_install(
        &mut state,
        InstallAction::LostResult {
            reason: "late error".to_string(),
        },
    );
    assert_eq!(state.phase, InstallPhase::Done);
}
