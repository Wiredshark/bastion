#!/usr/bin/env bash
# Single-arm launcher for TAG=clp (script-collapse.txt).
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-live.sh; see run-ack.sh's header for why no
# logic may live here.
set -u
EV=/e/veloren-master/bastion-test-evidence
TAG=clp GAME=21004 SCRIPT=script-collapse.txt
. "$EV/run-template-live.sh"
