# Calibrator: run-15 server-stdout (2026-08-09)

Replacement for the Run B calibrator lost in the driver-9..14 deletion
incident (see `STARVATION-FALLTHROUGH-LIVE-VERIFICATION.md` for that
history). Per Fable's ruling: calibrator regeneration is a deliberate task,
not an opportunistic hope. This is that task, folded into run-15 (the
`#63` `is_loaded` follow-on live re-run) since it was a loaded run with
`BASTION_NEED_SKIP_DIAG` on and despondency firing.

**A count in a note preserves the FACT that a calibrator existed; it
cannot calibrate anything itself.** `logcount.py` needs an actual FILE to
prove a reader can see U+2014. The committed artifact is
`bastion-test-evidence/calibrators/run15-extract.log` (~60 lines, 7,871
bytes) — the first 20 matches of each of `BREAKDOWN` (em-dash),
`no_food_found` (ASCII), and `rest below interrupt` (ASCII), extracted
verbatim from the full run so every byte is real log content, not
synthesized. That file is small enough to commit outright, so it survives
independent of anything else in this directory.

## Source

`bastion-test-evidence/calibrators/run15-server-stdout.log` — copied
verbatim from `bastion-test-evidence/live-playthrough/server-stdout-15.log`
at report time, same run that produced the `#63` read below. Kept on disk,
not committed to git (26.5 MB, ~85x the largest previously-committed raw
log in this tree — see the size table checked before this decision:
prior committed `server-stdout-N.log` files range 29,532–320,893 bytes).
This full copy is disposable now that `run15-extract.log` carries every
class the calibrator needs to prove — the earlier version of this note
treated the byte counts below as sufficient, which was wrong (a fact,
not a function); left here for the full-population counts only.

## Counts (byte-level, `bytes.count()`, no grep/locale involved)

```
bytes:            26,521,251
lines:             84,369
BREAKDOWN (em-dash literal, U+2014, 3-byte UTF-8): 14
reason= (ASCII, BASTION_NEED_SKIP_DIAG field):     38,415
no_food_found (ASCII):                             21,511
```

Computed via:

```python
data = open(r'...\server-stdout-15.log', 'rb').read()
data.count('BREAKDOWN'.encode())   # -> 14
data.count(b'reason=')             # -> 38415
data.count(b'no_food_found')       # -> 21511
```

## Why this is a valid calibrator

Same shape as the lost Run B calibrator: a mixed file with both em-dash
(`BREAKDOWN`, non-ASCII) and pure-ASCII (`reason=`) known-positive counts
in one log, letting a future byte-level reader be checked against a
known-nonzero value on both encodings before trusting a zero elsewhere.
`BREAKDOWN`'s 14 is smaller than Run B's 22 but nonzero and independently
counted here, not recalled from memory.
