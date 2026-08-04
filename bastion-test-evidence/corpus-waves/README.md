# Corpus-wave baselines (canonical copy, rescued off C: temp 2026-08-03)
One JSON per fan wave: seed -> full b5 report. Anchor: wave18_FULL.json = 12/48
failures @ a057ed66 (current baseline; chopfell fix 15850c61cc is harness-only).
wave13_EMPTY_zone-exhausted-zero-seeds.json is NOT DATA: that fan lost all 6 VMs
to ZONE_RESOURCE_POOL_EXHAUSTED and delivered zero seeds. It is renamed out of
the wave*_FULL.json glob so no comparison silently ingests an empty dict �
"couldn't measure" must never share a shape with "measured nothing."

## ★ PROVENANCE: filenames are labels — attested COMMIT= wins (checked 2026-08-04)

All six attestable waves were re-checked against their filenames. **Five match;
one does not — the problem is a one-off, not systemic:**

| artifact | filename says | attested `COMMIT=` | verdict |
|---|---|---|---|
| wave19 | `ed532c600e` | `ed532c60` | match |
| wave20 | `d010339a55` | `d010339a` | match |
| **wave21** | **`ed532c600e`** | **`1bf3ab2e`** | **MISMATCH** |
| wave22 | `34db70bac2` | `34db70ba` | match |
| wave23 | `b89cbc799d` | `b89cbc79` | match |
| wave24 | `d3235e5329` | `d3235e53` | match |

**wave21 ran on `1bf3ab2e1c`, not `ed532c600e`.** Resolved, and the comparison
is still sound: `1bf3ab2e1c` is exactly ONE commit ahead of `ed532c600e`, and
that commit changes **one markdown file (+24 lines) and ZERO code files** —
verified with `git diff --name-only ed532c600e 1bf3ab2e | grep -E '\.(rs|toml|ron)$'`
→ empty. Binaries are behaviourally identical, so **wave19 vs wave21 remains a
valid same-code determinism comparison.**

**Cite it as `1bf3ab2e` (docs-only ahead of `ed532c600e`), not as
`ed532c600e`.** An audit that trusted the filename would have recorded the
wrong provenance for the run that anchors our determinism claim.

**Rule this instance re-earns:** verify the attested commit before *citing* a
wave, not only before reading one. The mismatch was invisible for a day because
nothing compares the label to the attestation, and the label is what gets
quoted. See [[read-the-content-not-the-label]] — filenames are labels, and this
directory already carries one such trap (`wave13`, above).
