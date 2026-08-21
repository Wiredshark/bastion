#!/bin/bash
# THE PLAY HARNESS (Ben direct, 2026-08-21: "use them to actually play the real
# game like they would play dwarf fortress ... and collect what's going wrong").
#
# The pit runner (run-pit.sh) is a BATCH instrument: one fixed script, one fresh
# world, one verdict. A player is not batch — a player looks, decides, acts,
# looks again, and changes their mind. This harness gives an agent that loop:
# ONE persistent world it can return to, and a turn command that runs whatever
# script the agent just wrote against it.
#
#   play-harness.sh boot   <slot> <arm>     start a persistent world (once)
#   play-harness.sh turn   <slot> <script>  play one turn against it
#   play-harness.sh watch  <slot> [n]       tail the SERVER's own view
#   play-harness.sh status <slot>           is this world still alive?
#   play-harness.sh stop   <slot>           end the session
#
# The world at slot N owns port 26024+N*10 and userdata-play-N; two agents on
# different slots never see each other. Userdata is NOT wiped between turns —
# that is the whole point: the colony the agent left behind is the colony it
# comes back to.
#
# ★ THE SHARED CHECKOUT IS NOT YOURS (2026-08-21, two worlds lost back to back).
# This harness plays inside a checkout that OTHER sessions build in. Cargo does
# not overwrite target/no_overflow/veloren-server-cli.exe in place — it UNLINKS
# the name and relinks it to a freshly built object. So mid-session the path a
# live world was launched from can simply stop existing. Three rebuilds landed
# during a single two-hour session; one world stopped mid-log-line with no
# panic, no shutdown message and no error, and ~31 game days went with it.
#
# Three rules follow from that, and all three are enforced below:
#   1. boot COPIES the binaries it needs to a private per-slot directory and
#      runs the copy. A concurrent rebuild cannot reach a running world.
#   2. boot never destroys a saved world it cannot replace. Every check that can
#      fail runs ABOVE the `rm -rf`, because a boot that wipes userdata and THEN
#      discovers it has no binary has spent a world to learn nothing. That is
#      exactly what the reboot attempt after the first loss did.
#   3. a turn that cannot reach the server says WHY, in English. The driver's
#      own answer is a Rust panic about ConnectionRefused, which reads like a
#      harness bug. It usually is not — it usually means the world is gone.
set -u
WT=/e/veloren-master/.item29-wt
B=$WT/target/no_overflow
EV=$WT/bastion-test-evidence
PLAY=$EV/play
mkdir -p "$PLAY"

CMD=${1:?usage: boot|turn|watch|status|stop}
SLOT=${2:?slot number}
GAME=$((26024 + SLOT * 10))
WEB=$((26025 + SLOT * 10))
METRICS=$((18026 + SLOT * 10))
UD=$WT/userdata-play-$SLOT
SRVLOG=$PLAY/server-$SLOT.log
PIDF=$PLAY/.pid-$SLOT

# The binaries this slot actually runs: private copies, never the shared tree.
#
# The NAME matters as much as the path. SHARED-CHECKOUT-PROTOCOL.md §1 records
# that `taskkill /IM veloren-server-cli.exe /F` has already destroyed two
# sessions' worlds — and /IM matches the IMAGE NAME, so a private copy still
# called veloren-server-cli.exe dies with all the rest no matter where it lives.
# The suffix takes this world out of that blast radius while still containing
# the substring "veloren-server-cli", so the protocol's own
# `tasklist | grep -c veloren-server-cli` check keeps seeing that a world is up.
BINDIR=$PLAY/bin-$SLOT
SRV=$BINDIR/veloren-server-cli-play$SLOT.exe
DRV=$BINDIR/bastion_playtest-play$SLOT.exe
SRC_SRV=$B/veloren-server-cli.exe
SRC_DRV=$B/bastion_playtest.exe
# What the shared tree looked like at boot, so a turn three hours later can say
# "a rebuild landed under you" instead of leaving the reader to guess.
BOOTMETA=$PLAY/.boot-$SLOT

