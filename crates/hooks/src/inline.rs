use std::{ffi::c_void, ptr};

use crate::{memory, signature::Signature, SignatureScanner};

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;

#[derive(Clone, Copy, Debug)]
pub struct InlineHook {
    pub site: usize,
    pub trampoline: usize,
    pub overwrite_len: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct HookBuilder {
    signature: Signature,
    overwrite_len: usize,
}

impl HookBuilder {
    pub fn new(signature: Signature) -> Self {
        Self {
            signature,
            overwrite_len: signature.pattern.len(),
        }
    }

    pub fn overwrite_len(mut self, overwrite_len: usize) -> Self {
        self.overwrite_len = overwrite_len;
        self
    }

    pub fn scan(self) -> Result<ScannedHookBuilder, String> {
        let site = SignatureScanner::new().scan(self.signature)?;
        Ok(ScannedHookBuilder {
            signature: self.signature,
            site,
            overwrite_len: self.overwrite_len,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ScannedHookBuilder {
    signature: Signature,
    site: usize,
    overwrite_len: usize,
}

impl ScannedHookBuilder {
    pub fn site(self) -> usize {
        self.site
    }

    pub unsafe fn install_abs_jump(self, detour: usize) -> Result<InlineHook, String> {
        self.install_abs_jump_with_options(detour, TrampolineJump::ClobberRax)
    }

    pub unsafe fn install_abs_jump_preserve_rax(self, detour: usize) -> Result<InlineHook, String> {
        self.install_abs_jump_with_options(detour, TrampolineJump::PreserveRax)
    }

    pub unsafe fn install_abs_jump_with_return_address(
        self,
        detour: usize,
    ) -> Result<InlineHook, String> {
        let mut entry = Vec::with_capacity(17);
        asm::emit_mov_r9_rsp_deref(&mut entry);
        asm::emit_abs_jmp_r11(&mut entry, detour);
        let entry = allocate_executable(&entry)?;
        self.install_abs_jump_with_options(entry, TrampolineJump::PreserveRax)
    }

    unsafe fn install_abs_jump_with_options(
        self,
        detour: usize,
        trampoline_jump: TrampolineJump,
    ) -> Result<InlineHook, String> {
        if self.overwrite_len < 12 {
            return Err(format!(
                "hook {} overwrite_len={} cannot fit absolute jump",
                self.signature.name, self.overwrite_len
            ));
        }
        let original = read_original(self.site, self.overwrite_len)?;
        let mut trampoline_code = original;
        trampoline_jump.emit(&mut trampoline_code, self.site + self.overwrite_len);
        let trampoline = allocate_executable(&trampoline_code)?;

        let mut patch = Vec::with_capacity(self.overwrite_len);
        asm::emit_abs_jmp(&mut patch, detour);
        patch.resize(self.overwrite_len, 0x90);
        write_patch(self.site, &patch)?;

        Ok(InlineHook {
            site: self.site,
            trampoline,
            overwrite_len: self.overwrite_len,
        })
    }

    pub unsafe fn install_rel_jump(self, cave: usize) -> Result<(), String> {
        if self.overwrite_len < 5 {
            return Err(format!(
                "hook {} overwrite_len={} cannot fit rel32 jump",
                self.signature.name, self.overwrite_len
            ));
        }
        let mut patch = vec![0x90; self.overwrite_len];
        asm::write_rel32_jump(&mut patch, self.site, 5, cave)?;
        write_patch(self.site, &patch)
    }
}

#[derive(Clone, Copy, Debug)]
enum TrampolineJump {
    ClobberRax,
    PreserveRax,
}

impl TrampolineJump {
    fn emit(self, code: &mut Vec<u8>, target: usize) {
        match self {
            Self::ClobberRax => asm::emit_abs_jmp(code, target),
            Self::PreserveRax => asm::emit_abs_jmp_preserve_rax(code, target),
        }
    }
}

fn read_original(site: usize, len: usize) -> Result<Vec<u8>, String> {
    let mut original = vec![0u8; len];
    let result = unsafe { memory::read_memory(site, original.as_mut_ptr(), original.len()) };
    if result == 0 {
        Ok(original)
    } else {
        Err(format!(
            "read_memory failed site=0x{site:x} result={result}"
        ))
    }
}

fn write_patch(site: usize, patch: &[u8]) -> Result<(), String> {
    let result = unsafe { memory::write_memory(site, patch.as_ptr(), patch.len()) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!(
            "write_memory failed site=0x{site:x} result={result}"
        ))
    }
}

unsafe fn allocate_executable(bytes: &[u8]) -> Result<usize, String> {
    let memory = VirtualAlloc(
        ptr::null_mut(),
        bytes.len(),
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    );
    if memory.is_null() {
        return Err("VirtualAlloc failed for inline hook trampoline".to_string());
    }
    ptr::copy_nonoverlapping(bytes.as_ptr(), memory.cast::<u8>(), bytes.len());
    Ok(memory as usize)
}

extern "system" {
    fn VirtualAlloc(
        address: *mut c_void,
        size: usize,
        allocation_type: u32,
        protect: u32,
    ) -> *mut c_void;
}
