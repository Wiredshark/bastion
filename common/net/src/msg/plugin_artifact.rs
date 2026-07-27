//! `APEX-T2.5.10` — typed plugin-artifact wire messages + the pure
//! arrival collector.
//!
//! Replaces the raw `RequestPlugins(Vec<PluginHash>)` /
//! `PluginData(Vec<u8>)` shape (which made NETWORK ARRIVAL ORDER the
//! runtime order) with root/ordinal/digest/size descriptors and a
//! collector that is arrival-order-free by construction: every response
//! is verified against the deployment requirement set before it counts,
//! duplicates are classified (benign vs conflicting), and completion is
//! an explicit terminal — a missing artifact can never be silently
//! skipped. The legacy messages stay for the explicit legacy mode only.

use common::apex::digest::hash_artifact_bytes_v1;
use serde::{Deserialize, Serialize};

/// One exact artifact requirement as it crosses the wire. Digest/root are
/// raw 32-byte values here (the typed domain wrappers live in
/// common-state; the boundary converts and re-verifies — wire bytes are
/// never trusted as identity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginArtifactDescriptorV1 {
    pub deployment_root: [u8; 32],
    pub ordinal: u32,
    pub digest: [u8; 32],
    pub size_bytes: u64,
}

/// The deployment as `ServerInit::GameSync` carries it: everything a
/// client needs to acquire and verify artifacts BEFORE constructing
/// State. `None` in GameSync = explicit legacy mode (hash-list path).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginDeploymentSummaryV1 {
    pub deployment_root: [u8; 32],
    /// Every artifact in the deployment (ordinal-complete, not
    /// client-filtered: filtering is the client projection's job and the
    /// summary must stay byte-identical for every client).
    pub requirements: Vec<PluginArtifactDescriptorV1>,
    /// The client-mode projection: ordinals active on clients, plus the
    /// activation root the client must reproduce.
    pub client_activations: Vec<u32>,
    pub client_activation_root: [u8; 32],
    /// APEX-T2.5.14: the client-mode runtime ceilings from the operator
    /// policy — every governed client runs its modules under EXACTLY
    /// these (no client-local defaults).
    pub client_runtime_limits: PluginRuntimeLimitsWireV1,
    /// APEX-T2.5.20: command -> owning archive digest (sole claimant or
    /// the operator's ExclusiveOwner ruling), command-sorted — the
    /// client's one-lookup dispatch map mirrors the server's exactly.
    pub command_owners: Vec<(String, [u8; 32])>,
    /// APEX-T2.5.21: animation/skeleton key -> owning archive digest,
    /// same one-lookup discipline as commands.
    pub skeleton_owners: Vec<(String, [u8; 32])>,
}

/// Scalar mirror of the policy's per-mode runtime limits (wire crate has
/// no policy types on purpose).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRuntimeLimitsWireV1 {
    pub max_linear_memory_bytes: u64,
    pub max_fuel_per_event: u64,
    pub max_instances: u32,
}

/// Client → server: exact ordinals wanted for one deployment root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginArtifactRequestV1 {
    pub deployment_root: [u8; 32],
    pub ordinals: Vec<u32>,
}

