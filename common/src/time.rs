use serde::Deserialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Hash)]
pub enum DayPeriod {
    Night,
    Morning,
    Noon,
    Evening,
}

impl From<f64> for DayPeriod {
    fn from(time_of_day: f64) -> Self {
        let tod = time_of_day.rem_euclid(60.0 * 60.0 * 24.0);
        if tod < 60.0 * 60.0 * 6.0 {
            DayPeriod::Night
        } else if tod < 60.0 * 60.0 * 11.0 {
            DayPeriod::Morning
        } else if tod < 60.0 * 60.0 * 16.0 {
            DayPeriod::Noon
        } else if tod < 60.0 * 60.0 * 19.0 {
            DayPeriod::Evening
        } else {
            DayPeriod::Night
        }
    }
}

impl DayPeriod {
    pub fn is_dark(&self) -> bool { *self == DayPeriod::Night }

    pub fn is_light(&self) -> bool { !self.is_dark() }
}

pub const DAYS_IN_MONTH: f64 = 40.0;

/// A value ranging 0.0..1.0, to indicate the orbit period of the moon.
pub struct MoonPeriod(pub f64);

impl From<f64> for MoonPeriod {
    fn from(value: f64) -> Self { Self((value / (crate::resources::DAY * DAYS_IN_MONTH)).fract()) }
}

/// bastion (SEASON-0, row 42): the in-game YEAR's quarter — the
/// year-scale mirror of [`DayPeriod`]'s day-scale bucketing. A PURE
/// function of the ONE master clock ([`crate::resources::TimeOfDay`]):
/// no second clock, no stored state, zero per-entity cost, deterministic
/// by construction — pause/speed changes cannot drift it because there
/// is nothing to drift. NOT [`crate::calendar::Calendar`] (real-world
/// wall-clock holidays); this is the world's own annual rhythm.
/// Consumers arrive in SEASON-2; the day-D schedule hook is SEASON-1.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Hash)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// bastion (SEASON-0): the year's length in in-game DAYS when the
/// tunable asset is missing/broken — the graceful fallback only; the
/// real knob is [`SeasonConfig`] (`assets/common/season_config.ron`).
/// 160 = 4 seasons × the existing [`DAYS_IN_MONTH`] convention.
pub const DEFAULT_DAYS_IN_YEAR: f64 = 160.0;

/// bastion (SEASON-0): the RON-tunable season/year configuration.
#[derive(Debug, Copy, Clone, Deserialize)]
pub struct SeasonConfig {
    pub days_in_year: f64,
}

impl Default for SeasonConfig {
    fn default() -> Self {
        Self {
            days_in_year: DEFAULT_DAYS_IN_YEAR,
        }
    }
}

impl crate::assets::FileAsset for SeasonConfig {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(
        bytes: std::borrow::Cow<[u8]>,
    ) -> Result<Self, crate::assets::BoxedError> {
        crate::assets::load_ron(&bytes)
    }
}

impl SeasonConfig {
    /// The loaded tunable (hot-reloadable asset, read each call — a
    /// scalar copy); the default on a missing/broken asset — graceful,
    /// never a panic.
    pub fn current() -> Self {
        use crate::assets::AssetExt;
        Self::load("common.season_config")
            .map(|h| *h.read())
            .unwrap_or_default()
    }

    pub fn year_length_secs(&self) -> f64 {
        crate::resources::DAY * self.days_in_year
    }
}

/// bastion (SEASON-0): the year phase in `0.0..1.0` — [`MoonPeriod`]'s
/// shape at year scale (`rem_euclid`, not `fract`: correct for any
/// input even though the clock only grows).
pub fn year_phase(time_of_day: f64, days_in_year: f64) -> f64 {
    let year = crate::resources::DAY * days_in_year;
    time_of_day.rem_euclid(year) / year
}

/// bastion (SEASON-0): the day-of-year ordinal, `0..days_in_year`.
pub fn day_of_year(time_of_day: f64, days_in_year: f64) -> u32 {
    (year_phase(time_of_day, days_in_year) * days_in_year) as u32
}

impl Season {
    /// bastion (SEASON-0): bucket the master clock into QUARTERS —
    /// [`DayPeriod::from`]'s exact shape one scale up.
    pub fn at(time_of_day: f64, days_in_year: f64) -> Self {
        let phase = year_phase(time_of_day, days_in_year);
        if phase < 0.25 {
            Season::Spring
        } else if phase < 0.5 {
            Season::Summer
        } else if phase < 0.75 {
            Season::Autumn
        } else {
            Season::Winter
        }
    }
}

