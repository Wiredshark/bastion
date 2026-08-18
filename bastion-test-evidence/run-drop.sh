#!/usr/bin/env bash
# Single-arm launcher for TAG=drp (script-drop.txt).
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-live.sh; see run-ack.sh's header for why no
# logic may live here.
set -u
EV=/e/veloren-master/bastion-test-evidence
TAG=drp GAME=24004 SCRIPT=script-drop.txt
. "$EV/run-template-live.sh"
