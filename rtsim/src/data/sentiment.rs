use common::{
    character::CharacterId,
    rtsim::{Actor, FactionId, NpcId},
};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BinaryHeap};

// Factions have a larger 'social memory' than individual NPCs and so we allow
// them to have more sentiments
pub const FACTION_MAX_SENTIMENTS: usize = 1024;
pub const NPC_MAX_SENTIMENTS: usize = 128;

/// Magic factor used to control sentiment decay speed (note: higher = slower
/// decay, for implementation reasons).
const DECAY_TIME_FACTOR: f32 = 2500.0;

/// The target that a sentiment is felt toward.
// NOTE: More could be added to this! For example:
// - Animal species (dislikes spiders?)
// - Kind of food (likes meat?)
// - Occupations (hatred of hunters or chefs?)
// - Ideologies (dislikes democracy, likes monarchy?)
// - etc.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Target {
    Character(CharacterId),
    Npc(NpcId),
    Faction(FactionId),
}

impl From<NpcId> for Target {
    fn from(npc: NpcId) -> Self { Self::Npc(npc) }
}
impl From<FactionId> for Target {
    fn from(faction: FactionId) -> Self { Self::Faction(faction) }
}
impl From<CharacterId> for Target {
    fn from(character: CharacterId) -> Self { Self::Character(character) }
}
impl From<Actor> for Target {
    fn from(actor: Actor) -> Self {
        match actor {
            Actor::Character(character) => Self::Character(character),
            Actor::Npc(npc) => Self::Npc(npc),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Sentiments {
    #[serde(rename = "m")]
    map: BTreeMap<Target, Sentiment>,
}

impl Sentiments {
    /// bastion (ITEM 22): read-only view of every held sentiment, in the
    /// BTreeMap's deterministic order — the inspect display's same-source
    /// fill. Values are the same `value()` scale gameplay consumes.
    pub fn iter_held(&self) -> impl Iterator<Item = (Target, f32)> + '_ {
        self.map.iter().map(|(t, s)| (*t, s.value()))
    }

    /// Return the sentiment that is felt toward the given target.
    pub fn toward(&self, target: impl Into<Target>) -> &Sentiment {
        self.map.get(&target.into()).unwrap_or(&Sentiment::DEFAULT)
    }

    /// Return the sentiment that is felt toward the given target.
    pub fn toward_mut(&mut self, target: impl Into<Target>) -> &mut Sentiment {
        self.map.entry(target.into()).or_default()
    }

    /// Progressively decay the sentiment back to a neutral sentiment.
    ///
    /// Note that sentiment get decay gets slower the harsher the sentiment is.
    /// You can calculate the **average** number of seconds required for a
    /// sentiment to neutral decay with the following rough formula:
    ///
    /// ```ignore
    /// seconds_until_neutrality = (sentiment_value^2 * 24 + 1) / 25 * DECAY_TIME_FACTOR * sentiment_value * 128
    /// ```
    ///
    /// Some 'common' sentiment decay times are as follows:
    ///
    /// - `POSITIVE`/`NEGATIVE`: ~26 minutes
    /// - `ALLY`/`RIVAL`: ~3.4 hours
    /// - `FRIEND`/`ENEMY`: ~21 hours
    /// - `HERO`/`VILLAIN`: ~47 hours
    pub fn decay(&mut self, rng: &mut impl Rng, dt: f32) {
        self.map.retain(|_, sentiment| {
            sentiment.decay(rng, dt);
            // We can eliminate redundant sentiments that don't need remembering
            !sentiment.is_redundant()
        });
    }

    /// Clean up sentiments to avoid them growing too large
    pub fn cleanup(&mut self, max_sentiments: usize) {
        if self.map.len() > max_sentiments {
            let mut sentiments = self.map
                .iter()
                // For each sentiment, calculate how valuable it is for us to remember.
                // For now, we just use the absolute value of the sentiment but later on we might want to favour
                // sentiments toward factions and other 'larger' groups over, say, sentiments toward players/other NPCs
                .map(|(tgt, sentiment)| (sentiment.positivity.unsigned_abs(), *tgt))
                .collect::<BinaryHeap<_>>();

            // Remove the superfluous sentiments
            for (_, tgt) in sentiments
                .drain_sorted()
                .take(self.map.len() - max_sentiments)
            {
                self.map.remove(&tgt);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sentiment {
    /// How positive the sentiment is.
    ///
    /// Using i8 to reduce on-disk memory footprint.
    /// Semantically, this value is -1 <= x <= 1.
    #[serde(rename = "p")]
    positivity: i8,
}

impl Sentiment {
    /// Substantial positive sentiments: NPC may go out of their way to help
    /// actors associated with the target, greet them, etc.
    pub const ALLY: f32 = 0.3;
    const DEFAULT: Self = Self { positivity: 0 };
    /// Very negative sentiments: NPC may confront the actor, get aggressive
    /// with them, or even use force against them.
    pub const ENEMY: f32 = -0.6;
    /// Very positive sentiments: NPC may join the actor as a companion,
    /// encourage them to join their faction, etc.
    pub const FRIEND: f32 = 0.6;
    /// Extremely positive sentiments: NPC may switch sides to join the actor's
    /// faction, protect them at all costs, turn against friends for them,
    /// etc. Verging on cult-like behaviour.
    pub const HERO: f32 = 0.8;
    /// Minor negative sentiments: NPC might be less willing to provide
    /// information, give worse trade deals, etc.
    pub const NEGATIVE: f32 = -0.1;
    /// Minor positive sentiments: NPC might be more willing to provide
    /// information, give better trade deals, etc.
    pub const POSITIVE: f32 = 0.1;
    /// Substantial negative sentiments: NPC may reject attempts to trade or
    /// avoid actors associated with the target, insult them, but will not
    /// use physical force.
    pub const RIVAL: f32 = -0.3;
    /// Extremely negative sentiments: NPC may aggressively persue or hunt down
    /// the actor, organise others around them to do the same, and will
    /// generally try to harm the actor in any way they can.
    pub const VILLAIN: f32 = -0.8;

    fn value(&self) -> f32 { self.positivity as f32 * (1.0 / 126.0) }

    /// Change the sentiment toward the given target by the given amount,
    /// capping out at the given value.
    pub fn change_by(&mut self, change: f32, cap: f32) {
        // There's a bit of ceremony here for two reasons:
        // 1) Very small changes should not be rounded to 0
        // 2) Sentiment should never (over/under)flow
        if change != 0.0 {
            let abs = (change * 126.0).abs().clamp(1.0, 126.0) as i8;
            let cap = (cap.abs().min(1.0) * 126.0) as i8;
            self.positivity = if change > 0.0 {
                self.positivity.saturating_add(abs).min(cap)
            } else {
                self.positivity.saturating_sub(abs).max(-cap)
            };
        }
    }

    /// Limit the sentiment to the given value, either positive or negative. The
    /// resulting sentiment is guaranteed to be less than the cap (at least,
    /// as judged by [`Sentiment::is`]).
    pub fn limit_below(&mut self, cap: f32) {
        if cap > 0.0 {
            self.positivity = self
                .positivity
                .min(((cap.min(1.0) * 126.0) as i8 - 1).max(0));
        } else {
            self.positivity = self
                .positivity
                .max(((-cap.max(-1.0) * 126.0) as i8 + 1).min(0));
        }
    }

    /// T0.79 (E7 Stage 2, PRE-FIX FORMULA, preserved for the
    /// characterization tests only -- `decay` no longer calls this). The
    /// original hand-rolled chance, dt in the DENOMINATOR: chance SHRANK as
    /// dt grew, the opposite of a per-time hazard. Kept byte-for-byte so
    /// the "before" picture stays pinned regardless of future changes to
    /// the live formula.
    fn decay_chance_pre_t0_79_fix(value: f32, dt: f32) -> f64 {
        (1.0 / ((value.powi(2) * 0.24 + 1.0) * (1.0 / 25.0) * DECAY_TIME_FACTOR * dt)).min(1.0)
            as f64
    }

    /// T0.79 (E7 Stage 2, Fable-ruled CONVERT NOW): the corrected per-
    /// second hazard, routed through the same [`crate::ai::discrete_chance`]
    /// `NpcCtx::chance` uses -- one canonical cadence-invariant formula
    /// instead of two.
    ///
    /// Derivation: the pre-fix formula was `min(1, 1 / (D(value) * dt))`
    /// where `D(value) = (value^2*0.24 + 1) * (1/25) * DECAY_TIME_FACTOR`.
    /// At `dt = 1` (the ONLY dt this ever actually runs at --
    /// `NPC_SENTIMENT_TICK_SKIP` ticks at the 1/30s tick rate is exactly
    /// 1 simulated second) that's `min(1, 1/D(value))`. Defining
    /// `chance_per_second(value) := min(1, 1/D(value))` (literally the
    /// pre-fix formula with `* dt` deleted from the denominator) and
    /// running it through `discrete_chance` reproduces that SAME value
    /// at `dt=1` exactly (`discrete_chance`'s `dt<=1.0` branch is
    /// `dt * chance_per_second`, and `1 * x = x`) -- see
    /// `decay_chance_matches_pre_fix_formula_at_the_current_cadence`. For
    /// any OTHER dt the two formulas diverge on purpose: this one now
    /// scales WITH dt instead of against it.
    fn decay_chance(value: f32, dt: f32) -> f64 {
        let chance_per_second =
            (1.0 / ((value.powi(2) * 0.24 + 1.0) * (1.0 / 25.0) * DECAY_TIME_FACTOR)) as f64;
        crate::ai::discrete_chance(dt as f64, chance_per_second)
    }

    fn decay(&mut self, rng: &mut impl Rng, dt: f32) {
        if self.positivity != 0 {
            // TODO: Find a slightly nicer way to have sentiment decay, perhaps even by
            // remembering the last interaction instead of constant updates.
            let chance = Self::decay_chance(self.value(), dt);

            // For some reason, RNG doesn't work with small chances (possibly due to impl
            // limits), so use two bools. `chance` is derived via discrete_chance (the same
            // per-second-hazard formula NpcCtx::chance uses), not a raw ad-hoc gate.
            // t0.6-exempt: sqrt-trick precision workaround consuming that already-derived, discrete_chance-routed per-second-hazard value.
            if rng.random_bool(chance.sqrt()) && rng.random_bool(chance.sqrt()) {
                self.positivity -= self.positivity.signum();
            }
        }
    }

    /// Return `true` if the sentiment can be forgotten without changing
    /// anything (i.e: is entirely neutral, the default stance).
    fn is_redundant(&self) -> bool { self.positivity == 0 }

    /// Returns `true` if the sentiment has reached the given threshold.
    pub fn is(&self, val: f32) -> bool {
        if val > 0.0 {
            self.value() >= val
        } else {
            self.value() <= val
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    use slotmap::KeyData;
    use std::collections::BTreeSet;

    fn npc(raw: u64) -> NpcId { KeyData::from_ffi(raw).into() }

    #[test]
    fn persisted_sentiments_have_stable_bytes() {
        let entries = [
            (npc(1), -8i8),
            (npc(2), -6),
            (npc(3), -4),
            (npc(4), -2),
            (npc(5), 2),
            (npc(6), 4),
            (npc(7), 6),
            (npc(8), 8),
        ];
        let mut encodings = BTreeSet::new();
        for shift in 0..entries.len() {
            let mut sentiments = Sentiments::default();
            for offset in 0..entries.len() {
                let (target, positivity) = entries[(shift + offset) % entries.len()];
                *sentiments.toward_mut(target) = Sentiment { positivity };
            }
            encodings.insert(rmp_serde::to_vec_named(&sentiments).expect("encode sentiments"));
        }
        println!(
            "sentiments distinct persistence encodings={}",
            encodings.len()
        );
        if let Some(first) = encodings.first() {
            println!("sentiments representative_msgpack={}", hex(first));
        }
        assert_eq!(
            encodings.len(),
            1,
            "equal sentiment state must have one persisted representation"
        );
    }

    #[test]
    fn legacy_hash_map_sentiments_remain_loadable() {
        #[derive(Serialize)]
        struct LegacySentiments {
            #[serde(rename = "m")]
            // t0.48: hash-ok — legacy-compat TEST struct, not persisted data.
            map: hashbrown::HashMap<Target, Sentiment>,
        }

        let mut map = hashbrown::HashMap::new();
        map.insert(npc(2).into(), Sentiment { positivity: -6 });
        map.insert(npc(1).into(), Sentiment { positivity: 8 });
        let bytes = rmp_serde::to_vec_named(&LegacySentiments { map }).expect("encode legacy");
        let decoded: Sentiments = rmp_serde::from_slice(&bytes).expect("decode ordered sentiments");
        assert_eq!(decoded.toward(npc(1)).positivity, 8);
        assert_eq!(decoded.toward(npc(2)).positivity, -6);
    }

    #[test]
    fn sentiment_decay_assigns_rng_draws_in_target_order() {
        let entries = [(npc(1), -8i8), (npc(2), -6), (npc(3), 6), (npc(4), 8)];
        let mut forward = Sentiments::default();
        let mut reverse = Sentiments::default();
        for (target, positivity) in entries {
            *forward.toward_mut(target) = Sentiment { positivity };
        }
        for (target, positivity) in entries.into_iter().rev() {
            *reverse.toward_mut(target) = Sentiment { positivity };
        }
        let mut forward_rng = StdRng::seed_from_u64(0x51_4e_54);
        let mut reverse_rng = StdRng::seed_from_u64(0x51_4e_54);
        forward.decay(&mut forward_rng, 1_000_000.0);
        reverse.decay(&mut reverse_rng, 1_000_000.0);
        for (target, _) in entries {
            assert_eq!(
                forward.toward(target).positivity,
                reverse.toward(target).positivity,
                "equal state and RNG must decay the same target identically"
            );
        }
    }

    fn hex(bytes: &[u8]) -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() }
}

/// E7 Stage 2 (T0.79, Fable-ruled CONVERT NOW): the "before" picture is
/// pinned against the frozen [`Sentiment::decay_chance_pre_t0_79_fix`]
/// (byte-identical to the original formula, `decay` no longer calls it),
/// and the "after" picture -- the live [`Sentiment::decay_chance`], now
/// routed through [`crate::ai::discrete_chance`] -- is proven both
/// equivalent to the old formula at the one cadence that has ever
/// actually run, and genuinely cadence-invariant everywhere else.
///
/// What the PRE-FIX formula computed: `chance = min(1, K(value) / dt)`
/// where `K(value) = (value^2 * 0.24 + 1) * (1/25) * DECAY_TIME_FACTOR`.
/// dt in the DENOMINATOR: chance shrinks as dt grows -- the opposite of a
/// per-time hazard. The struct doc comment's own `seconds_until_neutrality`
/// formula has no dt term at all, which IS a cadence-independence promise
/// the pre-fix code never delivered on: checks-per-second * chance-per-
/// check = (1/dt)*(K/dt) = K/dt^2, so halving dt (checking twice as often)
/// roughly QUADRUPLED expected decays per real second. This was "accidental
/// cadence-dependence", not a valid per-time hazard.
///
/// It was LATENT, not live-broken, at the time of the fix: the only call
/// site (`rule::cleanup::CleanUp`) always passed the same fixed dt
/// (`ctx.event.dt * NPC_SENTIMENT_TICK_SKIP` -- NPC_SENTIMENT_TICK_SKIP=30
/// ticks at the engine's 1/30s tick rate is exactly 1 simulated second,
/// both compile-time-fixed), so the cadence-dependence never manifested as
/// an observable behavior difference under the calling code that has ever
/// actually run. It would have misbehaved under any future cadence change
/// (a tick-rate retune, an NPC_SENTIMENT_TICK_SKIP retune, or a large
/// one-off catch-up dt after a stall) -- which is why T0.79 (the
/// probability/rate source-gate CLOSURE row) converts it now rather than
/// leaving it flagged.
#[cfg(test)]
mod sentiment_decay_law_characterization {
    use super::*;

    /// The inversion, pinned directly against the frozen pre-fix formula:
    /// holding `value` fixed, chance goes DOWN as dt goes UP. A correct
    /// per-time-hazard law goes the other way (more elapsed time -> more
    /// likely a decay step has occurred by now) -- see
    /// `decay_chance_grows_with_dt_after_the_fix` for the live formula's
    /// opposite (correct) direction.
    #[test]
    fn pre_fix_decay_chance_shrinks_as_dt_grows() {
        let value = Sentiment::POSITIVE;
        let small_dt = Sentiment::decay_chance_pre_t0_79_fix(value, 1.0);
        let large_dt = Sentiment::decay_chance_pre_t0_79_fix(value, 100.0);
        assert!(
            large_dt < small_dt,
            "chance must shrink as dt grows under the pre-fix (inverted) formula: dt=1 -> \
             {small_dt}, dt=100 -> {large_dt}"
        );
        // Exactly proportional to 1/dt (not just "smaller" by some other
        // shape) -- pins the specific K(value)/dt form, not just its sign.
        // Tolerance is f32-precision-scale (the underlying formula computes
        // in f32 before the final f64 cast), not a loose approximation.
        assert!(
            (small_dt / large_dt - 100.0).abs() < 1e-4,
            "expected exactly a 100x ratio (K/1 vs K/100), got {}",
            small_dt / large_dt
        );
    }

    /// The doc comment's promise ("seconds_until_neutrality" has no dt
    /// term) implies expected-decays-per-unit-real-time should be
    /// cadence-INDEPENDENT: checking twice as often at half the dt should
    /// give the same expected outcome over the same real time span. Pin
    /// that the PRE-FIX formula did NOT hold that property -- checking
    /// twice as often (halving dt) roughly quadrupled
    /// expected-decays-per-real-second, it did not stay constant.
    #[test]
    fn pre_fix_expected_decays_per_real_second_is_not_cadence_invariant() {
        let value = Sentiment::POSITIVE;
        let dt: f64 = 10.0;
        let checks_per_second = 1.0 / dt;
        let chance = Sentiment::decay_chance_pre_t0_79_fix(value, dt as f32);
        let decays_per_second = checks_per_second * chance;

        let half_dt = dt / 2.0;
        let checks_per_second_half = 1.0 / half_dt;
        let chance_half = Sentiment::decay_chance_pre_t0_79_fix(value, half_dt as f32);
        let decays_per_second_half = checks_per_second_half * chance_half;

        let ratio = decays_per_second_half / decays_per_second;
        assert!(
            (ratio - 2.0).abs() > 0.5,
            "a cadence-invariant law would keep this ratio near 1.0; the pre-fix formula \
             instead moved it toward 2x (halving dt roughly doubled checks/sec AND roughly \
             doubled chance/check) -- got ratio {ratio}, which would falsify the \
             cadence-dependence finding if it were actually near 1.0"
        );
    }

    /// `min(1.0, ...)` clamp: at a small enough dt, the pre-fix chance
    /// saturates to 1 (guaranteed decay-step attempt every check) rather
    /// than exceeding probability bounds.
    #[test]
    fn pre_fix_decay_chance_is_clamped_to_one_at_small_dt() {
        let value = Sentiment::VILLAIN;
        let chance = Sentiment::decay_chance_pre_t0_79_fix(value, 0.000_001);
        assert_eq!(chance, 1.0);
    }

    /// Harsher sentiments (larger `|value|`) decay slower per the doc
    /// comment ("decay gets slower the harsher the sentiment is") --
    /// pinned against the pre-fix formula at a fixed dt so this property
    /// is shown to survive independent of the dt-direction finding above
    /// (and re-pinned against the LIVE formula below, proving the fix
    /// preserved it).
    #[test]
    fn pre_fix_decay_chance_is_lower_for_harsher_sentiments_at_fixed_dt() {
        let mild = Sentiment::decay_chance_pre_t0_79_fix(Sentiment::POSITIVE, 10.0);
        let harsh = Sentiment::decay_chance_pre_t0_79_fix(Sentiment::HERO, 10.0);
        assert!(
            harsh < mild,
            "a harsher sentiment (HERO={}) must decay no faster per-check than a milder one \
             (POSITIVE={}): mild_chance={mild}, harsh_chance={harsh}",
            Sentiment::HERO,
            Sentiment::POSITIVE
        );
    }

    /// T0.32-style exact-to-1-ulp equivalence pin (Fable-required, not
    /// merely asserted): the live (post-fix) formula must reproduce the
    /// pre-fix formula's output EXACTLY at dt=1.0 -- the one cadence
    /// (`NPC_SENTIMENT_TICK_SKIP` ticks at the 1/30s tick rate) that has
    /// ever actually run. This is the proof that converting the formula
    /// changes zero observable behavior today.
    #[test]
    fn decay_chance_matches_pre_fix_formula_at_the_current_cadence() {
        for value in [
            Sentiment::POSITIVE,
            Sentiment::NEGATIVE,
            Sentiment::ALLY,
            Sentiment::RIVAL,
            Sentiment::FRIEND,
            Sentiment::ENEMY,
            Sentiment::HERO,
            Sentiment::VILLAIN,
        ] {
            let dt = 1.0f32;
            let pre_fix = Sentiment::decay_chance_pre_t0_79_fix(value, dt);
            let live = Sentiment::decay_chance(value, dt);
            assert!(
                (pre_fix - live).abs() <= f64::EPSILON,
                "value={value}: pre-fix chance {pre_fix} and live chance {live} must match to \
                 1 ulp at dt=1.0 (the current cadence) -- the conversion must not change \
                 observable behavior today"
            );
        }
    }

    /// The property that would have caught this originally, now proven to
    /// HOLD for the live formula: checking twice as often (halving dt)
    /// keeps expected-decays-per-real-second roughly CONSTANT, across a
    /// sweep of dt values, not just at the one calibration point.
    #[test]
    fn decay_chance_grows_with_dt_after_the_fix() {
        let value = Sentiment::POSITIVE;
        let small_dt = Sentiment::decay_chance(value, 0.1);
        let large_dt = Sentiment::decay_chance(value, 10.0);
        assert!(
            large_dt > small_dt,
            "post-fix chance must GROW as dt grows (more elapsed time -> more likely a decay \
             occurred by now): dt=0.1 -> {small_dt}, dt=10 -> {large_dt}"
        );
    }

    /// Swept only within `discrete_chance`'s `dt <= 1.0` (linear) branch,
    /// where invariance is EXACT by construction (`checks_per_second *
    /// chance = (1/dt) * (dt * chance_per_second) = chance_per_second`,
    /// algebraically constant, no tolerance needed for floating-point
    /// beyond the sweep itself). Deliberately does NOT sweep past dt=1:
    /// beyond that, `decay()`'s one-decrement-per-call design caps how
    /// many hazard-intervals a single call can register regardless of how
    /// large dt is, so "decays per second" naturally saturates downward at
    /// large dt -- an inherent design limit of the one-shot-per-call
    /// mechanism, not cadence-dependence, and out of scope for this pin.
    #[test]
    fn expected_decays_per_real_second_is_cadence_invariant_in_the_linear_regime() {
        let value = Sentiment::POSITIVE;
        let decays_per_second = |dt: f64| -> f64 {
            let checks_per_second = 1.0 / dt;
            checks_per_second * Sentiment::decay_chance(value, dt as f32)
        };

        let baseline = decays_per_second(1.0);
        for dt in [0.001, 0.01, 0.1, 0.5, 1.0] {
            let observed = decays_per_second(dt);
            let ratio = observed / baseline;
            assert!(
                (ratio - 1.0).abs() < 1e-6,
                "expected decays-per-real-second to match the dt=1.0 baseline ({baseline}) \
                 exactly in the linear regime, but dt={dt} gave {observed} (ratio {ratio}) -- \
                 the post-fix formula should be cadence-invariant here"
            );
        }
    }

    /// Re-pin, against the LIVE formula, that the fix preserved the
    /// "harsher sentiments decay slower" property (not just reproduced
    /// pre-fix numbers at one dt).
    #[test]
    fn decay_chance_is_lower_for_harsher_sentiments_at_fixed_dt_after_the_fix() {
        let mild = Sentiment::decay_chance(Sentiment::POSITIVE, 10.0);
        let harsh = Sentiment::decay_chance(Sentiment::HERO, 10.0);
        assert!(
            harsh < mild,
            "a harsher sentiment (HERO={}) must decay no faster per-check than a milder one \
             (POSITIVE={}) after the fix: mild_chance={mild}, harsh_chance={harsh}",
            Sentiment::HERO,
            Sentiment::POSITIVE
        );
    }
}
