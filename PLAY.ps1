# PLAY.ps1 — launch a bastion world, for Ben. (PowerShell 5.1)
#
#   .\PLAY.ps1 town          adopt a real worldgen village: the SELECT STARTING
#                            AREA screen at character creation is the chooser —
#                            the town you click is the town you get, and its
#                            VILLAGERS become your colonists (Ben's expected
#                            flow, made the default 2026-08-23)
#   .\PLAY.ps1 town -Boot    the old zero-ceremony path: the scorer picks a
#                            settlement, founds instantly, you spawn in it
#   .\PLAY.ps1 flattown      THE FLAT MAP TOWN (2026-08-23): the WHOLE world is
#                            one flat plain (BASTION_FLAT_WORLD) with real
#                            worldgen towns on it — the lab world where terrain
#                            can never be the explanation. Small fast map
#                            (256x256 chunks, ~30s gen). Pick your town on the
#                            SELECT STARTING AREA screen, same as town.
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
    [Parameter(Position = 0)][ValidateSet('town', 'flattown', 'megatown', 'arena')][string]$Mode = 'town',
    [switch]$Client,
    [switch]$Stop,
    # Ben, 2026-08-22: "get the town working like real life then introduce
    # raiders and see what breaks." Raiders ON by default so the game is the
    # game; -NoRaids gives the clean colony the sweeps were measured on.
    [switch]$NoRaids,
    # ★ THE SELECT STARTING AREA SCREEN IS THE CHOOSER, BY DEFAULT (Ben,
    # 2026-08-23: "i'm expecting to use the select town screen in the
    # character creation is the way i adopt a town and the npcs in the
    # existing colony become my colonists"). -Pick is kept as a no-op alias
    # so muscle memory keeps working; -Boot restores the zero-ceremony
    # autofound ("just boot me into a town that i own") for tests that must
    # not depend on a screen interaction.
    [switch]$Pick,
    [switch]$Boot,
    # A second, side-by-side world for a headless test arm: its own userdata
    # (the default is Ben's, which is DELETED on every boot) and its own port.
    [string]$UserData,
    [int]$Port = 14004,
    # Headless arm: nobody will choose a town on the character screen, so let
    # the autofound adopt one itself. Proven 2026-09-01 19:23 on the fixed
    # flattown: chosen_houses=58 flat=true alt_range=0.0, wanted=48 -> 48 settled
    # (the village had no villagers of its own), target_pop=114 (sum of beds),
    # one settler sent for by tick 2,000. That is ONE MINUTE of run.
    [switch]$NoWait
)

$ErrorActionPreference = 'Stop'
$WT   = 'E:\veloren-master\.item29-wt'
$Bin  = Join-Path $WT 'target\no_overflow'
$UD   = if ($UserData) { $UserData } else { Join-Path $WT 'userdata-play-ben' }

