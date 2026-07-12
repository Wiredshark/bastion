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
