//! Panic-free parsing tests for backlog [5] / issue #10: `Abf::from_file` must
//! return a typed `AbfError` and must never panic, no matter how malformed or
//! truncated the input is.
use rust_abf::{Abf, AbfError};
use std::panic;
use std::path::Path;

const FIXTURE: &str = "tests/test_abf/18425108.abf";

/// Small deterministic xorshift64 PRNG so the fuzz tests are reproducible
/// without adding a `rand` dev-dependency.
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

/// Runs `Abf::from_file` on `path` inside `catch_unwind`, failing the test
/// with a clear message (instead of aborting the whole test binary) if it
/// panics, and returns the parse result otherwise.
fn from_file_no_panic(path: &Path) -> Result<Abf, AbfError> {
    match panic::catch_unwind(|| Abf::from_file(path)) {
        Ok(result) => result,
        Err(_) => panic!("Abf::from_file panicked while parsing {}", path.display()),
    }
}

#[test]
fn truncation_never_panics_and_is_always_an_error() {
    let original = std::fs::read(FIXTURE).expect("read fixture");
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("truncated.abf");

    let hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));

    let mut rng = Xorshift64::new(0xABCD_1234_5678_9EF0);
    let mut offsets: Vec<usize> = (0..8192.min(original.len())).collect();
    for _ in 0..100 {
        // 1..original.len() so every offset is a genuine truncation.
        offsets.push(1 + rng.next_below(original.len() - 1));
    }

    for offset in offsets {
        std::fs::write(&path, &original[..offset]).expect("write truncated fixture");
        let result = from_file_no_panic(&path);
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
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join("corrupted.abf");

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
        std::fs::write(&path, &corrupted).expect("write corrupted fixture");
        // Ok(_) or Err(_) are both fine here; only a panic is a failure.
        let _ = from_file_no_panic(&path);
    }

    panic::set_hook(hook);
}

/// Describes just the error side for assertion messages, so callers don't
/// need to require `T: Debug`.
fn describe_err<T>(result: &Result<T, AbfError>) -> String {
    match result {
        Ok(_) => "Ok(_)".to_string(),
        Err(e) => format!("Err({e:?})"),
    }
}

#[test]
fn abf1_fixture_is_unsupported_version_error() {
    let result = Abf::from_file(Path::new("tests/test_abf/05210017_vc_abf1.abf"));
    assert!(
        matches!(result, Err(AbfError::UnsupportedVersion(_))),
        "expected Err(AbfError::UnsupportedVersion(_)), got {}",
        describe_err(&result)
    );
}

#[test]
fn non_abf_file_is_invalid_signature_error() {
    let result = Abf::from_file(Path::new("tests/test_abf/wrong_signature.abf"));
    assert!(
        matches!(result, Err(AbfError::InvalidSignature)),
        "expected Err(AbfError::InvalidSignature), got {}",
        describe_err(&result)
    );
}

#[test]
fn missing_path_is_io_error() {
    let result = Abf::from_file(Path::new("tests/test_abf/does_not_exist.abf"));
    assert!(
        matches!(result, Err(AbfError::Io(_))),
        "expected Err(AbfError::Io(_)), got {}",
        describe_err(&result)
    );
}
