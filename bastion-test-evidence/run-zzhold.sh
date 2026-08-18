#!/usr/bin/env bash
# SYNTHETIC single-arm headless family for H2's plant/control pair
# (HOLD-EXTENSION-PREREG.md). Not a scored scenario -- exists to make the
# hold's two renderings fire on the real code path with a real server.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=zzhold
ARMS="zza:19904:-"
BASTION_EXTRA="BASTION_UNCAPPED_TPS=1"
. "$EV/run-template-fan.sh"
