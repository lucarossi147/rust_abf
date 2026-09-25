use crate::error::AbfError;

/// A bounds-checked little-endian reader over a borrowed byte buffer.
///
/// Every read validates that `offset..offset+len` fits within the buffer
/// before touching it, so a malformed or truncated file can only ever
/// produce an [`AbfError`], never an out-of-bounds panic.
#[derive(Clone, Copy)]
pub struct ByteReader<'a> {
    bytes: &'a [u8],
}

impl<'a> ByteReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn slice(
        &self,
        section: &'static str,
        offset: usize,
        len: usize,
    ) -> Result<&'a [u8], AbfError> {
        let end = offset
            .checked_add(len)
            .ok_or(AbfError::Truncated { section, offset })?;
        self.bytes
            .get(offset..end)
            .ok_or(AbfError::Truncated { section, offset })
    }

    pub fn read_u8(&self, section: &'static str, offset: usize) -> Result<u8, AbfError> {
        let s = self.slice(section, offset, 1)?;
        s.first()
            .copied()
            .ok_or(AbfError::Truncated { section, offset })
    }

    pub fn read_i8(&self, section: &'static str, offset: usize) -> Result<i8, AbfError> {
        self.read_u8(section, offset).map(|b| b as i8)
    }

    pub fn read_u16(&self, section: &'static str, offset: usize) -> Result<u16, AbfError> {
        let s = self.slice(section, offset, 2)?;
        let bytes: [u8; 2] = s
            .try_into()
            .map_err(|_| AbfError::Truncated { section, offset })?;
        Ok(u16::from_le_bytes(bytes))
    }

    pub fn read_i16(&self, section: &'static str, offset: usize) -> Result<i16, AbfError> {
        let s = self.slice(section, offset, 2)?;
        let bytes: [u8; 2] = s
            .try_into()
            .map_err(|_| AbfError::Truncated { section, offset })?;
        Ok(i16::from_le_bytes(bytes))
    }

    pub fn read_u32(&self, section: &'static str, offset: usize) -> Result<u32, AbfError> {
        let s = self.slice(section, offset, 4)?;
        let bytes: [u8; 4] = s
            .try_into()
            .map_err(|_| AbfError::Truncated { section, offset })?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub fn read_i32(&self, section: &'static str, offset: usize) -> Result<i32, AbfError> {
        let s = self.slice(section, offset, 4)?;
        let bytes: [u8; 4] = s
            .try_into()
            .map_err(|_| AbfError::Truncated { section, offset })?;
        Ok(i32::from_le_bytes(bytes))
    }

    pub fn read_f32(&self, section: &'static str, offset: usize) -> Result<f32, AbfError> {
        let s = self.slice(section, offset, 4)?;
        let bytes: [u8; 4] = s
            .try_into()
            .map_err(|_| AbfError::Truncated { section, offset })?;
        Ok(f32::from_le_bytes(bytes))
    }

    pub fn read_str(
        &self,
        section: &'static str,
        offset: usize,
        len: usize,
    ) -> Result<&'a str, AbfError> {
        let s = self.slice(section, offset, len)?;
        std::str::from_utf8(s).map_err(|_| AbfError::InvalidSection {
            section,
            reason: "not valid UTF-8".to_string(),
        })
    }

    pub fn read_bytes(
        &self,
        section: &'static str,
        offset: usize,
        len: usize,
    ) -> Result<&'a [u8], AbfError> {
        self.slice(section, offset, len)
    }
}

/// Adds `extra` to `base`, mapping overflow to a [`AbfError::Truncated`] for `section`.
pub fn checked_offset(base: usize, extra: usize, section: &'static str) -> Result<usize, AbfError> {
    base.checked_add(extra).ok_or(AbfError::Truncated {
        section,
        offset: base,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn reads_little_endian_values_in_bounds() {
        let bytes = [0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0xC0, 0x3F];
        let reader = ByteReader::new(&bytes);
        assert_eq!(reader.read_u16("t", 0).unwrap(), 1);
        assert_eq!(reader.read_u32("t", 2).unwrap(), 2);
        assert_eq!(reader.read_f32("t", 4).unwrap(), 1.5);
        assert!(matches!(
            reader.read_f32("t", 6),
            Err(AbfError::Truncated {
                section: "t",
                offset: 6
            })
        ));
    }

    #[test]
    fn out_of_bounds_reads_error_instead_of_panicking() {
        let bytes = [0xAAu8; 3];
        let reader = ByteReader::new(&bytes);
        assert!(matches!(
            reader.read_u32("sig", 0),
            Err(AbfError::Truncated {
                section: "sig",
                offset: 0
            })
        ));
        assert!(matches!(
            reader.read_u8("sig", 10),
            Err(AbfError::Truncated {
                section: "sig",
                offset: 10
            })
        ));
    }

    #[test]
    fn offset_overflow_errors_instead_of_panicking() {
        let bytes = [0u8; 4];
        let reader = ByteReader::new(&bytes);
        assert!(matches!(
            reader.read_u32("sig", usize::MAX),
            Err(AbfError::Truncated {
                section: "sig",
                offset: usize::MAX
            })
        ));
    }

    #[test]
    fn read_str_rejects_invalid_utf8() {
        let bytes = [0xFF, 0xFE, 0xFD, 0xFC];
        let reader = ByteReader::new(&bytes);
        assert!(matches!(
            reader.read_str("sig", 0, 4),
            Err(AbfError::InvalidSection { section: "sig", .. })
        ));
    }

    #[test]
    fn checked_offset_detects_overflow() {
        assert_eq!(checked_offset(1, 2, "t").unwrap(), 3);
        assert!(matches!(
            checked_offset(usize::MAX, 1, "t"),
            Err(AbfError::Truncated { section: "t", .. })
        ));
    }
}
