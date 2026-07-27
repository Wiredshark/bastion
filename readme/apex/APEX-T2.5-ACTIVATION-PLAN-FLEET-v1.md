# APEX-T2.5 — Fleet spec: activation plan and conflict ownership (MECHANISM)

> **STATUS: DRAFT — pending cross-review.** Author: Builder Opus 5,
> 2026-07-27. Not build-authorized. Chain: author → Sonnet cross-review →
> Fable approval → build. Registry: `specification=FLEET_AUTHORED`.
> Written under the sole-reader mode: grounding = the local row block
> (`E:/apex-rowrefs/row-APEX-T2.5.md`, Fable-provisioned), landed T2.1–T2.4
> seams (mine), and absorbed handoff notes. The Drive prose packet and its
> canary file are INVALID-marked / unpinned — NOT consulted; this row gets
> a FLEET-AUTHORED canary catalog (T1.2 precedent).
>
> **SCOPE = MECHANISM ONLY.** Every production-admission VALUE (limit
> numbers, collision decisions, waiver deadlines, rollout point) is a
> typed `NEEDS-DESIGN` slot for Fable's ruling
> (`PRODUCTION-ADMISSION-POLICY-UNAVAILABLE` per the row). The mechanism
> makes a missing policy BLOCK plugin-enabled startup — no defaults.

## 1. Row contract (from the row block)

One `PluginDeploymentPlanV1` (graph + artifacts + archive policy + global
content), shared identically client/server; three derived mode roots
(`Server`/`Client`/`SinglePlayer` `PluginActivationPlanV1`), each tied to
the deployment root. Activation receipts + actual-registration owner maps;
static claims stay CEILINGS. Duplicate command/body/asset claims detected;
fail-closed unless an explicit override relation names the displaced
provider AND the policy version. Artifacts verified before activation.
Instantiate/register strictly by plan ordinal. Client fetches missing
artifacts by plan ordinal + exact hash. All conflict/shadow decisions live
IN the plan root. Production `PluginArchiveLimitsV1` frozen from the
ObserveLegacy inventory. Legacy handling = one explicit decision
(observe-only | deterministic repack | typed waiver + deadline). Runtime
legacy admission posture is **StrictCanonicalOnly** once rolled out.
Acceptance: a topological order alone can never silently choose a winner;
legacy behavior can never be selected by ambient config or fallback.

## 2. Determinism story

The plan is a PURE FUNCTION of `(ResolvedPluginGraphV1, per-node preflight
results, PolicyRecordV1)`. Publication order = graph ordinals (supersedes
the current hash-ascending `canonical_plugin_order` last-wins on the strict
lane). Conflicts detected over the SORTED declared ceilings (T2.3 §5.7's
conservative rule); every decision is an explicit record in the plan root —
never an emergent property of iteration order. Roots: deployment plan under
`PluginActivationPlan = 3` (pre-registered); mode roots = domain-3 digests
over `(deployment_root, mode tag, mode-filtered activations)`. No
wall-clock, no discovery order, no HashMap iteration anywhere in plan
construction.

## 3. Landed seams consumed (live code, mine)

`resolver::ResolvedPluginGraphV1` (primary input; ordinals + roots) ·
`manifest::ValidatedPluginManifestV1.claims` (ceilings) + its four
DEFERRED-* terminals this row closes mechanically ·
`archive_profile::admit_strict_canonical(.., rollout_policy)` — the PAR-C14
gate whose VALUE TYPE this row defines (`StrictRolloutPolicyV1`) ·
`ObserveSummaryV1` per inspected archive (rollout evidence) · T2.1
`PreparedPluginBatch` one-commit publication seam +
`CombinedCache::prepare_tar/commit_prepared_tars` · Wasmtime world probe
region (`module.rs` load path) for the preflight seam.

## 4. Mechanism policies (cross-review targets)

1. **Owner maps**: for each publishable name-space (commands, bodies,
   skeleton/animation providers, asset paths) the plan carries
   `owner: PluginNodeKeyV1` per name, derived from ceilings + conflict
   decisions. Actual guest registrations at runtime are checked against
   the owner map: out-of-ceiling ⇒ typed reject (closes
   DEFERRED-RUNTIME-CLAIM-CHECK); in-ceiling-but-not-owner ⇒ typed
   shadow record (never silent last-wins).
2. **Conflict relation**: a duplicate claim across nodes is
   `ConflictV1 { name, claimants (sorted), decision }` where decision ∈
   {`FailClosed`, `Override { winner, displaced, policy_version }`}.
   V1 mechanism ships `FailClosed` as the only constructible decision;
   `Override` requires a `NEEDS-DESIGN` policy record (Fable) — the TYPE
   exists, no value can be minted by the mechanism.
