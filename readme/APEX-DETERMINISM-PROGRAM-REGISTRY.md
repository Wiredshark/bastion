# APEX Determinism Program Registry

Generated human projection of `APEX-DETERMINISM-PROGRAM-REGISTRY-v1.json`.
Implements `APEX-A.3`. **Never hand-edit this file** — regenerate from the JSON.

**Rows:** 55  **Findings:** 24  **Unresolved row references:** 0  
**Canonical guide:** `PROJECT-BASTION-APEX-DETERMINISM-STEP-BY-STEP-MASTER-BUILD-ORDER.md`  
**Finding matrix:** `readme/apex/APEX-FINDING-STATUS-MATRIX-v1.csv`  
**Last live commit checked:** `d1b8948369e00680f193a6935f52f66086aff0fa`

---

## Resolved: two spec defects found and fixed by Fable's ruling (2026-07-26)

The validator originally caught two real defects in the canonical guide, both intentionally left RED rather than silently worked around (see git history commit `8363d0fea7`). Both are now fixed by an explicit architect ruling, folded into this registry:

1. **`ORDER_VIOLATION` (was): `APEX-T4.3` depended on `APEX-T6.2`, which came later in the canonical guide's top-to-bottom sequence.** Fixed by **splitting T4.3** into `APEX-T4.3a` (manifest structure + world seed + worldgen/content protocol identities + site identity/origin/kind root; depends only on `T0.5`; kept the original tier position) and `APEX-T4.3b` (geometry root + economy baseline root; depends on `T6.2`; re-sequenced to after Tier 6). `APEX-T4.5` was re-scoped to depend on `T4.3a` only (schema-compatibility checking does not need `T4.3b`'s numerically-verified root values), which is what actually resolves the cascading order constraint rather than just moving it downstream.
2. **`UNRESOLVED_ROW_REFERENCE` (was): `APEX-T5.5` was cited by three findings (`DET-WTH-010`, `DET-PRD-008`, `DET-PRD-011`) but did not exist anywhere in the canonical guide.** Fixed by adding `APEX-T5.5` as a frozen `GUIDE_MISSING_ROW` placeholder row (empty dependencies, fixed title, no fabricated content) per the architect's ruling: content recovery is routed to the guide's author via Ben, never reconstructed by a builder. The validator's `check_guide_missing_row_fingerprints` re-flags (`GUIDE_MISSING_ROW_FINGERPRINT_DRIFT`) if this row's frozen title/dependencies ever change without an explicit registry-edit commit — the same tracked-red-with-a-fingerprint pattern used elsewhere in this project (M3A floor).

---

## Row sequence and dependency graph

