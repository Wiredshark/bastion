//! bastion (INSPECTOR-M1): the game clock, PURE.
//!
//! ★ WHY THIS MODULE EXISTS: TWO FRAMES COMPARED AS ONE.
//!
//! Bastion carries at least four different "times" and they are not
//! interchangeable:
//!
//! | name | unit | epoch | survives restart |
//! |---|---|---|---|
//! | `Tick` (`bastion_server::Tick`) | server ticks | process boot | NO |
//! | `Data.tick` (rtsim) | rtsim ticks | world creation | YES |
//! | `TimeOfDay` | GAME seconds | reset to `start_time` every boot | NO |
//! | `Time` | SIM seconds | process boot | NO |
//!
//! An inspector that prints a number without saying which of these it came
//! from is worse than one that prints nothing, because the reader will
//! assume. So every clock the inspector ships is named at the point of use
//! ([`crate::comp::bastion_inspect::InspectFramesV1`]), and every
//! conversion between them lives HERE, once.
//!
//! ★ THE DEFECT THIS MODULE IS SHAPED AROUND. `BastionColonist::born_day`
//! (`crate::bastion`, field docs) is stamped from `TimeOfDay`, which the
//! server resets to `settings.world.start_time` at EVERY boot. Measured on
//! world 109, restarted on its own save: `game_day` read 0 after both
//! boots having reached ~4 before the restart, so `today - born_day` goes
//! NEGATIVE and no child ever comes of age. The repair was `born_tick`
//! (rtsim `Data.tick`, persistent). [`age_days`] therefore accepts
//! `born_tick` ONLY — there is no overload that takes a day index, so the
//! boot-relative field cannot be fed to it by accident. `born_day` may be
//! DISPLAYED, but only under a label that says it is boot-relative.
//!
//! Nothing in here touches the ECS, rtsim, or the job board: it is
//! arithmetic over numbers the caller already read, so it is pinnable
//! without a world.

/// Sim seconds per server tick at the declared simulation cadence
/// (`bastion_server::SIM_TPS` = 30). Kept as a ratio rather than a
/// hard-coded 0.0333 so the pin below reads as arithmetic, not as a
/// remembered constant.
pub const DEFAULT_DT_SECS: f64 = 1.0 / 30.0;

/// `day_cycle_coefficient` at a default server: `1440.0 / day_length`
/// with `day_length = DAY_LENGTH_DEFAULT` (30 minutes,
/// `crate::consts::DAY_LENGTH_DEFAULT`). Derived here so the pin cannot
/// silently disagree with the server settings module.
pub const DEFAULT_DAY_CYCLE_COEFFICIENT: f64 = 1440.0 / crate::consts::DAY_LENGTH_DEFAULT;

/// The MEASURED default-server figure: 54,000 ticks = one game day.
///
/// This is a fallback and a pin target, never the primary producer —
/// [`ticks_per_game_day`] DERIVES the number so a harness running any
/// other `day_length` gets its own answer instead of silently meaning
/// something else. A fallback that is zero would age every child
/// instantly, which is why the guard below refuses non-finite and
/// non-positive denominators rather than dividing.
pub const FALLBACK_TICKS_PER_GAME_DAY: f64 = 54_000.0;

/// Ticks per GAME DAY, derived rather than assumed.
///
/// `TimeOfDay` advances by `dt * day_cycle_coefficient` game-seconds per
/// tick while a tick is `dt` of sim time, so one game day
/// ([`crate::resources::DAY`] game-seconds) takes
/// `DAY / (dt * coefficient)` ticks. At the default server
/// (`dt = 1/30`, coefficient 48) that is 54,000 — the figure measured off
/// the live log.
///
/// ★ THE CLIENT CANNOT COMPUTE THIS. It receives `day_cycle_coefficient`
/// in `ServerConstants` but never receives `dt`, so an age rendered
/// client-side from the coefficient alone would be wrong by exactly the
/// tick rate. That is why the frames block ships the finished number.
pub fn ticks_per_game_day(dt_secs: f64, day_cycle_coefficient: f64) -> f64 {
    let denom = dt_secs * day_cycle_coefficient;
    if denom.is_finite() && denom > 0.0 {
        crate::resources::DAY / denom
    } else {
        FALLBACK_TICKS_PER_GAME_DAY
    }
}