3. **Preflight**: per node, per declared module, the component's ACTUAL
   world is probed against the DECLARED `PluginModuleWorldV1` before any
   publication (closes DEFERRED-COMPONENT-WORLD-CHECK). Mismatch =
   per-node typed block; the plan records preflight results by ordinal.
4. **Artifact verification**: node's archive bytes re-hashed against
   `archive_artifact` at activation time; mismatch = typed block before
   instantiate (the plan is bound to exact bytes, not names).
5. **PolicyRecordV1** (the NEEDS-DESIGN carrier): archive limits policy id
   + resolver limits + rollout point + legacy-handling decision + override
   relations. EVERY field is mandatory; absence ⇒
   `BLOCK-ADMISSION-POLICY-MISSING` and plugin-enabled startup refuses.
   The record's root is embedded in the deployment root, so a policy
   change moves every plan.
6. **Client artifact fetch**: request list derived from the plan (ordinal
   + exact artifact hash); received bytes verified against the hash before
   cache admission (rides the landed DET-AST-030 atomic cache).

## 5. Data model (field IDs frozen at cross-review)

```rust
// common/state/src/plugin/activation.rs (NEW)
struct PolicyRecordV1 {            // 0 schema, 1 archive_limits_policy,
                                   // 2 resolver_limits, 3 rollout,
                                   // 4 legacy_decision, 5 overrides
    ..every field mandatory; root under domain 3..
}
enum LegacyDecisionV1 { ObserveOnly, DeterministicRepackRequired,
    Waiver { terminal: MachineTextV1, deadline: MachineTextV1 } }
enum StrictRolloutPolicyV1 { NotRolledOut, StrictCanonicalOnly { policy_record_root } }
struct ConflictV1 { name, kind, claimants: Vec<PluginNodeKeyV1>, decision }
struct OwnerMapV1 { kind, entries: Vec<(name, PluginNodeKeyV1)> }  // sorted
struct NodePreflightV1 { ordinal, module_results: Vec<(path, declared, actual, ok)> }
struct PluginDeploymentPlanV1 {
    graph_root, policy_record_root, conflicts (sorted), owner_maps,
    preflights (by ordinal), artifact list (ordinal + identity),
    deployment_root,               // domain 3, canonical_root() pattern
}
struct PluginActivationPlanV1 { mode, deployment_root, activations (ordinal-
    ordered, mode-filtered), mode_root }
struct ActivationReceiptV1 { ordinal, key, registrations (actual, sorted),
    within_ceiling: bool, shadows: Vec<name> }
```

Timestamps: none in any root (established pattern). Mode tags: fixed u16
(Server=0, Client=1, SinglePlayer=2 — matches `PluginClaimModeV1`).

## 6. Typed terminals

`PLAN-READY` · `BLOCK-ADMISSION-POLICY-MISSING` · `BLOCK-CONFLICT-FAIL-
CLOSED` (carries the sorted conflict list) · `BLOCK-PREFLIGHT-WORLD-
MISMATCH` · `BLOCK-ARTIFACT-VERIFICATION` · `BLOCK-OUT-OF-CEILING-
REGISTRATION` · `SHADOWED-PROVIDER-RECORDED` (non-fatal, in receipts) ·
`BLOCK-LEGACY-AMBIENT-SELECTION` (legacy reachable only via the explicit
decision) · `BLOCK-PLAN-ROOT-MISMATCH` (client/server deployment roots
differ). Fleet-authored canary catalog (~24 cases) built with the row:
every terminal bites + owner-map determinism under permutation + the
topological-order-cannot-choose-winner acceptance as an invariant canary.

## 7. Minute steps

.01 types + PolicyRecordV1 + roots (unit) · .02 conflict detection over
ceilings + owner maps (unit, permutation-proven) · .03 preflight seam
(Wasmtime world probe pre-publication; feature-gated tests) · .04 plan
construction (graph→plan pure fn; deployment + mode roots) · .05 receipts
+ ceiling enforcement at the registration sites · .06 artifact
verification + client fetch-by-hash derivation · .07 wiring: strict-lane
plan path behind `StrictRolloutPolicyV1` (production stays legacy until a
policy record exists — PAR-C14 finally consumable) · .08 fleet canary
catalog + runner (T2.2/T2.3/T2.4 pattern).

## 8. Non-goals

Policy VALUES (Fable) · network transfer protocol changes (fetch uses the
existing plugin-download path) · multi-version graphs (T2.4 policy 5.1
stands) · live migration of running plugins.
