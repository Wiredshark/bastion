//! Deterministic renderer bootstrap seed, plugin identity, and fixed-tick
//! substrate.

use crate::{DomainHashErrorV1, domain_hash_v1};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const SHA256_BLOCK_BYTES: usize = 64;
const SHA256_OUTPUT_BYTES: usize = 32;
pub const MAX_HKDF_OUTPUT_BYTES_V1: usize = 255 * SHA256_OUTPUT_BYTES;
pub const MAX_HKDF_INFO_BYTES_V1: usize = 4_096;
pub const MAX_SEED_DOMAIN_BYTES_V1: usize = 128;
pub const MAX_SEED_DOMAINS_V1: usize = 64;
pub const MAX_PLUGIN_MACHINE_ID_BYTES_V1: usize = 128;
pub const MAX_PLUGIN_IDENTITIES_V1: usize = 256;
pub const V1_TICK_CAP: u64 = 1_048_576;
pub const TICK_MAX_EXCLUSIVE: u64 = 1_u64 << 63;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BootstrapErrorV1 {
    InvalidSeedDomain,
    TooManySeedDomains {
        actual: usize,
        maximum: usize,
    },
    UndeclaredSeedDomain {
        domain: String,
    },
    DuplicateSeedDomain {
        domain: String,
    },
    HkdfOutputTooLarge {
        requested: usize,
        maximum: usize,
    },
    HkdfInfoTooLarge {
        actual: usize,
        maximum: usize,
    },
    LengthOverflow,
    DomainHash(DomainHashErrorV1),
    InvalidPluginMachineId,
    TooManyPluginIdentities {
        actual: usize,
        maximum: usize,
    },
    DuplicatePluginIdentity {
        plugin_machine_id: String,
    },
    PluginIdentityConflict {
        plugin_machine_id: String,
    },
    ZeroSimulationTps,
    TickRangeOverflow {
        terminal_tick_exclusive: u64,
    },
    TickCapExceeded {
        terminal_tick_exclusive: u64,
        cap: u64,
    },
}

