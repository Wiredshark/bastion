//! BUILD-007A10.20 (pure core) — reproducible atlas layout (design §29,
//! DC-083/084, BTL-343/344; RES-055 RectangleBinPack as prior art).
//!
//! Versioned MaxRects **Best Short Side Fit**, rotation DISABLED, with
//! canonical input order, canonical free-rectangle order, canonical candidate
//! tie order, and deterministic paging. Input encounter order can never change
//! UV placement (BTL-343): the packer sorts inputs by the frozen canonical key
//! before placement. Page limits come from the admitted capability profile —
//! a PARAMETER here — never a live adapter query (DC-084/BTL-344).
//!
//! The KTX2 texture-payload container and the mip policy (DC-085..087) land
//! with the payload packet; this module is the placement authority.

/// A rectangle to pack: identity digest + dimensions (texels, gutter included
/// by the caller per the retained `dim + 1` border semantics).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasInputV1 {
    pub content_digest: [u8; 32],
    pub width: u32,
    pub height: u32,
}

/// A placement: page ordinal + top-left position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasPlacementV1 {
    pub content_digest: [u8; 32],
    pub page: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Typed packing failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtlasError {
    /// A rectangle exceeds the page size — can never fit on any page.
    OversizedRect { width: u32, height: u32, page: u32 },
    /// The page budget from the capability profile is exhausted.
    PageBudgetExhausted { pages: u32 },
    /// Zero-area rectangle.
    EmptyRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FreeRect {
    // Canonical order: (y, x, w, h) — position-first so candidate ties resolve
    // top-left-most deterministically.
    y: u32,
    x: u32,
    w: u32,
    h: u32,
}

/// One page's MaxRects state with canonical free-list order.
struct Page {
    free: Vec<FreeRect>, // kept sorted (BTreeSet-like via sort after mutation)
}

impl Page {
    fn new(size: u32) -> Self {
        Self {
            free: vec![FreeRect {
                y: 0,
                x: 0,
                w: size,
                h: size,
            }],
        }
    }

    /// Find the Best-Short-Side-Fit candidate: minimize leftover short side,
    /// tie on leftover long side, final tie on canonical free-rect order
    /// (top-left-most). Returns the free-list index.
    fn find_bssf(&self, w: u32, h: u32) -> Option<usize> {
        let mut best: Option<(u32, u32, usize)> = None;
        for (i, f) in self.free.iter().enumerate() {
            if f.w < w || f.h < h {
                continue;
            }
            let leftover_w = f.w - w;
            let leftover_h = f.h - h;
            let short = leftover_w.min(leftover_h);
            let long = leftover_w.max(leftover_h);
            let cand = (short, long, i);
            // Strictly-less comparison on (short, long, index): the free list
            // is canonically sorted, so index order IS the canonical tie order.
            if best.is_none_or(|b| cand < b) {
                best = Some(cand);
            }
        }
        best.map(|(_, _, i)| i)
    }

    /// Place at the chosen free rect's top-left, then perform the MaxRects
    /// split: every free rect intersecting the placed area is split into up to
    /// four maximal remainders; contained free rects are pruned; the free list
    /// is re-sorted canonically so iteration order stays deterministic.
    fn place(&mut self, idx: usize, w: u32, h: u32) -> (u32, u32) {
        let target = self.free[idx];
        let (px, py) = (target.x, target.y);
        let placed = FreeRect { x: px, y: py, w, h };
        let mut next: Vec<FreeRect> = Vec::with_capacity(self.free.len() + 4);
        for f in &self.free {
            if f.x >= placed.x + placed.w
                || f.x + f.w <= placed.x
                || f.y >= placed.y + placed.h
                || f.y + f.h <= placed.y
            {
                next.push(*f); // disjoint
                continue;
            }
            // Up to four maximal remainders.
            if placed.x > f.x {
                next.push(FreeRect {
                    x: f.x,
                    y: f.y,
                    w: placed.x - f.x,
                    h: f.h,
                });
            }
            if placed.x + placed.w < f.x + f.w {
                next.push(FreeRect {
                    x: placed.x + placed.w,
                    y: f.y,
                    w: f.x + f.w - (placed.x + placed.w),
                    h: f.h,
                });
            }
            if placed.y > f.y {
                next.push(FreeRect {
                    x: f.x,
                    y: f.y,
                    w: f.w,
                    h: placed.y - f.y,
                });
            }
            if placed.y + placed.h < f.y + f.h {
                next.push(FreeRect {
                    x: f.x,
                    y: placed.y + placed.h,
                    w: f.w,
                    h: f.y + f.h - (placed.y + placed.h),
                });
            }
        }
        // Prune contained rects (canonical pairwise sweep after sort).
        next.sort();
        next.dedup();
        let mut pruned: Vec<FreeRect> = Vec::with_capacity(next.len());
        'outer: for (i, a) in next.iter().enumerate() {
            for (j, b) in next.iter().enumerate() {
                if i != j
                    && a.x >= b.x
                    && a.y >= b.y
                    && a.x + a.w <= b.x + b.w
                    && a.y + a.h <= b.y + b.h
                    && (a.w, a.h) != (b.w, b.h)
                {
                    continue 'outer; // a strictly contained in b
                }
                if i > j && a == b {
                    continue 'outer;
                }
            }
            pruned.push(*a);
        }
        self.free = pruned;
        (px, py)
    }
}

