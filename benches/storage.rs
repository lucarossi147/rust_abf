//! Storage-backend timing benchmarks on large files (issue [34]).
//!
//! For every file (synthetic + `RUST_ABF_BENCH_FILES`), case, backend and
//! cache state, runs one Criterion benchmark (flat sampling, 10 samples),
//! then prints a Markdown table of the means and writes it to
//! `target/bench-results/storage.md`. Configuration and cases: see
//! `benches/common/mod.rs` and BENCHMARKS.md.
//!
//! Criterion filters work as usual, e.g.
//! `cargo bench --bench storage -- stream_all`.

mod common;

use common::{
    bench_files, evict, machine_info, run_case, skip_reason, Backend, BenchFile, Cache, Case,
    Config,
};
use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode, Throughput};
use std::fmt::Write as _;
use std::hint::black_box;
use std::path::PathBuf;

struct Ran {
    file: String,
    file_mb: u64,
    case: Case,
    backend: Backend,
    cache: Cache,
    group: String,
}

fn main() {
    let (cfg, smoke) = Config::for_this_run().unwrap_or_else(|e| fail(&e));
    let files = bench_files(&cfg).unwrap_or_else(|e| fail(&e));
    eprintln!("machine: {}", machine_info());

    let mut c = Criterion::default().sample_size(10).configure_from_args();
    let mut ran = Vec::new();
    for file in &files {
        for &case in Case::ALL {
            bench_case(&mut c, &cfg, file, case, &mut ran);
        }
    }
    c.final_summary();
    if !smoke {
        report(&ran);
    }
}

fn bench_case(c: &mut Criterion, cfg: &Config, file: &BenchFile, case: Case, ran: &mut Vec<Ran>) {
    let group_name = sanitize(&format!("{}-{}", file.label, case.label()));
    let mut group = c.benchmark_group(&group_name);
    group.sampling_mode(SamplingMode::Flat);
    if case.reads_everything() {
        group.throughput(Throughput::Bytes(file.bytes));
    }
    for &cache in &cfg.caches {
        if cache == Cache::Cold {
            if let Err(e) = evict(&file.path) {
                eprintln!("skip {group_name} cold: {e}");
                continue;
            }
        }
        for &backend in Backend::ALL {
            if let Some(reason) = skip_reason(case, backend, file, cfg) {
                eprintln!("skip {group_name} {}: {reason}", backend.label());
                continue;
            }
            let path = &file.path;
            let id = BenchmarkId::new(backend.label(), cache.label());
            group.bench_function(id, |b| match cache {
                Cache::Warm => b.iter(|| black_box(run_case(case, backend, path).unwrap())),
                Cache::Cold => b.iter_batched(
                    || evict(path).unwrap(),
                    |()| black_box(run_case(case, backend, path).unwrap()),
                    BatchSize::PerIteration,
                ),
            });
            ran.push(Ran {
                file: file.label.clone(),
                file_mb: file.bytes >> 20,
                case,
                backend,
                cache,
                group: group_name.clone(),
            });
        }
    }
    group.finish();
}

/// Criterion directory names must not contain path separators etc.
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || "-_.".contains(ch) {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn criterion_dir() -> PathBuf {
    std::env::var_os("CRITERION_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| common::bench_data_dir().with_file_name("criterion"))
}

/// Mean and 95% CI in nanoseconds from Criterion's `estimates.json`, if
/// this run wrote one (it doesn't in `--test` mode).
fn estimate(r: &Ran) -> Option<(f64, f64, f64)> {
    let path = criterion_dir()
        .join(&r.group)
        .join(r.backend.label())
        .join(r.cache.label())
        .join("new/estimates.json");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    let mean = &json["mean"];
    let ci = &mean["confidence_interval"];
    Some((
        mean["point_estimate"].as_f64()?,
        ci["lower_bound"].as_f64()?,
        ci["upper_bound"].as_f64()?,
    ))
}

fn human(ns: f64) -> String {
    match ns {
        n if n >= 1e9 => format!("{:.3} s", n / 1e9),
        n if n >= 1e6 => format!("{:.2} ms", n / 1e6),
        n if n >= 1e3 => format!("{:.1} µs", n / 1e3),
        n => format!("{n:.0} ns"),
    }
}

fn report(ran: &[Ran]) {
    let mut md = format!("Machine: {}\n\n", machine_info());
    md.push_str("| file | MiB | case | backend | cache | mean | 95% CI | GiB/s |\n");
    md.push_str("|---|---:|---|---|---|---:|---|---:|\n");
    for r in ran {
        let (mean, ci, rate) = match estimate(r) {
            Some((m, lo, hi)) => {
                let rate = if r.case.reads_everything() && m > 0.0 {
                    format!("{:.2}", (r.file_mb as f64 / 1024.0) / (m / 1e9))
                } else {
                    String::new()
                };
                (human(m), format!("{} – {}", human(lo), human(hi)), rate)
            }
            None => ("n/a".into(), String::new(), String::new()),
        };
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {mean} | {ci} | {rate} |",
            r.file,
            r.file_mb,
            r.case.label(),
            r.backend.label(),
            r.cache.label()
        );
    }
    println!("\n{md}");
    let dir = common::bench_data_dir().with_file_name("bench-results");
    let out = dir.join("storage.md");
    match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&out, &md)) {
        Ok(()) => println!("written to {}", out.display()),
        Err(e) => eprintln!("could not write {}: {e}", out.display()),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("storage bench: {msg}");
    std::process::exit(2)
}