// ─── bastion (SEASON-2, row 42): THE seasonal consumer interface ────────
//
// The ONE read every seasonal consumer plugs into — DF-FARM growth,
// DF-ROT rate, DF-LIVESTOCK breeding, DF-NIGHT flavour, DF-FESTIVAL's
// schedule (already on [`SeasonalSchedule`]); later DF-TEMP/DF-BIOME-FX.
// NO consumer forks a private season counter (registered in the
// shared-substrate registry): everything derives from the four sibling
// reads below + [`SeasonConfig::current`] for the year length +
// [`SeasonalSchedule`] for day-of-year events. All PURE functions of the
// master clock — this block is the CONTRACT, not the behaviours (no
// consumer is wired here).
//
//   [`season`]      — the quarter bucket (discrete; UI labels, coarse
//                     gates).
//   [`year_phase`]  — the raw 0..1 annual position (custom curves).
//   [`day_of_year`] — the ordinal ([`SeasonalSchedule`]'s key).
//   [`season_bias`] — the canonical CONTINUOUS annual wave (below).

/// bastion (SEASON-2): [`Season::at`] as a free function — the documented
/// interface surface is these four sibling reads, uniformly callable.
pub fn season(time_of_day: f64, days_in_year: f64) -> Season {
    Season::at(time_of_day, days_in_year)
}

/// bastion (SEASON-2): the canonical seasonal WAVE, `-1.0..=1.0` — the
/// one continuous signal consumers MAP into their own semantics (FARM:
/// growth × (1 + k·bias); ROT: faster in the warm half; NIGHT: longer
/// in the cold half — each owns its k, none owns a private season).
/// A cosine anchored to the quarter definitions: +1 at MID-SUMMER
/// (phase 0.375), −1 at MID-WINTER (phase 0.875), 0 near the
/// spring/autumn midpoints — continuous across the year wrap (biology
/// doesn't step at quarter boundaries; consumers wanting steps bucket
/// via [`season`], the reverse being impossible is why the contract
/// ships the wave).
pub fn season_bias(time_of_day: f64, days_in_year: f64) -> f32 {
    let phase = year_phase(time_of_day, days_in_year);
    ((phase - 0.375) * std::f64::consts::TAU).cos() as f32
}

#[cfg(test)]
mod bastion_season2_tests {
    use super::*;

    /// SEASON-2's contract pinned: the wave's anchors are exact (+1
    /// mid-summer, −1 mid-winter, 0 at the spring/autumn midpoints),
    /// it stays in range, it's continuous across the year wrap, and the
    /// free-function surface agrees with the underlying derivations.
    #[test]
    fn bastion_season_bias_wave_anchors() {
        let days = 160.0;
        let day = crate::resources::DAY;
        let year = day * days;
        let at = |phase: f64| season_bias(year * phase, days);
        assert!((at(0.375) - 1.0).abs() < 1e-6, "mid-summer peak");
        assert!((at(0.875) + 1.0).abs() < 1e-6, "mid-winter trough");
        assert!(at(0.125).abs() < 1e-6, "mid-spring zero crossing");
        assert!(at(0.625).abs() < 1e-6, "mid-autumn zero crossing");
        // Range + wrap continuity (the wave never steps).
        for i in 0..=64 {
            let b = at(i as f64 / 64.0);
            assert!((-1.0..=1.0).contains(&b));
        }
        assert!((at(1.0 - 1e-9) - at(0.0)).abs() < 1e-3, "wrap continuity");
        // The uniform free-fn surface agrees with the originals.
        assert_eq!(season(year * 0.3, days), Season::at(year * 0.3, days));
        assert_eq!(season(year * 0.3, days), Season::Summer);
    }
}

/// bastion (SEASON-1, row 42): the day-of-year SCHEDULE — named events
/// fire on a configured in-game day (harvest = an autumn day, a holy-day
/// = day H): the in-game-calendar mirror of
/// [`crate::calendar::Calendar::is_event`], keyed on SEASON-0's derived
/// [`day_of_year`] instead of the real-world wall-clock date. The
/// real-world [`crate::calendar::Calendar`] stays completely orthogonal
/// — both can independently trigger the same festival (the design doc's
/// explicit invariant). This is ONLY the schedule/query mechanism:
/// no festival content, no consumers (DF-FESTIVAL subscribes later;
/// SEASON-2 owns the one-interface contract).
#[derive(Clone, Debug, Default, Deserialize)]
pub struct SeasonalSchedule {
    /// Named event → the day-of-year it fires (0-based, `< days_in_year`).
    pub events: std::collections::HashMap<String, u32>,
}

impl crate::assets::FileAsset for SeasonalSchedule {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(
        bytes: std::borrow::Cow<[u8]>,
    ) -> Result<Self, crate::assets::BoxedError> {
        crate::assets::load_ron(&bytes)
    }
}

