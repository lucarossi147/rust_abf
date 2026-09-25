use memmap2::Mmap;

/// Backing byte storage for a parsed [`crate::Abf`], shared (via `Arc`) by
/// every [`crate::channel::Channel`] so sample data can be decoded lazily,
/// straight out of the mapping, instead of being copied onto the heap at
/// open time.
///
/// Only memory-mapped files are supported today; issue 14 adds an
/// `Owned(Vec<u8>)` variant for an `Abf::from_bytes` constructor.
pub(crate) enum Storage {
    Mmap(Mmap),
}

impl Storage {
    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Storage::Mmap(mmap) => &mmap[..],
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use memmap2::MmapOptions;
    use std::io::Write;

    #[test]
    fn bytes_returns_the_mapped_content() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"hello storage").unwrap();
        file.flush().unwrap();
        let mmap = unsafe { MmapOptions::new().map(file.as_file()).unwrap() };
        let storage = Storage::Mmap(mmap);
        assert_eq!(storage.bytes(), b"hello storage");
    }
}