impl From<DomainHashErrorV1> for BootstrapErrorV1 {
    fn from(value: DomainHashErrorV1) -> Self { Self::DomainHash(value) }
}

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0_u8; SHA256_BLOCK_BYTES];
    if key.len() > SHA256_BLOCK_BYTES {
        key_block[..SHA256_OUTPUT_BYTES].copy_from_slice(&Sha256::digest(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0x36_u8; SHA256_BLOCK_BYTES];
    let mut outer_pad = [0x5c_u8; SHA256_BLOCK_BYTES];
    for index in 0..SHA256_BLOCK_BYTES {
        inner_pad[index] ^= key_block[index];
        outer_pad[index] ^= key_block[index];
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

pub fn hkdf_extract(salt: &[u8], input_key_material: &[u8]) -> [u8; 32] {
    hmac_sha256(salt, input_key_material)
}

pub fn hkdf_expand(
    pseudorandom_key: &[u8; 32],
    info: &[u8],
    output_length: usize,
) -> Result<Vec<u8>, BootstrapErrorV1> {
    if output_length > MAX_HKDF_OUTPUT_BYTES_V1 {
        return Err(BootstrapErrorV1::HkdfOutputTooLarge {
            requested: output_length,
            maximum: MAX_HKDF_OUTPUT_BYTES_V1,
        });
    }
    if info.len() > MAX_HKDF_INFO_BYTES_V1 {
        return Err(BootstrapErrorV1::HkdfInfoTooLarge {
            actual: info.len(),
            maximum: MAX_HKDF_INFO_BYTES_V1,
        });
    }
    let mut output = Vec::with_capacity(output_length);
    let mut previous = [0_u8; SHA256_OUTPUT_BYTES];
    let mut previous_length = 0;
    let mut counter = 1_u16;
    while output.len() < output_length {
        let mut message =
            Vec::with_capacity(previous_length + info.len() + std::mem::size_of::<u8>());
        message.extend_from_slice(&previous[..previous_length]);
        message.extend_from_slice(info);
        let counter_byte =
            u8::try_from(counter).map_err(|_| BootstrapErrorV1::HkdfOutputTooLarge {
                requested: output_length,
                maximum: MAX_HKDF_OUTPUT_BYTES_V1,
            })?;
        message.push(counter_byte);
        previous = hmac_sha256(pseudorandom_key, &message);
        previous_length = SHA256_OUTPUT_BYTES;
        let take = (output_length - output.len()).min(SHA256_OUTPUT_BYTES);
        output.extend_from_slice(&previous[..take]);
        counter += 1;
    }
    Ok(output)
}

pub fn bootstrap_salt_v1() -> [u8; 32] { Sha256::digest(b"bastion/r0d/bootstrap-seed/v1").into() }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeedDomainDeclarationV1 {
    domain: String,
    schema_major: u16,
    schema_minor: u16,
    owner_digest: [u8; 32],
}

impl SeedDomainDeclarationV1 {
    pub fn new(
        domain: &str,
        schema_major: u16,
        schema_minor: u16,
        owner_digest: [u8; 32],
    ) -> Result<Self, BootstrapErrorV1> {
        if domain.is_empty() || domain.len() > MAX_SEED_DOMAIN_BYTES_V1 || !domain.is_ascii() {
            return Err(BootstrapErrorV1::InvalidSeedDomain);
        }
        Ok(Self {
            domain: domain.to_owned(),
            schema_major,
            schema_minor,
            owner_digest,
        })
    }

    pub fn domain(&self) -> &str { &self.domain }

    fn info(&self) -> Result<Vec<u8>, BootstrapErrorV1> {
        let domain_length =
            u16::try_from(self.domain.len()).map_err(|_| BootstrapErrorV1::LengthOverflow)?;
        let mut output = Vec::with_capacity(2 + self.domain.len() + 4 + 32);
        output.extend_from_slice(&domain_length.to_le_bytes());
        output.extend_from_slice(self.domain.as_bytes());
        output.extend_from_slice(&self.schema_major.to_le_bytes());
        output.extend_from_slice(&self.schema_minor.to_le_bytes());
        output.extend_from_slice(&self.owner_digest);
        Ok(output)
    }
}

#[derive(Clone, Debug)]
pub struct SeedRegistryV1 {
    root_seed: [u8; 32],
    pseudorandom_key: [u8; 32],
    domains: BTreeMap<String, SeedDomainDeclarationV1>,
}

impl SeedRegistryV1 {
    pub fn new(
        root_seed: [u8; 32],
        declarations: Vec<SeedDomainDeclarationV1>,
    ) -> Result<Self, BootstrapErrorV1> {
        if declarations.len() > MAX_SEED_DOMAINS_V1 {
            return Err(BootstrapErrorV1::TooManySeedDomains {
                actual: declarations.len(),
                maximum: MAX_SEED_DOMAINS_V1,
            });
        }
        let mut domains = BTreeMap::new();
        for declaration in declarations {
            let domain = declaration.domain.clone();
            if domains.insert(domain.clone(), declaration).is_some() {
                return Err(BootstrapErrorV1::DuplicateSeedDomain { domain });
            }
        }
        let pseudorandom_key = hkdf_extract(&bootstrap_salt_v1(), &root_seed);
        Ok(Self {
            root_seed,
            pseudorandom_key,
            domains,
        })
    }

    pub fn seed(&self, domain: &str) -> Result<[u8; 32], BootstrapErrorV1> {
        if domain.is_empty() || domain.len() > MAX_SEED_DOMAIN_BYTES_V1 || !domain.is_ascii() {
            return Err(BootstrapErrorV1::InvalidSeedDomain);
        }
        let declaration =
            self.domains
                .get(domain)
                .ok_or_else(|| BootstrapErrorV1::UndeclaredSeedDomain {
                    domain: domain.to_owned(),
                })?;
        let info = declaration.info()?;
        let output = hkdf_expand(&self.pseudorandom_key, &info, 32)?;
        let mut seed = [0_u8; 32];
        seed.copy_from_slice(&output);
        Ok(seed)
    }

    pub fn registry_digest(&self) -> Result<[u8; 32], BootstrapErrorV1> {
        let mut payload = Vec::with_capacity(
            32 + 2 + self.domains.len() * (2 + MAX_SEED_DOMAIN_BYTES_V1 + 4 + 32),
        );
        payload.extend_from_slice(&self.root_seed);
        payload.extend_from_slice(
            &u16::try_from(self.domains.len())
                .map_err(|_| BootstrapErrorV1::LengthOverflow)?
                .to_le_bytes(),
        );
        for declaration in self.domains.values() {
            let info = declaration.info()?;
            payload.extend_from_slice(
                &u16::try_from(info.len())
                    .map_err(|_| BootstrapErrorV1::LengthOverflow)?
                    .to_le_bytes(),
            );
            payload.extend_from_slice(&info);
        }
        Ok(domain_hash_v1("bastion/r0d/seed-registry", 1, 0, &payload)?)
    }

    pub fn declared_domains(&self) -> impl ExactSizeIterator<Item = &str> {
        self.domains.keys().map(String::as_str)
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PluginIdentityV1 {
    plugin_machine_id: String,
    plugin_schema_version: u32,
    plugin_package_sha256: [u8; 32],
    plugin_content_root_sha256: [u8; 32],
    provided_type_registry_digest: [u8; 32],
}

impl PluginIdentityV1 {
    pub fn new(
        plugin_machine_id: &str,
        plugin_schema_version: u32,
        plugin_package_sha256: [u8; 32],
        plugin_content_root_sha256: [u8; 32],
        provided_type_registry_digest: [u8; 32],
    ) -> Result<Self, BootstrapErrorV1> {
        if plugin_machine_id.is_empty()
            || plugin_machine_id.len() > MAX_PLUGIN_MACHINE_ID_BYTES_V1
            || !plugin_machine_id.is_ascii()
        {
            return Err(BootstrapErrorV1::InvalidPluginMachineId);
        }
        Ok(Self {
            plugin_machine_id: plugin_machine_id.to_owned(),
            plugin_schema_version,
            plugin_package_sha256,
            plugin_content_root_sha256,
            provided_type_registry_digest,
        })
    }

    pub fn plugin_machine_id(&self) -> &str { &self.plugin_machine_id }
}

pub fn canonicalize_plugins(
    mut providers: Vec<PluginIdentityV1>,
) -> Result<Vec<PluginIdentityV1>, BootstrapErrorV1> {
    if providers.len() > MAX_PLUGIN_IDENTITIES_V1 {
        return Err(BootstrapErrorV1::TooManyPluginIdentities {
            actual: providers.len(),
            maximum: MAX_PLUGIN_IDENTITIES_V1,
        });
    }
    providers.sort();
    for pair in providers.windows(2) {
        if pair[0].plugin_machine_id == pair[1].plugin_machine_id {
            if pair[0] == pair[1] {
                return Err(BootstrapErrorV1::DuplicatePluginIdentity {
                    plugin_machine_id: pair[0].plugin_machine_id.clone(),
                });
            }
            return Err(BootstrapErrorV1::PluginIdentityConflict {
                plugin_machine_id: pair[0].plugin_machine_id.clone(),
            });
        }
    }
    Ok(providers)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickContractV1 {
    pub simulation_tps: u16,
    pub terminal_tick_exclusive: u64,
}

impl TickContractV1 {
    pub fn validate(self) -> Result<(), BootstrapErrorV1> {
        if self.simulation_tps == 0 {
            return Err(BootstrapErrorV1::ZeroSimulationTps);
        }
        if self.terminal_tick_exclusive > TICK_MAX_EXCLUSIVE {
            return Err(BootstrapErrorV1::TickRangeOverflow {
                terminal_tick_exclusive: self.terminal_tick_exclusive,
            });
        }
        if self.terminal_tick_exclusive > V1_TICK_CAP {
            return Err(BootstrapErrorV1::TickCapExceeded {
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

    fn declaration(domain: &str, owner: u8) -> SeedDomainDeclarationV1 {
        SeedDomainDeclarationV1::new(domain, 1, 0, [owner; 32]).unwrap()
    }

    fn plugin(id: &str, package: u8) -> PluginIdentityV1 {
        PluginIdentityV1::new(id, 1, [package; 32], [3; 32], [4; 32]).unwrap()
    }

    #[test]
    fn rfc5869_sha256_vectors_are_frozen() {
        let input_key_material = [0x0b_u8; 22];
        let salt: Vec<u8> = (0_u8..=12).collect();
        let info: Vec<u8> = (0xf0_u8..=0xf9).collect();
        let pseudorandom_key = hkdf_extract(&salt, &input_key_material);
        assert_eq!(
            hex_bytes(&pseudorandom_key),
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
        );
        assert_eq!(
            hex_bytes(&hkdf_expand(&pseudorandom_key, &info, 42).unwrap()),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );

        let pseudorandom_key = hkdf_extract(&[], &input_key_material);
        assert_eq!(
            hex_bytes(&pseudorandom_key),
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
        );
        assert_eq!(
            hex_bytes(&hkdf_expand(&pseudorandom_key, &[], 42).unwrap()),
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8"
        );
    }

    #[test]
    fn hkdf_illegal_expansion_sizes_are_typed() {
        let key = [0_u8; 32];
        assert_eq!(
            hkdf_expand(&key, &[], MAX_HKDF_OUTPUT_BYTES_V1)
                .unwrap()
                .len(),
            MAX_HKDF_OUTPUT_BYTES_V1
        );
        assert_eq!(
            hkdf_expand(&key, &[], MAX_HKDF_OUTPUT_BYTES_V1 + 1),
            Err(BootstrapErrorV1::HkdfOutputTooLarge {
                requested: MAX_HKDF_OUTPUT_BYTES_V1 + 1,
                maximum: MAX_HKDF_OUTPUT_BYTES_V1,
            })
        );
        let info = vec![0_u8; MAX_HKDF_INFO_BYTES_V1 + 1];
        assert_eq!(
            hkdf_expand(&key, &info, 1),
            Err(BootstrapErrorV1::HkdfInfoTooLarge {
                actual: info.len(),
                maximum: MAX_HKDF_INFO_BYTES_V1,
            })
        );
    }

    #[test]
    fn frozen_seed_vector_and_identity_order() {
        let registry =
            SeedRegistryV1::new([0x11; 32], vec![declaration("bastion/r0d/worldgen", 0x22)])
                .unwrap();
        assert_eq!(
            hex_bytes(&registry.seed("bastion/r0d/worldgen").unwrap()),
            "c789637774c855259d155ca4e7a1673c011fa8cae608f22d97185f2cd01c7cc8"
        );

        let first = SeedRegistryV1::new([0x11; 32], vec![
            declaration("bastion/r0d/a", 1),
            declaration("bastion/r0d/b", 2),
        ])
        .unwrap();
        let second = SeedRegistryV1::new([0x11; 32], vec![
            declaration("bastion/r0d/b", 2),
            declaration("bastion/r0d/a", 1),
        ])
        .unwrap();
        assert_eq!(first.declared_domains().collect::<Vec<_>>(), vec![
            "bastion/r0d/a",
            "bastion/r0d/b"
        ]);
        assert_eq!(
            first.registry_digest().unwrap(),
            second.registry_digest().unwrap()
        );
    }

    #[test]
    fn domain_and_owner_bind_distinct_seeds() {
        let domains = SeedRegistryV1::new([0x11; 32], vec![
            declaration("bastion/r0d/a", 1),
            declaration("bastion/r0d/b", 1),
        ])
        .unwrap();
        assert_ne!(
            domains.seed("bastion/r0d/a").unwrap(),
            domains.seed("bastion/r0d/b").unwrap()
        );
        let owner_a =
            SeedRegistryV1::new([0x11; 32], vec![declaration("bastion/r0d/a", 1)]).unwrap();
        let owner_b =
            SeedRegistryV1::new([0x11; 32], vec![declaration("bastion/r0d/a", 2)]).unwrap();
        assert_ne!(
            owner_a.seed("bastion/r0d/a").unwrap(),
            owner_b.seed("bastion/r0d/a").unwrap()
        );
    }

    #[test]
    fn closed_domain_registry_failures_are_typed() {
        let registry = SeedRegistryV1::new([0; 32], vec![declaration("bastion/r0d/a", 1)]).unwrap();
        assert_eq!(
            registry.seed("bastion/r0d/missing"),
            Err(BootstrapErrorV1::UndeclaredSeedDomain {
                domain: "bastion/r0d/missing".to_owned(),
            })
        );
        assert_eq!(
            SeedRegistryV1::new([0; 32], vec![
                declaration("bastion/r0d/a", 1),
                declaration("bastion/r0d/a", 2),
            ],)
            .unwrap_err(),
            BootstrapErrorV1::DuplicateSeedDomain {
                domain: "bastion/r0d/a".to_owned(),
            }
        );
        assert_eq!(
            SeedDomainDeclarationV1::new("bastion/r0d/é", 1, 0, [0; 32]),
            Err(BootstrapErrorV1::InvalidSeedDomain)
        );
        assert_eq!(
            registry.seed(&"x".repeat(MAX_SEED_DOMAIN_BYTES_V1 + 1)),
            Err(BootstrapErrorV1::InvalidSeedDomain)
        );

        let too_many = (0..=MAX_SEED_DOMAINS_V1)
            .map(|index| {
                SeedDomainDeclarationV1::new(
                    &format!("bastion/r0d/domain-{index:03}"),
                    1,
                    0,
                    [0; 32],
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            SeedRegistryV1::new([0; 32], too_many).unwrap_err(),
            BootstrapErrorV1::TooManySeedDomains {
                actual: MAX_SEED_DOMAINS_V1 + 1,
                maximum: MAX_SEED_DOMAINS_V1,
            }
        );
    }

    #[test]
    fn plugins_have_total_order_and_explicit_duplicate_policy() {
        let ordered =
            canonicalize_plugins(vec![plugin("z", 1), plugin("a", 9), plugin("m", 5)]).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(PluginIdentityV1::plugin_machine_id)
                .collect::<Vec<_>>(),
            vec!["a", "m", "z"]
        );
        assert_eq!(
            canonicalize_plugins(vec![plugin("a", 1), plugin("a", 1)]),
            Err(BootstrapErrorV1::DuplicatePluginIdentity {
                plugin_machine_id: "a".to_owned(),
            })
        );
        assert_eq!(
            canonicalize_plugins(vec![plugin("a", 1), plugin("a", 2)]),
            Err(BootstrapErrorV1::PluginIdentityConflict {
                plugin_machine_id: "a".to_owned(),
            })
        );
        assert_eq!(
            PluginIdentityV1::new("plugin/é", 1, [0; 32], [0; 32], [0; 32]),
            Err(BootstrapErrorV1::InvalidPluginMachineId)
        );
        let too_many = (0..=MAX_PLUGIN_IDENTITIES_V1)
            .map(|index| {
                PluginIdentityV1::new(&format!("plugin-{index:03}"), 1, [0; 32], [0; 32], [0; 32])
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            canonicalize_plugins(too_many),
            Err(BootstrapErrorV1::TooManyPluginIdentities {
                actual: MAX_PLUGIN_IDENTITIES_V1 + 1,
                maximum: MAX_PLUGIN_IDENTITIES_V1,
            })
        );
    }

    #[test]
    fn tick_contract_rejects_zero_range_and_cap() {
        assert_eq!(
            TickContractV1 {
                simulation_tps: 0,
                terminal_tick_exclusive: 1,
            }
            .validate(),
            Err(BootstrapErrorV1::ZeroSimulationTps)
        );
        assert_eq!(
            TickContractV1 {
                simulation_tps: 60,
                terminal_tick_exclusive: TICK_MAX_EXCLUSIVE + 1,
            }
            .validate(),
            Err(BootstrapErrorV1::TickRangeOverflow {
                terminal_tick_exclusive: TICK_MAX_EXCLUSIVE + 1,
            })
        );
        assert_eq!(
            TickContractV1 {
                simulation_tps: 60,
                terminal_tick_exclusive: V1_TICK_CAP + 1,
            }
            .validate(),
            Err(BootstrapErrorV1::TickCapExceeded {
                terminal_tick_exclusive: V1_TICK_CAP + 1,
                cap: V1_TICK_CAP,
            })
        );
        assert!(
            TickContractV1 {
                simulation_tps: 60,
                terminal_tick_exclusive: V1_TICK_CAP,
            }
            .validate()
            .is_ok()
        );
    }
}
