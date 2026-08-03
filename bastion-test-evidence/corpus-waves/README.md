# Corpus-wave baselines (canonical copy, rescued off C: temp 2026-08-03)
One JSON per fan wave: seed -> full b5 report. Anchor: wave18_FULL.json = 12/48
failures @ a057ed66 (current baseline; chopfell fix 15850c61cc is harness-only).
wave13_EMPTY_zone-exhausted-zero-seeds.json is NOT DATA: that fan lost all 6 VMs
to ZONE_RESOURCE_POOL_EXHAUSTED and delivered zero seeds. It is renamed out of
the wave*_FULL.json glob so no comparison silently ingests an empty dict —
"couldn't measure" must never share a shape with "measured nothing."
