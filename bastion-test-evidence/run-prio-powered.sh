#!/usr/bin/env bash
# ITEM 16 -- POWERED re-run. Two replicates per arm, 3x window, six servers in
# parallel with every listening socket isolated.
#
# A PARAMETER WRAPPER, NOT A LAUNCHER. All launch, wait, drive and teardown
# logic lives in run-template-fan.sh.
#
# FAM=pw, NOT powered: the historical run's logs are server-pw-<ARM>.log while
# its attestation is powered-attest.txt, and nothing anywhere relates the two
# -- the reconciliation row's sharpest specimen. The attest tag now equals the
# log family's name, so log names stay byte-stable with history and the new
# tag is reachable by declaration. powered-attest.txt and powered.log stay
# exactly as they are: closed historical artefacts.
#
# WHY the replicates exist (from the old body): the short run gave control=3
# hauls vs treatment=0, but an earlier row recorded two n=8 legs on one script
# hauling 5 and 0 -- zero is inside the control's own distribution, so a
# 3-vs-0 at n=1 cannot be scored. Replicates give the control a distribution;
# the longer window gives it a magnitude big enough for zero to mean
# something. The window lives in the script-prioL-*.txt drivers, untouched.
set -u
EV=/e/veloren-master/bastion-test-evidence
FAM=pw
ARMS="pwA1:14404:script-prioL-A.txt pwA2:14504:script-prioL-A.txt pwB1:14604:script-prioL-B.txt pwB2:14704:script-prioL-B.txt pwC1:14804:script-prioL-C.txt pwC2:14904:script-prioL-C.txt"
. "$EV/run-template-fan.sh"
