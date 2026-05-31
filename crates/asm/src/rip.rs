use crate::Disp32Patch;

pub fn emit_cmp_rip_u32(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0x83, 0x3d]);
    emit_disp32_placeholder(code);
    code.push(0x00);
    Disp32Patch {
        offset: code.len() - 5,
    }
}

pub fn emit_inc_rip_u32(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0xff, 0x05]);
    emit_disp32_placeholder(code)
}

pub fn emit_cmp_edx_rip(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0x3b, 0x15]);
    emit_disp32_placeholder(code)
}

pub fn emit_movss_xmm2_rip(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0xf3, 0x0f, 0x10, 0x15]);
    emit_disp32_placeholder(code)
}

pub fn emit_movss_xmm0_rip(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0xf3, 0x0f, 0x10, 0x05]);
    emit_disp32_placeholder(code)
}

pub fn emit_movss_rip_xmm2(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0xf3, 0x0f, 0x11, 0x15]);
    emit_disp32_placeholder(code)
}

pub fn emit_movss_rip_xmm0(code: &mut Vec<u8>) -> Disp32Patch {
    code.extend_from_slice(&[0xf3, 0x0f, 0x11, 0x05]);
    emit_disp32_placeholder(code)
}

fn emit_disp32_placeholder(code: &mut Vec<u8>) -> Disp32Patch {
    let offset = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    Disp32Patch { offset }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmp_rip_returns_displacement_offset() {
        let mut code = Vec::new();
        let disp = emit_cmp_rip_u32(&mut code);
        assert_eq!(disp.offset, 2);
        assert_eq!(code, [0x83, 0x3d, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn movss_rip_helpers_return_displacement_offset() {
        let mut code = Vec::new();
        let disp = emit_movss_xmm0_rip(&mut code);
        assert_eq!(disp.offset, 4);
        assert_eq!(&code[..4], &[0xf3, 0x0f, 0x10, 0x05]);
    }
}
