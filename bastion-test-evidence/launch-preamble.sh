#!/usr/bin/env bash
# THE SHARED LAUNCH PREAMBLE for scored Bastion runs.
#
# WHY THIS EXISTS: attestation was a RITUAL, not a property of the runner.
# At the time this was written there were 17 scripts that launch a server or
# a driver, ONE of them called `attest-run.sh` -- and THIRTEEN `*-attest.txt`
# artefacts sat in this directory. Twelve of them were produced by hand,
# with the arguments chosen fresh each time.
#
# That is the whole reason the two-binary gap survived: a step whose
# arguments a human picks every run is a step that will eventually be
# performed with one binary instead of two. It was, twice -- once as a
# wire-format skew and once as a plant that could not have fired, and both
# legs had to be voided.
#
# WHAT THIS DOES NOT FIX. A sourced preamble still depends on the runner
# sourcing it. This converts "remember four steps" into "remember one
# line" -- a real reduction, NOT a structural guarantee, and it should not
# be described as one. A runner that forgets to source this is exactly as
# unattested as the 16 that never called `attest-run.sh` at all.
#
# Usage, from a runner:
#     BASTION_ENV="BASTION_DETERMINISTIC=1 ..."      # declare the config
#     . "$EV/launch-preamble.sh"                      # attest + gate
#     ( cd "$WT" && ... env $BASTION_ENV "$B/veloren-server-cli.exe" ... )
#
# Requires, from the sourcing runner: EV, B, TAG, and BASTION_ENV.

# The config is DECLARED by the runner, never observed here -- runners set
# their vars inline on the launch line, so they are not in this shell's
# environment and reading `env` would silently record nothing. Exported so
# the attestation subprocess inherits it; see attest-run.sh's header for
# why an unset config must not render as an empty one.
export BASTION_ENV="${BASTION_ENV-}"

# THE OUTPUTS ARE DECLARED THE SAME WAY, AND FOR THE SAME REASON. There is
# nothing to observe: this runs before the launch, so none of the run's
# evidence files exist yet. The runner names them, the attestation records
# them, and `run-ledger.sh` resolves them against the disk afterwards -- one
# definition, three consumers.
#
# NOT DEFAULTED. `${BASTION_LOGS-}` here would hand every non-declaring runner
# a silent empty set, which is precisely the rendering `attest-run.sh` refuses
# to produce. Left untouched, an undeclaring runner's attestation says NO
# OUTPUTS DECLARED -- true, and visible.
export BASTION_LOGS

# ATTEST BOTH BINARIES AND GATE ON THE RESULT. `|| exit 1` is the point:
# a warning depends on an operator reading it, and the operator is the
# component that already failed twice.
bash "$EV/attest-run.sh" "$EV/${TAG}-attest.txt" \
     "$B/veloren-server-cli${PIT_EXE-.exe}" "$B/bastion_playtest${PIT_EXE-.exe}" || {
  echo "ATTESTATION REFUSED -- not launching $TAG" >&2
  exit 1
}

# ★ PRE-FLIGHT PORT CHECK (#97). An attestation proves what will be LAUNCHED.
# It says nothing about what the driver will TALK TO -- and those are different
# facts, which this program learned the expensive way on 2026-08-17.
#
# `launch-postamble.sh` already warns when a leg's own server survives
# teardown. That warning is printed at the END of the leg that leaked, so it
# cannot protect that leg -- it is a message to the NEXT one, delivered to
# stderr, where it waits for an operator to read it. Three consecutive legs
# then launched into a held port: their own server could not bind, their driver
# connected to the PREVIOUS leg's server, and their readings were attributed to
# an environment that server never had. One published finding had to be
# withdrawn over it.
#
# So the check moves to where it can actually refuse: HERE, before launch. If
# something already answers on $GAME, this leg cannot own the port it is about
# to measure, and a measurement whose subject is unidentified is worse than no
# measurement -- it looks exactly like a real one.
#
# Deliberately NOT a kill. A process this script did not start is not this
# script's to stop (the same rule the postamble's kill-by-recorded-pid obeys).
# It refuses and names the port; the operator decides what is holding it.
if (exec 3<>"/dev/tcp/127.0.0.1/$GAME") 2>/dev/null; then
  exec 3<&- 3>&-
  echo "PORT $GAME ALREADY HELD -- not launching $TAG" >&2
  echo "  A leg that cannot bind its own port has no provenance: the driver" >&2
  echo "  would connect to whatever is already there, and the attestation" >&2
  echo "  would describe a server that never answered a single query." >&2
  echo "  Stop the holder (it is almost always a previous leg's server that" >&2
  echo "  outlived its teardown), then re-run." >&2
  exit 1
fi
