use super::{RANK_THRESHOLD_COUNT, SLOT_COUNT, SLOT_SELECTOR_OFFSET, THRESHOLD_ROWS};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RankHelperRow {
    pub(super) slot: usize,
    pub(super) selectors: [u16; SLOT_COUNT],
    pub(super) thresholds: [u32; RANK_THRESHOLD_COUNT],
    pub(super) all_thresholds: [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
}

impl RankHelperRow {
    pub(super) fn read(row: usize, selector: u16) -> Option<Self> {
        if row == 0 {
            return None;
        }

        let selectors = unsafe { read_selectors(row) };
        let slot = selectors
            .iter()
            .position(|candidate| *candidate == selector)?;
        let all_thresholds = unsafe { read_all_thresholds(row) };
        let thresholds = all_thresholds[slot];
        Some(Self {
            slot,
            selectors,
            thresholds,
            all_thresholds,
        })
    }

    pub(super) fn thresholds_csv(self) -> String {
        csv(self.thresholds)
    }

    pub(super) fn threshold_ratios_csv(self, value: u32) -> String {
        if value == 0 {
            return "inf,inf,inf,inf,inf".to_string();
        }

        self.thresholds
            .map(|threshold| format!("{:.3}", threshold as f32 / value as f32))
            .join(",")
    }

    pub(super) fn all_thresholds_csv(self) -> String {
        self.all_thresholds
            .iter()
            .enumerate()
            .map(|(slot, thresholds)| format!("s{slot}:{}", csv(*thresholds)))
            .collect::<Vec<_>>()
            .join(";")
    }

    pub(super) fn selectors_csv(self) -> String {
        csv(self.selectors)
    }

    pub(super) fn matches_prefix(self, prefix: [u32; 3]) -> bool {
        self.thresholds[..prefix.len()] == prefix
    }

    pub(super) fn shifted_thresholds(self, inserted_first: u32) -> [u32; RANK_THRESHOLD_COUNT] {
        [
            inserted_first,
            self.thresholds[0],
            self.thresholds[1],
            self.thresholds[2],
            self.thresholds[3],
        ]
    }
}

pub(super) fn csv<const N: usize, T: ToString>(values: [T; N]) -> String {
    values
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

unsafe fn read_selectors(row: usize) -> [u16; SLOT_COUNT] {
    let ptr = row as *const u8;
    [
        read_u16(ptr, SLOT_SELECTOR_OFFSET),
        read_u16(ptr, SLOT_SELECTOR_OFFSET + 2),
        read_u16(ptr, SLOT_SELECTOR_OFFSET + 4),
    ]
}

unsafe fn read_thresholds(row: usize, slot: usize) -> [u32; RANK_THRESHOLD_COUNT] {
    let ptr = row as *const u8;
    THRESHOLD_ROWS.map(|offset| read_u32(ptr, offset + slot * 4))
}

unsafe fn read_all_thresholds(row: usize) -> [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT] {
    [
        read_thresholds(row, 0),
        read_thresholds(row, 1),
        read_thresholds(row, 2),
    ]
}

unsafe fn read_u16(ptr: *const u8, offset: usize) -> u16 {
    (ptr.add(offset) as *const u16).read_unaligned()
}

unsafe fn read_u32(ptr: *const u8, offset: usize) -> u32 {
    (ptr.add(offset) as *const u32).read_unaligned()
}

pub(super) unsafe fn write_thresholds(
    row: usize,
    slot: usize,
    thresholds: [u32; RANK_THRESHOLD_COUNT],
) {
    let ptr = row as *mut u8;
    for (offset, value) in THRESHOLD_ROWS.into_iter().zip(thresholds) {
        (ptr.add(offset + slot * size_of::<u32>()) as *mut u32).write_unaligned(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_rank_helper_row_for_selector_slot() {
        let mut row = [0u8; 0x70];
        write_u32(&mut row, 0x00, 100);
        write_u32(&mut row, 0x0c, 200);
        write_u32(&mut row, 0x18, 300);
        write_u32(&mut row, 0x24, 400);
        write_u32(&mut row, 0x30, 500);
        write_u16(&mut row, 0x64, 0);
        write_u16(&mut row, 0x66, 1);
        write_u16(&mut row, 0x68, 99);

        let snapshot = RankHelperRow::read(row.as_ptr() as usize, 0).expect("row");

        assert_eq!(snapshot.slot, 0);
        assert_eq!(snapshot.selectors, [0, 1, 99]);
        assert_eq!(snapshot.thresholds, [100, 200, 300, 400, 500]);
        assert_eq!(
            snapshot.all_thresholds_csv(),
            "s0:100,200,300,400,500;s1:0,0,0,0,0;s2:0,0,0,0,0"
        );
    }

    #[test]
    fn ignores_unknown_selector() {
        let mut row = [0u8; 0x70];
        write_u16(&mut row, 0x64, 0);
        write_u16(&mut row, 0x66, 1);
        write_u16(&mut row, 0x68, 99);

        assert_eq!(RankHelperRow::read(row.as_ptr() as usize, 2), None);
    }

    #[test]
    fn shifts_count_thresholds_one_slot_right() {
        let row = RankHelperRow {
            slot: 1,
            selectors: [0, 1, 99],
            thresholds: [60_000, 60_000, 48_000, 42_000, 30_000],
            all_thresholds: [[0; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
        };

        assert!(row.matches_prefix([60_000, 60_000, 48_000]));
        assert_eq!(
            row.shifted_thresholds(72_000),
            [72_000, 60_000, 60_000, 48_000, 42_000]
        );
    }

    fn write_u16(row: &mut [u8], offset: usize, value: u16) {
        row[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn write_u32(row: &mut [u8], offset: usize, value: u32) {
        row[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
