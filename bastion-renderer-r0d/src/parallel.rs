//! BUILD-007A10.12 — deterministic parallel partition, scan, and reproducible
//! reduction primitives (design §8.9 / packet A10.12).
//!
//! The controlling mitigation for parallel-execution risk. Follows the oneTBB
//! `parallel_deterministic_reduce` property — split and join structure is
//! independent of worker count, task mapping, and work stealing — implemented
//! and golden-tested here in Rust rather than imported:
//!
//! - `DeterministicParallelPlanV1`: the versioned plan (algorithm, grain,
//!   split/merge rules, overflow/numeric policy). Changing ANY field changes
//!   the plan digest and therefore the consumer schema version.
//! - Fixed recursive split tree with STATIC leaf ordinals and a fixed
//!   left-then-right merge tree. Leaves may be computed in any order into
//!   owner-local slots; the merge order is a pure function of the plan.
//! - Deterministic exclusive prefix scan + stable scatter (the only admissible
//!   compaction: §8.9 class 4 — atomics may not allocate output order).
//! - Checked exact integer reduction (§8.9 class 1) with typed overflow.
//! - Diagnostic-only fixed-tree floating accumulation (§8.9 class 3): bit-
//!   reproducible across completion orders, clearly nonsemantic.
//!
//! The negative canary test proves an ordinary completion-order (dynamic)
//! float reduction actually DIVERGES under permutation while the fixed tree
//! holds — demonstrating the mitigation is load-bearing, not ceremonial.

/// The versioned parallel plan (§8.9). Every parallel algorithm carries one;
/// changing any field is a schema-version change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterministicParallelPlanV1 {
    pub algorithm_id: String,
    pub algorithm_version: u16,
    pub grain_size: u32,
    /// Frozen split rule tag (V1: midpoint recursive split).
    pub split_rule: u16,
    /// Frozen merge rule tag (V1: left-then-right tree merge).
    pub merge_rule: u16,
    /// Overflow policy tag (V1: checked / typed terminal).
    pub overflow_policy: u16,
    /// Numeric policy tag (V1: exact integer canonical, float diagnostic-only).
    pub numeric_policy: u16,
}

impl DeterministicParallelPlanV1 {
    /// Domain-separated plan digest — the identity a consumer schema binds.
    #[must_use]
    pub fn plan_digest(&self) -> [u8; 32] {
        let mut p = Vec::new();
        p.extend_from_slice(&(self.algorithm_id.len() as u64).to_le_bytes());
        p.extend_from_slice(self.algorithm_id.as_bytes());
        for v in [
            self.algorithm_version,
            self.split_rule,
            self.merge_rule,
            self.overflow_policy,
            self.numeric_policy,
        ] {
            p.extend_from_slice(&v.to_le_bytes());
        }
        p.extend_from_slice(&self.grain_size.to_le_bytes());
        crate::domain_hash("bastion/r0d/parallel-plan", 1, 0, &p)
    }
}

/// One leaf of the fixed split tree: a range and its STATIC ordinal (left-to-
/// right position, independent of any execution order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafRange {
    pub ordinal: u32,
    pub start: usize,
    pub end: usize,
}

/// Build the fixed recursive split tree (V1 split rule: midpoint until
/// `len <= grain`), returning leaves in static left-to-right ordinal order.
/// The tree shape is a pure function of `(len, grain)` — worker count and
/// scheduling cannot appear anywhere.
#[must_use]
pub fn split_tree(len: usize, grain: usize) -> Vec<LeafRange> {
    fn rec(start: usize, end: usize, grain: usize, out: &mut Vec<LeafRange>) {
        if end - start <= grain {
            out.push(LeafRange {
                ordinal: out.len() as u32,
                start,
                end,
            });
        } else {
            let mid = start + (end - start) / 2;
            rec(start, mid, grain, out);
            rec(mid, end, grain, out);
        }
    }
    let mut out = Vec::new();
    if len > 0 {
        rec(0, len, grain.max(1), &mut out);
    }
    out
}