# A file's IDENTITY, not merely its timestamp. Cargo relinks the stable name to
# a different object, so the inode moves even when the mtime does not — these
# are hardlinks into deps/, and re-linking an unchanged artifact keeps the old
# mtime. mtime alone would have called a swapped binary "unchanged".
fp() { stat -c '%i:%s:%Y' "$1" 2>/dev/null || echo MISSING; }
when() { date -d "@$1" '+%H:%M:%S' 2>/dev/null || echo '?'; }
meta() { sed -n "s/^$1=//p" "$BOOTMETA" 2>/dev/null | head -1; }
listening() { netstat -ano 2>/dev/null | grep LISTENING | grep -qE "[:.]$GAME"; }
alive() { [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; }

# Take the private copy. 138 MB for the server plus 98 MB for the driver is
# cheap against the cost of losing a session. A hardlink would be free and would
# survive cargo unlinking the shared name, but it would still share an inode
# with the build tree — and sharing anything with the build tree is the bug.
pin() {
  local src=$1 dst=$2
  rm -f "$dst" 2>/dev/null
  cp "$src" "$dst" 2>/dev/null || return 1
  [ "$(stat -c %s "$dst" 2>/dev/null)" = "$(stat -c %s "$src" 2>/dev/null)" ]
}

# Why can't we reach this world? Answer in the order a player cares about: is
# the process there, is the port there, did the ground move underneath us.
postmortem() {
  local pid pidstate portstate now_srv boot_srv now_drv boot_drv bt lastbyte
  echo "--- SLOT $SLOT POST-MORTEM ---"
  if [ -f "$PIDF" ]; then pid=$(cat "$PIDF"); else pid='(no pidfile)'; fi
  if alive; then pidstate=ALIVE; else pidstate=DEAD; fi
  if listening; then portstate=LISTENING; else portstate='NOT LISTENING'; fi
  echo "  server pid $pid : $pidstate"
  echo "  port $GAME : $portstate"

  if [ -f "$BOOTMETA" ]; then
    bt=$(meta booted_at)
    echo "  booted at $(when "$bt") as arm '$(meta arm)'"
    boot_srv=$(meta src_srv_fp); now_srv=$(fp "$SRC_SRV")
    if [ "$now_srv" = MISSING ]; then
      echo "  shared server binary : GONE FROM THE TREE RIGHT NOW"
      echo "                         a build is in flight in $B"
    elif [ "$now_srv" != "$boot_srv" ]; then
      echo "  shared server binary : REPLACED since boot — a rebuild landed under this session"
      echo "                         at boot: $boot_srv"
      echo "                         now    : $now_srv"
    else
      echo "  shared server binary : unchanged since boot"
    fi
    boot_drv=$(meta src_drv_fp); now_drv=$(fp "$SRC_DRV")
    if [ "$now_drv" != "$boot_drv" ]; then
      echo "  shared driver binary : REPLACED since boot"
    fi
    if [ -f "$SRV" ]; then
      echo "  this slot's private binary : present, and a rebuild cannot touch it"
    else
      echo "  this slot's private binary : MISSING ($SRV) — something deleted $BINDIR"
    fi
  else
    echo "  NO BOOT RECORD for this slot. This world was booted before binary"
    echo "  pinning existed, so it was launched straight out of the shared tree,"
    echo "  where a rebuild can reach it. Nothing here can say what that tree"
    echo "  looked like at the time."
  fi

  # A killed process stops mid-line; a clean shutdown ends its log with a
  # newline. That one byte separates "something killed my world" from "my world
  # decided to stop", and it is the byte the first loss turned on.
  if [ -f "$SRVLOG" ]; then
    echo "  server log : $(stat -c %s "$SRVLOG") bytes, last written $(when "$(stat -c %Y "$SRVLOG")")"
    lastbyte=$(tail -c 1 "$SRVLOG" 2>/dev/null | xxd -p 2>/dev/null)
    if [ -n "$lastbyte" ] && [ "$lastbyte" != "0a" ]; then
      echo "               ENDS MID-LINE — the process was killed, it did not exit"
    fi
    echo "  the server's own last words:"
    sed 's/\x1b\[[0-9;]*m//g' "$SRVLOG" | tail -5 | sed 's/^/    | /'
    # The truncated last line has no newline of its own, so without this the
    # VERDICT below would be printed onto the end of it — in precisely the case
    # the verdict matters most.
    [ "$lastbyte" = "0a" ] || echo
  else
    echo "  server log : MISSING ($SRVLOG)"
  fi

  # All four states are spelled out. An earlier draft collapsed two of them and
  # told a player with a perfectly healthy server to "wait for the world to
  # generate" — which is the same species of misdirection as the raw
  # ConnectionRefused panic this whole function exists to replace.
  echo "  VERDICT:"
  if [ "$pidstate" = ALIVE ] && [ "$portstate" = LISTENING ]; then
    echo "    THE WORLD IS FINE — the server is running and holding port $GAME."
    echo "    Whatever just failed, failed on the driver/script side. Read the"
    echo "    driver output above; your colony is not affected."
  elif [ "$pidstate" = ALIVE ]; then
    echo "    The server process is alive but is not accepting connections yet."
    echo "    It is probably still generating the world — wait, then retry."
  elif [ "$portstate" = LISTENING ]; then
    echo "    Port $GAME is held, but NOT by the pid we recorded. Something else"
    echo "    is on this slot's port — run 'stop $SLOT' before booting."
  else
    echo "    THIS WORLD IS GONE — the server process is not running."
    echo "    This is not a driver bug and not a game bug. Everything since the"
    echo "    last rtsim save (they land every 60s) went with it."
    echo "    Do NOT 'boot' this slot hoping to find the colony: boot wipes userdata."
    echo "    Copy anything you want out of $UD first — it is still on disk."
  fi
}

case "$CMD" in
boot)
  ARM=${3:-arena}
  if [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; then
    echo "slot $SLOT already booted (pid $(cat "$PIDF"), port $GAME)"; exit 0
  fi

  # ★ NOTHING DESTRUCTIVE UNTIL THE BINARIES ARE IN HAND.
  # Everything between here and the `rm -rf` can fail, and every one of those
  # failures must leave the saved world exactly where it was.
  for f in "$SRC_SRV" "$SRC_DRV"; do
    if [ ! -s "$f" ]; then
      echo "REFUSING TO BOOT: $f is missing or empty." >&2
      echo "  A build is almost certainly in flight in $WT — cargo unlinks this" >&2
      echo "  path while it relinks it. Wait for the build to finish and retry." >&2
      echo "  Your saved world at $UD has NOT been touched." >&2
      exit 3
    fi
  done
  mkdir -p "$BINDIR"
  if ! pin "$SRC_SRV" "$SRV"; then
    echo "REFUSING TO BOOT: could not copy $SRC_SRV -> $SRV" >&2
    echo "  (a rebuild may have replaced it mid-copy). $UD is untouched." >&2
    exit 3
  fi
  if ! pin "$SRC_DRV" "$DRV"; then
    echo "REFUSING TO BOOT: could not copy $SRC_DRV -> $DRV. $UD is untouched." >&2
    exit 3
  fi
  # EXECUTE the copy, do not merely stat it: a binary caught mid-relink can sit
  # there at full size and still refuse to run.
  if ! "$SRV" --help >/dev/null 2>&1; then
    echo "REFUSING TO BOOT: $SRV was copied but will not run. $UD is untouched." >&2
    exit 3
  fi
  echo "slot $SLOT: private binaries in $BINDIR — a concurrent rebuild can no longer kill this world"

  # Past this line we are allowed to destroy the old world.
  rm -rf "$UD"
  VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets \
    "$SRV" --no-auth admin add player admin >/dev/null 2>&1
  S=$UD/server/server_config/settings.ron
  # The admin-add is what CREATES these files. If it did not, every line below
  # is editing a world that does not exist — which is how a failed boot came to
  # print `sed: can't read .../settings.ron` and then launch anyway.
  if [ ! -f "$S" ] || [ ! -f "$UD/server-cli/settings.template.ron" ]; then
    echo "BOOT FAILED: $SRV ran but produced no settings under $UD" >&2
    echo "  expected: $S" >&2
    echo "  and:      $UD/server-cli/settings.template.ron" >&2
    exit 4
  fi
  sed -i "s/:14004\"/:$GAME\"/g; s/:14006\"/:$METRICS\"/g" "$S"
  sed "s/:14005\"/:$WEB\"/" "$UD/server-cli/settings.template.ron" > "$UD/server-cli/settings.ron"
  # The two worlds a player can be dropped into. The arena is legible and fast;
  # the town is the real test of "use what is already there".
  case "$ARM" in
    town) ENVV="BASTION_ADOPT_TOWN=1 BASTION_AUTOFOUND_REAL_TERRAIN=1 BASTION_COLONY_PRESENCE_VD=3 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32 BASTION_SEED_MATERIALS=64" ;;
    *)    ENVV="BASTION_FLAT_ARENA=1 BASTION_FLAT_ARENA_RESOURCED=1 BASTION_AUTOFOUND_COLONY=8 BASTION_SEED_FOOD=32" ;;
  esac
  # NOT uncapped: a player watches at human speed, and an uncapped server makes
  # every observation a blur of thousands of ticks.
  # `$!` of a SUBSHELL is the subshell, not the server — `stop` reported
  # success while the server kept listening (caught by a play agent, who had
  # to kill it by hand). `exec` makes the backgrounded process BE the server,
  # so the recorded pid is the real one.
  ( cd "$WT" && exec env VELOREN_USERDATA=$UD VELOREN_ASSETS=$WT/assets \
      BASTION_DETERMINISTIC=1 $ENVV $PLAY_EXTRA_ENV \
      "$SRV" --no-auth > "$SRVLOG" 2>&1 ) &
  echo $! > "$PIDF"
  # The boot record. Its whole job is to let a later turn say what changed
  # underneath it.
  {
    echo "booted_at=$(date +%s)"
    echo "pid=$(cat "$PIDF")"
    echo "port=$GAME"
    echo "arm=$ARM"
    echo "pinned_srv_fp=$(fp "$SRV")"
    echo "src_srv_fp=$(fp "$SRC_SRV")"
    echo "src_drv_fp=$(fp "$SRC_DRV")"
  } > "$BOOTMETA"
  echo "booting slot $SLOT ($ARM) on port $GAME, pid $(cat "$PIDF")"
  for i in $(seq 1 90); do
    grep -q "ready to accept connections" "$SRVLOG" 2>/dev/null && { echo "READY after ${i}0s"; exit 0; }
    sleep 10
  done
  echo "NOT READY after 900s — read $SRVLOG"; exit 1
  ;;
