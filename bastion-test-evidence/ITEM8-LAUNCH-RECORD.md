# ITEM 8 (ENDURANCE RUN) — launch record

**Boot stamp:** binary built from commit `e6d0f6b3e8eff1a30333022e71685ee5c0eba984`
(`git describe --always --dirty` on the worktree at build time: `bastion-block-
T1CMD-677-ge6d0f6b3e8`, clean). Server's own version banner cosmetically shows
`803d38a6` (`common/build.rs` git-hash-caching quirk, already diagnosed benign
earlier this arc — mtime and `git describe` are the trustworthy signals, both
confirmed fresh/clean: `veloren-server-cli.exe` mtime 02:33, `bastion_playtest.exe`
mtime 02:28, both after every source edit this session).

**Boot config, read live (not from memory):**

    hunger_decay_per_sec=0.000889  hunger_interrupt=0.2  hunger_comfort=0.5
    rest_decay_per_sec=0.000444    rest_interrupt=0.2    rest_comfort=0.5
    recreation_decay_per_sec=n/a   recreation_interrupt=0.0  recreation_comfort=0.4

Matches the planning estimate in ITEM8-PREFLIGHT-BAR-PREREGISTRATION.md §1
exactly — cycle length stands at ≈1802 sim-sec (rest full-to-interrupt).

**Userdata:** fresh dir, `bastion-test-evidence/live-playthrough/userdata-item8-
endurance/`. `BASTION_ENTITY_EVENT_LOG=1`, `VELOREN_ASSETS` pinned explicitly to
this worktree's own `assets/` (avoids the nested-worktree asset trap).

**Server PID:** 197609 (Windows PID, `ps -W` col 4) / bash job pid 289.

**Founding + designation (`script-15-item8-endurance.txt`):** spawn position
`(15216.5, 16016.5, 419.0)` — landed on the SAME coordinates script-14/script-10
used from different fresh userdata dirs, confirming the world seed is fixed
across fresh dirs (not a new-world-per-boot risk, the trap that bit an earlier
farm-confirm attempt this arc). Stockpile/farm/bed designations registered
(`rev=3`, confirmed via `list_designations` at checkpoints 1–3). No `give_item`,
no `dropall` — founding stock (`FOUNDING_SEED_STOCK=8`) is the run's entire
food-producing capital from here on, per the scenario's own requirement.

**Releasing event:** driver script completed and disconnected cleanly at
driver-log timestamp `1786430157489`, server tick ≈3300–3600 (next sampled
tick after disconnect: `tick=3600, food_stock=0` — early, pre-first-harvest, as
expected). Driver process confirmed exited (`ps -W` shows no `bastion_playtest`
process post-disconnect). **The scored unattended window begins here.**

**Liveness baseline at launch:** server log `80645` bytes at `02:37:26` local.
Food-stock sampler firing on schedule (`tick=2700/3000/3300/3600`, 10 sim-sec /
~10 wall-sec cadence at boot — matches the always-on 300-tick period).

**Target:** N=5 cycles scored (≈150 sim-min), designed to continue to 7 if
healthy (≈210 sim-min) per Fable's ruling — score registers at 5 regardless.

No client will reconnect during the scored window. Liveness checked via
periodic reads of the server's own log only, per §5 of the pre-registration
doc (producer-alive ping = the food-stock sampler; releasing event = this
launch).
