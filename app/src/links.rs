#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    External(String),
    File {
        path: String,
        line: Option<u32>,
        end_line: Option<u32>,
    },
    Blocked,
}

pub fn classify_link(raw: &str) -> LinkTarget {
    let target = raw.trim();
    if target.is_empty() {
        return LinkTarget::Blocked;
    }
    let lower = target.to_ascii_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("vbscript:") || lower.starts_with("data:")
    {
        return LinkTarget::Blocked;
    }
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        return LinkTarget::External(target.to_string());
    }
    if lower.starts_with("file://") {
        return parse_file_reference(&target[7..]).unwrap_or(LinkTarget::Blocked);
    }
    parse_file_reference(target).unwrap_or(LinkTarget::Blocked)
}

pub fn parse_file_reference(raw: &str) -> Option<LinkTarget> {
    let target = raw.trim();
    if target.is_empty() {
        return None;
    }
    if let Some((path, fragment)) = target.split_once("#L") {
        if path.is_empty() {
            return None;
        }
        let (line, end_line) = parse_line_range(fragment)?;
        return Some(LinkTarget::File {
            path: path.to_string(),
            line: Some(line),
            end_line,
        });
    }
    if let Some((path, suffix)) = target.rsplit_once(':') {
        if path.contains('/') && suffix.chars().all(|c| c.is_ascii_digit() || c == '-') {
            let (line, end_line) = parse_line_range(suffix)?;
            return Some(LinkTarget::File {
                path: path.to_string(),
                line: Some(line),
                end_line,
            });
        }
    }
    Some(LinkTarget::File {
        path: target.to_string(),
        line: None,
        end_line: None,
    })
}

fn parse_line_range(text: &str) -> Option<(u32, Option<u32>)> {
    let (start, end) = match text.split_once('-') {
        Some((start, end)) => (start, Some(end)),
        None => (text, None),
    };
    let start = start.trim_start_matches(['L', 'l']).parse::<u32>().ok()?;
    let end = match end {
        Some(end) => Some(
            end.trim_start_matches(['L', 'l'])
                .parse::<u32>()
                .ok()?,
        ),
        None => None,
    };
    Some((start, end))
}
