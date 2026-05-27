use plugin_abi::Oppw4PluginApi;

use crate::{api::r#unsafe, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct MemoryService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> MemoryService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn module_base(self) -> PluginResult<usize> {
        let module_base = self
            .abi
            .module_base
            .ok_or(PluginError::MissingHostFunction("module_base"))?;
        Ok(r#unsafe::module_base(self.abi.host_context, module_base))
    }

    pub fn read(self, address: usize, out: &mut [u8]) -> PluginResult<()> {
        let read = self
            .abi
            .read_memory
            .ok_or(PluginError::MissingHostFunction("read_memory"))?;
        let code = r#unsafe::read_memory(self.abi.host_context, read, address, out);
        host_code_result("read_memory", code)
    }

    pub fn write(self, address: usize, bytes: &[u8]) -> PluginResult<()> {
        let write = self
            .abi
            .write_memory
            .ok_or(PluginError::MissingHostFunction("write_memory"))?;
        let code = r#unsafe::write_memory(self.abi.host_context, write, address, bytes);
        host_code_result("write_memory", code)
    }

    pub fn scan(self, pattern: &[u8], mask: &[u8]) -> PluginResult<usize> {
        if pattern.len() != mask.len() {
            return Err(PluginError::HostCallFailed {
                operation: "scan_memory",
                code: -2,
            });
        }
        let scan = self
            .abi
            .scan_memory
            .ok_or(PluginError::MissingHostFunction("scan_memory"))?;
        Ok(r#unsafe::scan_memory(
            self.abi.host_context,
            scan,
            pattern,
            mask,
        ))
    }
}

fn host_code_result(operation: &'static str, code: i32) -> PluginResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(PluginError::HostCallFailed { operation, code })
    }
}
