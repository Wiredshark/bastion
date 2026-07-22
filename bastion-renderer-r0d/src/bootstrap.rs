//! BUILD-007A10.2 — D1 authoritative-simulation determinism substrate:
//! deterministic bootstrap seed registry (design §6) and the fixed-step tick
//! contract (§7). This is the self-contained, golden-vector-checkable core of
//! the packet; the live-loop wiring (extract phase in the real dispatcher,
//! bootstrap-before-world/RTSim/server) is the integration surface built later.
//!
//! Seed derivation is HKDF-SHA256 (§6.3) implemented over the crate's single
//! `sha2` dependency — HMAC-SHA256 + HKDF built here so the derivation is
//! deterministic by construction and provable against RFC 5869 vectors. The
//! seed-domain registry is a CLOSED set: an undeclared request is a typed
//! `BootstrapUndeclaredSeedDomain`, never a silent OS-entropy fallback.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const SHA256_BLOCK: usize = 64;
const SHA256_OUT: usize = 32;

/// HMAC-SHA256(key, msg). Standard construction (RFC 2104): keys longer than
/// the block size are pre-hashed; shorter keys are zero-padded.
#[must_use]
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut block = [0u8; SHA256_BLOCK];
    if key.len() > SHA256_BLOCK {
        let kh = Sha256::digest(key);
        block[..SHA256_OUT].copy_from_slice(&kh);
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; SHA256_BLOCK];
    let mut opad = [0x5cu8; SHA256_BLOCK];
    for i in 0..SHA256_BLOCK {
        ipad[i] ^= block[i];
        opad[i] ^= block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg);
    let inner = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner);
    outer.finalize().into()
}

/// HKDF-Extract (RFC 5869): PRK = HMAC(salt, ikm).
#[must_use]
pub fn hkdf_extract(salt: &[u8], ikm: &[u8]) -> [u8; 32] {
    hmac_sha256(salt, ikm)
}

/// HKDF-Expand (RFC 5869) to `len` bytes. `len` must be `<= 255*32`.
#[must_use]
pub fn hkdf_expand(prk: &[u8; 32], info: &[u8], len: usize) -> Vec<u8> {
    assert!(len <= 255 * SHA256_OUT, "HKDF-Expand length out of range");
    let mut out = Vec::with_capacity(len);
    let mut t: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while out.len() < len {
        let mut msg = Vec::with_capacity(t.len() + info.len() + 1);
        msg.extend_from_slice(&t);
        msg.extend_from_slice(info);
        msg.push(counter);
        t = hmac_sha256(prk, &msg).to_vec();
        let take = (len - out.len()).min(SHA256_OUT);
        out.extend_from_slice(&t[..take]);
        counter = counter.wrapping_add(1);
    }
    out
}

/// Fixed R0D bootstrap salt (§6.3): `SHA256("bastion/r0d/bootstrap-seed/v1")`.
#[must_use]
pub fn bootstrap_salt() -> [u8; 32] {
    Sha256::digest(b"bastion/r0d/bootstrap-seed/v1").into()
}

/// A closed seed-domain declaration (§6.3/§6.4): every constructor reachable
/// before server creation must declare its domain identity here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedDomainDeclarationV1 {
    /// ASCII domain label, e.g. `"bastion/r0d/worldgen"`.
    pub domain: String,
    pub schema_major: u16,
    pub schema_minor: u16,
    /// 32-byte owner key digest binding the domain to its sole consumer.
    pub owner_digest: [u8; 32],
}

impl SeedDomainDeclarationV1 {
    /// HKDF-Expand info per §6.3: length-framed domain || schema || owner.
    /// Framing mirrors `crate::domain_hash` so a domain/schema change can never
    /// alias to another domain's seed.
    fn info(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(2 + self.domain.len() + 4 + 32);
        v.extend_from_slice(&(self.domain.len() as u16).to_le_bytes());
        v.extend_from_slice(self.domain.as_bytes());
        v.extend_from_slice(&self.schema_major.to_le_bytes());
        v.extend_from_slice(&self.schema_minor.to_le_bytes());
        v.extend_from_slice(&self.owner_digest);
        v
    }
}

