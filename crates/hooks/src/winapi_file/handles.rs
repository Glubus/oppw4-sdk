use super::types::{Handle, FAKE_HANDLE_BITS, FAKE_HANDLE_MASK};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct VirtualHandle(u64);

impl VirtualHandle {
    pub(crate) fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub(crate) fn as_raw(self) -> u64 {
        self.0
    }
}

pub(crate) fn handle_to_fake(handle: VirtualHandle) -> Handle {
    (FAKE_HANDLE_BITS | handle.as_raw() as usize) as Handle
}

pub(crate) fn returned_virtual_handle(handle: VirtualHandle) -> Handle {
    handle_to_fake(handle)
}

pub(crate) fn fake_to_handle(handle: Handle) -> Option<VirtualHandle> {
    let raw = handle as usize;
    if raw & FAKE_HANDLE_MASK != FAKE_HANDLE_BITS {
        return None;
    }
    Some(VirtualHandle::from_raw((raw & !FAKE_HANDLE_MASK) as u64))
}

pub(crate) fn virtual_handle_for_os_handle(handle: Handle) -> Option<VirtualHandle> {
    fake_to_handle(handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::winapi_file::types::INVALID_HANDLE_VALUE;

    #[test]
    fn fake_handle_round_trip_preserves_id() {
        let handle = VirtualHandle::from_raw(42);

        let round_trip = fake_to_handle(handle_to_fake(handle));

        assert_eq!(round_trip.map(VirtualHandle::as_raw), Some(42));
    }

    #[test]
    fn normal_handles_are_not_virtual() {
        assert_eq!(fake_to_handle(std::ptr::null_mut()), None);
        assert_eq!(fake_to_handle(INVALID_HANDLE_VALUE), None);
    }

    #[test]
    fn virtual_open_returns_fake_handle_value() {
        let virtual_handle = VirtualHandle::from_raw(0x21);

        let returned = returned_virtual_handle(virtual_handle);

        assert_eq!(
            fake_to_handle(returned).map(VirtualHandle::as_raw),
            Some(0x21)
        );
    }
}
