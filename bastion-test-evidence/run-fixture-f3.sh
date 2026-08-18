#!/usr/bin/env bash
# F3: the WITHIN-RUN test. Client connects, holds to ~tick 9000, disconnects;
# the server runs on to ~18000 with no client. Each leg is its own control.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive, hold and
# teardown logic lives in run-template-fan.sh. NOT uncapped: client-driven,
# and BASTION_UNCAPPED_TPS free-runs from boot with no wait for a client.
# The drivers exit at ~9000; HOLD_TICK carries every server to 18300 (the
# old body's own bound) BEFORE teardown -- the old body simply ended there,
# leaving four servers running.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=f3
ARMS="f3a:15404:script-f3.txt f3b:15504:script-f3.txt f3c:15604:script-f3.txt f3d:15704:script-f3.txt"
HOLD_TICK=18300
. "$EV/run-template-fan.sh"
