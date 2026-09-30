//! Optional multi-threaded decoding, enabled by the `parallel` Cargo feature.
//!
//! Only the *allocating* read paths ([`crate::Channel::sweep`],
//! [`crate::Channel::raw_sweep`], [`crate::Abf::sweep_all_channels`]) go
//! through here. [`crate::Channel::read_sweep_into`] and
//! [`crate::Abf::read_sweep_all_channels_into`] always decode on the calling
//! thread, because rayon's thread pool allocates on first use and may grow
//! its work queues later, which would break their zero-allocation guarantee.
//!
//! Without the feature, [`decode_maybe_par`] is just [`crate::decode::decode`].

/// Sweeps shorter than this many samples are always decoded on the calling
/// thread: below it, handing work to the pool costs more than it saves
/// (measured: at 250k samples a parallel decode into a freshly allocated
/// buffer was slower than a sequential one; from ~1M samples up it wins).
#[cfg(feature = "parallel")]
pub(crate) const PAR_THRESHOLD: usize = 1024 * 1024;

/// Samples per parallel work item.
#[cfg(feature = "parallel")]
const PAR_CHUNK: usize = 32 * 1024;

/// Decodes `out.len()` samples spaced `stride` bytes apart from `src` (see
/// [`crate::decode::decode`]), splitting the work across rayon's global
/// thread pool when the `parallel` feature is enabled and the sweep is at
/// least [`PAR_THRESHOLD`] samples long.
#[cfg(feature = "parallel")]
pub(crate) fn decode_maybe_par<T, F>(src: &[u8], stride: usize, out: &mut [T], f: F)
where
    T: Send,
    F: Fn(&[u8]) -> T + Sync + Send + Copy,
{
    use rayon::prelude::*;
    if out.len() < PAR_THRESHOLD {
        crate::decode::decode(src, stride, out, f);
        return;
    }
    out.par_chunks_mut(PAR_CHUNK)
        .enumerate()
        .for_each(|(k, chunk)| {
            let start = k.saturating_mul(PAR_CHUNK).saturating_mul(stride);
            if let Some(src_k) = src.get(start..) {
                crate::decode::decode(src_k, stride, chunk, f);
            }
        });
}

/// Sequential fallback used when the `parallel` feature is disabled.
#[cfg(not(feature = "parallel"))]
#[inline(always)]
pub(crate) fn decode_maybe_par<T, F>(src: &[u8], stride: usize, out: &mut [T], f: F)
where
    F: Fn(&[u8]) -> T,
{
    crate::decode::decode(src, stride, out, f);
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
    use crate::decode::le_i16;

    #[test]
    fn decode_maybe_par_matches_sequential_decode_for_large_inputs() {
        // Large enough to take the parallel path when the feature is on,
        // with a ragged final chunk.
        let channels = 2;
        let frames = 2_500_003;
        let bytes: Vec<u8> = (0..frames * channels)
            .flat_map(|i| ((i % 60_000) as i16).to_le_bytes())
            .collect();
        let src = &bytes[2..];
        let mut par = vec![0i16; frames];
        let mut seq = vec![0i16; frames];
        decode_maybe_par(src, channels * 2, &mut par, le_i16);
        crate::decode::decode(src, channels * 2, &mut seq, le_i16);
        assert_eq!(par, seq);
    }

    #[test]
    fn for_each_maybe_par_visits_every_item() {
        let mut items = vec![1, 2, 3];
        for_each_maybe_par(&mut items, |x| *x *= 10);
        assert_eq!(items, vec![10, 20, 30]);
    }
}