/// Sim seconds in one GAME hour. 3600 game-seconds divided by the
/// coefficient; at the default server 3600/48 = 75 sim-seconds, which is
/// the measured figure every duration constant in the colony should be
/// checked against.
pub fn game_hour_in_sim_secs(day_cycle_coefficient: f64) -> f64 {
    if day_cycle_coefficient.is_finite() && day_cycle_coefficient > 0.0 {
        3600.0 / day_cycle_coefficient
    } else {
        3600.0 / DEFAULT_DAY_CYCLE_COEFFICIENT
    }
}

/// Age in GAME DAYS from the PERSISTENT clock only.
///
/// * `rtsim_tick` — rtsim `Data.tick`, the counter that survives restart.
/// * `born_tick` — `BastionColonist::born_tick`, stamped from the same
///   counter. `None` for founders and settlers, who arrived grown and
///   must never age: the honest answer there is "unknown", not zero.
///
/// Returns `None` when the age is not knowable:
/// * no `born_tick` recorded, or
/// * `rtsim_tick < born_tick` — a counter that ran backwards means a
///   rolled-back save. REFUSE rather than underflow. This is also the
///   arm that makes feeding a boot-relative day index here fail loudly
///   instead of quietly reporting a negative age as a huge positive one.
///
/// There is deliberately NO variant of this function taking `born_day`.
pub fn age_days(rtsim_tick: u64, born_tick: Option<u64>, ticks_per_game_day: f64) -> Option<f64> {
    let born = born_tick?;
    let elapsed = rtsim_tick.checked_sub(born)?;
    let per_day = if ticks_per_game_day.is_finite() && ticks_per_game_day > 0.0 {
        ticks_per_game_day
    } else {
        FALLBACK_TICKS_PER_GAME_DAY
    };
    Some(elapsed as f64 / per_day)
}

/// Game-seconds since world start → hour of the game day, 0..=23.
///
/// `TimeOfDay` counts game-seconds and [`crate::resources::DAY`] is 86,400
/// of them. `rem_euclid` (not `%`) so a negative rotated clock — which
/// [`colonist_effective_tod`] produces every time a watchman's offset
/// exceeds the wall hour — wraps forward instead of yielding a negative
/// hour that then indexes the wrong schedule block.
pub fn hour_of_day(time_of_day: f64) -> u32 {
    let day = crate::resources::DAY;
    let secs = time_of_day.rem_euclid(day);
    ((secs / 3600.0).floor() as i64).clamp(0, 23) as u32
}

/// The WALL clock's day index (boot-relative, and labelled as such
/// wherever it is shown — `TimeOfDay` is reset at every boot).
pub fn wall_day_index(time_of_day: f64) -> i64 {
    (time_of_day / crate::resources::DAY).floor() as i64
}

/// The time of day IN ONE COLONIST'S OWN FRAME.
///
/// A rotated schedule means a colonist's own hour is NOT the wall hour.
/// The night watch runs at `offset = 14`, and reading the raw global clock
/// for a rotated colonist already shipped once as a real defect: the
/// evening palette fired on the watchman's wake-up and never on his real
/// evening, and his leisure hold was clamped against a window he is not
/// in.
///
/// Mirrors `bastion_server::bastion_jobs::colonist_effective_tod` exactly
/// — SUBTRACT `offset * 3600`. The sign matters and is the whole bug: an
/// inverted rotation still produces a plausible-looking hour for every
/// input, so it cannot be caught by eye.
///
/// Deliberately NOT wrapped to one day: consumers that want an hour take
/// their own `rem_euclid` (see [`hour_of_day`]), and a modulo here would
/// collapse every day to day 0 for anything hashing on `day * 24 + hour`.
pub fn colonist_effective_tod(time_of_day: f64, schedule_offset_hours: u32) -> f64 {
    time_of_day - f64::from(schedule_offset_hours % 24) * 3600.0
}

