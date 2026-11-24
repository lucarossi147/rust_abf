//! Sample-decoding kernels shared by [`crate::Channel`] and [`crate::Abf`].
//!
//! Every kernel takes a byte slice `src` that starts at a channel's first
//! sample of a sweep and ends right after its last one, with consecutive
//! samples `stride` bytes apart (`stride = channel_count * sample_width`,
//! since ABF interleaves channels frame by frame). The caller validates that
//! range once per sweep (see `Channel::sweep_bytes`), so the loops below
//! carry no per-sample offset arithmetic or fallible bounds checks, which is
//! what lets LLVM unroll and vectorise them.

/// Little-endian `i16` from the first two bytes of `c`, or `0` if `c` is too
/// short. The fallback never triggers for a range validated by the caller;
/// it exists so decoding stays panic-free, and the optimiser removes it on
/// the fixed-stride paths.
#[inline(always)]
pub(crate) fn le_i16(c: &[u8]) -> i16 {
    match *c {
        [a, b, ..] => i16::from_le_bytes([a, b]),
        _ => 0,
    }
}

/// Little-endian `f32` from the first four bytes of `c`, or `0.0` if `c` is
/// too short (see [`le_i16`]).
#[inline(always)]
pub(crate) fn le_f32(c: &[u8]) -> f32 {
    match *c {
        [a, b, c2, d, ..] => f32::from_le_bytes([a, b, c2, d]),
        _ => 0.0,
    }
}

/// Decodes `out.len()` samples spaced `STRIDE` bytes apart. `STRIDE` being a
/// compile-time constant lets LLVM vectorise the de-interleave. The last
/// sample is handled separately because `src` ends right after it, i.e.
/// there is no full frame left for `chunks_exact` to yield.
#[inline(always)]
fn decode_strided<const STRIDE: usize, T>(src: &[u8], out: &mut [T], f: &impl Fn(&[u8]) -> T) {
    let full = out.len().saturating_sub(1);
    let (head, last) = out.split_at_mut(full);
    for (o, c) in head.iter_mut().zip(src.chunks_exact(STRIDE)) {
        *o = f(c);
    }
    if let Some(o) = last.first_mut() {
        *o = f(src.get(full * STRIDE..).unwrap_or(&[]));
    }
}

/// Same as [`decode_strided`], for a stride only known at runtime.
fn decode_strided_dyn<T>(src: &[u8], stride: usize, out: &mut [T], f: &impl Fn(&[u8]) -> T) {
    for (o, c) in out.iter_mut().zip(src.chunks(stride.max(1))) {
        *o = f(c);
    }
}

/// Decodes `out.len()` samples spaced `stride` bytes apart from `src`,
/// dispatching to a fixed-stride kernel for the common layouts (1, 2, 4 or
/// 8 channels of `i16`, 1, 2 or 4 channels of `f32`).
#[inline(always)]
pub(crate) fn decode<T, F: Fn(&[u8]) -> T>(src: &[u8], stride: usize, out: &mut [T], f: F) {
    match stride {
        2 => decode_strided::<2, T>(src, out, &f),
        4 => decode_strided::<4, T>(src, out, &f),
        8 => decode_strided::<8, T>(src, out, &f),
        16 => decode_strided::<16, T>(src, out, &f),
        _ => decode_strided_dyn(src, stride, out, &f),
    }
}

/// Number of frames decoded per block by the single-pass, multi-channel
/// decoder. One block of frames (`BLOCK_FRAMES * stride` bytes, i.e. 16 KiB
/// for two `i16` channels) stays resident in L1 cache while every channel's
/// samples are pulled out of it, so the interleaved data is streamed from
/// memory once instead of once per channel.
pub(crate) const BLOCK_FRAMES: usize = 4096;