/// Typed bootstrap failures (§6). Every one is terminal — R0D never continues
/// best-effort past a bootstrap identity divergence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BootstrapError {
    /// A seed was requested for a domain not in the closed registry (§6.3).
    UndeclaredSeedDomain { domain: String },
    /// Two declarations share a domain label — the registry is ambiguous.
    DuplicateSeedDomain { domain: String },
    /// Two plugin providers share a machine id with differing digests (§6.4).
    PluginIdentityConflict { plugin_machine_id: String },
    /// `terminal_tick_exclusive` exceeds the signed-64 valid range (§7.1).
    TickRangeOverflow { terminal_tick_exclusive: u64 },
    /// `terminal_tick_exclusive` exceeds the V1 hard scenario cap (§7.1).
    TickCapExceeded { terminal_tick_exclusive: u64, cap: u64 },
    /// `simulation_tps` is zero — no valid fixed step exists (§7.1).
    ZeroSimulationTps,
}

/// The closed seed-domain registry. Construction validates uniqueness; seed
/// requests are checked against the declared set.
#[derive(Clone, Debug)]
pub struct SeedRegistryV1 {
    root_seed: [u8; 32],
    prk: [u8; 32],
    domains: BTreeMap<String, SeedDomainDeclarationV1>,
}

impl SeedRegistryV1 {
    /// Build the registry from the root seed and the closed declaration set.
    /// Declaration order is diagnostic only — the `BTreeMap` key order is what
    /// the identity depends on. Duplicate labels are a typed terminal error.
    pub fn new(
        root_seed: [u8; 32],
        declarations: Vec<SeedDomainDeclarationV1>,
    ) -> Result<Self, BootstrapError> {
        let salt = bootstrap_salt();
        let prk = hkdf_extract(&salt, &root_seed);
        let mut domains = BTreeMap::new();
        for d in declarations {
            if domains.insert(d.domain.clone(), d.clone()).is_some() {
                return Err(BootstrapError::DuplicateSeedDomain { domain: d.domain });
            }
        }
        Ok(Self {
            root_seed,
            prk,
            domains,
        })
    }

    /// Derive the 32-byte seed for a declared domain. An undeclared domain is a
    /// typed terminal failure — never OS entropy, wall time, or ambient RNG.
    pub fn seed(&self, domain: &str) -> Result<[u8; 32], BootstrapError> {
        let decl = self
            .domains
            .get(domain)
            .ok_or_else(|| BootstrapError::UndeclaredSeedDomain {
                domain: domain.to_string(),
            })?;
        let out = hkdf_expand(&self.prk, &decl.info(), 32);
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&out);
        Ok(seed)
    }

    /// Length-framed digest over the whole registry (root seed + every declared
    /// domain in sorted key order). Two registries with the same identity share
    /// this digest regardless of declaration order.
    #[must_use]
    pub fn registry_digest(&self) -> [u8; 32] {
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.root_seed);
        payload.extend_from_slice(&(self.domains.len() as u64).to_le_bytes());
        for d in self.domains.values() {
            payload.extend_from_slice(&d.info());
        }
        crate::domain_hash("bastion/r0d/seed-registry", 1, 0, &payload)
    }

    #[must_use]
    pub fn declared_domains(&self) -> Vec<&str> {
        self.domains.keys().map(String::as_str).collect()
    }
}

/// A plugin/content identity contribution (§6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginIdentityV1 {
    pub plugin_machine_id: String,
    pub plugin_schema_version: u32,
    pub plugin_package_sha256: [u8; 32],
    pub plugin_content_root_sha256: [u8; 32],
    pub provided_type_registry_digest: [u8; 32],
}

/// Canonicalize a plugin set (§6.4): sort by `(machine_id, package_sha256)`;
/// duplicate ids with differing digests are a typed terminal failure. Discovery
/// order is diagnostic only.
pub fn canonicalize_plugins(
    mut providers: Vec<PluginIdentityV1>,
) -> Result<Vec<PluginIdentityV1>, BootstrapError> {
    providers.sort_by(|a, b| {
        a.plugin_machine_id
            .cmp(&b.plugin_machine_id)
            .then(a.plugin_package_sha256.cmp(&b.plugin_package_sha256))
    });
    for w in providers.windows(2) {
        if w[0].plugin_machine_id == w[1].plugin_machine_id
            && w[0].plugin_package_sha256 != w[1].plugin_package_sha256
        {
            return Err(BootstrapError::PluginIdentityConflict {
                plugin_machine_id: w[0].plugin_machine_id.clone(),
            });
        }
    }
    Ok(providers)
}

