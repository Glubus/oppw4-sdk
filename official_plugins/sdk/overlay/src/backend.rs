use std::fmt;

use crate::config::OverlayBackend;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RendererProbe {
    pub(crate) requested: OverlayBackend,
    pub(crate) dxgi_loaded: bool,
    pub(crate) d3d11_loaded: bool,
    pub(crate) ready: bool,
    pub(crate) note: &'static str,
}

impl RendererProbe {
    pub(crate) fn detect(requested: OverlayBackend) -> Self {
        let dxgi_loaded = module_loaded("dxgi.dll");
        let d3d11_loaded = module_loaded("d3d11.dll");
        let candidate = matches!(requested, OverlayBackend::Auto | OverlayBackend::Dxgi)
            && dxgi_loaded
            && d3d11_loaded;
        Self {
            requested,
            dxgi_loaded,
            d3d11_loaded,
            ready: false,
            note: if candidate {
                "renderer candidate found; present hook backend not implemented yet"
            } else {
                "waiting for dxgi/d3d11 modules"
            },
        }
    }
}

impl fmt::Display for RendererProbe {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            out,
            "backend={:?} dxgi_loaded={} d3d11_loaded={} ready={} note={}",
            self.requested, self.dxgi_loaded, self.d3d11_loaded, self.ready, self.note
        )
    }
}

#[cfg(windows)]
fn module_loaded(name: &str) -> bool {
    use std::ffi::CString;

    let Ok(name) = CString::new(name) else {
        return false;
    };
    unsafe { !GetModuleHandleA(name.as_ptr()).is_null() }
}

#[cfg(not(windows))]
fn module_loaded(_name: &str) -> bool {
    false
}

#[cfg(windows)]
extern "system" {
    fn GetModuleHandleA(name: *const i8) -> *mut core::ffi::c_void;
}
