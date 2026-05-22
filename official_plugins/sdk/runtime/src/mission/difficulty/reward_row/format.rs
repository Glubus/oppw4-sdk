use super::RewardRowDump;

impl RewardRowDump {
    pub(crate) fn format_log(&self) -> String {
        let mut message = format!(
            "reward_row index={} fixed20=0x{:x} fixed28=0x{:x}",
            self.index, self.fixed20, self.fixed28
        );

        message.push_str(" u32=");
        push_u32_fields(&mut message, &self.direct_u32);
        message.push_str(" u16x4=");
        push_u16_arrays(&mut message, &self.arrays_u16);
        message.push_str(&format!(
            " bytes_39c=[{},{},{},{}]",
            self.bytes_39c[0], self.bytes_39c[1], self.bytes_39c[2], self.bytes_39c[3]
        ));
        message
    }
}

fn push_u32_fields(message: &mut String, fields: &[(usize, u32); 4]) {
    for (idx, (offset, value)) in fields.iter().enumerate() {
        if idx != 0 {
            message.push(',');
        }
        message.push_str(&format!("0x{offset:x}:{value}"));
    }
}

fn push_u16_arrays(message: &mut String, arrays: &[(usize, [u16; 4]); 10]) {
    for (idx, (offset, values)) in arrays.iter().enumerate() {
        if idx != 0 {
            message.push(',');
        }
        message.push_str(&format!(
            "0x{offset:x}:[{},{},{},{}]",
            values[0], values[1], values[2], values[3]
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_compact_reward_row_dump() {
        let dump = RewardRowDump {
            index: 42,
            fixed20: 0x1000,
            fixed28: 0x2000,
            direct_u32: [(0x334, 10), (0x33c, 20), (0x340, 30), (0x348, 40)],
            arrays_u16: [
                (0x34c, [1, 2, 3, 4]),
                (0x354, [5, 6, 7, 8]),
                (0x35c, [9, 10, 11, 12]),
                (0x364, [13, 14, 15, 16]),
                (0x36c, [17, 18, 19, 20]),
                (0x374, [21, 22, 23, 24]),
                (0x37c, [25, 26, 27, 28]),
                (0x384, [29, 30, 31, 32]),
                (0x38c, [33, 34, 35, 36]),
                (0x394, [37, 38, 39, 40]),
            ],
            bytes_39c: [4, 3, 2, 1],
        };

        let log = dump.format_log();
        assert!(log.contains("reward_row index=42"));
        assert!(log.contains("0x334:10"));
        assert!(log.contains("0x394:[37,38,39,40]"));
        assert!(log.contains("bytes_39c=[4,3,2,1]"));
    }
}
