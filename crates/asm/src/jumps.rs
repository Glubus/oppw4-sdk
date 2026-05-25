use crate::Rel32Patch;

pub fn emit_abs_jmp(code: &mut Vec<u8>, target: usize) {
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xe0]);
}

pub fn emit_abs_jmp_preserve_rax(code: &mut Vec<u8>, target: usize) {
    code.push(0x50);
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0x48, 0x87, 0x04, 0x24, 0xc3]);
}

pub fn emit_mov_r9_rsp_deref(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x4c, 0x8b, 0x0c, 0x24]);
}

pub fn emit_abs_jmp_r11(code: &mut Vec<u8>, target: usize) {
    code.extend_from_slice(&[0x49, 0xbb]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0x41, 0xff, 0xe3]);
}

pub fn emit_jmp(code: &mut Vec<u8>) -> Rel32Patch {
    let instruction_offset = code.len();
    code.push(0xe9);
    let immediate_offset = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    Rel32Patch {
        instruction_offset,
        immediate_offset,
        instruction_len: 5,
    }
}

pub fn emit_jz(code: &mut Vec<u8>) -> Rel32Patch {
    emit_jcc(code, &[0x0f, 0x84])
}

pub fn emit_jne(code: &mut Vec<u8>) -> Rel32Patch {
    emit_jcc(code, &[0x0f, 0x85])
}

pub fn emit_jbe(code: &mut Vec<u8>) -> Rel32Patch {
    emit_jcc(code, &[0x0f, 0x86])
}

fn emit_jcc(code: &mut Vec<u8>, opcode: &[u8]) -> Rel32Patch {
    let instruction_offset = code.len();
    code.extend_from_slice(opcode);
    let immediate_offset = code.len();
    code.extend_from_slice(&0i32.to_le_bytes());
    Rel32Patch {
        instruction_offset,
        immediate_offset,
        instruction_len: opcode.len() + 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_absolute_jump() {
        let mut code = Vec::new();
        emit_abs_jmp(&mut code, 0x1122_3344_5566_7788);
        assert_eq!(&code[..2], &[0x48, 0xb8]);
        assert_eq!(&code[10..], &[0xff, 0xe0]);
    }

    #[test]
    fn emits_absolute_jump_preserving_rax() {
        let mut code = Vec::new();
        emit_abs_jmp_preserve_rax(&mut code, 0x1122_3344_5566_7788);
        assert_eq!(code[0], 0x50);
        assert_eq!(&code[1..3], &[0x48, 0xb8]);
        assert_eq!(&code[11..], &[0x48, 0x87, 0x04, 0x24, 0xc3]);
    }

    #[test]
    fn emits_return_address_capture_to_r9() {
        let mut code = Vec::new();
        emit_mov_r9_rsp_deref(&mut code);
        assert_eq!(code, [0x4c, 0x8b, 0x0c, 0x24]);
    }

    #[test]
    fn emits_absolute_r11_jump() {
        let mut code = Vec::new();
        emit_abs_jmp_r11(&mut code, 0x1122_3344_5566_7788);
        assert_eq!(&code[..2], &[0x49, 0xbb]);
        assert_eq!(&code[10..], &[0x41, 0xff, 0xe3]);
    }

    #[test]
    fn emits_rel32_conditional_jump_placeholder() {
        let mut code = Vec::new();
        let patch = emit_jne(&mut code);
        assert_eq!(code, [0x0f, 0x85, 0, 0, 0, 0]);
        assert_eq!(patch.immediate_offset, 2);
        assert_eq!(patch.instruction_len, 6);
    }
}
