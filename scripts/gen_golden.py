#!/usr/bin/env python3
"""Generate golden reference JSON files from pyABF.

pyABF (https://github.com/swharden/pyABF) is the reference implementation used
as a numeric oracle for rust_abf. For every ``.abf`` file in ``tests/test_abf/``
that pyABF can open, this script writes ``tests/golden/<stem>.json`` containing
header metadata and, for every (channel, sweep) pair, a compact numeric summary
(first/last 32 samples, sum, min, max) of ``sweepY``/``sweepX``.

``tests/golden.rs`` loads these files and compares them against rust_abf's own
output. Regenerate with:

    pip install -r scripts/requirements.txt
    python3 scripts/gen_golden.py
"""
import json
import sys
from pathlib import Path

import numpy as np
import pyabf

REPO_ROOT = Path(__file__).resolve().parent.parent
FIXTURES_DIR = REPO_ROOT / "tests" / "test_abf"
GOLDEN_DIR = REPO_ROOT / "tests" / "golden"


def series_summary(values: np.ndarray) -> dict:
    values = np.asarray(values, dtype=np.float64)
    return {
        "first32": [float(v) for v in values[:32]],
        "last32": [float(v) for v in values[-32:]],
        "sum": float(values.sum()),
        "min": float(values.min()),
        "max": float(values.max()),
    }


def x_summary(values: np.ndarray) -> dict:
    values = np.asarray(values, dtype=np.float64)
    return {
        "first32": [float(v) for v in values[:32]],
        "last32": [float(v) for v in values[-32:]],
    }


def golden_for(path: Path) -> dict:
    abf = pyabf.ABF(str(path))

    channels = []
    for ch in abf.channelList:
        sweeps = []
        for sweep in abf.sweepList:
            abf.setSweep(sweep, channel=ch)
            sweeps.append(
                {
                    "sweep": int(sweep),
                    "sweepY": series_summary(abf.sweepY),
                    "sweepX": x_summary(abf.sweepX),
                }
            )
        channels.append(
            {
                "adcName": abf.adcNames[ch],
                "adcUnit": abf.adcUnits[ch],
                "sweeps": sweeps,
            }
        )

    return {
        # pyABF only exposes nDataFormat as a private attribute; there is no
        # public equivalent, see tests/golden/README.md.
        "abfVersion": abf.abfVersion,
        "nDataFormat": abf._nDataFormat,
        "nOperationMode": abf.nOperationMode,
        "sweepCount": abf.sweepCount,
        "channelCount": abf.channelCount,
        "dataRate": abf.dataRate,
        "sweepPointCount": abf.sweepPointCount,
        "channels": channels,
    }


def main() -> int:
    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    written = []
    for path in sorted(FIXTURES_DIR.glob("*.abf")):
        try:
            data = golden_for(path)
        except Exception as exc:  # pyABF can't open every fixture (e.g. deliberately invalid ones)
            print(f"skip {path.name}: pyABF could not open it ({exc})", file=sys.stderr)
            continue
        out_path = GOLDEN_DIR / f"{path.stem}.json"
        with out_path.open("w") as f:
            json.dump(data, f, indent=2, sort_keys=True)
            f.write("\n")
        written.append(out_path)
        print(f"wrote {out_path.relative_to(REPO_ROOT)}")

    if not written:
        print("no golden files written", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
