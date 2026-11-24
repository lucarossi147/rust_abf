//! Storage-backend timing benchmarks on large files (issue [34]).
//!
//! One Criterion benchmark (flat sampling, 10 samples) per file, case,
//! backend and cache state. See BENCHMARKS.md.

mod common;

use common::{bench_files, evict, run_case, skip_reason, Backend, BenchFile, Cache, Case, Config};
use criterion::{BatchSize, BenchmarkId, Criterion, SamplingMode, Throughput};
use std::hint::black_box;

fn main() {
    let (cfg, _smoke) = Config::for_this_run().unwrap_or_else(|e| fail(&e));
    let files = bench_files(&cfg).unwrap_or_else(|e| fail(&e));

    let mut c = Criterion::default().sample_size(10).configure_from_args();
    for file in &files {
        for &case in Case::ALL {
            bench_case(&mut c, &cfg, file, case);
        }
    }
    c.final_summary();
}

fn bench_case(c: &mut Criterion, cfg: &Config, file: &BenchFile, case: Case) {
    let group_name = format!("{}-{}", file.label, case.label());
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
        }
    }
    group.finish();
}

fn fail(msg: &str) -> ! {
    eprintln!("storage bench: {msg}");
    std::process::exit(2)
}
