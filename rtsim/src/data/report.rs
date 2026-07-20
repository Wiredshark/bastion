use common::{
    resources::TimeOfDay,
    rtsim::{Actor, SiteId},
    terrain::SpriteKind,
};
use hashbrown::HashSet;
use serde::{Deserialize, Serialize, Serializer, ser::SerializeSeq};
use slotmap::DenseSlotMap;
use std::ops::{Deref, DerefMut};
use vek::*;

pub use common::rtsim::ReportId;

/// A report set with hash-based membership and deterministic persistence and
/// iteration. Iteration is infrequent (site/NPC report exchange) and report
/// sets are bounded by cleanup, so sorting this boundary avoids changing the
/// O(1) membership path used by NPC AI.
#[derive(Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct KnownReports(HashSet<ReportId>);

impl KnownReports {
    pub fn iter(&self) -> impl Iterator<Item = &ReportId> {
        let mut reports = self.0.iter().collect::<Vec<_>>();
        reports.sort_unstable();
        reports.into_iter()
    }
}

impl Serialize for KnownReports {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.len()))?;
        for report in self.iter() {
            sequence.serialize_element(report)?;
        }
        sequence.end()
    }
}

impl Deref for KnownReports {
    type Target = HashSet<ReportId>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for KnownReports {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

/// Represents a single piece of information known by an rtsim entity.
///
/// Reports are the medium through which rtsim represents information sharing
/// between NPCs, factions, and sites. They can represent deaths, attacks,
/// changes in diplomacy, or any other piece of information representing a
/// singular event that might be communicated.
///
/// Note that they should not be used to communicate sentiments like 'this actor
/// is friendly': the [`crate::data::Sentiment`] system should be used for that.
/// Some events might generate both a report and a change in sentiment. For
/// example, the murder of an NPC might generate both a murder report and highly
/// negative sentiments.
#[derive(Clone, Serialize, Deserialize)]
pub struct Report {
    pub kind: ReportKind,
    pub at_tod: TimeOfDay,
}

impl Report {
    /// The time, in in-game seconds, for which the report will be remembered
    fn remember_for(&self) -> f64 {
        const DAYS: f64 = 60.0 * 60.0 * 24.0;
        match &self.kind {
            ReportKind::Death { killer, .. } => {
                if killer.is_some() {
                    // Murder is less easy to forget
                    DAYS * 15.0
                } else {
                    DAYS * 5.0
                }
            },
            // TODO: Could consider what was stolen here
            ReportKind::Theft { .. } => DAYS * 1.5,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum ReportKind {
    Death {
        actor: Actor,
        killer: Option<Actor>,
    },
    Theft {
        thief: Actor,
        /// Where the theft happened.
        site: Option<SiteId>,
        /// What was stolen.
        sprite: SpriteKind,
    },
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Reports {
    pub reports: DenseSlotMap<ReportId, Report>,
}

impl Reports {
    pub fn create(&mut self, report: Report) -> ReportId { self.reports.insert(report) }

    pub fn cleanup(&mut self, current_time: TimeOfDay) {
        // Forget reports that are too old
        self.reports.retain(|_, report| {
            (current_time.0 - report.at_tod.0).max(0.0) < report.remember_for()
        });
        // TODO: Limit global number of reports
    }
}

impl Deref for Reports {
    type Target = DenseSlotMap<ReportId, Report>;

    fn deref(&self) -> &Self::Target { &self.reports }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::KeyData;
    use std::collections::{BTreeSet, HashSet};

    #[test]
    fn persisted_known_reports_have_stable_bytes() {
        let reports = (1..=8)
            .map(|raw| ReportId::from(KeyData::from_ffi(raw)))
            .collect::<Vec<_>>();
        let mut encodings = BTreeSet::new();
        for shift in 0..reports.len() {
            let mut known = KnownReports::default();
            for offset in 0..reports.len() {
                known.insert(reports[(shift + offset) % reports.len()]);
            }
            encodings.insert(rmp_serde::to_vec_named(&known).expect("encode known reports"));
        }
        println!(
            "known_reports distinct persistence encodings={}",
            encodings.len()
        );
        if let Some(first) = encodings.first() {
            println!("known_reports representative_msgpack={}", hex(first));
        }
        assert_eq!(
            encodings.len(),
            1,
            "equal known-report state must have one persisted representation"
        );
    }

    #[test]
    fn legacy_hash_set_reports_remain_loadable() {
        let reports = (1..=3)
            .map(|raw| ReportId::from(KeyData::from_ffi(raw)))
            .collect::<HashSet<_>>();
        let bytes = rmp_serde::to_vec_named(&reports).expect("encode legacy report set");
        let decoded: KnownReports =
            rmp_serde::from_slice(&bytes).expect("decode ordered report set");
        assert_eq!(decoded.len(), reports.len());
        assert!(reports.iter().all(|report| decoded.contains(report)));
    }

    #[test]
    fn known_report_iteration_is_key_ordered() {
        let mut reports = KnownReports::default();
        reports.insert(ReportId::from(KeyData::from_ffi(3)));
        reports.insert(ReportId::from(KeyData::from_ffi(1)));
        reports.insert(ReportId::from(KeyData::from_ffi(2)));
        let ordered = reports.iter().copied().collect::<Vec<_>>();
        assert!(ordered.windows(2).all(|pair| pair[0] < pair[1]));
    }

    fn hex(bytes: &[u8]) -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() }
}
