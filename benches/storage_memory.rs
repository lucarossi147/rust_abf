//! Peak resident memory (RSS on Linux, working set on Windows) per storage
//! backend and access pattern (issue [34]).
//!
//! Unlike `tests/memory.rs`, which counts heap allocations, this counts
//! mapped file pages too, which is what differs between mmap and positional
//! reads. Each (file, case, backend) runs once in a fresh child process
//! (this binary re-executed with `--child`), which samples its own resident
//! memory every millisecond. Prints a Markdown table and writes it to
//! `target/bench-results/storage_memory.md`.

mod common;

use common::{bench_files, run_case, skip_reason, Backend, Case, Config};
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MIB: f64 = (1 << 20) as f64;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--child") {
        child(&args[i + 1..]);
    } else {
        parent();
    }
}

fn resident() -> usize {
    memory_stats::memory_stats().map_or(0, |m| m.physical_mem)
}

/// Runs one case and prints `baseline peak` in bytes.
fn child(args: &[String]) {
    let [case, backend, path] = args else {
        panic!("usage: --child <case> <backend> <path>")
    };
    let case = Case::from_label(case).expect("case");
    let backend = *Backend::ALL
        .iter()
        .find(|b| b.label() == backend)
        .expect("backend");

    let baseline = resident();
    let peak = Arc::new(AtomicUsize::new(baseline));
    let stop = Arc::new(AtomicBool::new(false));
    let sampler = {
        let (peak, stop) = (Arc::clone(&peak), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                peak.fetch_max(resident(), Ordering::Relaxed);
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    };
    let sum = run_case(case, backend, Path::new(path)).expect("run case");
    peak.fetch_max(resident(), Ordering::Relaxed);
    stop.store(true, Ordering::Relaxed);
    sampler.join().expect("sampler");
    std::hint::black_box(sum);
    println!("{baseline} {}", peak.load(Ordering::Relaxed));
}

fn parent() {
    let (cfg, smoke) = Config::for_this_run().unwrap_or_else(|e| fail(&e));
    let files = bench_files(&cfg).unwrap_or_else(|e| fail(&e));
    let exe = std::env::current_exe().unwrap_or_else(|e| fail(&e.to_string()));

    let mut md =
        String::from("| file | MiB | case | backend | peak RSS MiB | over baseline MiB |\n");
    md.push_str("|---|---:|---|---|---:|---:|\n");
    for file in &files {
        for &case in Case::ALL {
            for &backend in Backend::ALL {
                let name = format!("{}/{}/{}", file.label, case.label(), backend.label());
                let cells = match skip_reason(case, backend, file, &cfg) {
                    Some(reason) => format!("skipped: {reason} | |"),
                    None => {
                        let out = Command::new(&exe)
                            .arg("--child")
                            .args([case.label(), backend.label()])
                            .arg(&file.path)
                            .output()
                            .unwrap_or_else(|e| fail(&e.to_string()));
                        let text = String::from_utf8_lossy(&out.stdout);
                        let nums: Vec<f64> = text
                            .split_whitespace()
                            .filter_map(|n| n.parse().ok())
                            .collect();
                        match (out.status.success(), nums.as_slice()) {
                            (true, [base, peak]) => {
                                format!("{:.1} | {:.1} |", peak / MIB, (peak - base) / MIB)
                            }
                            _ => fail(&format!(
                                "{name} failed: {}",
                                String::from_utf8_lossy(&out.stderr)
                            )),
                        }
                    }
                };
                eprintln!("{name}: {cells}");
                let _ = writeln!(
                    md,
                    "| {} | {} | {} | {} | {cells}",
                    file.label,
                    file.bytes >> 20,
                    case.label(),
                    backend.label()
                );
            }
        }
    }
    println!("\n{md}");
    if smoke {
        return;
    }
    let dir = common::bench_data_dir().with_file_name("bench-results");
    let out = dir.join("storage_memory.md");
    match std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&out, &md)) {
        Ok(()) => println!("written to {}", out.display()),
        Err(e) => eprintln!("could not write {}: {e}", out.display()),
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("storage_memory bench: {msg}");
    std::process::exit(2)
}