/// Reduce leaf results by the fixed left-then-right merge tree: a balanced
/// binary tree over leaf ORDINALS whose shape is a pure function of the leaf
/// count (V1 merge rule). `results[i]` is leaf ordinal `i`'s value (computed in
/// ANY order into its owner-local slot); the merge order is static — worker
/// count, task mapping, and completion order cannot appear in it.
#[must_use]
pub fn merge_tree<T: Clone>(results: &[T], mut combine: impl FnMut(&T, &T) -> T) -> Option<T> {
    fn rec<T: Clone>(r: &[T], combine: &mut impl FnMut(&T, &T) -> T) -> T {
        match r.len() {
            1 => r[0].clone(),
            n => {
                // Mirror split_tree: left subtree gets ceil-balanced leaf count
                // by recursing the same midpoint structure over leaf COUNT.
                let k = n.div_ceil(2);
                let left = rec(&r[..k], combine);
                let right = rec(&r[k..], combine);
                combine(&left, &right)
            },
        }
    }
    if results.is_empty() {
        None
    } else {
        Some(rec(results, &mut combine))
    }
}

/// Typed overflow terminal (§8.9 class 1: checked integer arithmetic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionOverflow;

/// Exact checked integer sum over the fixed tree: leaves may complete in any
/// order; overflow is a typed terminal, never a wrap.
pub fn checked_sum_i64(items: &[i64], grain: usize) -> Result<i64, ReductionOverflow> {
    let leaves = split_tree(items.len(), grain);
    if leaves.is_empty() {
        return Ok(0);
    }
    let mut slots: Vec<Result<i64, ReductionOverflow>> = Vec::with_capacity(leaves.len());
    for leaf in &leaves {
        let mut acc = 0i64;
        for &v in &items[leaf.start..leaf.end] {
            acc = acc.checked_add(v).ok_or(ReductionOverflow)?;
        }
        slots.push(Ok(acc));
    }
    let slots: Vec<i64> = slots.into_iter().collect::<Result<_, _>>()?;
    let mut overflow = false;
    let total = merge_tree(&slots, |a, b| {
        a.checked_add(*b).unwrap_or_else(|| {
            overflow = true;
            0
        })
    })
    .expect("nonempty");
    if overflow {
        Err(ReductionOverflow)
    } else {
        Ok(total)
    }
}

/// Deterministic exclusive prefix scan over canonical indices (§8.9 class 4).
/// `flags[i]` marks whether canonical index `i` survives; the result gives each
/// surviving index its stable output position.
#[must_use]
pub fn exclusive_scan(flags: &[bool]) -> Vec<usize> {
    let mut out = Vec::with_capacity(flags.len());
    let mut acc = 0usize;
    for &f in flags {
        out.push(acc);
        acc += usize::from(f);
    }
    out
}

/// Stable compaction: visibility bit per canonical index → exclusive scan →
/// stable scatter. Output order is the canonical input order of survivors —
/// NEVER a completion/atomic-append order. Workers may compute flags in any
/// partition/order; the scatter is a pure function of the flags.
#[must_use]
pub fn stable_compact<T: Clone>(items: &[T], flags: &[bool]) -> Vec<T> {
    debug_assert_eq!(items.len(), flags.len());
    let scan = exclusive_scan(flags);
    let count = scan
        .last()
        .map_or(0, |&s| s + usize::from(*flags.last().unwrap_or(&false)));
    let mut out: Vec<Option<T>> = vec![None; count];
    for (i, &f) in flags.iter().enumerate() {
        if f {
            out[scan[i]] = Some(items[i].clone());
        }
    }
    out.into_iter()
        .map(|x| x.expect("scan slot filled"))
        .collect()
}

