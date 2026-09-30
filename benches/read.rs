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
                for channel in abf.channels() {
                    for sweep in channel.sweeps() {
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
                for channel in abf.channels() {
                    for sweep in 0..abf.sweep_count() {
                        black_box(channel.raw_sweep(sweep).unwrap());
                    }
                }
            });
        });
    }
    group.finish();
}

fn bench_read_all_sweeps_into(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_all_sweeps_into");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            b.iter(|| {
                for channel in abf.channels() {
                    let mut buf = vec![0.0f32; channel.sweep_len()];
                    for sweep in 0..abf.sweep_count() {
                        channel.read_sweep_into(sweep, &mut buf).unwrap();
                        black_box(&buf);
                    }
                }
            });
        });
    }
    group.finish();
}

fn bench_read_all_channels_into(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_all_channels_into");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            let mut buffers: Vec<Vec<f32>> = abf
                .channels()
                .map(|c| vec![0.0f32; c.sweep_len()])
                .collect();
            b.iter(|| {
                for sweep in 0..abf.sweep_count() {
                    abf.read_sweep_all_channels_into(sweep, &mut buffers)
                        .unwrap();
                    black_box(&buffers);
                }
            });
        });
    }
    group.finish();
}

fn bench_sweep_all_channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("sweep_all_channels");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            b.iter(|| {
                for sweep in 0..abf.sweep_count() {
                    black_box(abf.sweep_all_channels(sweep).unwrap());
                }
            });
        });
    }
    group.finish();
}

fn bench_sweep_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("sweep_iter");
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        group.bench_function(*fixture, |b| {
            b.iter(|| {
                for channel in abf.channels() {
                    let mut buf = vec![0.0f32; channel.sweep_len()];
                    for sweep in 0..abf.sweep_count() {
                        for (o, x) in buf.iter_mut().zip(channel.sweep_iter(sweep).unwrap()) {
                            *o = x;
                        }
                        black_box(&buf);
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
    bench_read_all_sweeps_raw,
    bench_read_all_sweeps_into,
    bench_read_all_channels_into,
    bench_sweep_all_channels,
    bench_sweep_iter
);
criterion_main!(benches);