/// Pack rectangles deterministically (DC-083). Inputs are sorted by the frozen
/// canonical key — (height desc, width desc, content_digest) — so encounter
/// order can never leak. `page_size`/`max_pages` come from the ADMITTED
/// capability profile, never a live adapter.
pub fn pack_atlas(
    inputs: &[AtlasInputV1],
    page_size: u32,
    max_pages: u32,
) -> Result<Vec<AtlasPlacementV1>, AtlasError> {
    for r in inputs {
        if r.width == 0 || r.height == 0 {
            return Err(AtlasError::EmptyRect);
        }
        if r.width > page_size || r.height > page_size {
            return Err(AtlasError::OversizedRect {
                width: r.width,
                height: r.height,
                page: page_size,
            });
        }
    }
    let mut sorted: Vec<AtlasInputV1> = inputs.to_vec();
    sorted.sort_by(|a, b| {
        b.height
            .cmp(&a.height)
            .then(b.width.cmp(&a.width))
            .then(a.content_digest.cmp(&b.content_digest))
    });
    let mut pages: Vec<Page> = vec![Page::new(page_size)];
    let mut out = Vec::with_capacity(sorted.len());
    for r in &sorted {
        // Deterministic paging: first page (in ordinal order) with a fit.
        let mut placed = None;
        for (pi, page) in pages.iter_mut().enumerate() {
            if let Some(idx) = page.find_bssf(r.width, r.height) {
                let (x, y) = page.place(idx, r.width, r.height);
                placed = Some((pi as u32, x, y));
                break;
            }
        }
        let (page, x, y) = match placed {
            Some(p) => p,
            None => {
                if pages.len() as u32 >= max_pages {
                    return Err(AtlasError::PageBudgetExhausted { pages: max_pages });
                }
                let mut page = Page::new(page_size);
                let idx = page
                    .find_bssf(r.width, r.height)
                    .expect("fits empty page (validated)");
                let (x, y) = page.place(idx, r.width, r.height);
                pages.push(page);
                (pages.len() as u32 - 1, x, y)
            },
        };
        out.push(AtlasPlacementV1 {
            content_digest: r.content_digest,
            page,
            x,
            y,
            width: r.width,
            height: r.height,
        });
    }
    Ok(out)
}

