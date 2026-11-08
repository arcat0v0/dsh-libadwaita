#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MaskKind {
    KeyBlock,
    Address,
    Prefixed,
}

#[derive(Debug, Clone, Copy)]
enum Shape {
    Run,
    MaxRun,
    DigitRun,
    Fixed,
    Jwt,
}

#[derive(Debug, Clone, Copy)]
enum Run {
    Key,
    Alnum,
    Word,
    AlnumDash,
    UpperAlnum,
}

#[derive(Debug, Clone, Copy)]
struct Format {
    literal: &'static str,
    kind: MaskKind,
    shape: Shape,
    run: Run,
    min: usize,
    max: usize,
}

const FORMATS: &[Format] = &[
    Format {
        literal: "-----BEGIN ",
        kind: MaskKind::KeyBlock,
        shape: Shape::Run,
        run: Run::Key,
        min: 0,
        max: 0,
    },
    Format {
        literal: "://",
        kind: MaskKind::Address,
        shape: Shape::Run,
        run: Run::Key,
        min: 0,
        max: 0,
    },
    Format {
        literal: "sk-",
        kind: MaskKind::Prefixed,
        shape: Shape::DigitRun,
        run: Run::Key,
        min: 20,
        max: 0,
    },
    Format {
        literal: "AIza",
        kind: MaskKind::Prefixed,
        shape: Shape::Fixed,
        run: Run::Key,
        min: 35,
        max: 0,
    },
    Format {
        literal: "ghp_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "gho_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "ghu_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "ghs_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "ghr_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "github_pat_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Word,
        min: 22,
        max: 0,
    },
    Format {
        literal: "glpat-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Key,
        min: 20,
        max: 0,
    },
    Format {
        literal: "npm_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 36,
        max: 0,
    },
    Format {
        literal: "hf_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 30,
        max: 0,
    },
    Format {
        literal: "AKIA",
        kind: MaskKind::Prefixed,
        shape: Shape::MaxRun,
        run: Run::UpperAlnum,
        min: 16,
        max: 16,
    },
    Format {
        literal: "ASIA",
        kind: MaskKind::Prefixed,
        shape: Shape::MaxRun,
        run: Run::UpperAlnum,
        min: 16,
        max: 16,
    },
    Format {
        literal: "LTAI",
        kind: MaskKind::Prefixed,
        shape: Shape::MaxRun,
        run: Run::Alnum,
        min: 12,
        max: 20,
    },
    Format {
        literal: "AKID",
        kind: MaskKind::Prefixed,
        shape: Shape::MaxRun,
        run: Run::Alnum,
        min: 28,
        max: 36,
    },
    Format {
        literal: "xoxa-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::AlnumDash,
        min: 10,
        max: 0,
    },
    Format {
        literal: "xoxb-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::AlnumDash,
        min: 10,
        max: 0,
    },
    Format {
        literal: "xoxp-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::AlnumDash,
        min: 10,
        max: 0,
    },
    Format {
        literal: "xoxr-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::AlnumDash,
        min: 10,
        max: 0,
    },
    Format {
        literal: "xoxs-",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::AlnumDash,
        min: 10,
        max: 0,
    },
    Format {
        literal: "sk_live_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 16,
        max: 0,
    },
    Format {
        literal: "sk_test_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 16,
        max: 0,
    },
    Format {
        literal: "rk_live_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 16,
        max: 0,
    },
    Format {
        literal: "rk_test_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 16,
        max: 0,
    },
    Format {
        literal: "whsec_",
        kind: MaskKind::Prefixed,
        shape: Shape::Run,
        run: Run::Alnum,
        min: 24,
        max: 0,
    },
    Format {
        literal: "eyJ",
        kind: MaskKind::Prefixed,
        shape: Shape::Jwt,
        run: Run::Key,
        min: 8,
        max: 2048,
    },
];

const VISIBLE_PREFIX: usize = 8;
const PRIVATE_KEY_SUFFIX: &str = "PRIVATE KEY-----";

#[derive(Debug, Clone, Copy)]
struct Candidate {
    start: usize,
    end: usize,
    kind: MaskKind,
}