/// The colonist's OWN hour, 0..=23 — the one their schedule block is
/// looked up with. For an unrotated colonist this is the wall hour.
pub fn colonist_hour(time_of_day: f64, schedule_offset_hours: u32) -> u32 {
    hour_of_day(colonist_effective_tod(time_of_day, schedule_offset_hours))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measured figure, reproduced from the settings the server
    /// actually ships rather than typed in.
    ///
    /// FALSIFIER: change `DEFAULT_DT_SECS` to 1/60 (a 60tps server) and
    /// this goes to 108,000 and RED. Change
    /// `crate::consts::DAY_LENGTH_DEFAULT` and it moves too — which is
    /// the point: the number is derived, so it tracks a settings change
    /// instead of quietly becoming a lie.
    #[test]
    fn ticks_per_game_day_is_54000_at_default() {
        assert!((DEFAULT_DAY_CYCLE_COEFFICIENT - 48.0).abs() < 1e-9, "coefficient moved");
        let t = ticks_per_game_day(DEFAULT_DT_SECS, DEFAULT_DAY_CYCLE_COEFFICIENT);
        assert!((t - 54_000.0).abs() < 1e-6, "ticks per game day is {t}, expected 54000");
        // One game hour = 75 sim-seconds at the same settings.
        let h = game_hour_in_sim_secs(DEFAULT_DAY_CYCLE_COEFFICIENT);
        assert!((h - 75.0).abs() < 1e-9, "game hour is {h} sim-seconds, expected 75");
        // And the two agree: 24 hours of sim-seconds x 30 ticks/sec.
        assert!((h * 24.0 / DEFAULT_DT_SECS - t).abs() < 1e-6);
    }

    /// A degenerate coefficient must not divide by zero, and must not
    /// return zero either — a zero day length ages every child instantly.
    #[test]
    fn ticks_per_game_day_refuses_degenerate_settings() {
        for (dt, c) in [(0.0, 48.0), (1.0 / 30.0, 0.0), (f64::NAN, 48.0), (-1.0, 48.0)] {
            let t = ticks_per_game_day(dt, c);
            assert_eq!(t, FALLBACK_TICKS_PER_GAME_DAY, "dt {dt} coeff {c} must fall back");
        }
    }

    /// ★ THE BOOT-RELATIVE REFUSAL.
    ///
    /// `age_days` takes `born_tick` only. The arm under test is the
    /// backwards clock: `rtsim_tick < born_tick` is what a rolled-back
    /// save looks like, and it is also what feeding a boot-relative day
    /// index looks like once the world has run past it.
    ///
    /// FALSIFIER: replace `checked_sub` with `-` (or with
    /// `saturating_sub`) in `age_days` and this test goes RED — saturating
    /// would report age 0 (a newborn) for a rolled-back record, which is
    /// the "plausible wrong number" this refuses to print.
    #[test]
    fn age_refuses_boot_relative_day() {
        let per_day = 54_000.0;
        // No record: unknown, not zero.
        assert_eq!(age_days(100_000, None, per_day), None);
        // Backwards clock (a rolled-back save, or a boot-relative index
        // handed to a persistent-clock function): refuse.
        assert_eq!(age_days(10, Some(54_010), per_day), None);
        // The ordinary case still answers.
        let a = age_days(54_000 * 4, Some(0), per_day).expect("a knowable age");
        assert!((a - 4.0).abs() < 1e-9, "age was {a}, expected 4 game days");
        // Exactly-equal ticks is age zero, not a refusal: a colonist born
        // this tick has a knowable age.
        assert_eq!(age_days(7, Some(7), per_day), Some(0.0));
    }

    /// The rotation's SIGN, pinned in both directions.
    ///
    /// An inverted rotation produces a plausible hour for every input, so
    /// only a two-sided pin says anything. Offset 0 must be the identity
    /// (the fallback-must-be-identity law): every unrotated colonist is
    /// bit-identical to the wall clock.
    #[test]
    fn colonist_hour_rotates_and_zero_is_identity() {
        const H: f64 = 3600.0;
        // Identity at offset 0, for every hour.
        for h in 0..24u32 {
            assert_eq!(colonist_hour(f64::from(h) * H, 0), h, "offset 0 must be identity");
        }
        // The night watch (offset 14): wall 02 -> own 12. SUBTRACTION.
        assert_eq!(colonist_hour(2.0 * H, 14), 12);
        // The other direction, which an inverted sign would also have to
        // satisfy and cannot: wall 12 -> own 22.
        assert_eq!(colonist_hour(12.0 * H, 14), 22);
        // An inverted sign would give wall 02 -> own 16 and wall 12 -> 02.
        assert_ne!(colonist_hour(2.0 * H, 14), 16);
        // Negative rotated clocks wrap FORWARD (rem_euclid, not %).
        assert!(colonist_effective_tod(2.0 * H, 14) < 0.0, "the fixture must go negative");
        assert_eq!(hour_of_day(-1.0), 23);
    }
}
