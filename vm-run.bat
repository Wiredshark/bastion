@echo off
REM Simple one-word entry point for the on-demand VM test runner.
REM Usage from any Windows shell:  vm-run.bat --mine-fidelity-scenario --mf-minutes 10
REM Wraps vm-run.sh (auto-start VM -> pull -> build -> run -> VM self-stops). See readme/BUILD-AND-TEST-PROCESS.md.
bash "%~dp0vm-run.sh" %*