pub fn mask_secret_patterns(text: &str) -> String {
    let mut candidates = collect_candidates(text);
    if candidates.is_empty() {
        return text.to_string();
    }
    candidates.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| kind_rank(left.kind).cmp(&kind_rank(right.kind)))
            .then_with(|| right.end.cmp(&left.end))
    });

    let mut masked = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut index = 0;
    while index < candidates.len() {
        let first = candidates[index];
        index += 1;
        if first.start < cursor {
            continue;
        }
        let mut end = first.end;
        let mut plain = first.kind == MaskKind::Prefixed;
        while index < candidates.len() && candidates[index].start < end {
            let candidate = candidates[index];
            index += 1;
            if candidate.end > end {
                end = candidate.end;
                if candidate.kind != MaskKind::Prefixed {
                    plain = false;
                }
            }
        }
        masked.push_str(&text[cursor..first.start]);
        masked.push_str(&mask_range(text, first.start, end, first.kind, plain));
        cursor = end;
    }
    masked.push_str(&text[cursor..]);
    masked
}

fn kind_rank(kind: MaskKind) -> u8 {
    match kind {
        MaskKind::KeyBlock => 0,
        MaskKind::Address => 1,
        MaskKind::Prefixed => 2,
    }
}

fn collect_candidates(text: &str) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    for format in FORMATS {
        match format.kind {
            MaskKind::KeyBlock => {
                collect_literal_candidates(text, format.literal, |at| {
                    key_block_end(text, at).map(|end| (at, end))
                }, &mut candidates);
            }
            MaskKind::Address => {
                for (at, _) in text.match_indices(format.literal) {
                    if let Some((start, end)) = address_candidate(text, at) {
                        candidates.push(Candidate {
                            start,
                            end,
                            kind: MaskKind::Address,
                        });
                    }
                }
            }
            MaskKind::Prefixed => {
                collect_literal_candidates(text, format.literal, |at| {
                    if is_word_before(text, at) {
                        return None;
                    }
                    prefixed_end(text, format, at).map(|end| (at, end))
                }, &mut candidates);
            }
        }
    }
    candidates
}

fn collect_literal_candidates<F>(
    text: &str,
    literal: &str,
    resolve: F,
    candidates: &mut Vec<Candidate>,
) where
    F: Fn(usize) -> Option<(usize, usize)>,
{
    let mut from = 0;
    while let Some(relative) = text[from..].find(literal) {
        let at = from + relative;
        if let Some((start, end)) = resolve(at) {
            candidates.push(Candidate {
                start,
                end,
                kind: if literal == "-----BEGIN " {
                    MaskKind::KeyBlock
                } else {
                    MaskKind::Prefixed
                },
            });
        }
        from = at + literal.len();
    }
}

fn is_word_before(text: &str, at: usize) -> bool {
    text[..at]
        .chars()
        .next_back()
        .map(|value| value.is_ascii_alphanumeric() || value == '_')
        .unwrap_or(false)
}

fn is_word_at(text: &str, at: usize) -> bool {
    text[at..]
        .chars()
        .next()
        .map(|value| value.is_ascii_alphanumeric() || value == '_')
        .unwrap_or(false)
}

fn is_js_space(value: char) -> bool {
    value.is_ascii_whitespace()
        || matches!(
            value,
            '\u{a0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
        )
}

fn scan_run(text: &str, start: usize, run: Run) -> usize {
    let bytes = text.as_bytes();
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        let allowed = match run {
            Run::Key => byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-',
            Run::Alnum => byte.is_ascii_alphanumeric(),
            Run::Word => byte.is_ascii_alphanumeric() || byte == b'_',
            Run::AlnumDash => byte.is_ascii_alphanumeric() || byte == b'-',
            Run::UpperAlnum => byte.is_ascii_uppercase() || byte.is_ascii_digit(),
        };
        if !allowed {
            break;
        }
        index += 1;
    }
    index
}

fn first_digit(text: &str, start: usize, end: usize) -> Option<usize> {
    text[start..end]
        .bytes()
        .position(|byte| byte.is_ascii_digit())
        .map(|offset| start + offset)
}

fn prefixed_end(text: &str, format: &Format, at: usize) -> Option<usize> {
    let start = at + format.literal.len();
    if start > text.len() {
        return None;
    }
    let run_end = scan_run(text, start, format.run);
    let length = run_end - start;
    match format.shape {
        Shape::DigitRun => {
            if length < format.min {
                return None;
            }
            let digit = first_digit(text, start, run_end)?;
            if digit - start > 255 {
                return None;
            }
            Some(run_end)
        }
        Shape::Run => (length >= format.min).then_some(run_end),
        Shape::MaxRun => {
            if length < format.min || length > format.max || is_word_at(text, run_end) {
                None
            } else {
                Some(run_end)
            }
        }
        Shape::Fixed => Some((start + format.min).min(text.len())),
        Shape::Jwt => {
            if length < format.min || length > format.max || !text[run_end..].starts_with(".eyJ") {
                return None;
            }
            let payload_start = run_end + 4;
            let payload_end = scan_run(text, payload_start, Run::Key);
            if payload_end - payload_start < format.min
                || !text[payload_end..].starts_with('.')
            {
                return None;
            }
            let signature_start = payload_end + 1;
            let signature_end = scan_run(text, signature_start, Run::Key);
            if signature_end - signature_start < format.min {
                return None;
            }
            Some(signature_end)
        }
    }
}

