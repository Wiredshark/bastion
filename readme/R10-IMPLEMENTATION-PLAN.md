# R10 fencing-token — builder implementation plan (post-extraction, pre-M3)

Prior art: distributed-lock fencing tokens (the triage's framing is right, and this block's
own history is the proof-of-need: the stale-release, the delayed-writer, the re-engage loops
were all stale-authority classes we hunted on tape after the fact).

## Where things live
- `TraversalAuthority { link_id: u64, epoch: u64, member: Uid }` → `bastion_traversal.rs`
  (next to BastionTraversalTask, which gains `epoch: u64` — the epoch it was created under).
- The authoritative epoch store → the JobBoard: `link_epochs: HashMap<u64, u64>` (bastion_jobs).
  `fn current_epoch(&self, link: u64) -> u64` (absent = 0).
- The validity predicate → `bastion_traversal.rs`:
  `fn authority_valid(current_epoch: u64, current_member: Option<Uid>, a: &TraversalAuthority) -> bool`
  (pure — unit-pinnable, same discipline as the release-decision extraction).

## ★ DESIGN FLAG (disagreement with the triage's seam, raised now while cheap)
The triage names `sys/agent/mod.rs` as "the controller-write boundary." That is the VANILLA
agent's boundary — but the OWNED traversal writers this block built do NOT pass through it:
the task phases write `controller.inputs` directly in the climb_iter loop, and the approach
corridor writes controller inputs directly in the upkeep loop (the layer-1 fix). The fence
must therefore live at a SHARED HELPER every owned movement-writer calls:
`fenced_movement_write(board, authority, controller, move_dir, move_z) -> bool` in
bastion_traversal.rs — validate-then-write, one choke point, rejected writes logged with the
stale tuple (the forensics field R10 promises). sys/agent stays untouched (vanilla writes are
suppressed during owned modes by the existing suppressor; fencing them is redundant).
Counter-proposal to confirm with the architect at packet time: fence = the shared helper at
the bastion write sites, NOT a sys/agent hook.

## Epoch advance-sites (enumerated from the release-decision work — I own all of them now)
Advance (release-class events): partial-route Abort leg; FullExit lost-member abort;
verified dismount (safely_dismounted); the (ii) exhausted-replan release; the (B)-bound
exhaustion release; the failsafe delivery; route teardown (cleanup_pending drain); M3's
queue re-election (when R9 lands). A NEW reservation ADOPTS the current epoch (does not
advance — advancing on acquire would fence the acquirer's own writes).

## R9-ordering nuance (flag)
Today `link_id` = the route OWNER's uid (task.link_id = owner.0.get()); persistent link
identity independent of the owner arrives with R9/M3. R10-before-M3 keys `link_epochs` by
the current owner-derived id and migrates to persistent link ids in the R9 fold — the epoch
semantics are unchanged by the migration (per-link monotone counter), so building R10 first
stays safe.

## Fixture proof (the un-fakeable episode)
N-FENCE: capture a live TraversalAuthority mid-climb via a read hook, force an abort
(N1B-style stimulus), then have the harness attempt a movement write with the CAPTURED
(stale) tuple through the fenced helper via a PERMITTED-TOUCH test hook — assert the write
is REJECTED (controller inputs unchanged, rejection logged) and the member's subsequent
legitimate re-engagement (new epoch) writes fine. Plus the unit truth-table on
authority_valid. CHESS-style seam perturbation = the later R12-adjacent infra, not the
first landing.

## Recorder additive
`ownership_epoch` on every owned writer event + the rejection event (`stale-write-rejected`,
carrying both tuples) — additive v2 fields per the Q6 ruling, landed alongside.

## Sequencing (architect-ruled)
1. Release-decision extraction (prepared patch, same file/lane) → 2. THIS (R10) → 3. M3
(R9-folded packet). R11 generalizes the watchdog after; R12 model-checks the R9/R10 model.
