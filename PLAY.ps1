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

# Refuse rather than start a second server nobody asked for.
$held = Test-PortHeld
if ($held) {
    Write-Host "A server is ALREADY on port $Port :"
    Write-Host "  $held"
    Write-Host "Run  .\PLAY.ps1 -Stop  first, or just join localhost:$Port as 'player'."
    return
}

if (-not (Test-Path "$Bin\veloren-server-cli.exe")) {
    Write-Host "server binary missing: $Bin\veloren-server-cli.exe"; return
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

# ★ ITS OWN WINDOW, NO REDIRECTION. server-cli is an interactive TUI; capturing
# its stdout with -RedirectStandardOutput/-NoNewWindow is what crashed it on
# launch. It writes its own logs under userdata regardless, so nothing is lost.
Start-Process -FilePath "$Bin\veloren-server-cli.exe" -ArgumentList '--no-auth' `
    -WorkingDirectory $WT | Out-Null

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
    Start-Process -FilePath "$Bin\veloren-voxygen.exe" -WorkingDirectory $WT
}
