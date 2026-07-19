//! R12 traversal-contract model: the ladder-traversal ownership contract as a
//! small exact finite-state system. Models the CONTRACT, not voxel physics.
//!
//! Ground truth: readme/RESEARCH-TRIAGE-R9-R12.md (§R9 TraversalLink/queue,
//! §R10 epoch fencing as amended — advance-on-release/adopt-on-acquire, the
//! validate-then-write choke point at owned-write sites), the release-decision
//! pure fns from commit d84005dc89 (three-outcome release + reengage bound),
//! readme/M3-BUILDER-PACKET-FINAL.md (fair queue key = (enqueue_tick, uid)).
//!
//! Abstractions (each documented in RESULTS.md §honest-gaps):
//! - `tick` is not part of state (traces order actions; queue Vec order IS the
//!   (enqueue_tick, uid) order because interleaving assigns distinct ticks).
//! - terrain_revision is abstracted to the one bit the properties consume:
//!   `revision_current` on the active reservation (mutate ⇒ false; a fresh
//!   reservation re-validates ⇒ true).
//! - R10 epoch + R9 generation collapse to one fencing abstraction: release
//!   advances the epoch, so a released owner's delayed write is exactly the
//!   `stale` token (an in-flight write carrying a pre-release epoch).
//! - reengage_count saturates at REENGAGE_CEIL to keep the space finite even
//!   in the broken-bound falsifier variant.

pub const REENGAGE_BOUND: u8 = 2; // model-small stand-in for EMERGENCY_REENGAGE_BOUND=5
pub const REENGAGE_CEIL: u8 = 3; // saturation cap (bound + 1): keeps broken-bound variant finite

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Phase {
    Idle,
    Queued,
    Approaching,
    Mounting,
    Traversing,
    TopExit,
    ExitConfirm,
    Delivered, // organic exit-confirm + release (absorbing)
    Netted,    // net delivery: the teleport floor (absorbing)
    Dead,      // absorbing
}

impl Phase {
    /// Owned phases: the member holds the link's movement authority.
    pub fn owned(self) -> bool {
        matches!(
            self,
            Phase::Approaching
                | Phase::Mounting
                | Phase::Traversing
                | Phase::TopExit
                | Phase::ExitConfirm
        )
    }

    /// Owned MOVING phases (S2's scope: on/entering the link body).
    pub fn owned_moving(self) -> bool {
        matches!(self, Phase::Mounting | Phase::Traversing | Phase::TopExit)
    }

