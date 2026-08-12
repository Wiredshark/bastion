#!/usr/bin/env bash
# n8-run-leg.sh <mode:capped|uncapped> <leg-number>
# One leg of the N=8 promotion-tick distribution test. Boots server-cli
# live (no BASTION_DETERMINISTIC/BASTION_FLAT_ARENA/BASTION_AUTOFOUND_
# COLONY -- this deliberately keeps the real chunk-gen wall-coupling the
# test measures), founds 8 colonists via the driver, disconnects, lets
# the server run 40 REAL wall-clock seconds unattended (long enough for
# promotion under either pacing mode, since chunk-gen time is real CPU
# work independent of tick pacing), then tears down and prints the last
# "colonist promoted to loaded" tick number found in the log.
set -u
mode="${1:?usage: n8-run-leg.sh <capped|uncapped> <leg-number>}"
leg="${2:?usage: n8-run-leg.sh <capped|uncapped> <leg-number>}"

# PORT SAFETY (v5 is flying on the default 14004/14005/14006 for the
# whole duration of this fill work -- "zero contact with the flying run"):
# every leg pre-seeds settings.ron on an alternate port block (24xxx)
# BEFORE first boot, so server-cli never even tries the default port.
# Confirmed necessary the hard way: leg 1's first attempt hit AddrInUse
# against v5's own listener before this was added; v5 held the port and
# was unaffected (verified: same PID, unbroken log), but this closes the
# hole rather than relying on v5 always winning the race.
dir="bastion-test-evidence/live-playthrough/n8-${mode}-${leg}"
rm -rf "$dir"
mkdir -p "$dir/userdata/server/server_config"

cat > "$dir/userdata/server/server_config/settings.ron" << 'RONEOF'
(
    gameserver_protocols: [
        Tcp(
            address: "[::]:24004",
        ),
        Tcp(
            address: "0.0.0.0:24004",
        ),
    ],
    auth_server_address: Some("https://auth.veloren.net"),
    query_address: Some("0.0.0.0:24006"),
    max_players: 100,
    world_seed: 130626853,
    server_name: "Veloren Server",
    day_length: 30.0,
    map_file: None,
    max_view_distance: Some(65),
    max_player_group_size: 6,
    client_timeout: (
        secs: 40,
        nanos: 0,
    ),
    max_player_for_kill_broadcast: None,
    calendar_mode: Auto,
    gameplay: (
        battle_mode: Global(PvP),
        explosion_burn_marks: true,
    ),
    moderation: (
        banned_words_files: [],
        automod: false,
        admins_exempt: true,
    ),
    world: (
        start_time: 32400.0,
    ),
)
RONEOF

mkdir -p "$dir/userdata/server-cli"
cat > "$dir/userdata/server-cli/settings.ron" << 'RONEOF'
(
    update_shutdown_grace_period_secs: 120,
    update_shutdown_message: "The server is restarting for an update",
    web_address: "127.0.0.1:24005",
    web_chat_secret: None,
    ui_api_secret: None,
    shutdown_signals: [],
)
RONEOF

env_extra=(BASTION_FINGERPRINT=1)
if [[ "$mode" == "uncapped" ]]; then
    env_extra+=(BASTION_UNCAPPED_TPS=1)
fi

env VELOREN_USERDATA="$dir/userdata" \
    VELOREN_ASSETS=/e/veloren-master/.engine-integration-wt/assets \
    "${env_extra[@]}" \
    ./target/no_overflow/veloren-server-cli.exe --no-auth \
    > "$dir/server-stdout.log" 2> "$dir/server-stderr.log" &
pid=$!
echo "$pid" > "$dir/server.pid"

for _ in $(seq 1 30); do
    if grep -q "Server version" "$dir/server-stdout.log" 2>/dev/null; then
        break
    fi
    sleep 2
done

./target/no_overflow/bastion_playtest.exe localhost:24004 bastion_llm_player \
    bastion-test-evidence/live-playthrough/script-n8-promotion.txt \
    "$dir/driver.log" > "$dir/driver-stdout.log" 2>&1

sleep 40

bastion-test-evidence/live-playthrough/reap-server.sh "$dir/server.pid"

promo_tick=$(python3 -c "
import re
ansi = re.compile(r'\x1b\[[0-9;]*m')
ts_tick = []
promo_ts = []
with open('$dir/server-stdout.log', encoding='utf-8', errors='replace') as f:
    for line in f:
        line = ansi.sub('', line)
        m = re.match(r'(\S+Z)\s+INFO.*TIME-COMPRESSION fingerprint.*tick=(\d+)', line)
        if m:
            ts_tick.append((m.group(1), int(m.group(2))))
        if 'promoted to loaded' in line:
            m2 = re.match(r'(\S+Z)', line)
            if m2:
                promo_ts.append(m2.group(1))
if not promo_ts:
    print('NO_PROMOTION')
else:
    last = promo_ts[-1]
    nearest = None
    for ts, tick in ts_tick:
        if ts >= last:
            nearest = tick
            break
    if nearest is None and ts_tick:
        nearest = ts_tick[-1][1]
    print(nearest if nearest is not None else 'UNKNOWN')
")

v5_boot_epoch=$(date -d "2026-08-11T23:55:10.897657Z" +%s 2>/dev/null || echo 0)
now_epoch=$(date -u +%s)
v5_offset_secs=$(( now_epoch - v5_boot_epoch ))

echo "LEG_RESULT mode=$mode leg=$leg promotion_tick=$promo_tick v5_concurrent=true v5_offset_secs=$v5_offset_secs"