turn)
  SCRIPT=${3:?script path}
  [ -f "$SCRIPT" ] || { echo "no such script: $SCRIPT"; exit 2; }
  # Worlds booted before pinning existed still run from the shared tree.
  D=$DRV; [ -s "$D" ] || D=$SRC_DRV
  if [ ! -s "$D" ]; then
    echo "NO PLAYTEST DRIVER: $D is missing or empty." >&2
    echo "  A build is probably in flight in $WT. The world itself may be fine —" >&2
    echo "  this is the driver, not the server. Wait for the build and retry." >&2
    postmortem
    exit 5
  fi
  TURN=$(( $(ls "$PLAY"/turn-$SLOT-*.log 2>/dev/null | wc -l) + 1 ))
  OUT=$PLAY/turn-$SLOT-$TURN.log
  ( cd "$WT" && VELOREN_ASSETS=$WT/assets \
      "$D" "localhost:$GAME" player "$SCRIPT" "$OUT" \
      > "$PLAY/turnout-$SLOT-$TURN.log" 2>&1 )
  RC=$?
  echo "turn $TURN rc=$RC -> $OUT"
  # The driver's own log is the player's eyes; print it whole.
  [ -f "$OUT" ] && sed 's/\x1b\[[0-9;]*m//g' "$OUT"
  if [ $RC -ne 0 ]; then
    echo "--- DRIVER FAILED (rc=$RC), last output ---"
    tail -20 "$PLAY/turnout-$SLOT-$TURN.log"
    # The post-mortem goes LAST so the verdict is the last thing on screen.
    postmortem
  fi
  # A failing turn used to exit 0, because the final `sed` succeeded and its
  # status became the script's. Report the turn, not the pretty-printer.
  exit $RC
  ;;
