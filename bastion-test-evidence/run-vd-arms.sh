#!/usr/bin/env bash
# V1: 4 client legs at VD 6 vs 4 at VD 25, same binary/seed/env.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-fan.sh. The VD split -- the V1 row's SCORED
# AXIS -- rides the per-arm driver-env field (driver only, never the
# server: the server env is BASTION_ENV and attested). VD is forced from
# settings.ron via handle_initialize_character's clamp; the server's
# "client view distance granted" emit is the V0 precondition each arm's
# granted value is checked against.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=vd
ARMS="vd6a:16004:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=6 vd6b:16104:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=6 vd6c:16204:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=6 vd6d:16304:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=6 vd25a:16404:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=25 vd25b:16504:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=25 vd25c:16604:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=25 vd25d:16704:script-vd.txt:BASTION_DRIVER_VIEW_DISTANCE=25"
. "$EV/run-template-fan.sh"
