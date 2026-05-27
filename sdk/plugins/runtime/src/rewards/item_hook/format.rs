use std::fmt::Write;

use super::{ItemRewardEntry, ItemRewardSnapshot, TRIPLET_WORDS};

pub(super) fn snapshot(
    call: usize,
    out: usize,
    reward_context: u64,
    previous: usize,
    result: u32,
    words: &[i32],
    max_entries: usize,
) -> ItemRewardSnapshot {
    ItemRewardSnapshot {
        call,
        out,
        reward_context,
        previous,
        result,
        entries: entries(words, max_entries),
    }
}

pub(super) fn entries_log(entries: &[ItemRewardEntry]) -> String {
    let mut text = String::new();
    for (written, entry) in entries.iter().enumerate() {
        if written > 0 {
            text.push(',');
        }
        let _ = write!(
            text,
            "#{}:amount={}:item={}:new={}",
            entry.index, entry.amount, entry.item_id, entry.is_new
        );
    }
    if text.is_empty() {
        "none".to_string()
    } else {
        text
    }
}

fn entries(words: &[i32], max_entries: usize) -> Vec<ItemRewardEntry> {
    let mut entries = Vec::new();
    for (index, entry) in words.chunks_exact(TRIPLET_WORDS).enumerate() {
        let amount = entry[0];
        if amount == 0 {
            continue;
        }
        let item_id = entry[1];
        let is_new = entry[2];
        entries.push(ItemRewardEntry {
            index,
            amount,
            item_id,
            is_new,
        });
        if entries.len() >= max_entries {
            break;
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_non_zero_triplets() {
        let words = [0, 0, 0, 5, 73, 1, 7, 31, 0];

        assert_eq!(
            entries_log(&entries(&words, 40)),
            "#1:amount=5:item=73:new=1,#2:amount=7:item=31:new=0"
        );
    }
}