watch)
  N=${3:-60}
  [ -f "$SRVLOG" ] || { echo "no server log for slot $SLOT ($SRVLOG)"; exit 2; }
  # An empty watch on a dead world looks exactly like a quiet one.
  alive || echo "!! slot $SLOT IS NOT RUNNING — what follows is the log of a world that has already ended (run: play-harness.sh status $SLOT)"
  sed 's/\x1b\[[0-9;]*m//g' "$SRVLOG" | grep -E "EXPERIENCE census|colony drive|dish produced|ate — hunger|slept|unreachable|stalled on materials|RE-TARGET|beds registered|COLONY TERMINAL" | tail -"$N"
  ;;
status)
  postmortem
  alive || exit 1
  ;;
stop)
  if [ -f "$PIDF" ]; then
    PID=$(cat "$PIDF")
    kill "$PID" 2>/dev/null
    for _ in 1 2 3 4 5 6 7 8 9 10; do
      kill -0 "$PID" 2>/dev/null || break
      sleep 1
    done
    kill -0 "$PID" 2>/dev/null && { kill -9 "$PID" 2>/dev/null; sleep 1; }
  fi
  # ★ VERIFY THE THING YOU CLAIM (2026-08-21, second strike): the first
  # version of this check verified the recorded PID — which was stale for any
  # world booted before the pid fix — and then announced "port released"
  # without ever looking at the port. A leftover server went on holding the
  # server binary and failed a build an hour later. The PORT is the claim, so
  # the PORT is what gets checked, and a stale pidfile cannot hide a live
  # listener: kill whoever actually holds it.
  HOLDER=$(netstat -ano 2>/dev/null | grep LISTENING | grep -E "[:.]$GAME" | awk '{print $NF}' | head -1)
  if [ -n "$HOLDER" ]; then
    echo "slot $SLOT: port $GAME still held by pid $HOLDER — killing it"
    taskkill //PID "$HOLDER" //F >/dev/null 2>&1 || kill -9 "$HOLDER" 2>/dev/null
    sleep 2
  fi
  if netstat -ano 2>/dev/null | grep LISTENING | grep -qE "[:.]$GAME"; then
    echo "slot $SLOT NOT STOPPED — port $GAME is still listening"; exit 1
  fi
  # Only now that the process is provably dead is it safe to drop its binaries.
  rm -f "$PIDF" "$BOOTMETA"
  rm -rf "$BINDIR"
  echo "slot $SLOT stopped (port $GAME verified free)"
  ;;
*)
  echo "usage: play-harness.sh boot|turn|watch|status|stop <slot> [arg]"; exit 2 ;;
esac
