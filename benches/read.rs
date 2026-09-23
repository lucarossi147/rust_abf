use criterion::{criterion_group, criterion_main, Criterion};
use rust_abf::Abf;
use std::hint::black_box;
use std::path::Path;

// ABF1 fixture is excluded: ABF1 parsing is not implemented yet (`Abf::from_file` panics on it).
const FIXTURES: &[&str] = &[
    "tests/test_abf/14o08011_ic_pair.abf",
    "tests/test_abf/18425108.abf",
];

fn bench_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("open");
    for fixture in FIXTURES {
        group.bench_function(*fixture, |b| {
            b.iter(|| Abf::from_file(black_box(Path::new(fixture))).unwrap());
        });
    }
    group.finish();
}

fn bench_read_all_sweeps_f32(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_all_sweeps_f32");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            b.iter(|| {
                for channel in abf.get_channels() {
                    for sweep in channel.get_sweeps() {
                        black_box(sweep.unwrap());
                    }
                }
            });
        });
    }
    group.finish();
}

fn bench_read_all_sweeps_raw(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_all_sweeps_raw");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            b.iter(|| {
                for channel in abf.get_channels() {
                    for sweep in 0..abf.get_sweeps_count() {
                        black_box(channel.get_raw_sweep(sweep).unwrap());
                    }
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_open,
    bench_read_all_sweeps_f32,
    bench_read_all_sweeps_raw
);
criterion_main!(benches);
