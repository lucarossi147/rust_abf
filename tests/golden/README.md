# Golden reference tests

These JSON files are a numeric oracle generated from
[pyABF](https://github.com/swharden/pyABF), the reference implementation for
the ABF format. `tests/golden.rs` loads them and compares the values against
what rust_abf produces for the same fixtures in `tests/test_abf/`, so that
future refactors (Phase 2) can be checked against known-correct numbers
instead of just lengths and labels.

## What's in each file

For every `<stem>.abf` in `tests/test_abf/` that pyABF can open, there is a
`<stem>.json` with:

- Header metadata: `abfVersion`, `nDataFormat`, `nOperationMode`,
  `sweepCount`, `channelCount`, `dataRate`, `sweepPointCount`.
- Per channel: `adcName`, `adcUnit`.
- Per (channel, sweep): the first and last 32 values of `sweepY`, plus its
  `sum`/`min`/`max`, and the first/last 32 values of `sweepX`.

Only summaries are stored (not full sweeps) to keep the fixtures small while
still catching scaling, offset, and off-by-one bugs at both ends of each
sweep.

`nDataFormat` is read from pyABF's private `abf._nDataFormat` attribute —
pyABF does not expose a public equivalent.

## Regenerating

```sh
pip install -r scripts/requirements.txt
python3 scripts/gen_golden.py
```

This overwrites every `tests/golden/*.json` file whose source fixture pyABF
can still open. Commit the diff. CI (the `golden` job) re-runs this script and
fails with `git diff --exit-code tests/golden` if the committed JSON is stale
relative to the fixtures.

## Known-wrong behaviour

Some golden assertions describe the *correct* behaviour per pyABF but
currently fail against rust_abf. These are marked `#[ignore = "fixed by
#<issue>"]` in `tests/golden.rs` so the fix for that issue is just removing
the `#[ignore]`:

- `golden_abf1_05210017` — ABF1 support is not implemented yet
  (`Abf::from_file` hits a `todo!()` for the `"ABF "` signature); see
  `[20] ABF1 support`.
- `golden_time_axis_matches_pyabf_for_multi_sweep_file` —
  `Abf::get_time_axis()` divides by `sweeps_count` twice for multi-sweep
  files, since `get_sweep_in_channel` already returns per-sweep data; see
  `[2] get_time_axis() is wrong for multi-sweep files`.
- `golden_get_channels_preserves_adc_order` — `Abf::get_channels()` iterates
  a `HashMap`, so channel order is not guaranteed to match ADC order (and is
  not stable across runs, since Rust randomizes the hasher seed per process);
  see `[1] get_channels() returns channels in random order`.
