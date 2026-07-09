# Bastion Automated Loop Runner v2 - FIXED
# ------------------------------------------------------------------------------------
# Fixes from v1 runaway (500 iterations):
#   1. EVERY git call is verified. If git fails for any reason (bad path, not a repo,
#      git missing), the loop STOPS LOUDLY. A safety check that can silently fail is
#      not a safety check - that hole is what let v1 spin.
#   2. Hard iteration guard INSIDE the loop body (belt + suspenders with the for-loop).
#   3. If $ClaudeCommand is not wired to a real command, the script REFUSES to loop.
#   4. Each Claude session launches in its OWN VISIBLE console window (watch it work);
#      output is also transcribed to a per-iteration log file so nothing is lost.
#   5. The loop waits for that window process to EXIT before continuing.
#
# USAGE:
#   powershell -ExecutionPolicy Bypass -File "E:\veloren-master\readme\bastion-loop-runner.ps1"
#
# BEFORE FIRST REAL RUN:
#   - Set $ClaudeCommand below to your REAL headless Claude Code invocation.
#   - Test it once by hand in a terminal first. If you do not have a headless CLI,
#     this loop cannot work - stop here and paste manually instead.
# ------------------------------------------------------------------------------------

$ErrorActionPreference = "Stop"   # any unhandled error kills the script instead of limping on

# ================================ CONFIG ================================
$RepoPath      = "E:\veloren-master"
$DocsDir       = "E:\veloren-master\readme"     # ALL design .md files live here
$MaxIterations = 6                               # hard cap per launch
$MegaPrompt    = Join-Path $DocsDir "MEGA-PROMPT-autonomous-batch-builder.md"
$AttachDocs    = @(
  (Join-Path $DocsDir "veloren-colony-rts-build-report.md"),
  (Join-Path $DocsDir "agency-bible.md"),
  (Join-Path $DocsDir "df-feature-gap-ledger.md"),
  (Join-Path $DocsDir "divine-politics-bible.md")
)
$RunLog        = Join-Path $DocsDir "BASTION_RUN_LOG.md"
$LoopLog       = Join-Path $DocsDir "BASTION_LOOP_LOG.md"
$SessionLogDir = Join-Path $DocsDir "loop-sessions"   # per-iteration Claude output logs
$StopSentinel  = "LOOP-STOP"

# >>> WIRE YOUR REAL COMMAND HERE. Leave $null and the script will refuse to loop. <<<
# This must be a SINGLE command line that: starts a fresh headless Claude Code session,
# feeds the mega-prompt, runs to completion, and EXITS. Test it by hand first.
# Example shape (VERIFY flags against your Claude Code docs - do not guess):
#   $ClaudeCommand = "claude -p --dangerously-skip-permissions `"$(Get-Content $MegaPrompt -Raw)`""
$ClaudeCommand = "Get-Content '$MegaPrompt' -Raw | claude -p --dangerously-skip-permissions --model fable-5"

# ================================ HELPERS ================================
function Write-LoopLog($msg) {
  $line = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  $msg"
  Add-Content -Path $LoopLog -Value $line
  Write-Host $line
}

function Fail-Loud($msg) {
  Write-LoopLog "FATAL: $msg"
  Write-Host ""
  Write-Host "  ## LOOP HALTED: $msg" -ForegroundColor Red
  Write-Host ""
  exit 1
}

function Invoke-Git([string[]]$GitArgs) {
  # Runs git in the repo. Returns stdout. FAILS LOUD on any git error.
  $out = & git -C $RepoPath @GitArgs 2>&1
  if ($LASTEXITCODE -ne 0) {
    Fail-Loud "git $($GitArgs -join ' ') failed (exit $LASTEXITCODE): $out"
  }
  return $out
}

function Get-LatestBlockTag {
  $tags = Invoke-Git @("tag", "--list", "bastion-block-*", "--sort=-creatordate")
  if ($null -eq $tags) { return "" }
  return ($tags | Select-Object -First 1).ToString().Trim()
}

function Test-TreeClean {
  $status = Invoke-Git @("status", "--porcelain")
  return [string]::IsNullOrWhiteSpace(($status -join ""))
}

# ================================ PRE-FLIGHT ================================
Write-Host "=== Bastion Loop Runner v2 ===" -ForegroundColor Cyan

# 0) Refuse to run unwired. This is what prevents a pointless spin.
if ([string]::IsNullOrWhiteSpace($ClaudeCommand)) {
  Fail-Loud "`$ClaudeCommand is not set. Wire your real headless Claude Code command into CONFIG first. Refusing to loop on a stub."
}

