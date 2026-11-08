use dsh_libadwaita::protocol::{MarketIndexMetadata, MarketSearchResult, MarketSort, MarketSortDirection};
use dsh_libadwaita::ui::plugins_page::market::*;

fn index_metadata() -> MarketIndexMetadata {
    serde_json::from_value(serde_json::json!({
        "categories": {
            "tools": {"title": {"en": "Tools", "zh": "工具"}},
            "themes": {"title": {"en": "Themes", "zh": "主题"}}
        },
        "labels": {
            "official": {"title": {"en": "Official", "zh": "官方"}},
            "community": {"title": {"en": "Community", "zh": "社区"}}
        },
        "sortKeys": {
            "stars": {"title": {"en": "Stars", "zh": "星标"}, "order": "desc"},
            "downloads": {"title": {"en": "Downloads", "zh": "下载量"}, "order": "asc"}
        }
    }))
    .unwrap()
}

fn listing_entry(json: serde_json::Value) -> dsh_libadwaita::protocol::MarketListingEntry {
    serde_json::from_value(json).unwrap()
}

fn entry_json() -> serde_json::Value {
    serde_json::json!({
        "name": "@community/whale-pet",
        "title": {"en": "Whale Pet", "zh": "鲸鱼宠物"},
        "description": {"en": "A whale pet.", "zh": "一只鲸鱼宠物。"},
        "author": "arcat",
        "metadata": {"keywords": [], "labels": ["community"]},
        "versions": [
            {
                "version": "1.1.0",
                "publishTimestamp": 1762419200,
                "source": {"type": "npm", "registry": "https://registry.npmmirror.com/"},
                "engines": {"dsh": "^0.4"},
                "install": {"spec": "@community/whale-pet@1.1.0"},
                "compatibility": {"compatible": true}
            },
            {
                "version": "1.0.0",
                "publishTimestamp": 1700000000,
                "source": {"type": "git", "url": "https://github.com/community/whale-pet", "commit": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2", "path": "packages/pet"},
                "engines": {},
                "install": {"spec": "git+https://github.com/community/whale-pet#a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"},
                "compatibility": {"compatible": false, "runtimeVersion": "0.4.0", "requirements": {"dsh": "^0.4"}, "exempted": true}
            }
        ],
        "defaultVersion": "1.1.0",
        "installation": {"status": "not-installed"}
    })
}

fn page_json(entries: &[serde_json::Value], fetched_at: u64, next_offset: Option<u64>) -> MarketSearchResult {
    serde_json::from_value(serde_json::json!({
        "url": "https://market.example/index.json",
        "title": {"en": "Example", "zh": "示例市场"},
        "metadata": {"categories": {}, "labels": {}, "sortKeys": {}},
        "fetchedAt": fetched_at,
        "entries": entries,
        "total": entries.len(),
        "nextOffset": next_offset
    }))
    .unwrap()
}

fn entry_named(name: &str) -> serde_json::Value {
    let mut entry = entry_json();
    entry["name"] = serde_json::Value::String(name.to_string());
    entry
}

#[test]
fn initial_query_starts_in_index_order() {
    let query = initial_query();
    assert_eq!(query.text, "");
    assert_eq!(query.category, None);
    assert!(query.labels.is_empty());
    assert_eq!(query.sort, MarketSort::Index);
}

#[test]
fn with_text_switches_index_and_relevance() {
    let query = initial_query();
    let typed = with_text(&query, "whale");
    assert_eq!(typed.sort, MarketSort::Relevance);
    let cleared = with_text(&typed, "  ");
    assert_eq!(cleared.sort, MarketSort::Index);

    let keyed = DiscoverQuery {
        text: String::new(),
        category: None,
        labels: Vec::new(),
        sort: MarketSort::Key {
            id: "stars".to_string(),
            direction: MarketSortDirection::Desc,
        },
    };
    let typed_keyed = with_text(&keyed, "whale");
    assert_eq!(
        typed_keyed.sort,
        MarketSort::Key {
            id: "stars".to_string(),
            direction: MarketSortDirection::Desc
        }
    );
    let typed_relevance = with_text(&typed, "pet");
    assert_eq!(typed_relevance.sort, MarketSort::Relevance);
}

#[test]
fn declared_query_prunes_undeclared_ids() {
    let metadata = index_metadata();
    let query = DiscoverQuery {
        text: String::new(),
        category: Some("tools".to_string()),
        labels: vec!["official".to_string()],
        sort: MarketSort::Key {
            id: "stars".to_string(),
            direction: MarketSortDirection::Desc,
        },
    };
    assert_eq!(declared_query(&query, &metadata), query);

    let stale = DiscoverQuery {
        text: String::new(),
        category: Some("removed".to_string()),
        labels: vec!["official".to_string(), "ghost".to_string()],
        sort: MarketSort::Key {
            id: "vanished".to_string(),
            direction: MarketSortDirection::Asc,
        },
    };
    let pruned = declared_query(&stale, &metadata);
    assert_eq!(pruned.category, None);
    assert_eq!(pruned.labels, vec!["official".to_string()]);
    assert_eq!(pruned.sort, MarketSort::Index);
}

#[test]
fn presented_version_prefers_the_default() {
    let entry = listing_entry(entry_json());
    let version = presented_version(&entry).unwrap();
    assert_eq!(version.version, "1.1.0");

    let mut no_default = entry.clone();
    no_default.default_version = None;
    assert_eq!(presented_version(&no_default).unwrap().version, "1.1.0");
}

#[test]
fn entry_actions_follow_installation_status() {
    let mut entry = listing_entry(entry_json());
    assert_eq!(entry_action(&entry), MarketEntryAction::Install);
    assert!(entry_install_blocked_reason(&entry).is_none());

    entry.installation.status = "update-available".to_string();
    assert_eq!(
        entry_action(&entry),
        MarketEntryAction::Status(MarketInstallStatus::UpdateAvailable)
    );
    entry.installation.status = "provided".to_string();
    assert_eq!(
        entry_action(&entry),
        MarketEntryAction::Status(MarketInstallStatus::Provided)
    );
    entry.installation.status = "different-version".to_string();
    assert_eq!(
        entry_action(&entry),
        MarketEntryAction::Status(MarketInstallStatus::Installed)
    );

    entry.installation.status = "not-installed".to_string();
    entry.default_version = Some("1.0.0".to_string());
    entry.versions[1].compatibility.exempted = false;
    assert!(entry_install_blocked_reason(&entry).is_some());
    assert_eq!(entry_action(&entry), MarketEntryAction::Install);
}

#[test]
fn version_tags_describe_each_version() {
    let entry = listing_entry(entry_json());
    let default_version = presented_version(&entry).unwrap();
    let tags = version_tags(&entry, default_version);
    assert_eq!(tags, vec![MarketVersionTag::Recommended]);
    assert!(version_install_offered(&entry, default_version));

    let older = &entry.versions[1];
    let tags = version_tags(&entry, older);
    assert!(tags.contains(&MarketVersionTag::Prerelease) == false);
    assert!(tags.contains(&MarketVersionTag::Exempted));
    assert!(version_install_offered(&entry, older));

    let mut installed = entry.clone();
    installed.installation.installed_version = Some("1.1.0".to_string());
    let tags = version_tags(&installed, presented_version(&installed).unwrap());
    assert!(tags.contains(&MarketVersionTag::Installed));
    assert!(!version_install_offered(&installed, presented_version(&installed).unwrap()));

    let mut provided = entry.clone();
    provided.installation.status = "provided".to_string();
    assert!(!version_install_offered(&provided, presented_version(&provided).unwrap()));
}

#[test]
fn version_tags_flag_prerelease_and_incompatible() {
    let mut entry = listing_entry(entry_json());
    entry.versions[0].version = "2.0.0-beta.1".to_string();
    entry.versions[0].compatibility.compatible = false;
    entry.versions[0].compatibility.exempted = false;
    entry.default_version = Some("2.0.0-beta.1".to_string());
    let tags = version_tags(&entry, presented_version(&entry).unwrap());
    assert!(tags.contains(&MarketVersionTag::Prerelease));
    assert!(tags.contains(&MarketVersionTag::Incompatible));
    assert!(!tags.contains(&MarketVersionTag::Exempted));
}

#[test]
fn source_facts_describe_install_origin() {
    let entry = listing_entry(entry_json());
    let npm = presented_version(&entry).unwrap();
    let facts = source_facts(&entry.name, npm);
    assert_eq!(facts[0].label(), "软件包");
    assert_eq!(facts[0].value(), "@community/whale-pet@1.1.0");
    assert_eq!(facts[1].label(), "注册表");
    assert_eq!(facts[1].value(), "https://registry.npmmirror.com/");

    let git = &entry.versions[1];
    let facts = source_facts(&entry.name, git);
    assert_eq!(facts[0].label(), "仓库");
    assert_eq!(facts[0].value(), "https://github.com/community/whale-pet");
    assert_eq!(facts[1].label(), "提交");
    assert_eq!(facts[2].label(), "目录");
    assert_eq!(facts[2].value(), "packages/pet");
}

#[test]
fn index_label_prefers_title_then_host() {
    let titled: dsh_libadwaita::protocol::MarketIndexSummary = serde_json::from_value(serde_json::json!({
        "url": "https://market.example/index.json",
        "title": {"en": "Example", "zh": "示例市场"}
    }))
    .unwrap();
    assert_eq!(index_label(&titled), "示例市场");

    let bare: dsh_libadwaita::protocol::MarketIndexSummary =
        serde_json::from_value(serde_json::json!({"url": "https://market.example/index.json"})).unwrap();
    assert_eq!(index_label(&bare), "market.example");

    let odd: dsh_libadwaita::protocol::MarketIndexSummary =
        serde_json::from_value(serde_json::json!({"url": "not a url"})).unwrap();
    assert_eq!(index_label(&odd), "not a url");
}

#[test]
fn publish_dates_format_as_iso_days() {
    assert_eq!(format_publish_date(0), "1970-01-01");
    assert_eq!(format_publish_date(951782400), "2000-02-29");
    assert_eq!(format_publish_date(1762419200), "2025-11-06");
}

#[test]
fn entry_meta_omits_missing_author() {
    let entry = listing_entry(entry_json());
    let version = presented_version(&entry).unwrap();
    assert_eq!(entry_meta(&entry, version), "1.1.0 · arcat · 2025-11-06");

    let mut anonymous = entry.clone();
    anonymous.author = None;
    assert_eq!(
        entry_meta(&anonymous, version),
        "1.1.0 · 2025-11-06"
    );
}

#[test]
fn sort_options_follow_text_and_declarations() {
    let metadata = index_metadata();
    let mut query = initial_query();
    let options = sort_options(&query, &metadata);
    assert_eq!(options[0].id, "index");
    assert_eq!(options[0].label, "默认顺序");
    assert_eq!(options[1].id, "published");
    assert_eq!(options[1].sort, MarketSort::Published { direction: MarketSortDirection::Desc });
    assert!(options[1].directed);
    assert_eq!(options[2].label, "下载量");
    assert_eq!(options[2].sort, MarketSort::Key { id: "downloads".to_string(), direction: MarketSortDirection::Asc });
    assert_eq!(options[3].label, "星标");
    assert_eq!(sort_label(&query, &metadata), "默认顺序");

    query.text = "whale".to_string();
    let options = sort_options(&query, &metadata);
    assert_eq!(options[0].id, "relevance");
    assert_eq!(options[0].label, "最佳匹配");
    query.sort = MarketSort::Relevance;
    assert_eq!(sort_label(&query, &metadata), "最佳匹配");

    query.sort = MarketSort::Key { id: "stars".to_string(), direction: MarketSortDirection::Asc };
    assert_eq!(sort_label(&query, &metadata), "星标");
    assert_eq!(
        flipped_direction(&query.sort),
        Some(MarketSort::Key { id: "stars".to_string(), direction: MarketSortDirection::Desc })
    );
    assert_eq!(flipped_direction(&MarketSort::Index), None);
}

#[test]
fn listing_failures_classify_from_messages() {
    assert_eq!(listing_failure("Could not fetch https://a: net down"), ListingFailure::Fetch);
    assert_eq!(listing_failure("Request timed out"), ListingFailure::Fetch);
    assert_eq!(listing_failure("Not a configured index source: https://a"), ListingFailure::Fetch);
    assert_eq!(
        listing_failure("https://a/index.json: plugins.0.version must be an exact SemVer version"),
        ListingFailure::Invalid
    );
    assert_eq!(listing_failure_text(ListingFailure::Fetch), "无法读取此索引");
    assert_eq!(listing_failure_text(ListingFailure::Invalid), "此索引的格式无效");
    assert_eq!(listing_failure_text(ListingFailure::Changed), "此索引在加载过程中发生了变化");
}

#[test]
fn source_failures_classify_from_messages() {
    assert_eq!(source_failure("Index source listed twice: https://a"), SourceFailure::Duplicate);
    assert_eq!(
        source_failure("Index sources require the settings service and a profile entry"),
        SourceFailure::Unavailable
    );
    assert_eq!(source_failure("Index URLs use http or https: ftp://a"), SourceFailure::Invalid);
    assert_eq!(source_failure("storage blew up"), SourceFailure::Other);
    assert_eq!(source_failure_text(SourceFailure::Duplicate), "已添加过这个索引源");
    assert_eq!(source_failure_text(SourceFailure::Invalid), "请使用 http 或 https 地址，且不要包含用户名、密码或 # 片段");
    assert_eq!(source_failure_text(SourceFailure::Unavailable), "当前环境无法保存索引源");
    assert_eq!(source_failure_text(SourceFailure::Other), "无法保存索引源");
}

#[test]
fn detail_failures_classify_from_messages() {
    assert_eq!(
        detail_failure_text("README.md exceeds 262144 bytes"),
        "详细介绍或它所在的仓库超过大小限制，未加载"
    );
    assert_eq!(detail_failure_text("Could not fetch https://a: refused"), "无法连接到详细介绍的来源");
    assert_eq!(
        detail_failure_text("git clone failed: remote denied access"),
        "无法访问详细介绍所在的仓库，请检查 Git 凭据"
    );
    assert_eq!(detail_failure_text("git could not run"), "详细介绍所在的仓库需要用 Git 读取，但无法运行 Git。请安装 Git 后重试");
    assert_eq!(detail_failure_text("meta.detail, named by meta.detail, does not exist"), "插件声明的详细介绍文件无效");
    assert_eq!(detail_failure_text("mysterious"), "无法读取详细介绍");
}

#[test]
fn query_params_skip_blank_filters() {
    let query = DiscoverQuery {
        text: "  ".to_string(),
        category: None,
        labels: Vec::new(),
        sort: MarketSort::Index,
    };
    let params = market_query_params(&query, 0, 50, None, false);
    let json = serde_json::to_value(&params).unwrap();
    assert!(json.get("text").is_none());
    assert!(json.get("category").is_none());
    assert!(json.get("labels").is_none());
    assert!(json.get("refresh").is_none());
    assert!(json.get("fetchedAt").is_none());
    assert_eq!(json["offset"], 0);
    assert_eq!(json["limit"], 50);
    assert_eq!(json["sort"], serde_json::json!({"kind": "index"}));

    let query = DiscoverQuery {
        text: "whale".to_string(),
        category: Some("tools".to_string()),
        labels: vec!["official".to_string()],
        sort: MarketSort::Published { direction: MarketSortDirection::Asc },
    };
    let params = market_query_params(&query, 50, 50, Some(1234), true);
    let json = serde_json::to_value(&params).unwrap();
    assert_eq!(json["text"], "whale");
    assert_eq!(json["category"], "tools");
    assert_eq!(json["labels"], serde_json::json!(["official"]));
    assert_eq!(json["sort"], serde_json::json!({"kind": "published", "direction": "asc"}));
    assert_eq!(json["offset"], 50);
    assert_eq!(json["fetchedAt"], 1234);
    assert_eq!(json["refresh"], true);
}

#[test]
fn pages_merge_and_dedupe_until_complete() {
    let mut pages = MarketPages::default();
    assert_eq!(pages.next_request(50), Some((0, 50)));

    let first = page_json(&[entry_named("a"), entry_named("b")], 100, Some(2));
    match pages.merge(first, 50) {
        PageMerge::Continue { done } => assert!(!done),
        other => panic!("unexpected merge: {other:?}"),
    }
    assert_eq!(pages.next_request(50), Some((2, 48)));

    let second = page_json(&[entry_named("b"), entry_named("c")], 100, None);
    match pages.merge(second, 50) {
        PageMerge::Continue { done } => assert!(done),
        other => panic!("unexpected merge: {other:?}"),
    }
    assert_eq!(pages.entries.iter().map(|entry| entry.name.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
    assert_eq!(pages.next_request(50), None);
    assert_eq!(pages.total(), 2);
}

#[test]
fn pages_stop_at_the_target_count() {
    let mut pages = MarketPages::default();
    let first = page_json(&[entry_named("a")], 100, Some(1));
    assert!(matches!(
        pages.merge(first, 1),
        PageMerge::Continue { done: true }
    ));
}

#[test]
fn pages_restart_once_on_a_replaced_document() {
    let mut pages = MarketPages::default();
    let first = page_json(&[entry_named("a")], 100, Some(1));
    pages.merge(first, 50);
    let replaced = page_json(&[entry_named("z")], 200, Some(1));
    assert_eq!(pages.merge(replaced, 50), PageMerge::Restart);
    assert!(pages.entries.is_empty());
    assert_eq!(pages.fetched_at, None);
    assert!(pages.restarted);

    let pinned = page_json(&[entry_named("y")], 200, Some(1));
    pages.merge(pinned, 50);
    let replaced_again = page_json(&[entry_named("x")], 300, None);
    assert_eq!(pages.merge(replaced_again, 50), PageMerge::Changed);
}

#[test]
fn search_result_parses_entries_and_metadata() {
    let page: MarketSearchResult = serde_json::from_value(serde_json::json!({
        "url": "https://market.example/index.json",
        "title": {"zh": "示例市场"},
        "metadata": {"categories": {"tools": {"title": {"zh": "工具"}}}, "labels": {}, "sortKeys": {}},
        "fetchedAt": 42,
        "entries": [entry_json()],
        "total": 1,
        "nextOffset": null
    }))
    .unwrap();
    let entry = &page.entries[0];
    assert_eq!(entry.name, "@community/whale-pet");
    assert_eq!(entry.versions.len(), 2);
    assert_eq!(entry.installation.status, "not-installed");
    assert_eq!(entry.versions[0].install.spec, "@community/whale-pet@1.1.0");
    assert!(entry.versions[0].compatibility.compatible);
    assert!(!entry.versions[1].compatibility.compatible);
    assert!(entry.versions[1].compatibility.exempted);
    assert_eq!(page.next_offset, None);
}
