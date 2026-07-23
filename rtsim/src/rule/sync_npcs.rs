use crate::{
    RtState, Rule, RuleError,
    event::{EventCtx, OnDeath, OnHealthChange, OnSetup, OnTick},
};
use common::{
    grid::Grid,
    rtsim::{Actor, NpcInput},
    terrain::CoordinateConversions,
};

pub struct SyncNpcs;

impl Rule for SyncNpcs {
    fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
        rtstate.bind::<Self, OnSetup>(on_setup);
        rtstate.bind::<Self, OnDeath>(on_death);
        rtstate.bind::<Self, OnHealthChange>(on_health_change);
        rtstate.bind::<Self, OnTick>(on_tick);

        Ok(Self)
    }
}

fn on_setup(ctx: EventCtx<SyncNpcs, OnSetup>) {
    let data = &mut *ctx.state.data_mut();

    // Create NPC grid
    data.npcs.npc_grid = Grid::new(ctx.world.sim().get_size().as_(), Default::default());

    // Add NPCs to home population
    for (npc_id, npc) in data.npcs.npcs.iter() {
        if let Some(home) = npc.home.and_then(|home| data.sites.get_mut(home)) {
            home.population.insert(npc_id);
        }
    }

    // Update the list of nearest sites by size for each site
    let sites_iter = data.sites.iter().filter_map(|(site_id, site)| {
        let world_site = site.world_site.map(|ws| ctx.index.sites.get(ws))?;
        Some((site_id, site, world_site))
    });
    let nearest_by_size = sites_iter.clone()
        .map(|(site_id, site, world_site)| {
            let other_sites = sites_iter.clone()
                // Only include sites in the list if they're not the current one and they're more populus
                .filter(|(other_id, _, other_site)| *other_id != site_id && other_site.plots().len() > world_site.plots().len())
                .collect::<Vec<_>>();
            // DET-ESIM-019: project each candidate to its canonical total key
            // (distance² to the home site, plot count, stable SiteId) and sort
            // via the helper (unit-tested in det_esim_019_tests). Distance alone
            // leaves ties broken by incidental slotmap iteration order, which the
            // monotone "Stalin sort" retain below then bakes into the persisted
            // nearby_sites_by_size list; the total key makes the retained
            // candidate set order-independent.
            let keyed: Vec<(i64, usize, u64, _)> = other_sites
                .into_iter()
                .map(|(other_id, other, other_site)| {
                    let dist2 = other.wpos.as_::<i64>().distance_squared(site.wpos.as_::<i64>());
                    let plots = other_site.plots().len();
                    let ffi = slotmap::Key::data(&other_id).as_ffi();
                    (dist2, plots, ffi, (other_id, other, other_site))
                })
                .collect();
            let mut other_sites: Vec<_> = canonical_nearby_site_order(keyed)
                .into_iter()
                .map(|(_, _, _, v)| v)
                .collect();
            let mut max_size = 0;
            // Remove sites that aren't in increasing order of size (Stalin sort?!)
            other_sites.retain(|(_, _, other_site)| {
                if other_site.plots().len() > max_size {
                    max_size = other_site.plots().len();
                    true
                } else {
                    false
                }
            });
            let nearest_by_size = other_sites
                .into_iter()
                .map(|(site_id, _, _)| site_id)
                .collect::<Vec<_>>();
            (site_id, nearest_by_size)
        })
        .collect::<Vec<_>>();
    for (site_id, nearest_by_size) in nearest_by_size {
        if let Some(site) = data.sites.get_mut(site_id) {
            site.nearby_sites_by_size = nearest_by_size;
        }
    }
}

fn on_health_change(ctx: EventCtx<SyncNpcs, OnHealthChange>) {
    let data = &mut *ctx.state.data_mut();

    // As this handler does not correctly handle death, ignore events that set the
    // health fraction to 0 (dead)
    if ctx.event.new_health_fraction != 0.0
        && let Actor::Npc(npc_id) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc_id)
    {
        npc.health_fraction = ctx.event.new_health_fraction;
    }
}

fn on_death(ctx: EventCtx<SyncNpcs, OnDeath>) {
    let data = &mut *ctx.state.data_mut();

    if let Actor::Npc(npc_id) = ctx.event.actor
        && let Some(npc) = data.npcs.get_mut(npc_id)
    {
        // Mark the NPC as dead, allowing us to clear them up later
        npc.health_fraction = 0.0;
    }
}

