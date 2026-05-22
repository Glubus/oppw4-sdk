use plugin_sdk::OwnedHostApi;

use crate::model::{AddressSpec, Scan, ScanHit, TargetValue, ValueType, Watch, WatchValue};

const MAX_SCAN_BYTES: usize = 4 * 1024 * 1024;

pub(crate) fn read_watch(host: &OwnedHostApi, watch: &Watch) -> Result<WatchValue, String> {
    let address = resolve(host, &watch.address)?;
    let mut bytes = vec![0u8; watch.value_type.width()];
    read_exact(host, address, &mut bytes, &watch.id)?;
    Ok(WatchValue {
        address,
        value_type: watch.value_type,
        bytes,
    })
}

pub(crate) fn run_scan(host: &OwnedHostApi, scan: &Scan) -> Result<Vec<ScanHit>, String> {
    let start = resolve(host, &scan.start)?;
    let mut bytes = vec![0u8; scan.bytes.clamp(scan.value_type.width(), MAX_SCAN_BYTES)];
    read_exact(host, start, &mut bytes, &scan.id)?;
    Ok(scan_bytes(start, &bytes, scan))
}

fn scan_bytes(start: usize, bytes: &[u8], scan: &Scan) -> Vec<ScanHit> {
    let width = scan.value_type.width();
    let mut hits = Vec::new();
    for offset in (0..=bytes.len().saturating_sub(width)).step_by(width) {
        let value = read_target_value(scan.value_type, &bytes[offset..offset + width]);
        if scan.values.contains(&value) {
            hits.push(ScanHit {
                address: start + offset,
                offset,
                value,
            });
        }
        if hits.len() >= scan.max_hits {
            break;
        }
    }
    hits
}

pub(crate) fn resolve(host: &OwnedHostApi, spec: &AddressSpec) -> Result<usize, String> {
    match spec {
        AddressSpec::Absolute(address) => Ok(*address),
        AddressSpec::ModuleRva(rva) => module_base(host).map(|base| base + rva),
        AddressSpec::PointerChain { base, offsets } => resolve_pointer_chain(host, base, offsets),
    }
}

fn resolve_pointer_chain(
    host: &OwnedHostApi,
    base: &AddressSpec,
    offsets: &[usize],
) -> Result<usize, String> {
    let mut address = resolve(host, base)?;
    let Some((last, deref_offsets)) = offsets.split_last() else {
        return Ok(address);
    };
    for offset in deref_offsets {
        address = read_usize(host, address + offset, "pointer_chain")?;
        if address == 0 {
            return Err("pointer chain resolved to null".to_string());
        }
    }
    Ok(address + last)
}

fn read_target_value(value_type: ValueType, bytes: &[u8]) -> TargetValue {
    match value_type {
        ValueType::U8 => TargetValue::Integer(i64::from(bytes[0])),
        ValueType::U16 => TargetValue::Integer(i64::from(u16::from_le_bytes([bytes[0], bytes[1]]))),
        ValueType::U32 => TargetValue::Integer(i64::from(u32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        ValueType::I32 => TargetValue::Integer(i64::from(i32::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
        ]))),
        ValueType::F32 => {
            TargetValue::Float(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        }
    }
}

fn module_base(host: &OwnedHostApi) -> Result<usize, String> {
    host.memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))
}

fn read_usize(host: &OwnedHostApi, address: usize, label: &str) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

fn read_exact(
    host: &OwnedHostApi,
    address: usize,
    out: &mut [u8],
    label: &str,
) -> Result<(), String> {
    host.memory()
        .read(address, out)
        .map_err(|error| format!("{label} read failed address=0x{address:x}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AddressSpec, Scan, ValueType};

    #[test]
    fn scans_aligned_values() {
        let scan = Scan {
            id: "test".to_string(),
            value_type: ValueType::U16,
            start: AddressSpec::Absolute(0),
            bytes: 8,
            values: vec![TargetValue::Integer(42)],
            max_hits: 8,
        };
        let bytes = [1, 0, 42, 0, 42, 0, 7, 0];

        let hits = scan_bytes(0x1000, &bytes, &scan);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].address, 0x1002);
        assert_eq!(hits[1].address, 0x1004);
    }
}
