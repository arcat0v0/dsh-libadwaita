pub fn stable_end(text: &str, previous_end: usize) -> usize {
    let previous = previous_end.min(text.len());
    if previous == text.len() {
        return text.len();
    }
    let fences = fence_positions(text);
    let mut candidates = Vec::new();
    for (index, _) in text.match_indices("\n\n") {
        candidates.push(index + 2);
    }
    for (index, _) in text.match_indices("\r\n\r\n") {
        candidates.push(index + 4);
    }
    candidates.sort_unstable();
    candidates.dedup();
    let mut best = previous;
    for candidate in candidates {
        if candidate < previous || candidate > text.len() {
            continue;
        }
        if fences.partition_point(|&position| position < candidate) % 2 == 0 {
            best = candidate;
        }
    }
    best
}

fn fence_positions(text: &str) -> Vec<usize> {
    let mut positions = Vec::new();
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with("```") {
            positions.push(line_start);
        }
        line_start += line.len();
    }
    positions
}
