# PLAY.ps1 — launch a bastion world, for Ben. (PowerShell 5.1)
#
#   .\PLAY.ps1 town          adopt a real worldgen village: the VILLAGERS
#                            become your colony (framework finished 2026-08-21)
#   .\PLAY.ps1 arena         found on flat test ground: the colony mines its own
#                            stone, builds its own beds, colonists carry tools
#
#   .\PLAY.ps1 town -Client  also launch the game once the server is up
#   .\PLAY.ps1 -Stop         stop the server (verifies the PORT, not just the pid)
#
# JOIN AT:  localhost:14004      USERNAME: player      (no password, --no-auth)

param(
    [Parameter(Position = 0)][ValidateSet('town', 'arena')][string]$Mode = 'town',
    [switch]$Client,
    [switch]$Stop
)

$ErrorActionPreference = 'Stop'
$WT   = 'E:\veloren-master\.item29-wt'
$Bin  = Join-Path $WT 'target\no_overflow'
$UD   = Join-Path $WT 'userdata-play-ben'
$Port = 14004

function Test-PortHeld {
    $held = netstat -ano | Select-String 'LISTENING' | Select-String ":$Port\s"
    if ($held) { return $held } else { return $null }
}

if ($Stop) {
    # ★ VERIFY THE PORT, NOT THE PID. A stale pidfile must never let this
    # announce success while a server is still listening — the play harness
    # made exactly that mistake twice and a leftover server then failed a
    # build an hour later.
    $held = Test-PortHeld
    if ($held) {
        $owner = ($held -split '\s+')[-1]
        Write-Host "port $Port held by pid $owner - killing it"
        taskkill /PID $owner /F | Out-Null
        Start-Sleep -Seconds 2
    }
    if (Test-PortHeld) { Write-Host "NOT STOPPED - port $Port is still listening" }
    else { Write-Host "stopped; port $Port verified free" }
    return
}

