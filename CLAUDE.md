# CLAUDE.md — rules for agents working on rust_abf

rust_abf reads Axon Binary Format (ABF) electrophysiology files. Current: 0.6.0; target: production ready.
Goals, in order: correct data, no panics on any input, low memory, speed.

## How work is organised

- Every task is a GitHub issue whose title starts with a backlog ID: `[F1]`, `[F2]`, `[F3]`, `[1]` … `[20]`.
  "Depends on" lines in issue bodies refer to these IDs, not to GitHub issue numbers.
- Before starting: check that every issue listed under "Depends on" is **closed**. If one is still open,
  comment "Blocked by [ID]" on the issue and stop.
- Open a PR whose description starts with `Closes #<github issue number>` and lists what was tested.

## Rules

- **One issue = one PR.** Branch `issue-<n>-<slug>`. Don't touch code outside the issue's scope; open a new issue instead.
- **TDD, visible in history.** Commit 1: failing test(s) that reproduce the bug or specify the feature (`test: …`). Commit 2+: implementation (`fix: …` / `feat: …`). CI must be red on commit 1 and green on the last one.
- **Coverage:** total line coverage ≥ 80% (enforced by CI, `cargo llvm-cov --fail-under-lines 80`). New/changed lines in the PR should be covered; never lower the total.
- **Must pass:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`.
- **Performance guard:** if the PR touches `src/` data paths, run `cargo bench` before/after and paste the numbers in the PR description. No regressions > 5% without justification.
- **Dependencies:** no new runtime dependency without a justification in the PR description. Dev-dependencies are fine.
- **Reference implementation:** pyABF (https://github.com/swharden/pyABF) is the source of truth for format semantics. When in doubt, match pyABF and cite the line.
- **Fixtures:** only add files < 2 MB, record origin + license in `tests/test_abf/README.md`.
- **CHANGELOG.md:** add a line under `## [Unreleased]` for every user-visible change.
- If blocked or the issue is ambiguous, comment on the issue with the specific question and stop.


## Commands

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo llvm-cov --fail-under-lines 80
cargo bench                            # compare against BENCHMARKS.md
```

## Codebase map (0.6.0)

- `src/lib.rs` — `Abf` public type, `from_file` / `from_bytes`, accessors, all-channels API
- `src/error.rs` — `AbfError` (every fallible API returns `Result<_, AbfError>`)
- `src/abf_v2.rs` — ABF2 header parsing, builds channels
- `src/abf_v2/section.rs`, `src/abf_v2/section/*` — typed section readers (protocol, ADC, strings, synch array, data)
- `src/byte_reader.rs` — bounds-checked little-endian `ByteReader`
- `src/storage.rs` — `Storage`: file bytes (mmap or `Arc<[u8]>`) shared by all channels, decoded lazily
- `src/channel.rs` — `Channel` (per-channel sweeps, scaling, `read_sweep_into`)
- `src/decode.rs` — sample-decoding kernels (i16/f32, strided de-interleave)
- `src/parallel.rs` — rayon-backed decoding behind the optional `parallel` feature
- `tests/*.rs` (shared helpers in `tests/common/`), golden pyABF data in `tests/golden/`, fixtures in `tests/test_abf/`
- `benches/read.rs` — Criterion benchmarks; baselines in `BENCHMARKS.md`