/// V1 hard scenario cap (§7.1): a canonical R0D run may not exceed this many
/// authoritative ticks.
pub const V1_TICK_CAP: u64 = 1_048_576;
/// Valid tick range upper bound (§7.1): `SimulationTickV1` occupies `0 ..= 2^63-1`.
pub const TICK_MAX_EXCLUSIVE: u64 = 1u64 << 63;

/// The fixed-step tick contract (§7.1). Validating a manifest's tick policy
/// before boot makes overflow impossible within a valid run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickContractV1 {
    pub simulation_tps: u16,
    pub terminal_tick_exclusive: u64,
}

impl TickContractV1 {
    /// Validate the tick policy (§7.1): non-zero TPS, within the signed-64
    /// range, and within the V1 scenario cap. Any breach is a typed terminal
    /// failure caught before world construction.
    pub fn validate(&self) -> Result<(), BootstrapError> {
        if self.simulation_tps == 0 {
            return Err(BootstrapError::ZeroSimulationTps);
        }
        if self.terminal_tick_exclusive > TICK_MAX_EXCLUSIVE {
            return Err(BootstrapError::TickRangeOverflow {
                terminal_tick_exclusive: self.terminal_tick_exclusive,
            });
        }
        if self.terminal_tick_exclusive > V1_TICK_CAP {
            return Err(BootstrapError::TickCapExceeded {
                terminal_tick_exclusive: self.terminal_tick_exclusive,
                cap: V1_TICK_CAP,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex_bytes;

    // RFC 5869 Appendix A Test Case 1 (SHA-256): proves HMAC-SHA256 + HKDF are
    // correct by construction against the standard's frozen vectors.
    #[test]
    fn rfc5869_test_case_1() {
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0u8..=12).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let prk = hkdf_extract(&salt, &ikm);
        assert_eq!(
            hex_bytes(&prk),
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
        );
        let okm = hkdf_expand(&prk, &info, 42);
        assert_eq!(
            hex_bytes(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf\
             34007208d5b887185865"
        );
    }

    // RFC 5869 Appendix A Test Case 3 (SHA-256, zero-length salt and info).
    #[test]
    fn rfc5869_test_case_3() {
        let ikm = [0x0bu8; 22];
        let prk = hkdf_extract(&[], &ikm);
        assert_eq!(
            hex_bytes(&prk),
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
        );
        let okm = hkdf_expand(&prk, &[], 42);
        assert_eq!(
            hex_bytes(&okm),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d\
             9d201395faa4b61a96c8"
        );
    }

    fn decl(domain: &str, owner: u8) -> SeedDomainDeclarationV1 {
        SeedDomainDeclarationV1 {
            domain: domain.to_string(),
            schema_major: 1,
            schema_minor: 0,
            owner_digest: [owner; 32],
        }
    }

    #[test]
    fn frozen_seed_derivation_vector() {
        // Fixed root seed + one declared domain => frozen 32-byte seed. A second
        // independent implementation of §6.3 must reproduce this exact value.
        let reg = SeedRegistryV1::new([0x11; 32], vec![decl("bastion/r0d/worldgen", 0x22)]).unwrap();
        let s = reg.seed("bastion/r0d/worldgen").unwrap();
        assert_eq!(
            hex_bytes(&s),
            "c789637774c855259d155ca4e7a1673c011fa8cae608f22d97185f2cd01c7cc8",
            "frozen seed vector drift"
        );
    }

    #[test]
    fn distinct_domains_yield_distinct_seeds() {
        let reg = SeedRegistryV1::new(
            [0x11; 32],
            vec![decl("bastion/r0d/a", 1), decl("bastion/r0d/b", 1)],
        )
        .unwrap();
        assert_ne!(
            reg.seed("bastion/r0d/a").unwrap(),
            reg.seed("bastion/r0d/b").unwrap()
        );
    }

    #[test]
    fn owner_binding_changes_seed() {
        let a = SeedRegistryV1::new([0x11; 32], vec![decl("bastion/r0d/x", 1)]).unwrap();
        let b = SeedRegistryV1::new([0x11; 32], vec![decl("bastion/r0d/x", 2)]).unwrap();
        assert_ne!(a.seed("bastion/r0d/x").unwrap(), b.seed("bastion/r0d/x").unwrap());
    }

    #[test]
    fn declaration_order_does_not_change_identity() {
        let a = SeedRegistryV1::new(
            [0x11; 32],
            vec![decl("bastion/r0d/a", 1), decl("bastion/r0d/b", 2)],
        )
        .unwrap();
        let b = SeedRegistryV1::new(
            [0x11; 32],
            vec![decl("bastion/r0d/b", 2), decl("bastion/r0d/a", 1)],
        )
        .unwrap();
        assert_eq!(a.registry_digest(), b.registry_digest());
    }

    #[test]
    fn undeclared_domain_is_typed_failure() {
        let reg = SeedRegistryV1::new([0x11; 32], vec![decl("bastion/r0d/a", 1)]).unwrap();
        assert_eq!(
            reg.seed("bastion/r0d/missing"),
            Err(BootstrapError::UndeclaredSeedDomain {
                domain: "bastion/r0d/missing".to_string()
            })
        );
    }

    #[test]
    fn duplicate_domain_is_typed_failure() {
        let e = SeedRegistryV1::new([0x11; 32], vec![decl("bastion/r0d/a", 1), decl("bastion/r0d/a", 2)])
            .unwrap_err();
        assert_eq!(
            e,
            BootstrapError::DuplicateSeedDomain {
                domain: "bastion/r0d/a".to_string()
            }
        );
    }

    fn plugin(id: &str, pkg: u8) -> PluginIdentityV1 {
        PluginIdentityV1 {
            plugin_machine_id: id.to_string(),
            plugin_schema_version: 1,
            plugin_package_sha256: [pkg; 32],
            plugin_content_root_sha256: [0; 32],
            provided_type_registry_digest: [0; 32],
        }
    }

    #[test]
    fn plugins_canonicalize_by_id_then_package() {
        // Distinct ids: sort is by machine id (package digest is the tiebreak,
        // but two entries can only share an id if their digest also matches).
        let out = canonicalize_plugins(vec![plugin("b", 1), plugin("a", 9), plugin("c", 5)]).unwrap();
        let ids: Vec<_> = out.iter().map(|p| (p.plugin_machine_id.as_str(), p.plugin_package_sha256[0])).collect();
        assert_eq!(ids, vec![("a", 9), ("b", 1), ("c", 5)]);
    }

    #[test]
    fn plugin_id_conflict_is_typed_failure() {
        // Same machine id, DIFFERENT package digest => conflict.
        let e = canonicalize_plugins(vec![plugin("a", 1), plugin("a", 2)]).unwrap_err();
        assert_eq!(
            e,
            BootstrapError::PluginIdentityConflict {
                plugin_machine_id: "a".to_string()
            }
        );
    }

    #[test]
    fn identical_plugin_duplicate_is_not_a_conflict() {
        // Same id AND same digest is a benign duplicate, not a conflict.
        assert!(canonicalize_plugins(vec![plugin("a", 1), plugin("a", 1)]).is_ok());
    }

    #[test]
    fn tick_contract_accepts_valid_policy() {
        assert!(TickContractV1 { simulation_tps: 30, terminal_tick_exclusive: 1000 }.validate().is_ok());
        assert!(TickContractV1 { simulation_tps: 30, terminal_tick_exclusive: V1_TICK_CAP }.validate().is_ok());
    }

    #[test]
    fn tick_contract_rejects_zero_tps() {
        assert_eq!(
            TickContractV1 { simulation_tps: 0, terminal_tick_exclusive: 10 }.validate(),
            Err(BootstrapError::ZeroSimulationTps)
        );
    }

    #[test]
    fn tick_contract_rejects_over_cap() {
        assert_eq!(
            TickContractV1 { simulation_tps: 30, terminal_tick_exclusive: V1_TICK_CAP + 1 }.validate(),
            Err(BootstrapError::TickCapExceeded {
                terminal_tick_exclusive: V1_TICK_CAP + 1,
                cap: V1_TICK_CAP
            })
        );
    }

    #[test]
    fn tick_contract_rejects_range_overflow() {
        // Above the signed-64 boundary trips the range check before the cap check.
        assert_eq!(
            TickContractV1 { simulation_tps: 30, terminal_tick_exclusive: TICK_MAX_EXCLUSIVE + 1 }.validate(),
            Err(BootstrapError::TickRangeOverflow {
                terminal_tick_exclusive: TICK_MAX_EXCLUSIVE + 1
            })
        );
    }
}
