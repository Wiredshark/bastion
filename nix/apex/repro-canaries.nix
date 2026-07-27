# APEX-T1.3.11 — known-good/known-bad reproducibility canary derivations.
#
# The smoke's comparator (`nix build --rebuild`) is admitted only if it
# proves it can SEE nondeterminism: `stable` must rebuild byte-identical;
# `time` (wall clock), `random` (OS entropy), and `tmppath` (per-build
# temporary path) must each FAIL a rebuild check for their own distinct
# mechanism. All four are leaf derivations — nothing production may ever
# depend on them.
#
# Every canary sets the same final-derivation policy as the repro package:
# local execution, no substitution — a substituted canary result would
# prove nothing about THIS builder.
{pkgs}: let
  canary = name: script:
    pkgs.runCommand "apex-repro-canary-${name}" {
      allowSubstitutes = false;
      preferLocalBuild = true;
    }
    script;
in {
  stable = canary "stable" ''
    echo "bastion-apex-repro-stable-v1" > $out
  '';
  # Wall-clock leak: nanosecond timestamp lands in the output. Two
  # executions crossing timestamp resolution MUST differ; the sleep makes
  # a lucky same-nanosecond false green practically impossible (packet
  # adversarial review 10.5).
  time = canary "time" ''
    date +%s%N > $out
    sleep 0.05
    date +%s%N >> $out
  '';
  # OS entropy leak.
  random = canary "random" ''
    head -c 32 /dev/urandom | base64 > $out
  '';
  # Per-build temporary path leak: mktemp's randomized name differs per
  # execution even inside the constant-/build sandbox.
  tmppath = canary "tmppath" ''
    mktemp -d -p . cnry.XXXXXXXX > $out
  '';
}
