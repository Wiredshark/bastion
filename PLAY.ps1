# PLAY.ps1 — launch a bastion world, for Ben. (PowerShell 5.1)
#
#   .\PLAY.ps1 town          adopt a real worldgen village: the VILLAGERS
#                            become your colony (framework finished 2026-08-21)
#   .\PLAY.ps1 flattown      a real worldgen village on FLAT ground -- but this
#                            world only has a HAMLET near centre (3 houses, no
#                            fields). Good for watching pathing, not a town.
#   .\PLAY.ps1 arena         found on flat test ground: the colony mines its own
#                            stone, builds its own beds, colonists carry tools
#
#   .\PLAY.ps1 town -Client  also launch the game once the server is up
#   .\PLAY.ps1 town -Client -NoRaids   the clean colony the sweeps measured
#                            (raiders off; add them back by dropping the flag)
#   .\PLAY.ps1 -Stop         stop the server (verifies the PORT, not just the pid)
#
# JOIN AT:  localhost:14004      USERNAME: player      (no password, --no-auth)

param(
    [Parameter(Position = 0)][ValidateSet('town', 'flattown', 'arena')][string]$Mode = 'town',
    [switch]$Client,
    [switch]$Stop,
    # Ben, 2026-08-22: "get the town working like real life then introduce
    # raiders and see what breaks." Raiders ON by default so the game is the
    # game; -NoRaids gives the clean colony the sweeps were measured on.
    [switch]$NoRaids,
    # ★ OPT IN to choosing your town on the SELECT STARTING AREA screen.
    # Off by default: see the `town` block below for why the main path must not
    # depend on the player performing a step correctly.
    [switch]$Pick
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

# ★ A STALE SERVER IS NOT "YOUR WORLD", IT IS THE OLD BUILD (2026-08-22).
#
# This burned Ben TWICE in one session, identically. The branch below saw the
# port held, said "that's fine - it's your world", and handed him a client
# attached to a server that had been running for over an hour -- from a binary
# built BEFORE the fix he had just been asked to test. He tested the old code,
# reported "it didn't work", and was right: the fix was not in the process he
# was talking to.
#
# Nothing on screen could have told him. The world loads, the colony is there,
# the client connects cleanly. A stale server and a fresh one look IDENTICAL
# from the game -- which is exactly the shape of bug that has to be caught by
# the launcher, because the player has no instrument for it.
#
# So: compare the running server's START TIME against the binary's build time.
# Older process than binary = the code under test is not running. Restart it.
if ($held) {
    $ownerPid = ($held -split '\s+')[-1]
    $proc = Get-Process -Id $ownerPid -ErrorAction SilentlyContinue
    $binTime = (Get-Item "$Bin\veloren-server-cli.exe" -ErrorAction SilentlyContinue).LastWriteTime
    if ($proc -and $binTime -and $proc.StartTime -lt $binTime) {
        Write-Host ''
        Write-Host 'STALE SERVER - restarting it so you test the build you actually have.'
        Write-Host ("  running server started : {0}  (pid {1})" -f $proc.StartTime, $ownerPid)
        Write-Host ("  server binary built    : {0}" -f $binTime)
        Write-Host '  The running process predates the binary, so it does NOT contain the'
        Write-Host '  latest changes. Attaching a client to it would silently test old code.'
        Write-Host ''
        taskkill /PID $ownerPid /F | Out-Null
        Start-Sleep -Seconds 3
        if (Test-PortHeld) {
            Write-Host "COULD NOT FREE PORT $Port - stop it manually with  .\PLAY.ps1 -Stop"
            return
        }
        Write-Host "port $Port freed; booting a fresh world below."
        $held = $null
    }
}

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

# The three worlds.
$EnvVars = if ($Mode -eq 'flattown') {
    # ★ THE FLAT-MAP TOWN. A REAL worldgen village on FLAT ground: the flatten
    # runs BEFORE civ generation, so houses/doors/roads/fields are placed onto
    # the levelled disc rather than having the ground pulled out from under
    # buildings whose heights are already baked.
    #
    # ★ MEASURED CAVEAT, three runs: the flat disc and the village SEARCH have
    # to agree or the town lands off the flat (with search left at its default
    # 16384 the scorer picked a village 11,229 blocks away). But binding them
    # is not free -- this world has no LARGE village near world centre, so a
    # flat town here is a HAMLET (3 houses, 0 fields). Use `town` to see a real
    # 46-house / 23-field settlement; use this to watch PATHING on legible
    # ground.
    #
    # No marker wait: with one candidate nearby there is nothing to choose.
    @{
        BASTION_FLAT_WORLD_RADIUS      = '64'
        BASTION_ADOPT_RADIUS           = '2000'
        BASTION_ADOPT_TOWN             = '1'
        BASTION_AUTOFOUND_REAL_TERRAIN = '1'
        BASTION_COLONY_PRESENCE_VD     = '3'
        BASTION_AUTOFOUND_COLONY       = '8'
        BASTION_SEED_FOOD              = '64'
        BASTION_SEED_MATERIALS         = '64'
    }
} elseif ($Mode -eq 'town') {
    # ★ JUST BOOT ME INTO A TOWN I OWN (Ben, 2026-08-22, verbatim: "just boot
    # me into a town that i own and have colonists that function").
    #
    # The default is now ZERO CEREMONY: the scorer picks the best settlement it
    # can find, founds on it immediately, and the player spawns IN it. No
    # marker, no chooser, nothing that can be got wrong.
    #
    # Choosing was made the default earlier today and that was the wrong call.
    # Picking a town is a nice-to-have; being dropped into a working colony is
    # the entire point of the mode. A feature that gates the main path behind a
    # step the player has to perform correctly is worse than no feature -- and
    # every failure mode of the chooser (stall, wrong town, silent fallback)
    # lands on the one path everybody uses. `-Pick` keeps it for when it is
    # wanted.
    @{
        BASTION_ADOPT_TOWN             = '1'
        # Put the player down in the colony rather than at the world default,
        # which is what "a town that i own" means when you spawn 11km away
        # from it.
        BASTION_SPAWN_AT_COLONY        = '1'
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

# -NoRaids: suppress the raid tick entirely. BASTION_NO_RAIDS is read by
# Server::bastion_raid_tick's first line, so this is a hard off, not a
# probability tweak.
if ($NoRaids) { $EnvVars['BASTION_NO_RAIDS'] = '1' }

# -Pick: hold founding until the player chooses a town on the SELECT STARTING
# AREA screen (or middle-clicks the in-game map). Opt-in, because the default
# path must not be able to stall waiting for a step.
if ($Pick) { $EnvVars['BASTION_ADOPT_WAIT_FOR_MARKER'] = '1' }

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
