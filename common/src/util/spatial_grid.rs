use super::GridHasher;
use vek::*;

#[derive(Debug)]
pub struct SpatialGrid {
    // Uses two scales of grids so that we can have a hard limit on how far to search in the
    // smaller grid
    grid: hashbrown::HashMap<Vec2<i32>, Vec<specs::Entity>, GridHasher>,
    large_grid: hashbrown::HashMap<Vec2<i32>, Vec<specs::Entity>, GridHasher>,
    // Log base 2 of the cell size of the spatial grid
    lg2_cell_size: usize,
    // Log base 2 of the cell size of the large spatial grid
    lg2_large_cell_size: usize,
    // Entities with a radius over this value are store in the coarser large_grid
    // This is the amount of buffer space we need to add when finding the intersections with cells
    // in the regular grid
    radius_cutoff: u32,
    // Stores the largest radius of the entities in the large_grid
    // This is the amount of buffer space we need to add when finding the intersections with cells
    // in the larger grid
    // note: could explore some distance field type thing for querying whether there are large
    // entities nearby that necessitate expanding the cells searched for collision (and querying
    // how much it needs to be expanded)
    // TODO: log this to metrics?
    largest_large_radius: u32,
}

impl SpatialGrid {
    pub fn new(lg2_cell_size: usize, lg2_large_cell_size: usize, radius_cutoff: u32) -> Self {
        Self {
            grid: Default::default(),
            large_grid: Default::default(),
            lg2_cell_size,
            lg2_large_cell_size,
            radius_cutoff,
            largest_large_radius: radius_cutoff,
        }
    }

    /// Add an entity at the provided 2d pos into the spatial grid
    pub fn insert(&mut self, pos: Vec2<i32>, radius: u32, entity: specs::Entity) {
        if radius <= self.radius_cutoff {
            let cell = pos.map(|e| e >> self.lg2_cell_size);
            self.grid.entry(cell).or_default().push(entity);
        } else {
            let cell = pos.map(|e| e >> self.lg2_large_cell_size);
            self.large_grid.entry(cell).or_default().push(entity);
            self.largest_large_radius = self.largest_large_radius.max(radius);
        }
    }

    /// Get an iterator over the entities overlapping the provided axis aligned
    /// bounding region.
    /// NOTE: for best optimization of the iterator use
    /// `for_each` rather than a for loop.
    pub fn in_aabr<'a>(&'a self, aabr: Aabr<i32>) -> impl Iterator<Item = specs::Entity> + 'a {
        let iter = |max_entity_radius, grid: &'a hashbrown::HashMap<_, _, _>, lg2_cell_size| {
            // Add buffer for other entity radius
            let min = aabr.min - max_entity_radius as i32;
            let max = aabr.max + max_entity_radius as i32;
            // Convert to cells
            let min = min.map(|e| e >> lg2_cell_size);
            let max = max.map(|e| (e + (1 << lg2_cell_size) - 1) >> lg2_cell_size);

            (min.x..=max.x)
                .flat_map(move |x| (min.y..=max.y).map(move |y| Vec2::new(x, y)))
                .flat_map(move |cell| grid.get(&cell).into_iter().flatten())
                .copied()
        };

        iter(self.radius_cutoff, &self.grid, self.lg2_cell_size).chain(iter(
            self.largest_large_radius,
            &self.large_grid,
            self.lg2_large_cell_size,
        ))
    }

    /// Get an iterator over the entities overlapping the
    /// axis aligned bounding region that contains the provided circle
    /// NOTE: for best optimization of the iterator use `for_each` rather than a
    /// for loop
    // TODO: using the circle directly would be tighter (how efficient would it be
    // to query the cells intersecting a circle?) (note: if doing this rename
    // the function)
    pub fn in_circle_aabr(
        &self,
        center: Vec2<f32>,
        radius: f32,
    ) -> impl Iterator<Item = specs::Entity> + '_ {
        let center = center.map(|e| e as i32);
        let radius = radius.ceil() as i32;
        // From conversion of center above
        const CENTER_TRUNCATION_ERROR: i32 = 1;
        let max_dist = radius + CENTER_TRUNCATION_ERROR;

        let aabr = Aabr {
            min: center - max_dist,
            max: center + max_dist,
        };

        self.in_aabr(aabr)
    }

    /// DET-PHY-005 (v5 deep-pass, reviewer ruling (c) = the audit's own
    /// fix): canonicalize every cell's candidate list by a stable identity
    /// key after construction — insertion followed ECS join order, so
    /// collision-candidate order was entity-INDEX order (not semantic
    /// identity), a divergence amplifier if allocation ever varies.
    /// Per-cell sort keeps the cost off the global hot path.
    pub fn canonicalize_cells(&mut self, mut key: impl FnMut(specs::Entity) -> u64) {
        for cell in self.grid.values_mut() {
            cell.sort_unstable_by_key(|e| key(*e));
        }
        for cell in self.large_grid.values_mut() {
            cell.sort_unstable_by_key(|e| key(*e));
        }
    }

    pub fn clear(&mut self) {
        self.grid.clear();
        self.large_grid.clear();
        self.largest_large_radius = self.radius_cutoff;
    }
}

