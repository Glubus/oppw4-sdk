pub fn emit_mov_dword_rdi_disp_f32(code: &mut Vec<u8>, disp: u32, value: f32) {
    code.extend_from_slice(&[0xc7, 0x87]);
    code.extend_from_slice(&disp.to_le_bytes());
    code.extend_from_slice(&value.to_bits().to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_rdi_displacement_float_write() {
        let mut code = Vec::new();
        emit_mov_dword_rdi_disp_f32(&mut code, 0x2e8, 2.0);
        assert_eq!(&code[..2], &[0xc7, 0x87]);
        assert_eq!(u32::from_le_bytes(code[2..6].try_into().unwrap()), 0x2e8);
        assert_eq!(
            u32::from_le_bytes(code[6..10].try_into().unwrap()),
            2.0f32.to_bits()
        );
    }
}
