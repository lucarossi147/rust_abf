//! Shared helpers for the storage-backend benchmarks (issue [34]):
//! `benches/storage.rs` (timing) and `benches/storage_memory.rs` (peak RSS).
//! Configuration (`RUST_ABF_BENCH_*` variables) and usage: see BENCHMARKS.md.

// Each bench binary and the test that includes this module uses a subset.
#![allow(dead_code)]

use rust_abf::{Abf, AbfError};
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Header donor for synthetic files: 2 int16 ADC channels.
pub const HEADER_FIXTURE: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_abf/18425108.abf");

const MIB: u64 = 1 << 20;
const BLOCK: usize = 512;
const PROTOCOL_MAP: usize = 76;
const DATA_MAP: usize = 236;
const SYNCH_ARRAY_MAP: usize = 316;
const GAP_FREE: i16 = 3;
const EPISODIC: i16 = 5;
/// Sweeps picked by `random_sweeps`.
const RANDOM_SWEEPS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cache {
    Warm,
    Cold,
}

impl Cache {
    pub fn label(self) -> &'static str {
        match self {
            Cache::Warm => "warm",
            Cache::Cold => "cold",
        }
    }
}

/// How the file is opened. `from_file` is whatever `Abf::from_file` uses
/// (mmap up to 0.6.0); `from_bytes` reads the whole file into memory first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    FromFile,
    FromBytes,
}

impl Backend {
    pub const ALL: &'static [Backend] = &[Backend::FromFile, Backend::FromBytes];

    pub fn label(self) -> &'static str {
        match self {
            Backend::FromFile => "from_file",
            Backend::FromBytes => "from_bytes",
        }
    }

    pub fn open(self, path: &Path) -> Result<Abf, AbfError> {
        match self {
            Backend::FromFile => Abf::from_file(path),
            Backend::FromBytes => Abf::from_bytes(std::fs::read(path)?),
        }
    }
}

/// What a benchmark iteration does. Every case includes opening the file,
/// so `cold` measures a real first access and backends compare end to end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    /// Open only.
    Open,
    /// Open, then the first second of every channel (`sweep_iter`).
    FirstSecond,
    /// Every sample of every channel through `sweep_iter` (constant memory).
    StreamAll,
    /// Every sweep through `read_sweep_all_channels_into`, reused buffers.
    ReadAllInto,
    /// 64 fixed-seed random sweeps through `read_sweep_all_channels_into`.
    RandomSweeps,
}