/// Per-channel decode parameters for [`decode_frames_blocked`].
#[derive(Clone, Copy)]
pub(crate) struct ChannelParams {
    /// Byte offset of this channel's sample within a frame.
    pub(crate) offset_in_frame: usize,
    /// `i16` scaling (`raw * gain + offset`); ignored for `f32` data.
    pub(crate) gain: f32,
    /// See `gain`.
    pub(crate) offset: f32,
}

/// Single-pass de-interleave of consecutive frames (each `stride` bytes,
/// one sample per channel) starting at `src[0]`, into `outs[c][range]` for
/// every channel `c`, with `params(c)` describing channel `c`.
///
/// Frames are processed [`BLOCK_FRAMES`] at a time: each block is read from
/// memory once and then served from cache while every channel's samples
/// are pulled out of it with the vectorised [`decode`] kernel. Output
/// samples whose bytes lie beyond the end of `src` are left untouched; the
/// caller validates the range (or zero-fills) beforehand. Does not allocate.
pub(crate) fn decode_frames_blocked<B: AsMut<[f32]>>(
    src: &[u8],
    stride: usize,
    is_f32: bool,
    outs: &mut [B],
    range: std::ops::Range<usize>,
    params: impl Fn(usize) -> ChannelParams,
) {
    let frames = range.len();
    let mut first = 0;
    while first < frames {
        let n = BLOCK_FRAMES.min(frames - first);
        let block_start = first.saturating_mul(stride);
        let to = range.start + first;
        for (c, out) in outs.iter_mut().enumerate() {
            let p = params(c);
            let Some(src_c) = src.get(block_start.saturating_add(p.offset_in_frame)..) else {
                continue;
            };
            let Some(out) = out.as_mut().get_mut(to..to + n) else {
                continue;
            };
            if is_f32 {
                decode(src_c, stride, out, le_f32);
            } else {
                decode(src_c, stride, out, |c| le_i16(c) as f32 * p.gain + p.offset);
            }
        }
        first += n;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn bytes_i16(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn decode_handles_every_dispatched_stride_and_the_dynamic_fallback() {
        // 7 channels exercise the runtime-stride fallback; the rest hit a
        // fixed-stride kernel.
        for channels in [1usize, 2, 3, 4, 7, 8] {
            let frames = 11;
            let values: Vec<i16> = (0..(frames * channels) as i16).collect();
            let bytes = bytes_i16(&values);
            for ch in 0..channels {
                let src = &bytes[ch * 2..(frames - 1) * channels * 2 + ch * 2 + 2];
                let mut out = vec![0i16; frames];
                decode(src, channels * 2, &mut out, le_i16);
                let expected: Vec<i16> = (0..frames).map(|j| values[j * channels + ch]).collect();
                assert_eq!(out, expected, "channels={channels} ch={ch}");
            }
        }
    }

    #[test]
    fn decode_of_an_empty_output_is_a_no_op() {
        let mut out: [i16; 0] = [];
        decode(&[], 4, &mut out, le_i16);
    }

    #[test]
    fn short_chunks_decode_to_zero_instead_of_panicking() {
        assert_eq!(le_i16(&[1]), 0);
        assert_eq!(le_f32(&[1, 2, 3]), 0.0);
    }

    #[test]
    fn blocked_decode_matches_per_channel_decode_across_block_boundaries() {
        let channels = 3;
        let frames = BLOCK_FRAMES * 2 + 17;
        let values: Vec<i16> = (0..frames * channels)
            .map(|i| (i % 30_000) as i16)
            .collect();
        let bytes = bytes_i16(&values);
        let mut outs = vec![vec![0.0f32; frames]; channels];
        decode_frames_blocked(&bytes, channels * 2, false, &mut outs, 0..frames, |c| {
            ChannelParams {
                offset_in_frame: c * 2,
                gain: 0.5,
                offset: c as f32,
            }
        });
        for (c, out) in outs.iter().enumerate() {
            for (j, v) in out.iter().enumerate() {
                assert_eq!(*v, values[j * channels + c] as f32 * 0.5 + c as f32);
            }
        }
    }
}
