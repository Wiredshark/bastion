#!/usr/bin/env bash
# V2 bridge: the headless leg pair on the v2 family -- same shape as F1
# (the old bodies were byte-siblings, headers included).
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, hold and teardown
# logic lives in run-template-fan.sh; see run-fixture-f1.sh for the false
# "is stopped" label this conversion retires.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=v2
# Ports are the old body's own (leg v2a 17004 17005 17006 / v2b 17104 ...),
# read from git history rather than invented -- the first draft of this
# wrapper guessed 15804/15904 and the read caught it. Derived +1/+2
# reproduces the old explicit web/query values exactly.
ARMS="v2a:17004:- v2b:17104:-"
HOLD_TICK=9600
BASTION_EXTRA="BASTION_UNCAPPED_TPS=1"
. "$EV/run-template-fan.sh"
