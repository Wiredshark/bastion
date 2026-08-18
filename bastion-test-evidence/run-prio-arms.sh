#!/usr/bin/env bash
# ITEM 16 -- haul priority live path. Three arms, PARALLEL on distinct ports.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-fan.sh. This file names the family, the arms,
# their game ports (web/query derive as PORT+1/PORT+2 per arm -- the old
# body rewrote only the game port, so three concurrent servers shared web
# 14005 and query 14006), and the per-arm driver scripts.
#
# Historical fixes the old body carried (now the template's job or moot):
# --no-auth on the admin grant (template does it); window 5400 -> 12000
# ticks (lives in the driver scripts, untouched); parallel ports rather
# than BASTION_UNCAPPED_TPS, because uncapped skips clock.tick() with no
# gate on a client being connected -- a HEADLESS-ONLY lever.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=prio2
ARMS="prioA:14104:script-prio-A.txt prioB:14204:script-prio-B.txt prioC:14304:script-prio-C.txt"
. "$EV/run-template-fan.sh"