fn key_block_end(text: &str, at: usize) -> Option<usize> {
    let header = label_end(text, at + "-----BEGIN ".len())?;
    let mut from = header;
    while let Some(relative) = text[from..].find("-----END ") {
        let line = from + relative;
        if let Some(end) = label_end(text, line + "-----END ".len()) {
            return Some(end);
        }
        from = line + 1;
    }
    Some(text.len())
}

fn label_end(text: &str, start: usize) -> Option<usize> {
    let limit = text.len().min(start + 52);
    let mut index = start;
    while index < limit {
        let value = text[index..].chars().next()?;
        if !(value.is_ascii_uppercase() || value.is_ascii_digit() || value == ' ') {
            break;
        }
        index += value.len_utf8();
    }
    if index >= start + 52 {
        return None;
    }
    let tail = index.checked_sub(11)?;
    if tail < start || !text[tail..].starts_with(PRIVATE_KEY_SUFFIX) {
        return None;
    }
    Some(tail + PRIVATE_KEY_SUFFIX.len())
}

fn address_candidate(text: &str, at: usize) -> Option<(usize, usize)> {
    if !scheme_before(text, at) {
        return None;
    }
    let user_start = at + 3;
    let user_end = scan_authority(text, user_start, true);
    if text[user_start..user_end].chars().count() > 256 {
        return None;
    }
    if !text[user_end..].starts_with(':') {
        return None;
    }
    let pw_start = user_end + 1;
    let pw_end = scan_authority(text, pw_start, false);
    if pw_end == pw_start || !text[pw_end..].starts_with('@') {
        return None;
    }
    Some((pw_start, pw_end))
}
fn scan_authority(text: &str, start: usize, user: bool) -> usize {
    for (offset, value) in text[start..].char_indices() {
        if is_js_space(value)
            || value == '/'
            || value == '@'
            || value == '?'
            || value == '#'
            || (user && value == ':')
        {
            return start + offset;
        }
    }
    text.len()
}

fn scheme_before(text: &str, at: usize) -> bool {
    let mut length = 0;
    for (index, value) in text[..at].char_indices().rev() {
        if length >= 16 {
            return false;
        }
        let allowed = value.is_ascii_lowercase()
            || value.is_ascii_digit()
            || matches!(value, '+' | '.' | '-');
        if !allowed {
            return false;
        }
        length += 1;
        if value.is_ascii_lowercase() && length >= 2 {
            let boundary = text[..index]
                .chars()
                .next_back()
                .map(|before| !(before.is_ascii_alphanumeric() || before == '_'))
                .unwrap_or(true);
            if boundary {
                return true;
            }
        }
    }
    false
}

fn mask_range(text: &str, start: usize, end: usize, kind: MaskKind, plain: bool) -> String {
    match kind {
        MaskKind::KeyBlock => mask_key_block(text, start, end),
        MaskKind::Address => mask_characters(&text[start..end]),
        MaskKind::Prefixed => {
            let range = &text[start..end];
            let mut chars = range.chars();
            let kept: String = chars.by_ref().take(VISIBLE_PREFIX).collect();
            if plain {
                let count = range.chars().count().saturating_sub(VISIBLE_PREFIX);
                format!("{}{}", kept, "*".repeat(count))
            } else {
                format!("{}{}", kept, mask_characters(&chars.collect::<String>()))
            }
        }
    }
}

fn mask_key_block(text: &str, start: usize, end: usize) -> String {
    let range = &text[start..end];
    let body_start = range.find('\n').map(|offset| start + offset + 1).unwrap_or(start);
    let body_end = text[body_start..end]
        .rfind("\n-----END")
        .map(|offset| body_start + offset)
        .unwrap_or(end);
    let mut masked = String::with_capacity(range.len());
    masked.push_str(&text[start..body_start]);
    masked.push_str(&mask_characters(&text[body_start..body_end]));
    masked.push_str(&text[body_end..end]);
    masked
}

fn mask_characters(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character == '\n' || character == '\r' {
                character
            } else {
                '*'
            }
        })
        .collect()
}
