use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ValueHit {
    pub(super) value: u32,
    pub(super) width: ValueWidth,
    pub(super) offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueWidth {
    U16,
    U32,
}

pub(super) fn scan_values(bytes: &[u8], values: &[u32], max_hits: usize) -> Vec<ValueHit> {
    let targets = values.iter().copied().collect::<HashSet<_>>();
    let mut hits = Vec::new();
    collect_u16_hits(bytes, &targets, max_hits, &mut hits);
    collect_u32_hits(bytes, &targets, max_hits, &mut hits);
    hits
}

fn collect_u16_hits(
    bytes: &[u8],
    targets: &HashSet<u32>,
    max_hits: usize,
    hits: &mut Vec<ValueHit>,
) {
    for offset in (0..bytes.len().saturating_sub(1)).step_by(2) {
        let value = u32::from(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]));
        if targets.contains(&value) {
            hits.push(ValueHit {
                value,
                width: ValueWidth::U16,
                offset,
            });
        }
        if hits.len() >= max_hits {
            return;
        }
    }
}

fn collect_u32_hits(
    bytes: &[u8],
    targets: &HashSet<u32>,
    max_hits: usize,
    hits: &mut Vec<ValueHit>,
) {
    for offset in (0..bytes.len().saturating_sub(3)).step_by(4) {
        let value = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        if targets.contains(&value) {
            hits.push(ValueHit {
                value,
                width: ValueWidth::U32,
                offset,
            });
        }
        if hits.len() >= max_hits {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_u16_and_u32_targets() {
        let mut bytes = vec![0u8; 16];
        bytes[2..4].copy_from_slice(&170u16.to_le_bytes());
        bytes[8..12].copy_from_slice(&487_000u32.to_le_bytes());

        let hits = scan_values(&bytes, &[170, 487_000], 8);

        assert_eq!(
            hits,
            vec![
                ValueHit {
                    value: 170,
                    width: ValueWidth::U16,
                    offset: 2,
                },
                ValueHit {
                    value: 487_000,
                    width: ValueWidth::U32,
                    offset: 8,
                },
            ]
        );
    }
}