# 1) Paths exist.
if (-not (Test-Path $RepoPath))   { Fail-Loud "Repo not found: $RepoPath" }
if (-not (Test-Path $MegaPrompt)) { Fail-Loud "Mega-prompt not found: $MegaPrompt" }
foreach ($doc in $AttachDocs) { if (-not (Test-Path $doc)) { Fail-Loud "Attach doc missing: $doc" } }
if (-not (Test-Path $SessionLogDir)) { New-Item -ItemType Directory -Path $SessionLogDir | Out-Null }

# 2) Git works HERE (this failing silently is what broke v1).
$null = Invoke-Git @("rev-parse", "--is-inside-work-tree")

# 3) Tree clean.
if (-not (Test-TreeClean)) { Fail-Loud "Working tree is dirty. Commit/stash before looping." }

$startTag = Get-LatestBlockTag
if ([string]::IsNullOrWhiteSpace($startTag)) { Fail-Loud "No bastion-block-* tags found. Wrong repo? Refusing to run." }
Write-LoopLog "=== LOOP START. Cap: $MaxIterations. Starting tag: $startTag ==="

# ================================ THE LOOP ================================
$prevTag = $startTag
$iterationsRun = 0

for ($i = 1; $i -le $MaxIterations; $i++) {

  # Belt-and-suspenders runaway guard: if this counter somehow exceeds the cap, die.
  $iterationsRun++
  if ($iterationsRun -gt $MaxIterations) { Fail-Loud "Runaway guard tripped ($iterationsRun > $MaxIterations)." }

  Write-LoopLog "--- Iteration $i of $MaxIterations ---"

  # Gate: clean tree before launching.
  if (-not (Test-TreeClean)) { Fail-Loud "Tree dirty before iteration $i. Human review needed." }

  # Gate: runner-requested stop.
  if ((Test-Path $RunLog) -and (Select-String -Path $RunLog -Pattern $StopSentinel -Quiet)) {
    Write-LoopLog "STOP: '$StopSentinel' found in run log. Runner requested halt. Ending loop."
    break
  }

  # ---- LAUNCH: new visible console window, transcribed to a log file ----
  $sessionLog = Join-Path $SessionLogDir ("session-{0}-{1}.log" -f (Get-Date -Format 'yyyyMMdd-HHmmss'), $i)
  Write-LoopLog "Launching Claude session in a NEW console window. Log: $sessionLog"

  # The child window runs the command, tees output to the log, then closes.
  # -Wait blocks THIS loop until that window process exits - that's how we know the session ended.
  $childCmd = "& { $ClaudeCommand } 2>&1 | Tee-Object -FilePath `"$sessionLog`""
  $proc = Start-Process -FilePath "powershell.exe" `
                        -ArgumentList "-ExecutionPolicy","Bypass","-NoProfile","-Command",$childCmd `
                        -WorkingDirectory $RepoPath `
                        -PassThru -Wait

  Write-LoopLog "Session window exited (code $($proc.ExitCode))."

  # Gate: clean tree after (runner must finish-or-rollback to clean).
  if (-not (Test-TreeClean)) { Fail-Loud "Tree dirty AFTER iteration $i. Runner left an unclean state. Human review needed." }

  # Progress check: a NEW tag must exist, or we stop (never retry into the same wall).
  $newTag = Get-LatestBlockTag
  if ($newTag -eq $prevTag) {
    Write-LoopLog "STOP: no new block tag (still '$prevTag'). Stalled/failed/undesigned. Ending loop - read $sessionLog and the run log."
    break
  }

  Write-LoopLog "PROGRESS: '$prevTag' -> '$newTag'."
  $prevTag = $newTag
}

# ================================ WRAP UP ================================
Write-LoopLog "=== LOOP END. Iterations run: $iterationsRun. Final green tag: $(Get-LatestBlockTag) ==="
Write-Host ""
Write-Host "Read $RunLog for per-block reports." -ForegroundColor Cyan
Write-Host "Read $LoopLog for the loop decisions." -ForegroundColor Cyan
Write-Host "Per-session Claude output is in $SessionLogDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "ROLLBACK CHEAT SHEET:" -ForegroundColor Yellow
Write-Host "  git -C $RepoPath tag --list `"bastion-block-*`" --sort=creatordate"
Write-Host "  git -C $RepoPath reset --hard <previous-tag>   # revert main one block"
