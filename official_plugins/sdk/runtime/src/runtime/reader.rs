use plugin_sdk::OwnedHostApi;

pub(crate) fn read_u8(host: &OwnedHostApi, address: usize, label: &str) -> Result<u8, String> {
    let mut bytes = [0u8; 1];
    read_exact(host, address, &mut bytes, label)?;
    Ok(bytes[0])
}

pub(crate) fn read_u16(host: &OwnedHostApi, address: usize, label: &str) -> Result<u16, String> {
    let mut bytes = [0u8; 2];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u16::from_le_bytes(bytes))
}

pub(crate) fn read_u32(host: &OwnedHostApi, address: usize, label: &str) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u32::from_le_bytes(bytes))
}

pub(crate) fn read_f32(host: &OwnedHostApi, address: usize, label: &str) -> Result<f32, String> {
    let mut bytes = [0u8; 4];
    read_exact(host, address, &mut bytes, label)?;
    Ok(f32::from_le_bytes(bytes))
}

pub(crate) fn read_usize(
    host: &OwnedHostApi,
    address: usize,
    label: &str,
) -> Result<usize, String> {
    let mut bytes = [0u8; 8];
    read_exact(host, address, &mut bytes, label)?;
    Ok(u64::from_le_bytes(bytes) as usize)
}

pub(crate) fn read_u16_block<const N: usize>(
    host: &OwnedHostApi,
    address: usize,
    label: &str,
) -> Result<[u16; N], String> {
    let mut bytes = vec![0u8; N * size_of::<u16>()];
    read_exact(host, address, &mut bytes, label)?;

    let mut values = [0u16; N];
    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
        values[index] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }
    Ok(values)
}

pub(crate) fn read_exact(
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
    use super::super::probe::snapshot_interval;
    use std::time::Duration;

    #[test]
    fn snapshot_interval_zero_disables_periodic_logging() {
        assert_eq!(snapshot_interval(0), None);
    }

    #[test]
    fn snapshot_interval_has_minimum_for_log_safety() {
        assert_eq!(snapshot_interval(1), Some(Duration::from_millis(250)));
        assert_eq!(snapshot_interval(1000), Some(Duration::from_millis(1000)));
    }
}
