//! Tests for `Abf::from_bytes`, the owned-buffer constructor added for
//! backlog [14]: it must agree with `Abf::from_file` on every golden fixture,
//! and must handle malformed/truncated input exactly like `Abf::from_file`
//! does (backlog [5] / issue #10), without needing temp files since the
//! input is already an in-memory buffer.
use rust_abf::{Abf, AbfError};
use std::panic;
use std::path::Path;

const FIXTURE: &str = "tests/test_abf/18425108.abf";

/// Small deterministic xorshift64 PRNG so the fuzz tests are reproducible
/// without adding a `rand` dev-dependency (mirrors `tests/errors.rs`).
struct Xorshift64(u64);

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Returns a value in `0..bound`. `bound` must be non-zero.
    fn next_below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

/// Runs `Abf::from_bytes` on `bytes` inside `catch_unwind`, failing the test
/// with a clear message (instead of aborting the whole test binary) if it
/// panics, and returns the parse result otherwise.
fn from_bytes_no_panic(bytes: Vec<u8>) -> Result<Abf, AbfError> {
    match panic::catch_unwind(|| Abf::from_bytes(bytes)) {
        Ok(result) => result,
        Err(_) => panic!("Abf::from_bytes panicked"),
    }
}

/// Describes just the error side for assertion messages, so callers don't
/// need to require `T: Debug`.
fn describe_err<T>(result: &Result<T, AbfError>) -> String {
    match result {
        Ok(_) => "Ok(_)".to_string(),
        Err(e) => format!("Err({e:?})"),
    }
}

fn assert_abf_eq(from_file: &Abf, from_bytes: &Abf, context: &str) {
    assert_eq!(
        from_file.channel_count(),
        from_bytes.channel_count(),
        "{context}: channel_count"
    );
    assert_eq!(
        from_file.sweep_count(),
        from_bytes.sweep_count(),
        "{context}: sweep_count"
    );
    assert_eq!(
        from_file.sampling_rate(),
        from_bytes.sampling_rate(),
        "{context}: sampling_rate"
    );
    assert_eq!(
        from_file.time_axis(),
        from_bytes.time_axis(),
        "{context}: time_axis"
    );
    assert_eq!(from_bytes.path(), Path::new(""), "{context}: path");

    for ch in 0..from_file.channel_count() {
        let file_channel = from_file
            .channel(ch)
            .unwrap_or_else(|| panic!("{context}: missing channel {ch} in `from_file` result"));
        let bytes_channel = from_bytes
            .channel(ch)
            .unwrap_or_else(|| panic!("{context}: missing channel {ch} in `from_bytes` result"));
        assert_eq!(
            file_channel.label(),
            bytes_channel.label(),
            "{context}: channel {ch} label"
        );
        assert_eq!(
            file_channel.uom(),
            bytes_channel.uom(),
            "{context}: channel {ch} uom"
        );

        for sweep in 0..from_file.sweep_count() {
            assert_eq!(
                from_file.sweep(ch, sweep),
                from_bytes.sweep(ch, sweep),
                "{context}: channel {ch} sweep {sweep}"
            );
        }
    }
}

#[test]
fn from_bytes_matches_from_file_on_every_golden_fixture() {
    for path in [
        "tests/test_abf/18425108.abf",
        "tests/test_abf/14o08011_ic_pair.abf",
    ] {
        let from_file = Abf::from_file(Path::new(path)).unwrap();
        let bytes = std::fs::read(path).unwrap();
        let from_bytes = Abf::from_bytes(bytes).unwrap();
        assert_abf_eq(&from_file, &from_bytes, path);
    }
}

#[test]
fn from_bytes_accepts_a_vec_and_an_arc_slice() {
    let bytes = std::fs::read(FIXTURE).unwrap();
    Abf::from_bytes(bytes.clone()).unwrap();
    let arc: std::sync::Arc<[u8]> = std::sync::Arc::from(bytes.as_slice());
    Abf::from_bytes(arc).unwrap();
}

#[test]
fn truncation_never_panics_and_is_always_an_error() {
    let original = std::fs::read(FIXTURE).expect("read fixture");

    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let mut rng = Xorshift64::new(0xABCD_1234_5678_9EF0);
    let mut offsets: Vec<usize> = (0..8192.min(original.len())).collect();
    for _ in 0..100 {
        // 1..original.len() so every offset is a genuine truncation.
        offsets.push(1 + rng.next_below(original.len() - 1));
    }

    for offset in offsets {
        let truncated = original[..offset].to_vec();
        let result = from_bytes_no_panic(truncated);
        assert!(
            result.is_err(),
            "truncating at offset {offset} unexpectedly parsed successfully"
        );
    }

    panic::set_hook(hook);
}

#[test]
fn single_byte_corruption_never_panics() {
    let original = std::fs::read(FIXTURE).expect("read fixture");

    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let corrupt_len = original.len().min(4096);
    let mut rng = Xorshift64::new(0x1357_9BDF_2468_ACE0);
    for _ in 0..1000 {
        let idx = rng.next_below(corrupt_len);
        let mut corrupted = original.clone();
        if let Some(byte) = corrupted.get_mut(idx) {
            *byte ^= 0xFF;
        }
        // Ok(_) or Err(_) are both fine here; only a panic is a failure.
        let _ = from_bytes_no_panic(corrupted);
    }

    panic::set_hook(hook);
}

#[test]
fn abf1_fixture_is_unsupported_version_error() {
    let bytes = std::fs::read("tests/test_abf/05210017_vc_abf1.abf").unwrap();
    let result = Abf::from_bytes(bytes);
    assert!(
        matches!(result, Err(AbfError::UnsupportedVersion(_))),
        "expected Err(AbfError::UnsupportedVersion(_)), got {}",
        describe_err(&result)
    );
}

#[test]
fn non_abf_bytes_is_invalid_signature_error() {
    let bytes = std::fs::read("tests/test_abf/wrong_signature.abf").unwrap();
    let result = Abf::from_bytes(bytes);
    assert!(
        matches!(result, Err(AbfError::InvalidSignature)),
        "expected Err(AbfError::InvalidSignature), got {}",
        describe_err(&result)
    );
}

#[test]
fn empty_bytes_is_invalid_signature_error() {
    let result = Abf::from_bytes(Vec::new());
    assert!(
        matches!(result, Err(AbfError::InvalidSignature)),
        "expected Err(AbfError::InvalidSignature), got {}",
        describe_err(&result)
    );
}