impl Case {
    pub const ALL: &'static [Case] = &[
        Case::Open,
        Case::FirstSecond,
        Case::StreamAll,
        Case::ReadAllInto,
        Case::RandomSweeps,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Case::Open => "open",
            Case::FirstSecond => "first_second",
            Case::StreamAll => "stream_all",
            Case::ReadAllInto => "read_all_into",
            Case::RandomSweeps => "random_sweeps",
        }
    }

    pub fn from_label(label: &str) -> Option<Case> {
        Case::ALL.iter().copied().find(|c| c.label() == label)
    }

    /// Whether the case reads the whole data section (for throughput).
    pub fn reads_everything(self) -> bool {
        matches!(self, Case::StreamAll | Case::ReadAllInto)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SynthMode {
    GapFree,
    Episodic,
}

impl SynthMode {
    fn label(self) -> &'static str {
        match self {
            SynthMode::GapFree => "gapfree",
            SynthMode::Episodic => "episodic",
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub files: Vec<PathBuf>,
    pub synth_mb: Vec<u64>,
    pub caches: Vec<Cache>,
    pub owned_max_mb: u64,
    pub buf_max_mb: u64,
}

impl Config {
    pub fn from_env() -> Result<Config, String> {
        Config::from_lookup(|key| std::env::var_os(key))
    }

    /// `from_env` under `cargo bench`. Under `cargo test --all-targets`
    /// (no `--bench` argument) returns a quick smoke configuration instead:
    /// one 1 MB synthetic file, warm and (where supported) cold, no real
    /// files. Callers must not write result files in that mode.
    pub fn for_this_run() -> Result<(Config, bool), String> {
        if is_bench_run() {
            return Config::from_env().map(|cfg| (cfg, false));
        }
        let mut cfg = Config::from_lookup(|_| None)?;
        cfg.synth_mb = vec![1];
        if cfg!(any(windows, target_os = "linux")) {
            cfg.caches = vec![Cache::Warm, Cache::Cold];
        }
        Ok((cfg, true))
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<OsString>) -> Result<Config, String> {
        let text = |key: &str| -> Result<Option<String>, String> {
            lookup(key)
                .map(|v| {
                    v.into_string()
                        .map_err(|_| format!("{key} is not valid UTF-8"))
                })
                .transpose()
        };
        let number = |key: &str, default: u64| -> Result<u64, String> {
            match text(key)? {
                None => Ok(default),
                Some(v) => v
                    .trim()
                    .parse()
                    .map_err(|_| format!("{key}: not a number: {v:?}")),
            }
        };
        Ok(Config {
            files: lookup("RUST_ABF_BENCH_FILES")
                .map(|v| {
                    std::env::split_paths(&v)
                        .filter(|p| !p.as_os_str().is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            synth_mb: parse_mb_list(
                &text("RUST_ABF_BENCH_SYNTH_MB")?.unwrap_or_else(|| "64".into()),
            )?,
            caches: parse_caches(&text("RUST_ABF_BENCH_CACHE")?.unwrap_or_else(|| "warm".into()))?,
            owned_max_mb: number("RUST_ABF_BENCH_OWNED_MAX_MB", 2048)?,
            buf_max_mb: number("RUST_ABF_BENCH_BUF_MAX_MB", 512)?,
        })
    }
}

/// `cargo bench` passes `--bench` to every bench binary; `cargo test` doesn't.
pub fn is_bench_run() -> bool {
    std::env::args().any(|a| a == "--bench")
}

pub fn parse_mb_list(s: &str) -> Result<Vec<u64>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.parse().map_err(|_| format!("not a size in MB: {v:?}")))
        .collect()
}

pub fn parse_caches(s: &str) -> Result<Vec<Cache>, String> {
    s.split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| match v {
            "warm" => Ok(Cache::Warm),
            "cold" => Ok(Cache::Cold),
            _ => Err(format!("cache state must be warm or cold, got {v:?}")),
        })
        .collect()
}

/// Directory for generated files: `$CARGO_TARGET_DIR/bench-data` or
/// `target/bench-data`.
pub fn bench_data_dir() -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"))
        .join("bench-data")
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Writes (or reuses) a synthetic ABF2 file of about `mb` MiB of int16
/// data, using `HEADER_FIXTURE`'s header: 2 channels, gap-free (one sweep)
/// or episodic (1 s sweeps). The data is xorshift noise, so nothing is
/// sparse or compressible. Reused when a file of the expected size exists.
pub fn synth_file(dir: &Path, mb: u64, mode: SynthMode) -> io::Result<PathBuf> {
    let path = dir.join(format!("synth-{}-{mb}MB.abf", mode.label()));
    let mut header = std::fs::read(HEADER_FIXTURE)?;
    let channels = u64::from(u32_at(&header, 92 + 8));
    let rate = Abf::from_file(Path::new(HEADER_FIXTURE))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .sampling_rate() as u64;

    let total_samples = mb * MIB / 2;
    let (entries, sweeps) = match mode {
        SynthMode::GapFree => (total_samples / channels * channels, 1),
        SynthMode::Episodic => {
            let per_sweep = channels * rate;
            let sweeps = (total_samples / per_sweep).max(1);
            (sweeps * per_sweep, sweeps)
        }
    };

    let data_start = u32_at(&header, DATA_MAP) as usize * BLOCK;
    header.truncate(data_start);
    let expected_len = data_start as u64 + entries * 2;
    if std::fs::metadata(&path).map(|m| m.len()).ok() == Some(expected_len) {
        return Ok(path);
    }

    let op_mode = match mode {
        SynthMode::GapFree => GAP_FREE,
        SynthMode::Episodic => EPISODIC,
    };
    let protocol = u32_at(&header, PROTOCOL_MAP) as usize * BLOCK;
    header[protocol..protocol + 2].copy_from_slice(&op_mode.to_le_bytes());
    header[12..16].copy_from_slice(&(sweeps as u32).to_le_bytes());
    header[SYNCH_ARRAY_MAP..SYNCH_ARRAY_MAP + 16].fill(0);
    header[DATA_MAP + 8..DATA_MAP + 16].copy_from_slice(&(entries as i64).to_le_bytes());

    std::fs::create_dir_all(dir)?;
    let mut out = BufWriter::with_capacity(4 << 20, File::create(&path)?);
    out.write_all(&header)?;
    let mut state = 0x9E37_79B9_7F4A_7C15_u64 ^ mb;
    let mut chunk = Vec::with_capacity(1 << 20);
    let mut left = entries;
    while left > 0 {
        let n = left.min(1 << 19);
        chunk.clear();
        for _ in 0..n {
            // Small values so f32 sums stay exact enough to compare.
            let v = (xorshift(&mut state) >> 54) as i16 - 512;
            chunk.extend_from_slice(&v.to_le_bytes());
        }
        out.write_all(&chunk)?;
        left -= n;
    }
    out.flush()?;
    Ok(path)
}

pub fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// A file to benchmark, with the metadata needed to pick cases.
#[derive(Debug, Clone)]
pub struct BenchFile {
    pub label: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub channels: usize,
    pub sweeps: usize,
    pub sweep_len: usize,
}

impl BenchFile {
    /// Bytes of decoded `f32` output buffers for one sweep of every channel.
    pub fn sweep_buffer_bytes(&self) -> u64 {
        (self.channels * self.sweep_len * 4) as u64
    }
}

pub fn bench_file(path: PathBuf) -> Result<BenchFile, String> {
    let abf = Abf::from_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    let label = path
        .file_stem()
        .map_or_else(|| "file".into(), |s| s.to_string_lossy().into_owned());
    Ok(BenchFile {
        label,
        bytes,
        channels: abf.channel_count(),
        sweeps: abf.sweep_count(),
        sweep_len: abf.channel(0).map_or(0, |c| c.sweep_len()),
        path,
    })
}

/// Synthetic files from `cfg.synth_mb`, then `cfg.files`.
pub fn bench_files(cfg: &Config) -> Result<Vec<BenchFile>, String> {
    let dir = bench_data_dir();
    let mut paths = Vec::new();
    for &mb in &cfg.synth_mb {
        for mode in [SynthMode::GapFree, SynthMode::Episodic] {
            paths.push(
                synth_file(&dir, mb, mode).map_err(|e| format!("generating {mb} MB file: {e}"))?,
            );
        }
    }
    paths.extend(cfg.files.iter().cloned());
    paths.into_iter().map(bench_file).collect()
}

/// Why `case` doesn't run for this file/backend, or `None` if it does.
pub fn skip_reason(case: Case, backend: Backend, file: &BenchFile, cfg: &Config) -> Option<String> {
    if file.channels == 0 || file.sweep_len == 0 {
        return Some("no data".into());
    }
    if backend == Backend::FromBytes && file.bytes > cfg.owned_max_mb * MIB {
        return Some(format!(
            "file > RUST_ABF_BENCH_OWNED_MAX_MB ({} MB)",
            cfg.owned_max_mb
        ));
    }
    match case {
        Case::ReadAllInto if file.sweep_buffer_bytes() > cfg.buf_max_mb * MIB => Some(format!(
            "sweep buffers {} MB > RUST_ABF_BENCH_BUF_MAX_MB ({} MB)",
            file.sweep_buffer_bytes() / MIB,
            cfg.buf_max_mb
        )),
        Case::RandomSweeps if file.sweeps < 2 => Some("single sweep (gap-free)".into()),
        _ => None,
    }
}

/// Runs one iteration of `case`, opening the file with `backend`. Returns
/// a checksum of everything read, so the work can't be optimised away and
/// backends can be checked against each other.
pub fn run_case(case: Case, backend: Backend, path: &Path) -> Result<f64, AbfError> {
    let abf = backend.open(path)?;
    let mut sum = 0.0_f64;
    match case {
        Case::Open => sum += abf.sweep_count() as f64,
        Case::FirstSecond => {
            let n = abf.sampling_rate() as usize;
            for channel in abf.channels() {
                if let Some(it) = channel.sweep_iter(0) {
                    sum += it.take(n).map(f64::from).sum::<f64>();
                }
            }
        }
        Case::StreamAll => {
            for channel in abf.channels() {
                for sweep in 0..abf.sweep_count() {
                    if let Some(it) = channel.sweep_iter(sweep) {
                        sum += it.map(f64::from).sum::<f64>();
                    }
                }
            }
        }
        Case::ReadAllInto => {
            let mut buffers = sweep_buffers(&abf);
            for sweep in 0..abf.sweep_count() {
                abf.read_sweep_all_channels_into(sweep, &mut buffers)?;
                sum += checksum(&buffers);
            }
        }
        Case::RandomSweeps => {
            let mut buffers = sweep_buffers(&abf);
            let mut state = 0x2545_F491_4F6C_DD1D_u64;
            for _ in 0..RANDOM_SWEEPS {
                let sweep = (xorshift(&mut state) % abf.sweep_count() as u64) as usize;
                abf.read_sweep_all_channels_into(sweep, &mut buffers)?;
                sum += checksum(&buffers);
            }
        }
    }
    Ok(sum)
}

fn sweep_buffers(abf: &Abf) -> Vec<Vec<f32>> {
    abf.channels().map(|c| vec![0.0; c.sweep_len()]).collect()
}

fn checksum(buffers: &[Vec<f32>]) -> f64 {
    buffers.iter().flatten().map(|&v| f64::from(v)).sum()
}

/// Drops `path`'s pages from the OS page cache, without admin rights, so
/// the next read comes from disk.
///
/// - Windows: opening a handle with `FILE_FLAG_NO_BUFFERING` makes the cache
///   manager flush and purge the file's cached data (only works when no
///   other handle or mapping of the file is open).
/// - Linux: `posix_fadvise(POSIX_FADV_DONTNEED)` after `fsync` (dirty pages
///   can't be dropped).
#[cfg(windows)]
pub fn evict(path: &Path) -> io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_NO_BUFFERING)
        .open(path)
        .map(drop)
}

#[cfg(target_os = "linux")]
pub fn evict(path: &Path) -> io::Result<()> {
    let file = File::open(path)?;
    file.sync_all()?;
    rustix::fs::fadvise(&file, 0, None, rustix::fs::Advice::DontNeed)?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn evict(_: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "cold-cache benchmarks need Windows or Linux",
    ))
}
