use crate::Rel32Patch;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AbsoluteJumpMode {
    ClobberRax,
    PreserveRax,
    UseR11,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Condition {
    Zero,
    NotZero,
    BelowOrEqual,
}

pub fn emit_absolute_jump(code: &mut Vec<u8>, target: usize, mode: AbsoluteJumpMode) {
    match mode {
        AbsoluteJumpMode::ClobberRax => emit_abs_jump_rax(code, target),
        AbsoluteJumpMode::PreserveRax => emit_abs_jump_preserve_rax(code, target),
        AbsoluteJumpMode::UseR11 => emit_abs_jump_r11(code, target),
    }
}

pub fn emit_return_address_to_r9(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x4c, 0x8b, 0x0c, 0x24]);
}

pub fn emit_jump(code: &mut Vec<u8>) -> Rel32Patch {
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

pub fn emit_conditional_jump(code: &mut Vec<u8>, condition: Condition) -> Rel32Patch {
    emit_jcc(code, condition.opcode())
}

fn emit_abs_jump_rax(code: &mut Vec<u8>, target: usize) {
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xe0]);
}

/// Emits `push rax; mov rax, target; xchg [rsp], rax; ret`.
///
/// The target address replaces the pushed return address and `rax` is restored
/// from the stack slot before `ret` transfers control.
fn emit_abs_jump_preserve_rax(code: &mut Vec<u8>, target: usize) {
    code.push(0x50);
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0x48, 0x87, 0x04, 0x24, 0xc3]);
}

fn emit_abs_jump_r11(code: &mut Vec<u8>, target: usize) {
    code.extend_from_slice(&[0x49, 0xbb]);
    code.extend_from_slice(&(target as u64).to_le_bytes());
    code.extend_from_slice(&[0x41, 0xff, 0xe3]);
}

impl Condition {
    fn opcode(self) -> &'static [u8] {
        match self {
            Self::Zero => &[0x0f, 0x84],
            Self::NotZero => &[0x0f, 0x85],
            Self::BelowOrEqual => &[0x0f, 0x86],
        }
    }
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
    fn emits_absolute_jump_clobbering_rax() {
        let mut code = Vec::new();
        emit_absolute_jump(
            &mut code,
            0x1122_3344_5566_7788,
            AbsoluteJumpMode::ClobberRax,
        );
        assert_eq!(&code[..2], &[0x48, 0xb8]);
        assert_eq!(&code[10..], &[0xff, 0xe0]);
    }

    #[test]
    fn emits_absolute_jump_preserving_rax() {
        let mut code = Vec::new();
        emit_absolute_jump(
            &mut code,
            0x1122_3344_5566_7788,
            AbsoluteJumpMode::PreserveRax,
        );
        assert_eq!(code[0], 0x50);
        assert_eq!(&code[1..3], &[0x48, 0xb8]);
        assert_eq!(&code[11..], &[0x48, 0x87, 0x04, 0x24, 0xc3]);
    }

    #[test]
    fn emits_return_address_capture_to_r9() {
        let mut code = Vec::new();
        emit_return_address_to_r9(&mut code);
        assert_eq!(code, [0x4c, 0x8b, 0x0c, 0x24]);
    }

    #[test]
    fn emits_absolute_jump_using_r11() {
        let mut code = Vec::new();
        emit_absolute_jump(&mut code, 0x1122_3344_5566_7788, AbsoluteJumpMode::UseR11);
        assert_eq!(&code[..2], &[0x49, 0xbb]);
        assert_eq!(&code[10..], &[0x41, 0xff, 0xe3]);
    }

    #[test]
    fn emits_rel32_conditional_jump_placeholder() {
        let mut code = Vec::new();
        let patch = emit_conditional_jump(&mut code, Condition::NotZero);
        assert_eq!(code, [0x0f, 0x85, 0, 0, 0, 0]);
        assert_eq!(patch.immediate_offset, 2);
        assert_eq!(patch.instruction_len, 6);
    }
}
