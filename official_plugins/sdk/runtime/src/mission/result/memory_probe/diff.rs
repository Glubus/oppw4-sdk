pub(super) fn describe(previous: Option<&Vec<u8>>, current: &[u8], limit: usize) -> String {
    previous
        .map(|previous| changed_words(previous, current, limit))
        .unwrap_or_else(|| non_zero_words(current, limit))
}

fn changed_words(previous: &[u8], current: &[u8], limit: usize) -> String {
    let mut changes = Vec::new();
    for (idx, (before, after)) in previous
        .chunks_exact(4)
        .zip(current.chunks_exact(4))
        .enumerate()
    {
        let before = u32::from_le_bytes([before[0], before[1], before[2], before[3]]);
        let after = u32::from_le_bytes([after[0], after[1], after[2], after[3]]);
        if before != after {
            changes.push(format!("+0x{:03x}:{}->{}", idx * 4, before, after));
            if changes.len() >= limit {
                break;
            }
        }
    }
    format_words(changes)
}

fn non_zero_words(bytes: &[u8], limit: usize) -> String {
    let mut words = Vec::new();
    for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
        let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if value != 0 {
            words.push(format!("+0x{:03x}:{}", idx * 4, value));
            if words.len() >= limit {
                break;
            }
        }
    }
    format_words(words)
}

fn format_words(words: Vec<String>) -> String {
    if words.is_empty() {
        "none".to_string()
    } else {
        words.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_words_reports_offsets_and_values() {
        let previous = [1, 0, 0, 0, 2, 0, 0, 0];
        let current = [1, 0, 0, 0, 5, 0, 0, 0];

        assert_eq!(changed_words(&previous, &current, 4), "+0x004:2->5");
    }

    #[test]
    fn non_zero_words_reports_initial_values() {
        let bytes = [0, 0, 0, 0, 7, 0, 0, 0, 9, 0, 0, 0];

        assert_eq!(non_zero_words(&bytes, 1), "+0x004:7");
    }
}
