# Project Bastion — fast inner-loop type check (B0).
# Type-checks the simulation-side crates Bastion work touches, plus the harness,
# without building voxygen or linking binaries.
#
# Portable equivalent (any platform, or use the `cargo bastion-check` alias
# defined in .cargo/config.toml):
#   cargo check -p veloren-common -p veloren-server -p veloren-rtsim -p bastion-harness

$ErrorActionPreference = "Continue"

# This machine's toolchain is a per-user install that some shells don't inherit
# (see BASELINE.md); only prepend when cargo isn't already resolvable.
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:USERPROFILE\toolchains\mingw64\bin;$env:Path"
}

Set-Location (Split-Path $PSScriptRoot -Parent)
cargo check -p veloren-common -p veloren-server -p veloren-rtsim -p bastion-harness
exit $LASTEXITCODE