#[cfg(test)]
mod det_phy_005_tests {
    use super::*;
    use specs::{Builder, WorldExt};

    /// Insert the given entities (by index into `entities`) into a single cell
    /// in the specified order, canonicalize by entity id, then return the query
    /// order for that cell.
    fn query_order(insertion: &[usize], entities: &[specs::Entity]) -> Vec<u32> {
        build_and_query(insertion, entities, true)
    }

    fn build_and_query(
        insertion: &[usize],
        entities: &[specs::Entity],
        canonicalize: bool,
    ) -> Vec<u32> {
        let mut grid = SpatialGrid::new(5, 6, 8);
        // All at the same position => one regular-grid cell (radius 0 <= cutoff 8).
        for &i in insertion {
            grid.insert(Vec2::new(0, 0), 0, entities[i]);
        }
        if canonicalize {
            grid.canonicalize_cells(|e| e.id() as u64);
        }
        grid.in_aabr(Aabr {
            min: Vec2::new(-1, -1),
            max: Vec2::new(1, 1),
        })
        .map(|e| e.id())
        .collect()
    }

    /// PHY-02 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof): DET-PHY-005 —
    /// `canonicalize_cells` makes each cell's collision-candidate order a pure
    /// function of the stable identity key, independent of insertion (ECS-join)
    /// order. Without it the candidate order was entity-INDEX order, a
    /// cross-run divergence amplifier if allocation ever varied. No executable
    /// evidence existed (the file had no tests).
    #[test]
    fn spatial_grid_canonicalize_cells_is_insertion_order_independent() {
        let mut world = specs::World::new();
        let entities: Vec<specs::Entity> =
            (0..5).map(|_| world.create_entity().build()).collect();
        let mut expected: Vec<u32> = entities.iter().map(|e| e.id()).collect();
        expected.sort_unstable();

        // Two DIFFERENT insertion orders of the same entities into the cell.
        let a = query_order(&[4, 0, 2, 1, 3], &entities);
        let b = query_order(&[1, 3, 0, 4, 2], &entities);

        // Canonical: sorted by identity key (entity id).
        assert_eq!(
            a, expected,
            "cell candidate order is not canonical by identity key (DET-PHY-005): {a:?}"
        );
        // Insertion-order-independent: the two orders produce the same result.
        assert_eq!(
            a, b,
            "cell candidate order depends on insertion order — DET-PHY-005 regressed"
        );

        // APEX-T6.3: the test asserts its own precondition. Without this,
        // a `canonicalize_cells` that silently became a no-op would still
        // pass whenever the two chosen insertion orders happened to agree,
        // and the green above would be a lottery rather than evidence.
        let a_raw = build_and_query(&[4, 0, 2, 1, 3], &entities, false);
        let b_raw = build_and_query(&[1, 3, 0, 4, 2], &entities, false);
        assert_ne!(
            a_raw, b_raw,
            "the two insertion orders produce the same raw order even WITHOUT canonicalization, \
             so this fixture cannot distinguish a working canonicalize_cells from a no-op — pick \
             orders that differ"
        );
    }
}
