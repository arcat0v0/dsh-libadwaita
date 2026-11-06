use dsh_libadwaita::transcript::*;

#[test]
fn test_parse_bash_tool() {
    let args = r#"{"command":"cargo test","cwd":"/home/project"}"#;
    let res = "running 1 test\ntest test_ok ... ok\n[exit code: 0]";
    let info = parse_bash_tool(args, res);

    assert_eq!(info.command, "cargo test");
    assert_eq!(info.cwd.as_deref(), Some("/home/project"));
    assert_eq!(info.exit_code, Some(0));
    assert!(info.output.contains("test_ok ... ok"));

    let args_fail = r#"{"command":"cargo build"}"#;
    let res_fail = "error: could not compile\n[exit code: 101]";
    let info_fail = parse_bash_tool(args_fail, res_fail);
    assert_eq!(info_fail.command, "cargo build");
    assert_eq!(info_fail.exit_code, Some(101));
}

#[test]
fn test_parse_file_diff() {
    let args = r#"{"path":"src/main.rs","diff":"--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1 +1 @@\n-old line\n+new line"}"#;
    let res = "File updated successfully";
    let info = parse_file_diff(args, res);

    assert_eq!(info.path, "src/main.rs");
    assert!(info.diff.contains("+new line"));
    assert!(info.diff.contains("-old line"));
    assert!(!info.is_patch);

    let patch_args = r#"{"patch":"*** patch content ***"}"#;
    let patch_info = parse_file_diff(patch_args, "Applied patch");
    assert!(patch_info.is_patch);
}

#[test]
fn test_parse_file_read() {
    let args = r#"{"path":"Cargo.toml","offset":10,"limit":20}"#;
    let res = "[dependencies]\ngtk4 = \"0.10\"";
    let info = parse_file_read(args, res);

    assert_eq!(info.path, "Cargo.toml");
    assert_eq!(info.range, "10-30");
    assert_eq!(info.content, res);

    let simple_args = r#"{"path":"README.md"}"#;
    let simple_info = parse_file_read(simple_args, "# Title");
    assert_eq!(simple_info.path, "README.md");
    assert_eq!(simple_info.range, "1-");
}

#[test]
fn test_parse_search_tool() {
    let grep_args = r#"{"pattern":"fn main","path":"src"}"#;
    let grep_res = "src/main.rs:8:fn main()";
    let grep_info = parse_search_tool("grep", grep_args, grep_res);

    assert_eq!(grep_info.tool, "grep");
    assert_eq!(grep_info.pattern, "fn main");
    assert_eq!(grep_info.path, "src");
    assert_eq!(grep_info.results, grep_res);

    let glob_args = r#"{"pattern":"**/*.rs"}"#;
    let glob_info = parse_search_tool("glob", glob_args, "src/lib.rs\nsrc/main.rs");
    assert_eq!(glob_info.tool, "glob");
    assert_eq!(glob_info.pattern, "**/*.rs");
}

#[test]
fn test_parse_web_tool() {
    let search_args = r#"{"query":"libadwaita rust"}"#;
    let search_res = "Found 10 results";
    let search_info = parse_web_tool("web_search", search_args, search_res);

    assert_eq!(search_info.tool, "web_search");
    assert_eq!(search_info.query_or_url, "libadwaita rust");
    assert_eq!(search_info.content, search_res);

    let fetch_args = r#"{"url":"https://example.com"}"#;
    let fetch_res = "[status: 200]\nExample Domain content";
    let fetch_info = parse_web_tool("web_fetch", fetch_args, fetch_res);
    assert_eq!(fetch_info.tool, "web_fetch");
    assert_eq!(fetch_info.query_or_url, "https://example.com");
    assert_eq!(fetch_info.status_code, Some(200));
}