impl SeasonalSchedule {
    /// The loaded schedule (hot-reloadable asset); an EMPTY schedule on a
    /// missing/broken asset — graceful: nothing fires, nothing panics.
    pub fn current() -> Self {
        use crate::assets::AssetExt;
        Self::load("common.seasonal_schedule")
            .map(|h| h.read().clone())
            .unwrap_or_default()
    }

    /// [`crate::calendar::Calendar::is_event`]'s mirror, one axis over:
    /// does `name` fire on this in-game day-of-year? Pure lookup —
    /// deterministic by construction (same day, same answer, always).
    pub fn is_event_on(&self, day_of_year: u32, name: &str) -> bool {
        self.events.get(name).is_some_and(|d| *d == day_of_year)
    }

    /// Every named event firing on this day, name-sorted (a deterministic
    /// iteration order for future consumers; the map itself is unordered).
    pub fn events_on(&self, day_of_year: u32) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .events
            .iter()
            .filter(|(_, d)| **d == day_of_year)
            .map(|(n, _)| n.as_str())
            .collect();
        names.sort_unstable();
        names
    }
}

#[cfg(test)]
mod bastion_season1_tests {
    use super::*;

    fn sample() -> SeasonalSchedule {
        crate::assets::load_ron(
            br#"(
    events: {
        "harvest": 90,
        "holy_day": 20,
        "founders_day": 20,
    },
)"#,
        )
        .expect("inline schedule RON parses")
    }

    /// SEASON-1's done-when in miniature: a named event fires on exactly
    /// its configured day-of-year (not adjacent days), same-day events
    /// coexist, unknown names and the empty default never fire, and the
    /// harvest entry really is an AUTUMN day under the shipped year shape.
    #[test]
    fn bastion_seasonal_schedule_fires_on_day() {
        let s = sample();
        assert!(s.is_event_on(90, "harvest"));
        assert!(!s.is_event_on(89, "harvest"));
        assert!(!s.is_event_on(91, "harvest"));
        assert!(s.is_event_on(20, "holy_day"));
        assert!(!s.is_event_on(90, "holy_day"));
        assert!(!s.is_event_on(90, "no_such_event"));
        assert_eq!(s.events_on(20), vec!["founders_day", "holy_day"]);
        assert_eq!(s.events_on(90), vec!["harvest"]);
        assert!(s.events_on(37).is_empty());
        assert!(!SeasonalSchedule::default().is_event_on(90, "harvest"));
        // The done-when's own phrasing pinned: harvest = an AUTUMN day
        // (day 90 of a 160-day year sits in the third quarter), through
        // SEASON-0's derivation.
        let day = crate::resources::DAY;
        assert_eq!(Season::at(day * 90.0, 160.0), Season::Autumn);
        assert_eq!(day_of_year(day * 90.0, 160.0), 90);
    }
}

#[cfg(test)]
mod bastion_season_tests {
    use super::*;

    /// SEASON-0's done-when in miniature: correct season at any
    /// TimeOfDay for a given year length, quarter boundaries exact,
    /// wrap-around clean, phase/day-of-year consistent, and a different
    /// year length re-buckets correctly (the tunable is real).
    #[test]
    fn bastion_season_quarters_exact() {
        let days = 160.0;
        let day = crate::resources::DAY;
        let year = day * days;
        assert_eq!(Season::at(0.0, days), Season::Spring);
        assert_eq!(Season::at(year * 0.25 - 1.0, days), Season::Spring);
        assert_eq!(Season::at(year * 0.25, days), Season::Summer);
        assert_eq!(Season::at(year * 0.5, days), Season::Autumn);
        assert_eq!(Season::at(year * 0.75, days), Season::Winter);
        assert_eq!(Season::at(year - 1.0, days), Season::Winter);
        // Wrap-around: year N+1 buckets like year N (pure rem_euclid).
        assert_eq!(Season::at(year + day, days), Season::Spring);
        assert_eq!(Season::at(year * 7.5, days), Season::Autumn);
        // Phase + ordinal consistency.
        assert!((year_phase(year * 0.5, days) - 0.5).abs() < 1e-9);
        assert_eq!(day_of_year(0.0, days), 0);
        assert_eq!(day_of_year(day * 39.0 + 1.0, days), 39);
        assert_eq!(day_of_year(year + day * 3.0, days), 3);
        // A different RON year length re-buckets correctly (tunable).
        assert_eq!(Season::at(day * 30.0, 40.0), Season::Winter);
        assert_eq!(Season::at(day * 30.0, 160.0), Season::Spring);
    }
}
