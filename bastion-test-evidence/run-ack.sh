#!/usr/bin/env bash
# Single-arm launcher for TAG=ack (script-ack.txt).
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-live.sh -- this file exists to name the tag,
# the port family, and the driver script, and MUST NOT grow logic of its
# own: eight copies of one launcher propagated the same five holes for a
# month, and even their header comments were copied verbatim (all eight
# claimed to be the inspector's ENTITY arm; seven were not).
set -u
EV=/e/veloren-master/bastion-test-evidence
TAG=ack GAME=25004 SCRIPT=script-ack.txt
. "$EV/run-template-live.sh"
