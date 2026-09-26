use memmap2::Mmap;
use std::sync::Arc;

/// Backing byte storage for a parsed [`crate::Abf`], shared (via `Arc`) by
/// every [`crate::channel::Channel`] so sample data can be decoded lazily,
/// straight out of the mapping, instead of being copied onto the heap at
/// open time.
pub(crate) enum Storage {
    /// Backs [`crate::Abf::from_file`].
    Mmap(Mmap),
    /// Backs [`crate::Abf::from_bytes`]: caller-supplied bytes already
    /// resident in memory, with no `unsafe` and no file I/O.
    Owned(Arc<[u8]>),
}

impl Storage {
    pub(crate) fn bytes(&self) -> &[u8] {
        match self {
            Storage::Mmap(mmap) => &mmap[..],
            Storage::Owned(bytes) => bytes,
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

    #[test]
    fn owned_bytes_returns_the_owned_content() {
        let storage = Storage::Owned(Arc::from(b"hello owned".as_slice()));
        assert_eq!(storage.bytes(), b"hello owned");
    }
}
