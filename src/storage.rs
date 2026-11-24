use crate::byte_reader::ByteReader;
use memmap2::Mmap;
use std::io;
use std::sync::Arc;

/// Largest chunk [`Storage::for_each_chunk`] hands out.
pub(crate) const CHUNK_BYTES: usize = 64 * 1024;

/// Backing byte storage for a parsed [`crate::Abf`], shared (via `Arc`) by
/// every [`crate::channel::Channel`] so sample data can be decoded lazily,
/// straight out of the mapping (or buffer), instead of being copied onto the
/// heap at open time.
pub(crate) enum Storage {
    Mmap(Mmap),
    Owned(Arc<[u8]>),
}

impl Storage {
    /// A reader over the header and section bytes.
    pub(crate) fn header(&self) -> ByteReader<'_> {
        ByteReader::new(self.slice())
    }

    fn slice(&self) -> &[u8] {
        match self {
            Storage::Mmap(mmap) => &mmap[..],
            Storage::Owned(bytes) => bytes,
        }
    }

    /// Calls `f` on consecutive chunks of `start..start + len`, each
    /// `chunk_len` bytes long except possibly the last. Fails with
    /// `UnexpectedEof` if the range runs past the end of the data.
    pub(crate) fn for_each_chunk(
        &self,
        start: usize,
        len: usize,
        chunk_len: usize,
        f: impl FnMut(&[u8]),
    ) -> io::Result<()> {
        let eof = || io::Error::from(io::ErrorKind::UnexpectedEof);
        let end = start.checked_add(len).ok_or_else(eof)?;
        let bytes = self.slice().get(start..end).ok_or_else(eof)?;
        bytes.chunks(chunk_len.max(1)).for_each(f);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    /// A reader over `data` that returns at most `max` bytes per call.
    fn short_reads<'a>(
        data: &'a [u8],
        max: usize,
    ) -> impl FnMut(&mut [u8], u64) -> io::Result<usize> + 'a {
        move |buf, offset| {
            let rest = data.get(offset as usize..).unwrap_or(&[]);
            let n = buf.len().min(max).min(rest.len());
            buf[..n].copy_from_slice(&rest[..n]);
            Ok(n)
        }
    }

    #[test]
    fn read_exact_at_continues_after_short_reads() {
        let data: Vec<u8> = (0..20).collect();
        let mut buf = [0u8; 10];
        read_exact_at_with(short_reads(&data, 3), &mut buf, 5).unwrap();
        assert_eq!(buf, data[5..15]);
    }

    #[test]
    fn read_exact_at_retries_interrupted_reads() {
        let data: Vec<u8> = (0..8).collect();
        let mut inner = short_reads(&data, 8);
        let mut interrupted = false;
        let read = |buf: &mut [u8], offset| {
            if !interrupted {
                interrupted = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            inner(buf, offset)
        };
        let mut buf = [0u8; 8];
        read_exact_at_with(read, &mut buf, 0).unwrap();
        assert_eq!(buf, data[..]);
    }

    #[test]
    fn read_exact_at_reports_eof_when_a_read_returns_zero() {
        let data = [1u8, 2, 3];
        let mut buf = [0u8; 4];
        let err = read_exact_at_with(short_reads(&data, 8), &mut buf, 0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_exact_at_passes_other_errors_through() {
        let mut buf = [0u8; 4];
        let read = |_: &mut [u8], _| Err(io::ErrorKind::PermissionDenied.into());
        let err = read_exact_at_with(read, &mut buf, 0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    fn storages(data: &[u8]) -> (tempfile::NamedTempFile, [Storage; 2]) {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(data).unwrap();
        file.flush().unwrap();
        let on_disk = Storage::open(file.reopen().unwrap()).unwrap();
        (file, [on_disk, Storage::Owned(Arc::from(data))])
    }

    fn chunks(
        storage: &Storage,
        start: usize,
        len: usize,
        chunk_len: usize,
    ) -> io::Result<Vec<Vec<u8>>> {
        let mut out = Vec::new();
        storage.for_each_chunk(start, len, chunk_len, |c| out.push(c.to_vec()))?;
        Ok(out)
    }

    #[test]
    fn chunks_cover_the_range_in_order() {
        let data: Vec<u8> = (0..20).collect();
        let (_file, storages) = storages(&data);
        for storage in &storages {
            let got = chunks(storage, 3, 10, 4).unwrap();
            assert_eq!(got, [&data[3..7], &data[7..11], &data[11..13]]);
            assert!(chunks(storage, 5, 0, 4).unwrap().is_empty());
        }
    }

    #[test]
    fn ranges_past_the_end_are_unexpected_eof() {
        let data: Vec<u8> = (0..20).collect();
        let (_file, storages) = storages(&data);
        for storage in &storages {
            for (start, len) in [(15, 6), (20, 1), (usize::MAX, 2)] {
                let err = chunks(storage, start, len, 4).unwrap_err();
                assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof, "{start}+{len}");
            }
        }
    }

    #[test]
    fn chunks_larger_than_the_scratch_buffer_are_rejected() {
        let data = vec![0u8; CHUNK_BYTES + 2];
        let (_file, [on_disk, _]) = storages(&data);
        assert!(chunks(&on_disk, 0, data.len(), CHUNK_BYTES + 1).is_err());
    }
}
