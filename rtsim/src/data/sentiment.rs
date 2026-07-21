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

    fn decay(&mut self, rng: &mut impl Rng, dt: f32) {
        if self.positivity != 0 {
            // TODO: Find a slightly nicer way to have sentiment decay, perhaps even by
            // remembering the last interaction instead of constant updates.
            let chance = (1.0
                / ((self.value().powi(2) * 0.24 + 1.0) * (1.0 / 25.0) * DECAY_TIME_FACTOR * dt))
                .min(1.0) as f64;

            // For some reason, RNG doesn't work with small chances (possibly due to impl
            // limits), so use two bools
            // FLAGGED, not converted: this hand-rolled decay draw has dt in
            // the DENOMINATOR (larger dt => smaller per-check chance) —
            // suspicious inversion vs a hazard's dt-proportional form, but
            // re-deriving the intended decay curve is its own row; converting
            // blind would distort sentiment tuning. t0.6-exempt
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
