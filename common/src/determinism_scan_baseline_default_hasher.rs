[
    ("bastion-server/src/bastion_jobs.rs", "// was DefaultHasher (SipHash, version-unstable); the terrain-revision id", 0),
    ("common/src/comp/inventory/item/mod.rs", "// upgrades — was std::hash::DefaultHasher (SipHash), which is NOT a", 0),
    ("common/src/state_hash.rs", "/// `std::hash::DefaultHasher` (SipHash) is explicitly NOT stable across Rust", 0),
    // TIME-COMPRESSION fingerprint (6eb221148e, 2026-08-11): same class as the
    // three above -- a COMMENT stating the code does NOT use DefaultHasher, at
    // the digest-backed per-tick ECS state hook. Zero live usage.
    //
    // PRE-EXISTING STALENESS, found 2026-08-16 by running the FULL suite rather
    // than a filtered subset: the comment landed five days before this pin and
    // the baseline was never bumped, so this guard had been RED that whole time
    // while every targeted `cargo test <name>` run stayed green.
    ("bastion-server/src/bastion_jobs.rs", "// Domain-separated digest, not a generic Hasher: DefaultHasher", 0),
]
