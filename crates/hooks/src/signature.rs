#[derive(Clone, Copy, Debug)]
pub struct Signature {
    pub name: &'static str,
    pub pattern: &'static [u8],
    pub mask: &'static [u8],
}

impl Signature {
    pub const fn new(name: &'static str, pattern: &'static [u8], mask: &'static [u8]) -> Self {
        Self {
            name,
            pattern,
            mask,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        if self.pattern.is_empty() {
            return Err(format!("signature {} is empty", self.name));
        }
        if self.pattern.len() != self.mask.len() {
            return Err(format!(
                "signature {} pattern/mask mismatch pattern={} mask={}",
                self.name,
                self.pattern.len(),
                self.mask.len()
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SignatureScanner;

impl SignatureScanner {
    pub fn new() -> Self {
        Self
    }

    pub fn scan(self, signature: Signature) -> Result<usize, String> {
        signature.validate()?;
        let site = unsafe {
            crate::memory::scan_memory(
                signature.pattern.as_ptr(),
                signature.mask.as_ptr(),
                signature.pattern.len(),
            )
        };
        if site == 0 {
            Err(format!("signature {} not found", signature.name))
        } else {
            Ok(site)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_mask_length() {
        let signature = Signature::new("bad", &[1, 2], &[1]);
        assert!(signature.validate().is_err());
    }
}
