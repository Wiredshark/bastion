# PLAY.ps1 — launch a bastion world and the client, for Ben. (PowerShell)
#
#   .\PLAY.ps1 town     -> adopt a real worldgen village: the VILLAGERS become
#                          your colony (the framework finished 2026-08-21)
#   .\PLAY.ps1 arena    -> found a colony on flat test ground: it mines its own
#                          stone, builds its own beds, colonists carry tools
#
#   .\PLAY.ps1 town -Client    -> also launches the game client
#   .\PLAY.ps1 -Stop           -> stop the server
#
# JOIN AT:  localhost:14004     username: player   (no password — --no-auth)

param(
    [Parameter(Position = 0)][ValidateSet('town', 'arena')][string]$Mode = 'town',
    [switch]$Client,
    [switch]$Stop
)

$WT  = 'E:\veloren-master\.item29-wt'
$Bin = Join-Path $WT 'target\no_overflow'
$UD  = Join-Path $WT 'userdata-play-ben'
$Log = Join-Path $WT 'play-server.log'
$PidFile = Join-Path $WT '.play-pid'

if ($Stop) {
    if (Test-Path $PidFile) {
        $serverPid = Get-Content $PidFile
        try { Stop-Process -Id $serverPid -Force -ErrorAction Stop; Write-Host "stopped server (pid $serverPid)" }
        catch { Write-Host "pid $serverPid was not running" }
        Remove-Item $PidFile -ErrorAction SilentlyContinue
    }
    # Verify the PORT, not just the pid: a stale pidfile must not report success
    # while a server is still holding 14004.
    $held = (netstat -ano | Select-String 'LISTENING' | Select-String ':14004')
    if ($held) { Write-Host "WARNING: port 14004 is still held:`n$held" }
    else { Write-Host "port 14004 is free" }
    return
}

# The two worlds. `town` is the new adoption framework; `arena` is flat test
# ground for watching the founding loop.
$EnvVars = @{}
if ($Mode -eq 'town') {
    $EnvVars = @{
        BASTION_ADOPT_TOWN            = '1'
        BASTION_AUTOFOUND_REAL_TERRAIN = '1'
        BASTION_COLONY_PRESENCE_VD    = '3'
        BASTION_AUTOFOUND_COLONY      = '8'
        BASTION_SEED_FOOD             = '64'
        BASTION_SEED_MATERIALS        = '64'
    }
} else {
    $EnvVars = @{
        BASTION_FLAT_ARENA           = '1'
        BASTION_FLAT_ARENA_RESOURCED = '1'
        BASTION_AUTOFOUND_COLONY     = '8'
        BASTION_SEED_FOOD            = '32'
    }
}

# Fresh userdata each run, so a test is never confused by a previous colony.
if (Test-Path $UD) { Remove-Item $UD -Recurse -Force }

$env:VELOREN_USERDATA = $UD
$env:VELOREN_ASSETS   = Join-Path $WT 'assets'

# Grant admin to the account you will log in as.
& "$Bin\veloren-server-cli.exe" --no-auth admin add player admin *> $null

foreach ($k in $EnvVars.Keys) { Set-Item -Path "Env:$k" -Value $EnvVars[$k] }

Write-Host "booting a '$Mode' world... (worldgen takes a few minutes the first time)"
$proc = Start-Process -FilePath "$Bin\veloren-server-cli.exe" `
    -ArgumentList '--no-auth' -PassThru -NoNewWindow `
    -RedirectStandardOutput $Log -RedirectStandardError "$Log.err"
$proc.Id | Out-File -FilePath $PidFile -Encoding ascii

$ready = $false
for ($i = 1; $i -le 120; $i++) {
    Start-Sleep -Seconds 10
    if ((Test-Path $Log) -and (Select-String -Path $Log -Pattern 'ready to accept connections' -Quiet)) {
        Write-Host "READY after $($i * 10)s"
        $ready = $true
        break
    }
}
if (-not $ready) { Write-Host "NOT READY after 20 min - read $Log" }

Write-Host ''
Write-Host '  JOIN AT:  localhost:14004'
Write-Host '  USERNAME: player      (no password - the server runs --no-auth)'
Write-Host ''
Write-Host "  Server log: $Log"
Write-Host "  Stop it:    .\PLAY.ps1 -Stop"
Write-Host ''

if ($Client) {
    Set-Location $WT
    & "$Bin\veloren-voxygen.exe"
}
