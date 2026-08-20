# Item 29 (Trade with vanilla world) — PRE-REGISTRATION

**Substrate, read not assumed (prior-art-first):**
- Vanilla PLAYER↔NPC trade is SESSION-based (`server/src/events/trade.rs`,
  invite flow, `PendingTrade`) — an interactive shape a colony job cannot
  ride; it is prior art for the EXCHANGE bookkeeping only, not the driver.
- Vanilla site economies are REAL: `world/src/site/economy` runs stocks +
  prices per site, `Economy::get_site_prices()` exposes `SitePrices`
  (values per `Good`), and `Merchant` professions exist in rtsim.
- The colony side already has: surplus visibility (`colony_food_stock`, the
  stockpile census), hauling machinery (fetch jobs, `required_item`), and
  the adopt-a-town path proving colonists can operate AT a vanilla site.

## Build shape (v1 = one mission, one exchange, priced by the site)

1. **A `TradeMission` job** (JobKind tail-append, wire rule): colonist
   carries N surplus units (v1: the colony's over-par food or logs) to the
   nearest vanilla site with an economy, exchanges at `SitePrices`-derived
   ratio for the colony's scarcest class (par-stock pull: the renewable
   demand signal charter names the selector), and hauls the proceeds home.
2. **Conservation is one-sided in v1**: the vanilla site's abstract stocks
   absorb/emit (its economy already models this); colony-side items are
   REAL and conserved (T1.13's conservation instrument applies to our half).
3. **Witnesses**: mission start (what, how much, where, at what ratio),
   exchange (delivered vs received, BOTH counted), mission end. Treatment
   beside outcome per mission.
4. **Deterministic by construction**: site selection by stable order (id),
   ratio read from the deterministic economy state, mission cadence on tick.

## BARS (long-horizon doc, made concrete)

1. One full mission completes live: N surplus out, M received in, both
   witnessed, colony stock reflects the delta (conservation on our side).
2. The ratio is the SITE's, not a constant: two sites with different
   `SitePrices` yield different M for the same N (A/B or same-leg pair).
3. Par-stock pull: the mission buys the SCARCEST class, and a planted
   glut of that class redirects the next mission's purchase.
4. Twin-run determinism (same seed ⇒ same missions, same ratios).

VOID branches: no site within mission radius (fixture — adopt-mode map);
economy returns degenerate prices (report values, do not normalise);
mission job never claimed (precondition: colonist availability witness).