/// Diagnostic-only reproducible floating accumulation (§8.9 class 3): sums by
/// the fixed merge tree, so the result is bit-identical across worker counts
/// and completion orders. Clearly nonsemantic — never selects LOD, visibility,
/// identity, or pass/fail.
#[must_use]
pub fn diagnostic_fixed_tree_sum_f64(items: &[f64], grain: usize) -> f64 {
    let leaves = split_tree(items.len(), grain);
    if leaves.is_empty() {
        return 0.0;
    }
    let slots: Vec<f64> = leaves
        .iter()
        .map(|leaf| items[leaf.start..leaf.end].iter().sum::<f64>())
        .collect();
    merge_tree(&slots, |a, b| a + b).expect("nonempty")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A deterministic pseudo-permutation of leaf-completion order.
    fn permuted(n: usize, salt: u64) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..n).collect();
        // Simple LCG-driven Fisher-Yates — deterministic per salt.
        let mut s = salt.wrapping_mul(6364136223846793005).wrapping_add(1);
        for i in (1..n).rev() {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (s % (i as u64 + 1)) as usize;
            idx.swap(i, j);
        }
        idx
    }

    #[test]
    fn split_tree_is_a_pure_function_of_len_and_grain() {
        let a = split_tree(1000, 64);
        let b = split_tree(1000, 64);
        assert_eq!(a, b);
        // Static ordinals are contiguous left-to-right and cover the range.
        assert_eq!(a[0].start, 0);
        assert_eq!(a.last().unwrap().end, 1000);
        for (i, leaf) in a.iter().enumerate() {
            assert_eq!(leaf.ordinal, i as u32);
        }
        for w in a.windows(2) {
            assert_eq!(w[0].end, w[1].start, "leaves tile the range exactly");
        }
        // Grain honored.
        assert!(a.iter().all(|l| l.end - l.start <= 64));
    }

    #[test]
    fn checked_sum_matches_serial_and_detects_overflow() {
        let items: Vec<i64> = (0..10_000).collect();
        let serial: i64 = items.iter().sum();
        for grain in [1, 7, 64, 1024, 100_000] {
            assert_eq!(checked_sum_i64(&items, grain), Ok(serial), "grain {grain}");
        }
        assert_eq!(checked_sum_i64(&[i64::MAX, 1], 1), Err(ReductionOverflow));
        assert_eq!(checked_sum_i64(&[], 8), Ok(0));
    }

    #[test]
    fn leaf_completion_order_cannot_change_fixed_tree_float_sum() {
        // Compute leaf partials in many different completion orders, writing
        // each into its OWNER-LOCAL slot; the merged result must be
        // bit-identical every time.
        let items: Vec<f64> = (0..1000).map(|i| 1.0 / (f64::from(i) + 1.0)).collect();
        let leaves = split_tree(items.len(), 32);
        let reference = diagnostic_fixed_tree_sum_f64(&items, 32);
        for salt in 0..20u64 {
            let mut slots = vec![0.0f64; leaves.len()];
            for &li in &permuted(leaves.len(), salt) {
                let leaf = leaves[li];
                slots[leaf.ordinal as usize] = items[leaf.start..leaf.end].iter().sum::<f64>();
            }
            let merged = merge_tree(&slots, |a, b| a + b).unwrap();
            assert_eq!(merged.to_bits(), reference.to_bits(), "salt {salt}");
        }
    }

    #[test]
    fn negative_canary_dynamic_completion_order_reduction_diverges() {
        // THE MUTATION CANARY: an ordinary dynamic reduction (fold in
        // completion order) must actually produce different float bits under
        // permutation — proving the fixed tree is load-bearing.
        let items: Vec<f64> = (0..1000).map(|i| 1.0 / (f64::from(i) + 1.0)).collect();
        let leaves = split_tree(items.len(), 32);
        let partial = |li: usize| -> f64 {
            let leaf = leaves[li];
            items[leaf.start..leaf.end].iter().sum::<f64>()
        };
        let mut seen = std::collections::BTreeSet::new();
        for salt in 0..20u64 {
            let dynamic: f64 = permuted(leaves.len(), salt)
                .into_iter()
                .map(partial)
                .fold(0.0, |a, b| a + b);
            seen.insert(dynamic.to_bits());
        }
        assert!(
            seen.len() > 1,
            "dynamic completion-order fold unexpectedly stable — canary is vacuous"
        );
    }

    #[test]
    fn stable_compact_preserves_canonical_order() {
        let items: Vec<u32> = (0..100).collect();
        let flags: Vec<bool> = items.iter().map(|i| i % 3 == 0).collect();
        let compacted = stable_compact(&items, &flags);
        let expected: Vec<u32> = items.iter().copied().filter(|i| i % 3 == 0).collect();
        assert_eq!(compacted, expected);
        // Empty and all-false edges.
        assert!(stable_compact(&Vec::<u32>::new(), &[]).is_empty());
        assert!(stable_compact(&items, &vec![false; 100]).is_empty());
    }

    #[test]
    fn exclusive_scan_positions_are_stable() {
        let flags = [true, false, true, true, false, true];
        assert_eq!(exclusive_scan(&flags), vec![0, 1, 1, 2, 3, 3]);
    }

    #[test]
    fn plan_digest_is_field_sensitive() {
        let plan = DeterministicParallelPlanV1 {
            algorithm_id: "bastion/r0d/mesh-reduce".to_string(),
            algorithm_version: 1,
            grain_size: 64,
            split_rule: 1,
            merge_rule: 1,
            overflow_policy: 1,
            numeric_policy: 1,
        };
        let d0 = plan.plan_digest();
        let mut p2 = plan.clone();
        p2.grain_size = 128; // ANY field change changes the schema identity
        assert_ne!(d0, p2.plan_digest());
    }
}
