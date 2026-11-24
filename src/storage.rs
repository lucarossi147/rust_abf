use crate::byte_reader::ByteReader;
use std::fs::File;
use std::io;
use std::sync::Arc;

#[cfg(not(any(unix, windows)))]
compile_error!(
    "rust_abf: from_file needs positional file reads (unix or windows); use Abf::from_bytes"
);

/// Largest chunk [`Storage::for_each_chunk`] hands out: the size of the
/// stack buffer file reads go through, so reading never allocates.
pub(crate) const CHUNK_BYTES: usize = 256 * 1024;

/// The ABF2 header block, which holds the section map.
const HEADER_BLOCK: u64 = 512;
/// Section-map entries of the sections the parser reads: protocol, ADC,
/// DAC, strings, synch array.
const PARSED_SECTIONS: [usize; 5] = [76, 92, 108, 220, 316];
/// Sections are read whole at open; a corrupt size can't make us read more.
const MAX_SECTION_BYTES: u64 = 64 << 20;

/// Backing byte storage for a parsed [`crate::Abf`], shared (via `Arc`) by
/// every [`crate::channel::Channel`] so sample data is read and decoded
/// lazily instead of being copied onto the heap at open time.
pub(crate) enum Storage {
    /// A file read on demand with positional reads. `header` holds the
    /// header block and every parsed section, keyed by file offset; sample
    /// data is never cached.
    File {
        file: File,
        len: u64,
        header: Vec<(usize, Vec<u8>)>,
    },
    Owned(Arc<[u8]>),
}

impl Storage {
    /// Reads the header block and the sections the parser needs, wherever
    /// they are in the file. Ranges past the end of the file are cut short,
    /// so the parser reports them as truncated.
    pub(crate) fn open(file: File) -> io::Result<Storage> {
        let len = file.metadata()?.len();
        let read = |start: u64, size: u64| -> io::Result<Vec<u8>> {
            let size = size.min(len.saturating_sub(start)).min(MAX_SECTION_BYTES);
            let mut buf = vec![0; size as usize];
            read_exact_at(&file, &mut buf, start)?;
            Ok(buf)
        };
        let block = read(0, HEADER_BLOCK)?;
        let mut header = Vec::new();
        for map in PARSED_SECTIONS {
            let Some((start, size)) = section_range(&block, map) else {
                continue;
            };
            if let Ok(at) = usize::try_from(start) {
                header.push((at, read(start, size)?));
            }
        }
        header.push((0, block));
        Ok(Storage::File { file, len, header })
    }

    /// A reader over the header and section bytes.
    pub(crate) fn header(&self) -> ByteReader<'_> {
        match self {
            Storage::File { len, header, .. } => ByteReader::pieces(header, *len),
            Storage::Owned(bytes) => ByteReader::new(bytes),
        }
    }

    /// Calls `f` on consecutive chunks of `start..start + len`, each
    /// `chunk_len` bytes long except possibly the last. Fails with
    /// `UnexpectedEof` if the range runs past the end of the data, and for
    /// a file if `chunk_len` exceeds [`CHUNK_BYTES`].
    pub(crate) fn for_each_chunk(
        &self,
        start: usize,
        len: usize,
        chunk_len: usize,
        mut f: impl FnMut(&[u8]),
    ) -> io::Result<()> {
        let eof = || io::Error::from(io::ErrorKind::UnexpectedEof);
        let end = start.checked_add(len).ok_or_else(eof)?;
        let chunk_len = chunk_len.max(1);
        match self {
            Storage::Owned(bytes) => {
                let bytes = bytes.get(start..end).ok_or_else(eof)?;
                bytes.chunks(chunk_len).for_each(f);
            }
            Storage::File { file, .. } => {
                let mut scratch = [0u8; CHUNK_BYTES];
                let mut at = start;
                while at < end {
                    let n = chunk_len.min(end - at);
                    let buf = scratch
                        .get_mut(..n)
                        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
                    read_exact_at(file, buf, at as u64)?;
                    f(buf);
                    at += n;
                }
            }
        }
        Ok(())
    }
}

/// `(start, size)` in bytes of the section whose map entry (`u32` block,
/// `u32` item size, `i64` item count) is at `map` in the header block.
fn section_range(block: &[u8], map: usize) -> Option<(u64, u64)> {
    let entry = block.get(map..map + 16)?;
    let field = |at: usize| {
        entry
            .get(at..at + 4)?
            .try_into()
            .ok()
            .map(u32::from_le_bytes)
    };
    let count = u64::try_from(i64::from_le_bytes(entry.get(8..16)?.try_into().ok()?)).ok()?;
    let start = u64::from(field(0)?).checked_mul(HEADER_BLOCK)?;
    Some((start, u64::from(field(4)?).checked_mul(count)?))
}

/// Fills `buf` from `file` at `offset`; the only place that touches
/// platform file APIs.
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    #[cfg(unix)]
    use std::os::unix::fs::FileExt;
    #[cfg(windows)]
    use std::os::windows::fs::FileExt;
    #[cfg(unix)]
    let read = |b: &mut [u8], at| file.read_at(b, at);
    #[cfg(windows)]
    let read = |b: &mut [u8], at| file.seek_read(b, at);
    read_exact_at_with(read, buf, offset)
}

/// Calls `read(buf, offset)` until `buf` is full: short reads continue,
/// `Interrupted` retries, `Ok(0)` is `UnexpectedEof`.
fn read_exact_at_with(
    mut read: impl FnMut(&mut [u8], u64) -> io::Result<usize>,
    mut buf: &mut [u8],
    mut offset: u64,
) -> io::Result<()> {
    while !buf.is_empty() {
        match read(buf, offset) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(n) => {
                buf = std::mem::take(&mut buf).get_mut(n..).unwrap_or_default();
                offset = offset.saturating_add(n as u64);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
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
