//! Optional multi-threaded decoding, enabled by the `parallel` Cargo feature.
//!
//! Only the *allocating* read paths ([`crate::Channel::sweep`],
//! [`crate::Channel::raw_sweep`], [`crate::Abf::sweep_all_channels`]) go
//! through here. [`crate::Channel::read_sweep_into`] and
//! [`crate::Abf::read_sweep_all_channels_into`] always decode on the calling
//! thread, because rayon's thread pool allocates on first use and may grow
//! its work queues later, which would break their zero-allocation guarantee.
//!
//! Without the feature, [`split_maybe_par`] reads on the calling thread.

/// Sweeps shorter than this many samples are always decoded on the calling
/// thread: below it, handing work to the pool costs more than it saves
/// (measured: at 250k samples a parallel decode into a freshly allocated
/// buffer was slower than a sequential one; from ~1M samples up it wins).
#[cfg(feature = "parallel")]
pub(crate) const PAR_THRESHOLD: usize = 1024 * 1024;

/// Samples per parallel work item.
#[cfg(feature = "parallel")]
const PAR_CHUNK: usize = 32 * 1024;

/// Runs `read(first, out)`, which fills `out` with samples `first..`, split
/// into pieces read on rayon's global thread pool when the `parallel`
/// feature is enabled and `out` is at least [`PAR_THRESHOLD`] long.
#[cfg(feature = "parallel")]
pub(crate) fn split_maybe_par<T, F>(first: usize, out: &mut [T], read: F) -> std::io::Result<()>
where
    T: Send,
    F: Fn(usize, &mut [T]) -> std::io::Result<()> + Sync,
{
    use rayon::prelude::*;
    if out.len() < PAR_THRESHOLD {
        return read(first, out);
    }
    out.par_chunks_mut(PAR_CHUNK)
        .enumerate()
        .try_for_each(|(k, piece)| read(first.saturating_add(k * PAR_CHUNK), piece))
}

/// Sequential fallback used when the `parallel` feature is disabled.
#[cfg(not(feature = "parallel"))]
pub(crate) fn split_maybe_par<T, F>(first: usize, out: &mut [T], read: F) -> std::io::Result<()>
where
    F: Fn(usize, &mut [T]) -> std::io::Result<()>,
{
    read(first, out)
}

/// Length, in frames, of the ranges a single-pass multi-channel sweep decode
/// is split into for parallel decoding; `frames` itself (i.e. no split) when
/// the sweep is short or the `parallel` feature is disabled.
#[cfg(feature = "parallel")]
pub(crate) fn frame_range_len(frames: usize) -> usize {
    if frames < PAR_THRESHOLD {
        frames
    } else {
        PAR_CHUNK
    }
}

/// Without the `parallel` feature, sweeps are never split.
#[cfg(not(feature = "parallel"))]
pub(crate) fn frame_range_len(frames: usize) -> usize {
    frames
}

/// Runs `f` once per item of `items`, in parallel when the `parallel`
/// feature is enabled (and there is more than one item), sequentially
/// otherwise. Used to decode several channels' output buffers at once.
#[cfg(feature = "parallel")]
pub(crate) fn for_each_maybe_par<I, F>(items: &mut [I], f: F)
where
    I: Send,
    F: Fn(&mut I) + Sync + Send,
{
    use rayon::prelude::*;
    if items.len() > 1 {
        items.par_iter_mut().for_each(f);
    } else {
        items.iter_mut().for_each(f);
    }
}

/// Sequential fallback used when the `parallel` feature is disabled.
#[cfg(not(feature = "parallel"))]
pub(crate) fn for_each_maybe_par<I, F>(items: &mut [I], f: F)
where
    F: Fn(&mut I),
{
    items.iter_mut().for_each(f);
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn split_maybe_par_reads_every_sample_at_its_index() {
        // Large enough to take the parallel path when the feature is on,
        // with a ragged final piece.
        let mut out = vec![0usize; 2_500_003];
        split_maybe_par(7, &mut out, |first, piece| {
            for (i, v) in piece.iter_mut().enumerate() {
                *v = first + i;
            }
            Ok(())
        })
        .unwrap();
        assert!(out.iter().enumerate().all(|(i, &v)| v == i + 7));
    }

    #[test]
    fn split_maybe_par_reports_errors() {
        let mut out = vec![0u8; 2_500_003];
        let result = split_maybe_par(0, &mut out, |_, _| Err(std::io::ErrorKind::Other.into()));
        assert!(result.is_err());
    }

    #[test]
    fn for_each_maybe_par_visits_every_item() {
        let mut items = vec![1, 2, 3];
        for_each_maybe_par(&mut items, |x| *x *= 10);
        assert_eq!(items, vec![10, 20, 30]);
    }
}