/// Domain-separated digest over the placement set (already in canonical order
/// because `pack_atlas` outputs in canonical input order).
#[must_use]
pub fn atlas_digest(placements: &[AtlasPlacementV1]) -> [u8; 32] {
    let mut p = Vec::with_capacity(8 + placements.len() * 52);
    p.extend_from_slice(&(placements.len() as u64).to_le_bytes());
    for pl in placements {
        p.extend_from_slice(&pl.content_digest);
        for v in [pl.page, pl.x, pl.y, pl.width, pl.height] {
            p.extend_from_slice(&v.to_le_bytes());
        }
    }
    crate::domain_hash("bastion/r0d/atlas-layout", 1, 0, &p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(d: u8, w: u32, h: u32) -> AtlasInputV1 {
        AtlasInputV1 {
            content_digest: [d; 32],
            width: w,
            height: h,
        }
    }

    fn overlaps(a: &AtlasPlacementV1, b: &AtlasPlacementV1) -> bool {
        a.page == b.page
            && a.x < b.x + b.width
            && b.x < a.x + a.width
            && a.y < b.y + b.height
            && b.y < a.y + a.height
    }

    #[test]
    fn encounter_order_cannot_change_placement() {
        let rects = vec![
            rect(1, 30, 40),
            rect(2, 50, 20),
            rect(3, 10, 10),
            rect(4, 25, 25),
        ];
        let a = pack_atlas(&rects, 128, 4).unwrap();
        let mut rev = rects.clone();
        rev.reverse();
        let b = pack_atlas(&rev, 128, 4).unwrap();
        assert_eq!(
            atlas_digest(&a),
            atlas_digest(&b),
            "BTL-343: encounter order leaked"
        );
    }

    #[test]
    fn placements_never_overlap_and_stay_in_bounds() {
        let rects: Vec<AtlasInputV1> = (0..40u8)
            .map(|i| rect(i, 8 + u32::from(i % 13) * 3, 8 + u32::from(i % 7) * 5))
            .collect();
        let placed = pack_atlas(&rects, 128, 8).unwrap();
        assert_eq!(placed.len(), 40);
        for p in &placed {
            assert!(p.x + p.width <= 128 && p.y + p.height <= 128, "in bounds");
        }
        for (i, a) in placed.iter().enumerate() {
            for b in placed.iter().skip(i + 1) {
                assert!(!overlaps(a, b), "overlap: {a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn equal_dims_tie_resolves_by_content_digest() {
        // Two identical-size rects: canonical key falls through to digest.
        let a = pack_atlas(&[rect(9, 16, 16), rect(1, 16, 16)], 64, 1).unwrap();
        let b = pack_atlas(&[rect(1, 16, 16), rect(9, 16, 16)], 64, 1).unwrap();
        assert_eq!(atlas_digest(&a), atlas_digest(&b));
        // Digest 1 sorts first => canonical position.
        assert_eq!(a[0].content_digest, [1; 32]);
    }

    #[test]
    fn paging_is_deterministic_and_budgeted() {
        // Four 64x64 rects on 64-texel pages: one per page.
        let rects: Vec<AtlasInputV1> = (0..4u8).map(|i| rect(i, 64, 64)).collect();
        let placed = pack_atlas(&rects, 64, 4).unwrap();
        let pages: Vec<u32> = placed.iter().map(|p| p.page).collect();
        assert_eq!(pages, vec![0, 1, 2, 3]);
        // Budget of 3 pages: typed exhaustion.
        assert_eq!(
            pack_atlas(&rects, 64, 3).unwrap_err(),
            AtlasError::PageBudgetExhausted { pages: 3 }
        );
    }

    #[test]
    fn oversized_and_empty_are_typed() {
        assert_eq!(
            pack_atlas(&[rect(1, 65, 10)], 64, 1).unwrap_err(),
            AtlasError::OversizedRect {
                width: 65,
                height: 10,
                page: 64
            }
        );
        assert_eq!(
            pack_atlas(&[rect(1, 0, 10)], 64, 1).unwrap_err(),
            AtlasError::EmptyRect
        );
    }

    #[test]
    fn frozen_layout_vector() {
        let placed =
            pack_atlas(&[rect(1, 30, 40), rect(2, 50, 20), rect(3, 10, 10)], 128, 2).unwrap();
        assert_eq!(
            crate::hex32(&atlas_digest(&placed)),
            "f68649e0b07b65c6c1ed3413e7f85d5344188a0fd351cbc9719f57229f218181",
            "frozen atlas layout drift"
        );
    }
}