# Refuse rather than start a second server nobody asked for -- but if the
# caller asked for the CLIENT, give them the client. The first version bailed
# out here unconditionally, so `.\PLAY.ps1 town -Client` against an
# already-running world printed "a server is ALREADY on port 14004" and
# launched nothing at all. That is the single most likely way to run this
# script -- the world is up, you want to go and look at it -- and it was the
# one path that did nothing.
$held = Test-PortHeld
if ($held) {
    Write-Host "A server is already on port $Port (that's fine - it's your world)."
    if ($Client) {
        Write-Host 'Launching the game against it...'
        $env:VELOREN_ASSETS = Join-Path $WT 'assets'
        # --bastion-overseer: the god view. It is NOT on by default -- the flag
        # is auto-enabled for the asset-arena and flat-arena paths only, and the
        # TOWN path (the one Ben actually plays) never got it. So F9 did
        # nothing, because the toggle itself is gated on this flag, and the
        # session started in third-person with no way into the overseer camera.
        Start-Process -FilePath "$Bin\veloren-voxygen.exe" `
            -ArgumentList '--bastion-overseer' -WorkingDirectory $WT
        Write-Host ''
        Write-Host "  In game: Multiplayer -> localhost:$Port -> username 'player' (no password)"
    } else {
        Write-Host "Join localhost:$Port as 'player', or run  .\PLAY.ps1 -Stop  to end it."
        Write-Host "To launch the game against it:  .\PLAY.ps1 town -Client"
    }
    return
}

if (-not (Test-Path "$Bin\veloren-server-cli.exe")) {
    Write-Host "server binary missing: $Bin\veloren-server-cli.exe"; return
}

# ★ REFUSE A MISMATCHED PAIR (2026-08-21). A server and client built from
# different source produce "Network error: deserialize error on message:
# UnexpectedEnd { additional: 8 }" at the join screen, which looks exactly like
# a protocol bug and is not. I caused this for Ben by rebuilding ONLY
# veloren-server-cli for a server-side fix and leaving voxygen 11 minutes
# behind. There was nothing on screen to suggest the cause, so he had no way to
# tell it from a real network fault.
#
# Compare BUILD TIMES, and fail loudly rather than launching into the error.
$srv = (Get-Item "$Bin\veloren-server-cli.exe").LastWriteTime
$cli = if (Test-Path "$Bin\veloren-voxygen.exe") { (Get-Item "$Bin\veloren-voxygen.exe").LastWriteTime } else { $null }
if ($null -eq $cli) {
    Write-Host "client binary missing: $Bin\veloren-voxygen.exe"; return
}
$skewMin = [Math]::Abs(($srv - $cli).TotalMinutes)
if ($skewMin -gt 3) {
    Write-Host ''
    Write-Host 'REFUSING TO LAUNCH - server and client were built from different source.'
    Write-Host ("  veloren-server-cli.exe  {0}" -f $srv)
    Write-Host ("  veloren-voxygen.exe     {0}" -f $cli)
    Write-Host ("  skew: {0:N1} minutes" -f $skewMin)
    Write-Host ''
    Write-Host 'This produces "deserialize error on message: UnexpectedEnd" at the join'
    Write-Host 'screen, which is NOT a network problem. Rebuild BOTH together:'
    Write-Host '  cargo build --profile no_overflow -p veloren-server-cli -p veloren-voxygen'
    Write-Host ''
    return
}

# The two worlds.
$EnvVars = if ($Mode -eq 'town') {
    @{
        BASTION_ADOPT_TOWN             = '1'
        BASTION_AUTOFOUND_REAL_TERRAIN = '1'
        BASTION_COLONY_PRESENCE_VD     = '3'
        BASTION_AUTOFOUND_COLONY       = '8'
        BASTION_SEED_FOOD              = '64'
        BASTION_SEED_MATERIALS         = '64'
    }
} else {
    @{
        BASTION_FLAT_ARENA           = '1'
        BASTION_FLAT_ARENA_RESOURCED = '1'
        BASTION_AUTOFOUND_COLONY     = '8'
        BASTION_SEED_FOOD            = '32'
    }
}

# Fresh userdata, so a test is never confused by a previous colony.
if (Test-Path $UD) { Remove-Item $UD -Recurse -Force }

$env:VELOREN_USERDATA = $UD
$env:VELOREN_ASSETS   = Join-Path $WT 'assets'
foreach ($k in $EnvVars.Keys) { Set-Item -Path "Env:$k" -Value $EnvVars[$k] }

# Grant admin to the account you log in as. This one CAN be quiet: it exits.
& "$Bin\veloren-server-cli.exe" --no-auth admin add player admin 2>&1 | Out-Null

Write-Host "booting a '$Mode' world... (worldgen takes a few minutes the first time)"

# ★ ITS OWN WINDOW (never -NoNewWindow: server-cli is an interactive TUI and
# starving it of a console is what crashed it on launch), BUT the output IS
# captured. The earlier note here said "it writes its own logs under userdata
# regardless, so nothing is lost" -- that is FALSE, and I checked rather than
# trusted it: after a boot, userdata holds rtsim/, saves/ and save_universe/
# and NO log file anywhere. Everything the server said about founding, adoption
# and the colony went to a console window and was gone.
#
# That cost a verification pass: the adopt-a-town census prints ONCE at
# founding, so "did the villagers get adopted" was unanswerable from a world
# that was running correctly. Redirecting with a new window is fine -- it was
# only the -NoNewShell/-NoNewWindow combination that broke the TUI.
$Log = Join-Path $WT 'play-server.log'
if (Test-Path $Log) { Remove-Item $Log -Force }
Start-Process -FilePath "$Bin\veloren-server-cli.exe" -ArgumentList '--no-auth' `
    -WorkingDirectory $WT -RedirectStandardOutput $Log `
    -RedirectStandardError (Join-Path $WT 'play-server.err.log') | Out-Null

# ★ WAIT ON THE PORT, not on a log line. The log is in the server's own window
# now, and "is it accepting connections" is exactly what the port answers.
$ready = $false
for ($i = 1; $i -le 120; $i++) {
    Start-Sleep -Seconds 5
    if (Test-PortHeld) { Write-Host "READY after $($i * 5)s"; $ready = $true; break }
}

Write-Host ''
if ($ready) {
    Write-Host '  JOIN AT:  localhost:14004'
    Write-Host '  USERNAME: player       (no password - the server runs --no-auth)'
} else {
    Write-Host "  NOT READY after 10 min - check the server window for errors."
}
Write-Host "  Stop it:  .\PLAY.ps1 -Stop"
Write-Host ''

if ($Client -and $ready) {
    # Same reason as above: without --bastion-overseer there is no god view and
    # F9 is inert.
    Start-Process -FilePath "$Bin\veloren-voxygen.exe" `
        -ArgumentList '--bastion-overseer' -WorkingDirectory $WT
}
