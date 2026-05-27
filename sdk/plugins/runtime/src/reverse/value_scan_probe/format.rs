use super::{
    scan::{ValueHit, ValueWidth},
    snapshot::ValueSnapshot,
};

impl ValueSnapshot {
    pub(super) fn format_log(&self) -> String {
        format!(
            "value_probe mission_id={} difficulty={} mode_type={} global=0x{:x} hits={}",
            self.mission_id,
            self.difficulty,
            self.mode_type,
            self.global,
            format_hits(&self.hits),
        )
    }
}

fn format_hits(hits: &[ValueHit]) -> String {
    if hits.is_empty() {
        return "none".to_string();
    }

    hits.iter().map(format_hit).collect::<Vec<_>>().join(",")
}

fn format_hit(hit: &ValueHit) -> String {
    let width = match hit.width {
        ValueWidth::U16 => "u16",
        ValueWidth::U32 => "u32",
    };
    format!("{}:{}@+0x{:x}", hit.value, width, hit.offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hits_compactly() {
        let hit = ValueHit {
            value: 992_250,
            width: ValueWidth::U32,
            offset: 0x1234,
        };

        assert_eq!(format_hit(&hit), "992250:u32@+0x1234");
    }
}
