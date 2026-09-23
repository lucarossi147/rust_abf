# Test fixtures

| File | Origin | License |
| --- | --- | --- |
| `05210017_vc_abf1.abf` | Pre-existing fixture (predates this file); origin not recorded. | Unknown — please backfill if known. |
| `14o08011_ic_pair.abf` | Pre-existing fixture (predates this file); origin not recorded. | Unknown — please backfill if known. |
| `18425108.abf` | Pre-existing fixture (predates this file); origin not recorded. | Unknown — please backfill if known. |
| `wrong_signature.abf` | Synthetic, hand-authored for this repo (16 bytes: valid UTF-8 `"FAKEjunkdata1234"`, first 4 bytes deliberately not `ABF ` / `ABF2`). Used to test `Abf::from_file`'s invalid-signature error path. | MIT (same as the crate) — not real recorded data. |
| `invalid_utf8_signature.abf` | Synthetic, hand-authored for this repo (16 bytes starting with the invalid UTF-8 sequence `FF FE FD FC`). Used to test `Abf::from_file`'s invalid-signature error path when the header isn't valid UTF-8. | MIT (same as the crate) — not real recorded data. |

All fixtures are under 2 MB.