    pub fn terminal(self) -> bool {
        matches!(self, Phase::Delivered | Phase::Netted | Phase::Dead)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Member {
    pub phase: Phase,
    /// Physics contact (meaningful only in Mounting/Traversing; normalized
    /// false elsewhere to keep states canonical).
    pub contact: bool,
    /// A+B+C progress-discrimination: real progress since the last abort.
    pub progress: bool,
    /// Exhausted-replan counter (release-decision (ii)); saturates at
    /// REENGAGE_CEIL. Cleared on real progress (FrontierComplete).
    pub reengage: u8,
}

impl Member {
    fn idle() -> Self {
        Member { phase: Phase::Idle, contact: false, progress: false, reengage: 0 }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Reservation {
    pub member: u8,
    /// Terrain-revision snapshot validity: true while the terrain still
    /// matches what the route was validated against (adopt at reserve;
    /// TerrainMutate flips it false). S4's bit.
    pub revision_current: bool,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct State {
    pub members: Vec<Member>,
    /// Capacity-one link's sole reservation (R9: reservation is the sole
    /// authority; the Option IS the capacity=1 structure).
    pub reservation: Option<Reservation>,
    /// FIFO arrival order == (enqueue_tick, uid) fair order (interleaving
    /// assigns distinct ticks; uid tiebreak cannot arise).
    pub queue: Vec<u8>,
    /// R10 fencing abstraction: Some(m) = a delayed movement write from m's
    /// RELEASED episode (old epoch) is still in flight. Set on every release
    /// (advance-on-release makes the old epoch stale); consumed by StaleWrite.
    pub stale: Option<u8>,
    /// Pending external interruption (agent-inbox class) aimed at the owner.
    pub interrupt: Option<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    Enqueue(u8),
    Reserve(u8),
    ApproachStep(u8),
    ContactAcquire(u8),
    ContactLose(u8),
    EnterLink(u8),
    TerrainMutate,
    FrontierComplete(u8),
    TopExit(u8),
    ExitConfirm(u8),
    /// Verified-stable-exit release (release-decision outcome (i)).
    DeliverRelease(u8),
    /// Classified abort + release (contact-lost / stale-terrain / interruption).
    AbortRelease(u8),
    /// Post-abort re-engage: re-enqueue under a fresh epoch (bounded by the
    /// release-decision (ii) reengage bound).
    Reacquire(u8),
    /// The always-armed no-progress terminator (MOUNT_NO_PROGRESS_TICKS /
    /// the positional stuck-watch): an owner that stops progressing is
    /// eventually aborted. Untimed model: enabled in every owned phase,
    /// weak fairness supplies the "eventually".
    WatchdogAbort(u8),
    /// The net: teleport floor, fires exactly where designed (bound exhausted).
    NetDeliver(u8),
    MemberDeath(u8),
    /// A delayed write carrying a RELEASED (pre-advance) epoch arrives at the
    /// validate-then-write choke point (R10).
    StaleWrite,
    Interrupt(u8),
}

impl Action {
    /// System (scheduler-driven) actions carry the weak-fairness assumption.
    /// Environment actions (terrain mutation, contact loss, interruptions,
    /// death, a delayed stale packet, and a member's own CHOICE to start
    /// using the link — Enqueue) may never fire — no fairness owed.
    pub fn is_system(self) -> bool {
        !matches!(
            self,
            Action::TerrainMutate
                | Action::ContactLose(_)
                | Action::Interrupt(_)
                | Action::MemberDeath(_)
                | Action::StaleWrite
                | Action::Enqueue(_)
        )
    }

    /// Progress actions for L2: a cycle containing none of these and no exit
    /// is a livelock.
    pub fn is_progress(self) -> bool {
        matches!(
            self,
            Action::FrontierComplete(_)
                | Action::TopExit(_)
                | Action::ExitConfirm(_)
                | Action::DeliverRelease(_)
                | Action::NetDeliver(_)
        )
    }
}

/// Faithful-contract config; every `break_*` knob disables ONE mechanism so
/// the checker's falsifier tests can prove it DETECTS that bug class.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub n_members: usize,
    /// R10 epoch fence at the owned-write sites (break ⇒ S3/S2 class).
    pub epoch_fence: bool,
    /// R9 (enqueue_tick, uid) fair queue; broken = min-UID selection, the
    /// live anti-pattern (break ⇒ L1 starvation).
    pub fair_queue: bool,
    /// Release-decision (ii) reengage bound terminating into the net
    /// (break ⇒ L2 livelock).
    pub reengage_bound: bool,
    /// Terrain-revision validation at owned progress sites (break ⇒ S4).
    pub revision_guard: bool,
    /// Despawn advance-site: death releases the reservation (break ⇒ S5).
    pub death_releases: bool,
}

impl Config {
    pub fn faithful(n_members: usize) -> Self {
        Config {
            n_members,
            epoch_fence: true,
            fair_queue: true,
            reengage_bound: true,
            revision_guard: true,
            death_releases: true,
        }
    }
}

pub fn initial_state(cfg: &Config) -> State {
    State {
        members: vec![Member::idle(); cfg.n_members],
        reservation: None,
        queue: Vec::new(),
        stale: None,
        interrupt: None,
    }
}

fn holds_reservation(s: &State, m: u8) -> bool {
    s.reservation.map(|r| r.member) == Some(m)
}

fn revision_ok(s: &State, cfg: &Config) -> bool {
    !cfg.revision_guard || s.reservation.map(|r| r.revision_current).unwrap_or(false)
}

/// Abort causes, mirroring the classified-abort inventory: contact loss on
/// the link, stale terrain revision, external interruption.
fn abort_cause(s: &State, m: u8) -> bool {
    let mem = &s.members[m as usize];
    let contact_lost = mem.phase == Phase::Traversing && !mem.contact;
    let stale_terrain = s.reservation.map(|r| !r.revision_current).unwrap_or(false);
    let interrupted = s.interrupt == Some(m);
    contact_lost || stale_terrain || interrupted
}

pub fn enabled_actions(s: &State, cfg: &Config) -> Vec<Action> {
    let mut out = Vec::new();
    let n = s.members.len() as u8;
    for m in 0..n {
        let mem = &s.members[m as usize];
        match mem.phase {
            Phase::Idle => {
                if mem.reengage == 0 {
                    out.push(Action::Enqueue(m));
                } else {
                    // Post-abort: release-decision (ii) — bounded re-engage,
                    // exhaustion terminates into the net.
                    let exhausted = mem.reengage >= REENGAGE_BOUND;
                    if !exhausted || !cfg.reengage_bound {
                        out.push(Action::Reacquire(m));
                    }
                    if exhausted && cfg.reengage_bound {
                        out.push(Action::NetDeliver(m));
                    }
                }
            },
            Phase::Queued => {
                if s.reservation.is_none() {
                    let selected = if cfg.fair_queue {
                        // R9 fair key: head of (enqueue_tick, uid) order.
                        s.queue.first() == Some(&m)
                    } else {
                        // The live anti-pattern R9 kills: min-UID-alone.
                        s.queue.iter().min() == Some(&m)
                    };
                    if selected {
                        out.push(Action::Reserve(m));
                    }
                }
            },
            Phase::Approaching => {
                if holds_reservation(s, m) {
                    out.push(Action::ApproachStep(m));
                }
            },
            Phase::Mounting => {
                if !mem.contact {
                    out.push(Action::ContactAcquire(m));
                } else {
                    out.push(Action::ContactLose(m));
                    if holds_reservation(s, m) && revision_ok(s, cfg) {
                        out.push(Action::EnterLink(m));
                    }
                }
            },
            Phase::Traversing => {
                if mem.contact {
                    out.push(Action::ContactLose(m));
                    if holds_reservation(s, m) && revision_ok(s, cfg) {
                        out.push(Action::FrontierComplete(m));
                        out.push(Action::TopExit(m));
                    }
                }
            },
            Phase::TopExit => {
                if holds_reservation(s, m) {
                    out.push(Action::ExitConfirm(m));
                }
            },
            Phase::ExitConfirm => {
                if holds_reservation(s, m) {
                    out.push(Action::DeliverRelease(m));
                }
            },
            Phase::Delivered | Phase::Netted | Phase::Dead => {},
        }
        // Classified abort: any owned phase with a cause.
        if mem.phase.owned() && holds_reservation(s, m) && abort_cause(s, m) {
            out.push(Action::AbortRelease(m));
        }
        // The no-progress watchdog is ALWAYS armed over the owner (M2's
        // MOUNT_NO_PROGRESS_TICKS + the positional watch; releases to the
        // bounded reengage machinery, never replaces the net).
        if mem.phase.owned() && holds_reservation(s, m) {
            out.push(Action::WatchdogAbort(m));
        }
        // External interruption can arrive at the owner at any owned point.
        if mem.phase.owned() && holds_reservation(s, m) && s.interrupt.is_none() {
            out.push(Action::Interrupt(m));
        }
        if !mem.phase.terminal() {
            out.push(Action::MemberDeath(m));
        }
    }
    if s.reservation.map(|r| r.revision_current).unwrap_or(false) {
        out.push(Action::TerrainMutate);
    }
    if s.stale.is_some() {
        out.push(Action::StaleWrite);
    }
    out
}

/// Release helper: advance-on-release (R10) — the released member's epoch is
/// now stale, so a delayed write from that episode may still be in flight.
fn release(s: &mut State, m: u8) {
    s.reservation = None;
    s.stale = Some(m);
}

pub fn apply(s: &State, a: Action, cfg: &Config) -> State {
    let mut t = s.clone();
    match a {
        Action::Enqueue(m) => {
            t.members[m as usize].phase = Phase::Queued;
            t.queue.push(m);
        },
        Action::Reserve(m) => {
            t.queue.retain(|&q| q != m);
            // Adopt-on-acquire: the new reservation carries the CURRENT epoch
            // and a fresh terrain validation.
            t.reservation = Some(Reservation { member: m, revision_current: true });
            t.members[m as usize].phase = Phase::Approaching;
        },
        Action::ApproachStep(m) => t.members[m as usize].phase = Phase::Mounting,
        Action::ContactAcquire(m) => t.members[m as usize].contact = true,
        Action::ContactLose(m) => t.members[m as usize].contact = false,
        Action::EnterLink(m) => t.members[m as usize].phase = Phase::Traversing,
        Action::TerrainMutate => {
            if let Some(r) = t.reservation.as_mut() {
                r.revision_current = false;
            }
        },
        Action::FrontierComplete(m) => {
            let mem = &mut t.members[m as usize];
            mem.progress = true;
            mem.reengage = 0; // (C): the bound clears on REAL progress only
        },
        Action::TopExit(m) => {
            let mem = &mut t.members[m as usize];
            mem.phase = Phase::TopExit;
            mem.contact = false; // off the rungs, on the rim path
        },
        Action::ExitConfirm(m) => t.members[m as usize].phase = Phase::ExitConfirm,
        Action::DeliverRelease(m) => {
            t.members[m as usize].phase = Phase::Delivered;
            release(&mut t, m);
        },
        Action::AbortRelease(m) => {
            let mem = &mut t.members[m as usize];
            mem.phase = Phase::Idle;
            mem.contact = false;
            mem.progress = false; // progress-since-abort resets
            mem.reengage = (mem.reengage + 1).min(REENGAGE_CEIL);
            release(&mut t, m);
            if t.interrupt == Some(m) {
                t.interrupt = None; // the interruption is consumed by the abort
            }
        },
        Action::Reacquire(m) => {
            t.members[m as usize].phase = Phase::Queued;
            t.queue.push(m);
        },
        Action::WatchdogAbort(m) => {
            let mem = &mut t.members[m as usize];
            mem.phase = Phase::Idle;
            mem.contact = false;
            mem.progress = false;
            mem.reengage = (mem.reengage + 1).min(REENGAGE_CEIL);
            release(&mut t, m);
            if t.interrupt == Some(m) {
                t.interrupt = None;
            }
        },
        Action::NetDeliver(m) => {
            let mem = &mut t.members[m as usize];
            mem.phase = Phase::Netted;
            mem.contact = false;
        },
        Action::MemberDeath(m) => {
            let mem = &mut t.members[m as usize];
            mem.phase = Phase::Dead;
            mem.contact = false;
            t.queue.retain(|&q| q != m);
            if holds_reservation(s, m) && cfg.death_releases {
                // The despawn advance-site (S5's mechanism).
                release(&mut t, m);
            }
            if t.interrupt == Some(m) {
                t.interrupt = None;
            }
        },
        Action::StaleWrite => {
            let f = t.stale.take().expect("guarded");
            if !cfg.epoch_fence {
                // BROKEN-FENCE SEMANTICS (the bug class R10 kills): the
                // delayed old-epoch movement write lands — it shoves the
                // former owner back into an owned moving phase, and claims
                // the link if free. With the fence, none of this happens.
                let mem = &mut t.members[f as usize];
                if mem.phase != Phase::Dead {
                    mem.phase = Phase::Traversing;
                    mem.contact = true;
                    t.queue.retain(|&q| q != f);
                    if t.reservation.is_none() {
                        t.reservation =
                            Some(Reservation { member: f, revision_current: true });
                    }
                }
            }
            // With the fence: validate-then-write rejects; consumed, no-op.
        },
        Action::Interrupt(m) => t.interrupt = Some(m),
    }
    // Canonicalization: contact is only meaningful mid-link.
    for mem in t.members.iter_mut() {
        if !matches!(mem.phase, Phase::Mounting | Phase::Traversing) {
            mem.contact = false;
        }
    }
    t
}
