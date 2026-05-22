use std::fmt::Write;

use super::TRIPLET_WORDS;

pub(super) fn entries(words: &[i32], max_entries: usize) -> String {
    let mut text = String::new();
    let mut written = 0usize;
    for (index, entry) in words.chunks_exact(TRIPLET_WORDS).enumerate() {
        let amount = entry[0];
        if amount == 0 {
            continue;
        }
        if written > 0 {
            text.push(',');
        }
        let item_id = entry[1];
        let is_new = entry[2];
        let _ = write!(text, "#{index}:amount={amount}:item={item_id}:new={is_new}");
        written += 1;
        if written >= max_entries {
            break;
        }
    }
    if text.is_empty() {
        "none".to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_non_zero_triplets() {
        let words = [0, 0, 0, 5, 73, 1, 7, 31, 0];

        assert_eq!(
            entries(&words, 40),
            "#1:amount=5:item=73:new=1,#2:amount=7:item=31:new=0"
        );
    }
}
