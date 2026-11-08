use dsh_libadwaita::secret_mask::mask_secret_patterns;

fn repeat(ch: char, count: usize) -> String {
    std::iter::repeat_n(ch, count).collect()
}

fn stars(count: usize) -> String {
    repeat('*', count)
}

fn begin_line() -> String {
    concat!("-----BEGIN ", "PRIVATE KEY-----").to_string()
}

fn end_line() -> String {
    concat!("-----END ", "PRIVATE KEY-----").to_string()
}

#[test]
fn test_masks_sk_key_with_visible_prefix() {
    let token = format!("sk-abc123{}", repeat('x', 14));
    let masked = mask_secret_patterns(&format!("key {} done", token));
    assert_eq!(masked, format!("key sk-abc12{} done", stars(15)));
}

#[test]
fn test_sk_key_needs_a_digit() {
    let token = format!("sk-{}", repeat('a', 20));
    assert_eq!(mask_secret_patterns(&token), token);
}

#[test]
fn test_masks_github_token_family() {
    let token = format!("ghp_{}", repeat('a', 36));
    assert_eq!(mask_secret_patterns(&token), format!("ghp_aaaa{}", stars(32)));
}

#[test]
fn test_masks_github_pat_with_partial_literal() {
    let token = format!("github_pat_{}", repeat('A', 22));
    assert_eq!(mask_secret_patterns(&token), format!("github_p{}", stars(25)));
}

#[test]
fn test_masks_aws_access_key_only_at_its_length() {
    let token = format!("AKIA{}", repeat('A', 16));
    assert_eq!(mask_secret_patterns(&token), format!("AKIAAAAA{}", stars(12)));
    let longer = format!("AKIA{}x", repeat('A', 16));
    assert_eq!(mask_secret_patterns(&longer), longer);
}

#[test]
fn test_masks_jwt_with_three_segments() {
    let token = format!(
        "eyJ{}.eyJ{}.{}",
        repeat('a', 16),
        repeat('b', 28),
        repeat('c', 8)
    );
    assert_eq!(mask_secret_patterns(&token), format!("eyJaaaaa{}", stars(52)));
}

#[test]
fn test_masks_google_api_key_at_fixed_length() {
    let token = format!("AIza{}XYZ", repeat('A', 35));
    assert_eq!(
        mask_secret_patterns(&token),
        format!("AIzaAAAA{}XYZ", stars(31))
    );
}

#[test]
fn test_key_block_keeps_begin_and_end_lines() {
    let text = format!("{}\n{}\n{}", begin_line(), repeat('A', 4), end_line());
    let expected = format!("{}\n{}\n{}", begin_line(), stars(4), end_line());
    assert_eq!(mask_secret_patterns(&text), expected);
}

#[test]
fn test_key_block_preserves_inner_line_breaks() {
    let text = format!(
        "{}\n{}\n{}\n{}",
        begin_line(),
        repeat('A', 4),
        repeat('B', 4),
        end_line()
    );
    let expected = format!(
        "{}\n{}\n{}\n{}",
        begin_line(),
        stars(4),
        stars(4),
        end_line()
    );
    assert_eq!(mask_secret_patterns(&text), expected);
}

#[test]
fn test_one_line_key_block_is_masked_whole() {
    let text = format!("{}AA{}", begin_line(), repeat('A', 2));
    let masked = mask_secret_patterns(&text);
    assert_eq!(masked, stars(text.chars().count()));
}

#[test]
fn test_key_block_without_end_runs_to_text_end() {
    let body = repeat('A', 8);
    let text = format!("{}\n{}", begin_line(), body);
    let masked = mask_secret_patterns(&text);
    assert_eq!(masked, format!("{}\n{}", begin_line(), stars(8)));
}

#[test]
fn test_masks_url_credentials() {
    let text = format!("https://alice:{}@example.com/path", repeat('p', 6));
    let masked = mask_secret_patterns(&text);
    assert_eq!(masked, format!("https://alice:{}@example.com/path", stars(6)));
}

#[test]
fn test_url_without_credentials_is_unchanged() {
    assert_eq!(
        mask_secret_patterns("https://example.com/path"),
        "https://example.com/path"
    );
    assert_eq!(
        mask_secret_patterns("HTTPS://alice:pw@example.com"),
        "HTTPS://alice:pw@example.com"
    );
}

#[test]
fn test_url_pw_char_count() {
    let text = format!("https://user:{}@example.com", repeat('\u{5bc6}', 2));
    assert_eq!(
        mask_secret_patterns(&text),
        "https://user:**@example.com"
    );
}

#[test]
fn test_prefixed_literal_requires_word_boundary() {
    let body = repeat('a', 20);
    let embedded = format!("xsk-abc123{}", body);
    assert_eq!(mask_secret_patterns(&embedded), embedded);
    let delimited = format!("(sk-abc123{}", body);
    assert_eq!(
        mask_secret_patterns(&delimited),
        format!("(sk-abc12{}", stars(21))
    );
}

#[test]
fn test_key_block_absorbs_prefixed_candidates_inside_body() {
    let body = format!("sk-abc123{}", repeat('x', 14));
    let text = format!("{}\n{}\n{}", begin_line(), body, end_line());
    let expected = format!(
        "{}\n{}\n{}",
        begin_line(),
        stars(body.chars().count()),
        end_line()
    );
    assert_eq!(mask_secret_patterns(&text), expected);
}

#[test]
fn test_plain_text_is_unchanged() {
    assert_eq!(mask_secret_patterns(""), "");
    assert_eq!(
        mask_secret_patterns("nothing sensitive here"),
        "nothing sensitive here"
    );
    assert_eq!(
        mask_secret_patterns("risk-averse task-loop"),
        "risk-averse task-loop"
    );
}
