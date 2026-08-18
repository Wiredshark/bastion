#!/usr/bin/env bash
# Single-arm launcher for TAG=insp (script-inspector.txt) -- the original
# ITEM 9 leg: the inspector's ENTITY arm, the arm the HUD uses. Of the eight
# copied launchers this is the ONE whose copied header was true.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-live.sh; see run-ack.sh's header for why no
# logic may live here.
set -u
EV=/e/veloren-master/bastion-test-evidence
TAG=insp GAME=18004 SCRIPT=script-inspector.txt
. "$EV/run-template-live.sh"