| Seq | Row | Title | Hard dependencies | Findings | Status (spec/research/impl/verify/deploy) |
|---|---|---|---|---|---|
| 1 | `APEX-A.1` | Source-current admission | — | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 2 | `APEX-A.2` | Live implementation-status matrix | `APEX-A.1` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 3 | `APEX-A.3` | Program registry and bidirectional traceability | `APEX-A.1`, `APEX-A.2` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 4 | `APEX-T0.1` | Canonical fixed-width scalars and semantic identifier foundations | `APEX-A.3` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 5 | `APEX-T0.2` | BastionManifestEncodingV1 deterministic CBOR profile | `APEX-A.3`, `APEX-T0.1` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 6 | `APEX-T0.3` | Domain-separated digests and content identity | `APEX-A.3`, `APEX-T0.1` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 7 | `APEX-T0.4` | Authoritative lifecycle-identity foundations | `APEX-T0.1`, `APEX-T0.2`, `APEX-T0.3` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 8 | `APEX-T0.5` | Subsystem descriptor / compatibility-profile registry | `APEX-T0.1`, `APEX-T0.2`, `APEX-T0.3`, `APEX-T0.4` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 9 | `APEX-T1.1` | Nix harness package, source-neutral VM | `APEX-A.3` | `DET-BLD-032` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 10 | `APEX-T1.2` | Declared source/asset closure | `APEX-A.3` | `DET-BLD-032` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 11 | `APEX-T1.3` | Local reproducibility smoke test | `APEX-T1.1`, `APEX-T1.2` | `DET-BLD-032` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 12 | `APEX-T1.4` | Fresh-environment rebuild pair | `APEX-T1.1`, `APEX-T1.2` | `DET-BLD-032` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 13 | `APEX-T1.5` | Separate build/artifact/execution evidence | `APEX-T1.1`, `APEX-T1.2`, `APEX-T1.3`, `APEX-T1.4` | `DET-BLD-032` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 14 | `APEX-T2.1` | Two-phase plugin loading substrate | `APEX-A.3` | `DET-AST-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 15 | `APEX-T2.2` | Canonical plugin archive profile | `APEX-A.3` | `DET-AST-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 16 | `APEX-T2.3` | PluginManifestV1 | `APEX-T0.2`, `APEX-T0.3`, `APEX-T2.1`, `APEX-T2.2` | `DET-AST-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 17 | `APEX-T2.4` | Canonical plugin dependency DAG | `APEX-T2.3` | `DET-AST-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 18 | `APEX-T2.5` | PluginDeploymentPlanV1 / PluginActivationPlanV1 | `APEX-T2.1`, `APEX-T2.2`, `APEX-T2.3`, `APEX-T2.4` | `DET-AST-026`, `DET-AST-028` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 19 | `APEX-T3.1` | Server boot-scoped authority (ServerBootId) | `APEX-A.3` | `DET-NET-013`, `DET-NET-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 20 | `APEX-T3.2` | Authenticated logical sessions / ConnectionEpoch | `APEX-T3.1` | `DET-NET-022`, `DET-NET-024`, `DET-NET-025`, `DET-NET-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 21 | `APEX-T3.3` | Semantic NetEnvelopeV1 | `APEX-T3.1`, `APEX-T3.2` | `DET-NET-006`, `DET-NET-008`, `DET-NET-009`, `DET-NET-013`, `DET-NET-022`, `DET-NET-024`, `DET-NET-026` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 22 | `APEX-T3.4` | Cross-stream checkpoint watermarks | `APEX-T3.3` | `DET-NET-006`, `DET-NET-009`, `DET-NET-013` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 23 | `APEX-T3.5` | Boot-scoped command idempotency (CommandId) | `APEX-T3.3`, `APEX-T3.4` | `DET-NET-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 24 | `APEX-T3.6` | PhysicsGeneration correction fencing | `APEX-T3.3` | `DET-PRD-014` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 25 | `APEX-T4.1` | BootstrapManifestV1 | `APEX-T0.5`, `APEX-T1.5`, `APEX-T2.5`, `APEX-T3.3` | `DET-AST-028`, `DET-NET-025`, `DET-NET-033` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 26 | `APEX-T4.2` | Bootstrap freshness / anti-rollback binding | `APEX-T3.1`, `APEX-T4.1` | `DET-NET-033` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 27 | `APEX-T4.3a` | WorldBaselineManifestV1: manifest structure + world seed + worldgen/content protocol identities + site identity/origin/kind root | `APEX-T0.5` | `DET-SVC-021` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 28 | `APEX-T4.4` | NonAuthoritativeSaveInventoryV1 sidecars | `APEX-T0.5` | `DET-SVC-021` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 29 | `APEX-T4.5` | Historical save corpus + migration policy | `APEX-T4.3a`, `APEX-T4.4` | `DET-SVC-021` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 30 | `APEX-T4.6` | SaveUniverseManifestV1 staged epoch commit | `APEX-T4.5` | `DET-SVC-021` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 31 | `APEX-T5.1` | Server-authoritative physics cohort lane | `APEX-T3.1`, `APEX-T3.2`, `APEX-T3.3` | `DET-PRD-008`, `DET-WTH-010` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 32 | `APEX-T5.2` | ClientInputFrameV1 | `APEX-T3.3`, `APEX-T3.6` | `DET-NET-008`, `DET-PRD-001`, `DET-PRD-005`, `DET-PRD-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 33 | `APEX-T5.3` | InputReceiptV1 / PlayerPredictionProbeV1 | `APEX-T5.2` | `DET-PRD-005` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 34 | `APEX-T5.4` | Tick-identified glider/weather prediction input | `APEX-T3.4`, `APEX-T5.2` | `DET-PRD-005`, `DET-WTH-010` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 35 | `APEX-T5.5` | RESERVED (GUIDE_MISSING_ROW) -- no APEX-T5.5 packet exists in the canonical guide; content TBD from source, routed to guide author | — | `DET-PRD-008`, `DET-PRD-011`, `DET-WTH-010` | GUIDE_MISSING_ROW/NOT_APPLICABLE/BLOCKED_ON_MISSING_SPEC/NOT_STARTED/NOT_DEPLOYED |
| 36 | `APEX-T6.1` | Numeric attack-surface inventory | `APEX-T0.5` | `DET-PHY-008`, `DET-PHY-024`, `DET-WTH-003` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 37 | `APEX-T6.2` | Dual raw/semantic-quantized state-digest probes | `APEX-T0.2`, `APEX-T0.3` | `DET-PHY-024`, `DET-WTH-003` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 38 | `APEX-T6.3` | Canonicalize PHY-008 collision candidate/contribution order | `APEX-T6.1` | `DET-PHY-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 39 | `APEX-T6.4` | NumericProfileV1 empirical certified profile | `APEX-T1.5`, `APEX-T6.2` | `DET-WTH-003` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 40 | `APEX-T6.5` | Selective deterministic transcendental kernels | `APEX-T6.1`, `APEX-T6.2`, `APEX-T6.3`, `APEX-T6.4` | `DET-PHY-024` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_EVIDENCE_GATE/NOT_STARTED/NOT_DEPLOYED |
| 41 | `APEX-T6.6` | Authoritative quantization at commit boundaries | `APEX-T6.2` | `DET-PHY-024`, `DET-PRD-011`, `DET-WTH-003` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_EVIDENCE_GATE/NOT_STARTED/NOT_DEPLOYED |
| 42 | `APEX-T7.1` | Local-player prediction-state kernel scope | — | `DET-PRD-008`, `DET-PRD-011` | NEEDS_DESIGN/NOT_APPLICABLE/BLOCKED_ON_DESIGN/NOT_STARTED/NOT_DEPLOYED |
| 43 | `APEX-T7.2` | Shared fixed-tick locomotion/orientation/glider kernel | `APEX-T5.1`, `APEX-T5.2`, `APEX-T5.3`, `APEX-T5.4`, `APEX-T6.4` | `DET-PRD-014` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 44 | `APEX-T7.3` | Prediction-history ring | `APEX-T7.2` | `DET-PRD-014` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 45 | `APEX-T7.4` | Authoritative-correction reconciliation/replay | `APEX-T7.3` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 46 | `APEX-T7.5` | Bounded fallback to full authoritative snap | `APEX-T7.4` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 47 | `APEX-T4.3b` | WorldBaselineManifestV1: geometry root + economy baseline root | `APEX-T6.2` | `DET-ESIM-007` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 48 | `APEX-T8.1` | Per-site/per-phase economy state-digest instrumentation | `APEX-T0.2`, `APEX-T0.3` | `DET-ESIM-007`, `DET-ESIM-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 49 | `APEX-T8.2` | Lane A: numeric-portability isolation | `APEX-T8.1` | `DET-ESIM-007` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 50 | `APEX-T8.3` | Lane B: order-sensitivity isolation | `APEX-T8.1` | `DET-ESIM-007`, `DET-ESIM-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 51 | `APEX-T8.4` | Lane C: quantization/ULP model-sensitivity isolation | `APEX-T8.1` | `DET-ESIM-007`, `DET-ESIM-008` | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/NOT_STARTED/NOT_STARTED/NOT_DEPLOYED |
| 52 | `APEX-T8.5` | Select smallest justified economy numeric remedy | `APEX-T8.2`, `APEX-T8.3`, `APEX-T8.4` | — | NEEDS_DESIGN/NOT_APPLICABLE/BLOCKED_ON_DESIGN/NOT_STARTED/NOT_DEPLOYED |
| 53 | `APEX-T9.1` | Full-bootstrap reconnect under new connection epoch | `APEX-T3.1`, `APEX-T3.2`, `APEX-T3.3`, `APEX-T3.4`, `APEX-T4.1`, `APEX-T4.2` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 54 | `APEX-T9.2` | Explicit save rollback / UniverseBranchId authorization | `APEX-T4.6` | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/BLOCKED_ON_PREREQUISITE/NOT_STARTED/NOT_DEPLOYED |
| 55 | `APEX-T9.3` | Complete apex campaign certification | — | — | SPECIFICATION_COMPLETE/MICROSTEP_RESEARCH_COMPLETE/DEFERRED/NOT_STARTED/NOT_DEPLOYED |

---

## Finding coverage

| Finding | Status | Closure rule | Last checked |
|---|---|---|---|
| `DET-AST-026` | PARTIAL | AllOf(`APEX-T2.1`, `APEX-T2.2`, `APEX-T2.3`, `APEX-T2.4`, `APEX-T2.5`) | `d1b8948369e0` |
| `DET-AST-028` | PARTIAL | AllOf(`APEX-T2.5`, `APEX-T4.1`) | `d1b8948369e0` |
| `DET-BLD-032` | PARTIAL | AllOf(`APEX-T1.1`, `APEX-T1.2`, `APEX-T1.3`, `APEX-T1.4`, `APEX-T1.5`) | `d1b8948369e0` |
| `DET-ESIM-007` | OPEN | AllOf(`APEX-T4.3b`, `APEX-T8.1`, `APEX-T8.2`, `APEX-T8.3`, `APEX-T8.4`) | `d1b8948369e0` |
| `DET-ESIM-008` | OPEN | AllOf(`APEX-T8.1`, `APEX-T8.3`, `APEX-T8.4`) | `d1b8948369e0` |
| `DET-NET-006` | OPEN | AllOf(`APEX-T3.3`, `APEX-T3.4`) | `d1b8948369e0` |
| `DET-NET-008` | OPEN | AllOf(`APEX-T3.3`, `APEX-T3.5`, `APEX-T5.2`) | `d1b8948369e0` |
| `DET-NET-009` | OPEN | AllOf(`APEX-T3.3`, `APEX-T3.4`) | `d1b8948369e0` |
| `DET-NET-013` | OPEN | AllOf(`APEX-T3.1`, `APEX-T3.3`, `APEX-T3.4`) | `d1b8948369e0` |
| `DET-NET-022` | SUPERSEDED | SupersededBy(`APEX-T3.2`, `APEX-T3.3`) | `d1b8948369e0` |
| `DET-NET-024` | SUPERSEDED | SupersededBy(`APEX-T3.2`, `APEX-T3.3`) | `d1b8948369e0` |
| `DET-NET-025` | PARTIAL | AllOf(`APEX-T3.2`, `APEX-T4.1`) | `d1b8948369e0` |
| `DET-NET-026` | OPEN | AllOf(`APEX-T3.1`, `APEX-T3.2`, `APEX-T3.3`) | `d1b8948369e0` |
| `DET-NET-033` | PARTIAL | AllOf(`APEX-T4.1`, `APEX-T4.2`) | `d1b8948369e0` |
| `DET-PHY-008` | OPEN | AllOf(`APEX-T6.1`, `APEX-T6.3`) | `d1b8948369e0` |
| `DET-PHY-024` | OPEN | AllOf(`APEX-T6.1`, `APEX-T6.2`, `APEX-T6.5`, `APEX-T6.6`) | `d1b8948369e0` |
| `DET-PRD-001` | OPEN | Row(`APEX-T5.2`) | `d1b8948369e0` |
| `DET-PRD-005` | OPEN | AllOf(`APEX-T5.2`, `APEX-T5.3`, `APEX-T5.4`) | `d1b8948369e0` |
| `DET-PRD-008` | PARTIAL | AllOf(`APEX-T5.1`, `APEX-T5.2`, `APEX-T5.5`, `APEX-T7.1`) | `d1b8948369e0` |
| `DET-PRD-011` | PARTIAL | AllOf(`APEX-T5.5`, `APEX-T6.6`, `APEX-T7.1`) | `d1b8948369e0` |
| `DET-PRD-014` | OPEN | AllOf(`APEX-T3.6`, `APEX-T7.2`, `APEX-T7.3`) | `d1b8948369e0` |
| `DET-SVC-021` | OPEN | AllOf(`APEX-T4.3a`, `APEX-T4.4`, `APEX-T4.5`, `APEX-T4.6`) | `d1b8948369e0` |
| `DET-WTH-003` | OPEN | AllOf(`APEX-T6.1`, `APEX-T6.2`, `APEX-T6.4`, `APEX-T6.6`) | `d1b8948369e0` |
| `DET-WTH-010` | OPEN | AllOf(`APEX-T5.1`, `APEX-T5.4`, `APEX-T5.5`) | `d1b8948369e0` |

