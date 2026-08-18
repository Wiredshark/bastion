#!/usr/bin/env bash
# F1: is the HEADLESS control reliable and bit-identical? 4 legs, no client.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, hold and teardown
# logic lives in run-template-fan.sh. Headless is the one place
# BASTION_UNCAPPED_TPS is safe -- there is no client whose arrival the
# free-running server could outrun.
#
# THE OLD BODY'S COMMENT SAID EACH LEG "IS STOPPED and counted at exactly
# that tick" AND STOPPED NOTHING -- four servers orphaned per invocation,
# every invocation. The hold+teardown now actually exist: each leg runs
# past HOLD_TICK (the determinism baseline's own 9000-tick window + margin,
# unchanged), then is killed by pid and port-witnessed.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=f1
ARMS="f1a:15004:- f1b:15104:- f1c:15204:- f1d:15304:-"
HOLD_TICK=9600
BASTION_EXTRA="BASTION_UNCAPPED_TPS=1"
. "$EV/run-template-fan.sh"