# ★ THE LOCKED-TARGET FALLBACK (2026-08-24). Twice now a running game held
# file locks on target\no_overflow's exes, so a rebuild compiled everything
# and failed only at the final hardlink — the fresh binaries exist in
# deps\, get harvested to lab-bin\, and target stays STALE. Pre-fix, the
# only way to hand Ben the new build was a manual relink after he exited,
# which stranded a validated build behind one human step. So: if lab-bin
# holds a CERTIFIED pair (PAIR-OK marker, written only by the harvest
# process after content-verifying both halves against the same source
# lineage) and its server is NEWER than target's, launch from lab-bin.
# The marker replaces the mtime skew gate for this path on purpose: a
# harvested pair can be hours apart in mtime yet byte-compatible (a
# server-only fix leaves voxygen + common untouched); mtime skew is the
# wrong comparability instrument for a certified pair, and the right one
# (same common/ lineage) is exactly what the marker attests.
$LabBin = Join-Path $WT 'lab-bin'
$labMarker = Join-Path $LabBin 'PAIR-OK'
if ((Test-Path "$LabBin\veloren-server-cli.exe") -and
    (Test-Path "$LabBin\veloren-voxygen.exe") -and
    (Test-Path $labMarker)) {
    $tgtSrv = Get-Item "$Bin\veloren-server-cli.exe" -ErrorAction SilentlyContinue
    $labSrv = Get-Item "$LabBin\veloren-server-cli.exe"
    if (($null -eq $tgtSrv) -or ($labSrv.LastWriteTime -gt $tgtSrv.LastWriteTime)) {
        Write-Host ''
        Write-Host 'USING lab-bin PAIR - target\no_overflow is stale (a running game held'
        Write-Host 'its exes during the last rebuild). This pair was harvested from the'
        Write-Host 'same build and content-verified:'
        Get-Content $labMarker | ForEach-Object { Write-Host ("  " + $_) }
        Write-Host ''
        $Bin = $LabBin
        $script:SkipSkewGate = $true
    }
}

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
if ($skewMin -gt 3 -and -not $script:SkipSkewGate) {
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

# The four worlds.
$EnvVars = if ($Mode -eq 'megatown') {
    # ★ THE MEGA TOWN (Ben, 2026-08-24: "a big colony nearly city sized").
    # Same flat-lab worldgen as flattown, with the town-size roll pinned to
    # the generator's maximum: ~200 plot attempts weighted 64/134 toward
    # houses gives a real city — 60-100 houses, taverns, workshops, guard
    # towers, fields, an airship dock. Population follows housing (the
    # standing rule), so the colony starts at 48 and the city's houses
    # absorb them. Fresh userdata recommended for the first boot: the size
    # pin only affects WORLD GENERATION, an existing world keeps its size.
    @{
        BASTION_FLAT_WORLD             = '1'
        BASTION_TOWN_SIZE              = '1.0'
        BASTION_ADOPT_TOWN             = '1'
        BASTION_ADOPT_WAIT_FOR_MARKER  = '1'
        BASTION_AUTOFOUND_REAL_TERRAIN = '1'
        # VD 7, not the village's 3 (measured, city-pass soak: at VD=3 only
        # the 30 core houses ever streamed in, the outer 55 NEVER registered
        # — nobody lived out there so it never loaded so nobody could live
        # there. A city needs its whole footprint resident.
        BASTION_COLONY_PRESENCE_VD     = '7'
        # POP LADDER (Ben): keep the city footprint, limit the population,
        # and work the count UP rung by rung to find where it breaks.
        # Rung 1 = 24. Raise only after a rung looks clean in a flyover.
        BASTION_AUTOFOUND_COLONY       = '24'
        BASTION_SEED_FOOD              = '256'
        BASTION_SEED_MATERIALS         = '256'
        # ★ KINEMATIC MOVER: ON (Ben's ruling, 2026-08-25: "DO NOT bench
        # it — this is the right approach; physics movement is broken
        # too"). Iterating LIVE with his sessions: the dt bug (fast-
        # forward mass freeze) and the walking-in-place ghost are fixed;
        # unloaded-chunk tolerance under a moving camera is the open face.
        BASTION_KINEMATIC_MOVER        = '1'
    }
} elseif ($Mode -eq 'flattown') {
    # ★ THE FLAT MAP TOWN, v2 (2026-08-23). The old flat DISC
    # (BASTION_FLAT_WORLD_RADIUS) flattened a patch of a mountainous world and
    # could only ever catch whatever hamlet stood near centre. This is the
    # WHOLE-WORLD flatten: every chunk levelled to one plain BEFORE civs run,
    # forest bands laid for building material, the civ roll pinned to the one
    # town kind that can exist here — so REAL multi-house towns generate ON
    # the plain (first gen: 3 towns, the adopted one 18 houses, alt_range=0).
    # The map is small (256×256 chunks via map_file below) so worldgen takes
    # ~30 seconds, not minutes. Chooser flow, same as town: pick your
    # settlement on the SELECT STARTING AREA screen.
    @{
        BASTION_FLAT_WORLD             = '1'
        BASTION_ADOPT_TOWN             = '1'
        BASTION_ADOPT_WAIT_FOR_MARKER  = '1'
        BASTION_AUTOFOUND_REAL_TERRAIN = '1'
        BASTION_COLONY_PRESENCE_VD     = '7'
        BASTION_AUTOFOUND_COLONY       = '48'   # was 8: a pre-cap default; the beds cap (houses*2) now decides, so give it room
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
    # ★ DEFAULT FLIPPED BACK TO CHOOSING (Ben, 2026-08-23, explicit): "i'm
    # expecting to use the select town screen in the character creation is
    # the way i adopt a town and the npcs in the existing colony become my
    # colonists." The 2026-08-22 zero-ceremony flip answered a different ask
    # ("just boot me in") and survives as -Boot. In the chooser flow,
    # BASTION_SPAWN_AT_COLONY must NOT be set: it suppresses the start-site
    # waypoint (the thing that puts you AT the town you clicked) in favour of
    # a colony spawn that does not exist yet at character-creation time —
    # you would land at the world default, 11km from your own pick.
    if ($Boot) {
        @{
            BASTION_ADOPT_TOWN             = '1'
            BASTION_SPAWN_AT_COLONY        = '1'
            BASTION_AUTOFOUND_REAL_TERRAIN = '1'
            BASTION_COLONY_PRESENCE_VD     = '3'
            BASTION_AUTOFOUND_COLONY       = '8'
            BASTION_SEED_FOOD              = '64'
            BASTION_SEED_MATERIALS         = '64'
        }
    } else {
        @{
            BASTION_ADOPT_TOWN             = '1'
            # Hold founding until the SELECT STARTING AREA pick arrives; the
            # picked town becomes the adopt target and its residents the
            # colony (character_screen.rs writes start_site_adopt_target).
            BASTION_ADOPT_WAIT_FOR_MARKER  = '1'
            BASTION_AUTOFOUND_REAL_TERRAIN = '1'
            BASTION_COLONY_PRESENCE_VD     = '3'
            BASTION_AUTOFOUND_COLONY       = '8'
            BASTION_SEED_FOOD              = '64'
            BASTION_SEED_MATERIALS         = '64'
        }
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
# -Pick is the DEFAULT now (see the town block); kept as an accepted switch so
# an old command line still does exactly what it always did.
if ($Pick) { $EnvVars['BASTION_ADOPT_WAIT_FOR_MARKER'] = '1' }

# Fresh userdata, so a test is never confused by a previous colony.
if (Test-Path $UD) { Remove-Item $UD -Recurse -Force }

$env:VELOREN_USERDATA = $UD
$env:VELOREN_ASSETS   = Join-Path $WT 'assets'
foreach ($k in $EnvVars.Keys) { Set-Item -Path "Env:$k" -Value $EnvVars[$k] }
# The server tests PRESENCE of this variable (var_os(..).is_some()), so '0' still
# waits: -NoWait must REMOVE it.
if ($NoWait) { Remove-Item -Path 'Env:BASTION_ADOPT_WAIT_FOR_MARKER' -ErrorAction SilentlyContinue }

# Grant admin to the account you log in as. This one CAN be quiet: it exits.
& "$Bin\veloren-server-cli.exe" --no-auth admin add player admin 2>&1 | Out-Null

# -Port: splice the game port into the settings the admin-add just created,
# and refuse to continue silently if the splice missed.
if ($Port -ne 14004) {
    $SP = Join-Path $UD 'server\server_config\settings.ron'
    $ptxt = (Get-Content $SP -Raw) -replace ':14004"', ":$Port`""
    [System.IO.File]::WriteAllText($SP, $ptxt, (New-Object System.Text.UTF8Encoding $false))
    if (-not (Select-String -Path $SP -Pattern ":$Port`"" -Quiet)) {
        Write-Host "port splice FAILED in $SP - not booting on the wrong port"; exit 1
    }
}

# flattown: a small fast map — the plain is the point, not the continent. The
# admin-add above is what created settings.ron; splice the generator in, and
# REFUSE to continue silently if the splice missed (an unsplice world would
# quietly generate 20 minutes of full-size erosion instead of 30 seconds).
if ($Mode -eq 'flattown') {
    $S = Join-Path $UD 'server\server_config\settings.ron'
    $txt = (Get-Content $S -Raw) -replace 'map_file: None,',
        'map_file: Some(Generate((x_lg: 8, y_lg: 8, scale: 2.0, map_kind: Square, erosion_quality: 0.25))),'
    # BOM-free: PS 5.1's `Set-Content -Encoding utf8` writes a BOM, RON refuses
    # it, and the server silently falls back to DEFAULT settings.
    [System.IO.File]::WriteAllText($S, $txt, (New-Object System.Text.UTF8Encoding $false))
    if (-not (Select-String -Path $S -Pattern 'map_file: Some' -Quiet)) {
        Write-Host "flattown map_file splice FAILED in $S - not booting a wrong-size world"
        return
    }
}

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
$Log = if ($UserData) { Join-Path $UD 'play-server.log' } else { Join-Path $WT 'play-server.log' }
if (Test-Path $Log) { Remove-Item $Log -Force }
Start-Process -FilePath "$Bin\veloren-server-cli.exe" -ArgumentList '--no-auth' `
    -WorkingDirectory $WT -RedirectStandardOutput $Log `
    -RedirectStandardError (Join-Path (Split-Path $Log) 'play-server.err.log') | Out-Null

# ★ WAIT ON THE PORT, not on a log line. The log is in the server's own window
# now, and "is it accepting connections" is exactly what the port answers.
$ready = $false
for ($i = 1; $i -le 120; $i++) {
    Start-Sleep -Seconds 5
    if (Test-PortHeld) { Write-Host "READY after $($i * 5)s"; $ready = $true; break }
}

# ★ THE SERVER, NOT THE FILE, SAYS WHETHER SETTINGS APPLIED. A BOM'd settings.ron
# parses as nothing and the server runs DEFAULTS (default map, default port)
# while the file still reads correctly — every "flattown" before 2026-09-01 was
# the default continent for exactly this reason. Fail loudly.
if (Test-Path $Log) {
    if (Select-String -Path $Log -Pattern 'Failed to parse setting file' -Quiet) {
        Write-Host ''
        Write-Host '  SETTINGS NOT PARSED - the server fell back to DEFAULT settings (default map, default port).'
        Write-Host "  See $Log. This is NOT the world you asked for; stop it with  .\PLAY.ps1 -Stop"
        $ready = $false
    }
}

Write-Host ''
if ($ready) {
    Write-Host "  JOIN AT:  localhost:$Port"
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