fn on_tick(ctx: EventCtx<SyncNpcs, OnTick>) {
    let data = &mut *ctx.state.data_mut();
    for (npc_id, npc) in data.npcs.npcs.iter_mut() {
        // Update the NPC's current site, if any
        npc.current_site = ctx
            .world
            .sim()
            .get(npc.wpos.xy().as_().wpos_to_cpos())
            .and_then(|chunk| {
                chunk
                    .sites
                    .iter()
                    .find_map(|site| data.sites.world_site_map.get(site).copied())
            });

        // Share known reports with current site, if it's our home
        // TODO: Only share new reports
        if let Some(current_site) = npc.current_site
            && Some(current_site) == npc.home
            && let Some(site) = data.sites.get_mut(current_site)
        {
            // TODO: Sites should have an inbox and their own AI code
            site.known_reports.extend(npc.known_reports.iter().copied());
            // DET-ESIM-011 (v8 rtsim-economy, Critical): `site.known_reports`
            // is a HashSet, so extending the NPC's ORDERED inbox by iterating
            // it directly made inbox order — and therefore report-processing
            // order, sentiment application, and the chosen action — ride the
            // process hash seed. Collect the newly-shared reports and sort them
            // by ReportId so the inbox receives them in a canonical order.
            // (The set-extend above is order-independent; only this ordered
            // target needs canonicalising.)
            let mut new_reports = site
                .known_reports
                .iter()
                .copied()
                .filter(|report| !npc.known_reports.contains(report))
                .collect::<Vec<_>>();
            new_reports.sort_unstable();
            npc.inbox
                .extend(new_reports.into_iter().map(NpcInput::Report));
        }

        // Update the NPC's grid cell
        let chunk_pos = npc.wpos.xy().as_().wpos_to_cpos();
        if npc.chunk_pos != Some(chunk_pos) {
            if let Some(cell) = npc
                .chunk_pos
                .and_then(|chunk_pos| data.npcs.npc_grid.get_mut(chunk_pos))
                && let Some(index) = cell.npcs.iter().position(|id| *id == npc_id)
            {
                cell.npcs.swap_remove(index);
            }
            npc.chunk_pos = Some(chunk_pos);
            if let Some(cell) = data.npcs.npc_grid.get_mut(chunk_pos) {
                cell.npcs.push(npc_id);
            }
        }
    }
}

/// DET-ESIM-019: order candidate nearby sites by a canonical TOTAL key —
/// (distance² to the home site, then plot count, then stable SiteId as-ffi) — so
/// the monotone "Stalin sort" retain that bakes the result into the persisted
/// nearby_sites_by_size list is independent of slotmap iteration order (distance
/// alone leaves ties broken by that incidental order). Generic over the carried
/// value; the caller pre-projects each candidate to its (dist², plots, ffi) key.
pub fn canonical_nearby_site_order<T>(
    mut keyed: Vec<(i64, usize, u64, T)>,
) -> Vec<(i64, usize, u64, T)> {
    keyed.sort_by_key(|(dist2, plots, ffi, _)| (*dist2, *plots, *ffi));
    keyed
}

#[cfg(test)]
mod det_esim_019_tests {
    use super::*;

    /// ESIM-019 (det-fixture, SPECIFIED_NOT_EVIDENCED -> direct proof): the
    /// nearby-sites candidate order is a canonical total order (dist², plots,
    /// SiteId), so the persisted nearby_sites_by_size list is independent of the
    /// slotmap iteration order the candidates were gathered in. The inline sort
    /// had no test.
    #[test]
    fn canonical_nearby_site_order_is_slotmap_order_independent() {
        // (dist2, plots, site_ffi, tag). Same candidate set in two different
        // (slotmap-iteration) orders; ties on dist2 break by plots then ffi.
        let set_a: Vec<(i64, usize, u64, u32)> = vec![
            (100, 3, 50, 1),
            (100, 3, 20, 2), // ties (100,3) with tag 1 -> ffi 20 < 50 sorts first
            (100, 5, 10, 3), // dist 100, more plots -> after the plots=3 pair
            (40, 2, 99, 4),  // nearest -> first
        ];
        let set_b: Vec<(i64, usize, u64, u32)> = vec![
            (100, 5, 10, 3),
            (40, 2, 99, 4),
            (100, 3, 50, 1),
            (100, 3, 20, 2),
        ];
        let a: Vec<u32> = canonical_nearby_site_order(set_a)
            .iter()
            .map(|(_, _, _, t)| *t)
            .collect();
        let b: Vec<u32> = canonical_nearby_site_order(set_b)
            .iter()
            .map(|(_, _, _, t)| *t)
            .collect();
        // Canonical (dist2, plots, ffi): 4 (40,2,99), 2 (100,3,20), 1 (100,3,50), 3 (100,5,10).
        assert_eq!(
            a,
            vec![4, 2, 1, 3],
            "nearby sites not in canonical (dist2, plots, ffi) order (DET-ESIM-019)"
        );
        assert_eq!(
            a, b,
            "nearby-site order depends on slotmap iteration order — DET-ESIM-019 regressed"
        );
    }
}