/// Server → client: one artifact, self-describing. Transport order free.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginArtifactResponseV1 {
    pub descriptor: PluginArtifactDescriptorV1,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginArtifactWireErrorV1 {
    /// Response for a different deployment root than this collector's.
    StaleRoot { got: [u8; 32] },
    /// Ordinal not in the requirement set.
    UnknownOrdinal { ordinal: u32 },
    /// Descriptor does not match the requirement for that ordinal.
    DescriptorMismatch { ordinal: u32 },
    SizeMismatch { ordinal: u32, expected: u64, got: u64 },
    DigestMismatch { ordinal: u32 },
    /// Same ordinal already verified with DIFFERENT bytes claimed — the
    /// second response is refused and the stream is poisoned for audit.
    ConflictingDuplicate { ordinal: u32 },
    /// Terminal check: requested artifacts never arrived.
    MissingArtifacts { ordinals: Vec<u32> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginArtifactAcceptV1 {
    /// Verified and recorded.
    Accepted { ordinal: u32 },
    /// Byte-identical duplicate: no state change, benign.
    DuplicateBenign { ordinal: u32 },
}

/// Pure arrival collector for ONE deployment. Feed responses in ANY
/// order; the verified set is a function of the requirement set and the
/// response CONTENTS only.
pub struct PluginArtifactCollectorV1 {
    deployment_root: [u8; 32],
    requirements: Vec<PluginArtifactDescriptorV1>,
    verified: Vec<(u32, Vec<u8>)>,
}

impl PluginArtifactCollectorV1 {
    /// `requirements` must all carry `deployment_root` (enforced here so
    /// a mixed-set collector is unconstructible).
    pub fn new(
        deployment_root: [u8; 32],
        requirements: Vec<PluginArtifactDescriptorV1>,
    ) -> Result<Self, PluginArtifactWireErrorV1> {
        if let Some(bad) = requirements.iter().find(|r| r.deployment_root != deployment_root) {
            return Err(PluginArtifactWireErrorV1::StaleRoot { got: bad.deployment_root });
        }
        Ok(Self { deployment_root, requirements, verified: Vec::new() })
    }

    /// Verify one response: root → membership → descriptor → size →
    /// digest → duplicate classification. Refusals change no state.
    pub fn accept(
        &mut self,
        response: &PluginArtifactResponseV1,
    ) -> Result<PluginArtifactAcceptV1, PluginArtifactWireErrorV1> {
        let d = &response.descriptor;
        if d.deployment_root != self.deployment_root {
            return Err(PluginArtifactWireErrorV1::StaleRoot { got: d.deployment_root });
        }
        let required = self
            .requirements
            .iter()
            .find(|r| r.ordinal == d.ordinal)
            .ok_or(PluginArtifactWireErrorV1::UnknownOrdinal { ordinal: d.ordinal })?;
        if required != d {
            return Err(PluginArtifactWireErrorV1::DescriptorMismatch { ordinal: d.ordinal });
        }
        if response.bytes.len() as u64 != required.size_bytes {
            return Err(PluginArtifactWireErrorV1::SizeMismatch {
                ordinal: d.ordinal,
                expected: required.size_bytes,
                got: response.bytes.len() as u64,
            });
        }
        let identity = hash_artifact_bytes_v1(&response.bytes);
        if identity.digest.bytes.as_array() != &required.digest {
            return Err(PluginArtifactWireErrorV1::DigestMismatch { ordinal: d.ordinal });
        }
        if let Some((_, existing)) = self.verified.iter().find(|(o, _)| *o == d.ordinal) {
            // Digest equality above means bytes match too — but check
            // bytes directly so a hash-collision duplicate can never pass
            // as benign.
            return if existing == &response.bytes {
                Ok(PluginArtifactAcceptV1::DuplicateBenign { ordinal: d.ordinal })
            } else {
                Err(PluginArtifactWireErrorV1::ConflictingDuplicate { ordinal: d.ordinal })
            };
        }
        self.verified.push((d.ordinal, response.bytes.clone()));
        Ok(PluginArtifactAcceptV1::Accepted { ordinal: d.ordinal })
    }

    /// The explicit completion terminal: every requirement verified, or a
    /// typed list of what never arrived. Output is ordinal-sorted —
    /// arrival order is not observable downstream.
    pub fn finish(self) -> Result<Vec<(u32, Vec<u8>)>, PluginArtifactWireErrorV1> {
        let mut missing: Vec<u32> = self
            .requirements
            .iter()
            .filter(|r| !self.verified.iter().any(|(o, _)| *o == r.ordinal))
            .map(|r| r.ordinal)
            .collect();
        if !missing.is_empty() {
            missing.sort_unstable();
            return Err(PluginArtifactWireErrorV1::MissingArtifacts { ordinals: missing });
        }
        let mut out = self.verified;
        out.sort_by_key(|(o, _)| *o);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(root: [u8; 32], ordinal: u32, bytes: &[u8]) -> PluginArtifactDescriptorV1 {
        let id = hash_artifact_bytes_v1(bytes);
        PluginArtifactDescriptorV1 {
            deployment_root: root,
            ordinal,
            digest: *id.digest.bytes.as_array(),
            size_bytes: bytes.len() as u64,
        }
    }

    #[test]
    fn plugin_artifact_wire_v1_is_arrival_order_free_and_fails_closed() {
        let root = [3u8; 32];
        let payloads: Vec<Vec<u8>> = (0..3u8).map(|i| vec![i; 8 + i as usize]).collect();
        let reqs: Vec<_> = payloads.iter().enumerate().map(|(i, b)| descriptor(root, i as u32, b)).collect();
        let response = |i: usize| PluginArtifactResponseV1 { descriptor: reqs[i].clone(), bytes: payloads[i].clone() };

        // Reverse order and interleaved-duplicate order produce the SAME
        // verified set as forward order.
        let run = |order: &[usize]| {
            let mut c = PluginArtifactCollectorV1::new(root, reqs.clone()).unwrap();
            for &i in order {
                let r = c.accept(&response(i)).unwrap();
                assert!(matches!(
                    r,
                    PluginArtifactAcceptV1::Accepted { .. } | PluginArtifactAcceptV1::DuplicateBenign { .. }
                ));
            }
            c.finish().unwrap()
        };
        let forward = run(&[0, 1, 2]);
        assert_eq!(forward, run(&[2, 1, 0]), "reverse arrival must not change the outcome");
        assert_eq!(forward, run(&[1, 2, 0, 1]), "benign duplicate must not change the outcome");

        // Wrong bytes for a requested ordinal: refused, and completion
        // still reports the artifact missing (never silently degraded).
        let mut c = PluginArtifactCollectorV1::new(root, reqs.clone()).unwrap();
        let mut bad = response(0);
        bad.bytes[0] ^= 0xff;
        assert!(matches!(c.accept(&bad), Err(PluginArtifactWireErrorV1::DigestMismatch { ordinal: 0 })));
        c.accept(&response(1)).unwrap();
        c.accept(&response(2)).unwrap();
        assert!(matches!(
            c.finish(),
            Err(PluginArtifactWireErrorV1::MissingArtifacts { ordinals }) if ordinals == vec![0]
        ));

        // Stale root refused at both construction and accept.
        let mut c = PluginArtifactCollectorV1::new(root, reqs.clone()).unwrap();
        let mut stale = response(0);
        stale.descriptor.deployment_root = [9u8; 32];
        assert!(matches!(c.accept(&stale), Err(PluginArtifactWireErrorV1::StaleRoot { .. })));
        assert!(matches!(
            PluginArtifactCollectorV1::new([9u8; 32], reqs.clone()),
            Err(PluginArtifactWireErrorV1::StaleRoot { .. })
        ));

        // Unknown ordinal + tampered descriptor.
        let mut c = PluginArtifactCollectorV1::new(root, reqs.clone()).unwrap();
        let mut unknown = response(0);
        unknown.descriptor.ordinal = 9;
        assert!(matches!(c.accept(&unknown), Err(PluginArtifactWireErrorV1::UnknownOrdinal { ordinal: 9 })));
        let mut lied = response(0);
        lied.descriptor.size_bytes += 1;
        assert!(matches!(c.accept(&lied), Err(PluginArtifactWireErrorV1::DescriptorMismatch { ordinal: 0 })));

        // Conflicting duplicate: ordinal 0 verified, then a second
        // response for ordinal 0 whose descriptor was re-pointed at other
        // bytes — descriptor mismatch catches it before any state change.
        let mut c = PluginArtifactCollectorV1::new(root, reqs.clone()).unwrap();
        c.accept(&response(0)).unwrap();
        let conflicting = PluginArtifactResponseV1 {
            descriptor: PluginArtifactDescriptorV1 { ordinal: 0, ..descriptor(root, 0, b"other-bytes") },
            bytes: b"other-bytes".to_vec(),
        };
        assert!(matches!(
            c.accept(&conflicting),
            Err(PluginArtifactWireErrorV1::DescriptorMismatch { ordinal: 0 })
        ));
        assert_eq!(c.finish().unwrap_err(), PluginArtifactWireErrorV1::MissingArtifacts { ordinals: vec![1, 2] });
    }
}
