# SPDX-License-Identifier: MIT
#
# Installs one mod of ffx-mods (the folder this script is in, mods\yuta.<name>):
#  1. adds it and the shared library yuta.ffx-mods to Fahrenheit's mods\loadorder;
#  2. makes its settings.toml from settings.default.toml (the defaults, shipped in the zip)
#     when there is none; to a settings.toml from an older version it adds the settings it
#     lacks (with their comments and defaults). It never changes a value in settings.toml,
#     so unpacking a newer zip and running install.cmd keeps your settings;
#  3. for a mod with PS2 files, builds them from your PS2 disc (an image file, or the disc
#     in a drive) and your PC game into this folder, with demaster.exe (next to this
#     script, or in mods\yuta.ffx-mods as in the PS2 bundle).
# In a bundle's folder (mods\yuta.ps2-bundle: a mod whose manifest depends on other mods of
# the family, but not ffx-remix's yuta.ffxremix.*) and in mods\yuta.ffx-mods it installs
# every mod of the family in the mods folder: all of them into mods\loadorder (the bundles
# after the mods), then one build of
# all their PS2 files from one pass over the disc (demaster.exe build --all), each with its
# own settings.toml.
# After that the mods need neither the disc nor the image. Running it again is safe: the
# build only redoes what changed (after a change in a settings.toml, run it again).
#
# Switched-off mods: mods\ffx-mods-off.txt lists the mods install.cmd leaves out of
# mods\loadorder and does not build, even when their folder is there. -Disable adds a mod to
# it (and takes it out of mods\loadorder), -Enable takes it out again (then the install puts
# it back and builds it). A mod whose line was deleted from mods\loadorder by hand goes into
# the list too (install.cmd remembers what it added, in mods\yuta.ffx-mods\installed.txt).
#
# Double-click install.cmd, or from a terminal:
#   install.cmd                         asks where the disc is
#   install.cmd -Iso 'D:\PS2\FFX.iso'   the disc image
#   install.cmd -Drive E                the disc in drive E:
#   install.cmd -Game 'D:\Steam\...'    the game folder, if it is not found by itself
#   install.cmd -NoDisc                 PS2 lighting only: without the disc
#   install.cmd -Force                  build everything again
#   install.cmd -Disable ps2-movies     switch a mod off (several: -Disable ps2-movies,ps2-maps)
#   install.cmd -Enable ps2-movies      switch it on again
# Windows PowerShell 5.1 (part of Windows) runs it.

param(
    [string]$Iso,
    [string]$Drive,
    [string]$Game,
    [switch]$NoDisc,
    [switch]$Force,
    [string]$Disable,
    [string]$Enable,
    [switch]$NoPause
)

# Not 'Stop': Windows PowerShell turns a native program's stderr into errors, and the
# converter reports progress there. Failures are checked by exit code.
$ErrorActionPreference = 'Continue'
$Common = 'yuta.ffx-mods'
# In the mods folder: the switched-off mods (the player's list).
$OffName = 'ffx-mods-off.txt'
# In mods\yuta.ffx-mods: the mods install.cmd put into mods\loadorder.
$RecordName = 'installed.txt'

# ---- the logic without side effects (tools/install/install.tests.ps1 tests it) --------------

# A mod id from what a player types: ps2-movies, yuta.ps2-movies, mods\yuta.ps2-movies\.
function Mod-Id([string]$name) {
    $n = $name.Trim().Trim('"', "'", ' ').TrimEnd('\', '/')
    $n = ($n -split '[\\/]')[-1]
    if (-not $n) { return '' }
    if ($n -notlike 'yuta.*') { $n = "yuta.$n" }
    return $n.ToLowerInvariant()
}

# The mod ids in -Disable/-Enable: separated by commas or spaces.
function Split-Ids([string]$text) {
    if (-not $text) { return @() }
    return @($text -split '[,; ]+' | Where-Object { $_ } | ForEach-Object { Mod-Id $_ } | Where-Object { $_ } | Select-Object -Unique)
}

# The mod ids in a list file (one per line, '#' starts a comment); none when there is no file.
function Read-Ids([string]$file) {
    if (-not (Test-Path -LiteralPath $file)) { return @() }
    $ids = @()
    foreach ($l in [IO.File]::ReadAllLines($file)) {
        $t = ($l -replace '#.*$', '').Trim()
        if ($t) {
            $id = Mod-Id $t
            if ($id -and $ids -notcontains $id) { $ids += $id }
        }
    }
    return $ids
}

# The text of mods\ffx-mods-off.txt.
function Off-Text([string[]]$ids) {
    return @(
        '# The FFX mods that are switched off: install.cmd leaves each mod listed here out of'
        '# mods\loadorder and does not build it, even when its folder is there.'
        '# Switch one off: mods\yuta.<any mod>\install.cmd -Disable <mod> (or add its line here).'
        '# Switch it on again: install.cmd -Enable <mod> (or delete its line here and run install.cmd).'
    ) + @($ids)
}

# The new mods\loadorder from its lines: the shared library before the first mod of the family,
# then the mods of $add that are not listed yet (in $add's order); without the switched-off mods ($off). With $all
# (a bundle's install.cmd), the bundles after the mods, and without yuta.demaster (FFX
# Demaster's old all-in-one mod, which writes the same files).
function New-LoadOrder([string[]]$lines, [string[]]$add, [string[]]$bundles, [string[]]$off, [bool]$all) {
    $out = New-Object System.Collections.Generic.List[string]
    $placed = $false
    foreach ($l in $lines) {
        if ($l -eq $Common -or $off -contains $l) { continue }
        if (-not $placed -and $l -like 'yuta.*') { $out.Add($Common); $placed = $true }
        $out.Add($l)
    }
    if (-not $placed) { $out.Add($Common) }
    # A mod not listed yet goes before the next mod of $add that is (so a mod switched on again
    # gets its place back), else at the end (the bundles go last below).
    for ($i = 0; $i -lt $add.Count; $i++) {
        $t = $add[$i]
        if ($t -eq $Common -or $off -contains $t -or $out.Contains($t)) { continue }
        $at = -1
        for ($j = $i + 1; $j -lt $add.Count -and $at -lt 0; $j++) { if ($bundles -notcontains $add[$j]) { $at = $out.IndexOf($add[$j]) } }
        if ($at -ge 0) { $out.Insert($at, $t) } else { $out.Add($t) }
    }
    if ($all) {
        foreach ($b in $bundles) { if ($off -notcontains $b -and $out.Remove($b)) { $out.Add($b) } }
        [void]$out.Remove('yuta.demaster')
    }
    return $out.ToArray()
}

# Whether install.cmd switches a mod on and off: not the shared library, not yuta.demaster, not
# ffx-remix's yuta.ffxremix.* (its launcher adds and removes them in mods\loadorder).
function Switchable([string]$id) {
    return $id -like 'yuta.*' -and $id -ne $Common -and $id -ne 'yuta.demaster' -and $id -notlike 'yuta.ffxremix.*'
}

# The mods install.cmd put into mods\loadorder that are no longer in it: switched off by hand.
# $exists tells whether a mod's folder (with its manifest) is still there.
function Removed-By-Hand([string[]]$record, [string[]]$lines, [scriptblock]$exists) {
    return @($record | Where-Object { (Switchable $_) -and $lines -notcontains $_ -and (& $exists $_) })
}

# The lines of a settings file and what each is: a table header, a key, a comment, a blank, or
# something this simple reader leaves alone (dotted or quoted keys, inline tables, values over
# several lines, arrays of tables). Our files have only the first four.
function Scan-Toml([string]$text) {
    $lines = @()
    if ($text) {
        $lines = @($text -split "`r?`n")
        if ($lines.Count -gt 0 -and $lines[-1] -eq '') { $lines = @($lines[0..($lines.Count - 2)]) }
    }
    $items = @()
    $table = ''
    foreach ($l in $lines) {
        $kind = 'other'; $key = $null
        if ($l -match '^\s*$') { $kind = 'blank' }
        elseif ($l -match '^\s*#') { $kind = 'comment' }
        elseif ($l -match '^\s*\[\s*([A-Za-z0-9_-]+(\s*\.\s*[A-Za-z0-9_-]+)*)\s*\]\s*(#.*)?$') {
            $kind = 'table'; $table = $Matches[1] -replace '\s', ''
        }
        elseif ($l -match '^\s*([A-Za-z0-9_-]+)\s*=\s*(.*)$') {
            $v = $Matches[2]
            $kind = 'key'
            $key = if ($table) { "$table.$($Matches[1])" } else { $Matches[1] }
            if ($v -match "^('''|""""""|\{|\[)") { $kind = 'other' }
        }
        $items += , @{ Line = $l; Kind = $kind; Table = $table; Key = $key }
    }
    return , $items
}

# The player's settings.toml ($user) with the settings of settings.default.toml ($defaults) it
# lacks: each with the comment lines above it in $defaults, after the last line of its table
# (the table added at the end of the file when the file has none). Every other line stays as it
# is. Returns @{ Text; Added = the keys added; Unsafe = why the file was left as it is, or $null }.
function Merge-Settings([string]$user, [string]$defaults) {
    $u = Scan-Toml $user
    $d = Scan-Toml $defaults
    $result = @{ Text = $user; Added = @(); Unsafe = $null }
    if (@($u | Where-Object { $_.Kind -eq 'other' }).Count -gt 0) {
        $result.Unsafe = 'it has lines this installer does not read (dotted or quoted keys, inline tables, values over several lines)'
        return $result
    }
    if (@($d | Where-Object { $_.Kind -eq 'other' }).Count -gt 0) {
        $result.Unsafe = 'settings.default.toml has lines this installer does not read'
        return $result
    }
    $have = @($u | Where-Object { $_.Kind -eq 'key' } | ForEach-Object { $_.Key })
    # The missing settings by table, in the defaults' order, each with its comments.
    $tables = New-Object System.Collections.Generic.List[string]
    $blocks = @{}
    for ($i = 0; $i -lt $d.Count; $i++) {
        $it = $d[$i]
        if ($it.Kind -ne 'key' -or $have -contains $it.Key) { continue }
        $start = $i
        while ($start -gt 0 -and $d[$start - 1].Kind -eq 'comment') { $start-- }
        if (-not $blocks.ContainsKey($it.Table)) { $blocks[$it.Table] = @(); $tables.Add($it.Table) }
        $blocks[$it.Table] += @($d[$start..$i] | ForEach-Object { $_.Line })
        $result.Added += $it.Key
    }
    if ($result.Added.Count -eq 0) { return $result }

    $nl = if ($user -match "`r`n") { "`r`n" } elseif ($user) { "`n" } elseif ($defaults -match "`r`n") { "`r`n" } else { "`n" }
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($it in $u) { $lines.Add($it.Line) }
    $inserts = @()   # (index, lines) into the file as it is
    $append = @()    # tables the file lacks
    foreach ($t in $tables) {
        if ($t -eq '') {
            # keys outside any table go before the first table header (and its comments)
            $at = $u.Count
            for ($i = 0; $i -lt $u.Count; $i++) { if ($u[$i].Kind -eq 'table') { $at = $i; break } }
            while ($at -gt 0 -and $u[$at - 1].Kind -eq 'comment' -and $at -lt $u.Count) { $at-- }
            while ($at -gt 0 -and $u[$at - 1].Kind -eq 'blank') { $at-- }
            $inserts += , @($at, $blocks[$t])
            continue
        }
        $header = -1
        for ($i = 0; $i -lt $u.Count; $i++) { if ($u[$i].Kind -eq 'table' -and $u[$i].Table -eq $t) { $header = $i; break } }
        if ($header -lt 0) {
            $append += , (@("[$t]") + $blocks[$t])
            continue
        }
        $end = $u.Count
        for ($i = $header + 1; $i -lt $u.Count; $i++) { if ($u[$i].Kind -eq 'table') { $end = $i; break } }
        while ($end -gt $header + 1 -and $u[$end - 1].Kind -eq 'blank') { $end-- }
        $inserts += , @($end, $blocks[$t])
    }
    foreach ($ins in @($inserts | Sort-Object -Property { $_[0] } -Descending)) {
        $lines.InsertRange($ins[0], [string[]]$ins[1])
    }
    foreach ($a in $append) {
        if ($lines.Count -gt 0 -and $lines[$lines.Count - 1].Trim() -ne '') { $lines.Add('') }
        $lines.AddRange([string[]]$a)
    }
    $result.Text = ($lines -join $nl) + $nl
    return $result
}

# Writes text as UTF-8 without a BOM, through a temporary file (a crash leaves the old file).
function Write-Text([string]$file, [string]$text) {
    $tmp = "$file.tmp"
    [IO.File]::WriteAllText($tmp, $text, (New-Object Text.UTF8Encoding $false))
    Move-Item -LiteralPath $tmp -Destination $file -Force
}

# A mod folder's settings.toml: made from settings.default.toml when missing, else the newer
# settings added (Merge-Settings). Returns what it did, for the output; $null when nothing.
function Update-Settings([string]$dir) {
    $defaults = Join-Path $dir 'settings.default.toml'
    $file = Join-Path $dir 'settings.toml'
    if (-not (Test-Path -LiteralPath $defaults)) { return $null }
    if (-not (Test-Path -LiteralPath $file)) {
        Copy-Item -LiteralPath $defaults -Destination $file
        return 'made settings.toml from settings.default.toml'
    }
    $text = [IO.File]::ReadAllText($file)
    $m = Merge-Settings $text ([IO.File]::ReadAllText($defaults))
    if ($m.Unsafe) {
        # Roughly which settings it lacks: those whose name is on no line of it.
        $missing = @((Scan-Toml ([IO.File]::ReadAllText($defaults))) | Where-Object { $_.Kind -eq 'key' } | ForEach-Object { $_.Key } |
            Where-Object { $text -notmatch ('(?m)\b' + [regex]::Escape(($_ -split '\.')[-1]) + '\s*=') })
        if ($missing) { return "settings.toml was left as it is ($($m.Unsafe)); it lacks $($missing -join ', '), which have their defaults (see settings.default.toml)" }
        return $null
    }
    if ($m.Added.Count -eq 0) { return $null }
    Write-Text $file $m.Text
    return "settings.toml: added the new settings $($m.Added -join ', ') (their defaults; your values stay)"
}

if ($env:FFXMODS_INSTALL_FUNCTIONS_ONLY -eq '1') { return }

# ---- the install ----------------------------------------------------------------------------

function Finish([int]$code) {
    if (-not $NoPause) {
        Write-Host ''
        [void](Read-Host 'Press Enter to close')
    }
    exit $code
}

function Fail([string]$message) {
    Write-Host "error: $message" -ForegroundColor Red
    Finish 1
}

$ModDir = $PSScriptRoot
$manifest = Get-ChildItem -LiteralPath $ModDir -Filter '*.manifest.json' -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $manifest) { Fail "there is no mod manifest in $ModDir. Run install.cmd from the mod's folder, mods\yuta.<name>." }
$ModId = $manifest.Name -replace '\.manifest\.json$', ''
$Name = $ModId -replace '^yuta\.', ''
$ModsDir = Split-Path -Parent $ModDir
$OffFile = Join-Path $ModsDir $OffName
$RecordFile = Join-Path (Join-Path $ModsDir $Common) $RecordName

function Has-Mod([string]$id) { return Test-Path -LiteralPath (Join-Path (Join-Path $ModsDir $id) "$id.manifest.json") }

# A bundle: a mod whose manifest depends on mods of the family besides the shared library.
# ffx-remix's mods (yuta.ffxremix.*: they use the shared library and its F8 window, and depend
# on their own yuta.ffxremix.core) are not bundles: ffx-remix's launcher installs them, and they
# never count as the mods of a bundle either.
function Bundled([string]$file) {
    if ((Split-Path -Leaf (Split-Path -Parent $file)) -like 'yuta.ffxremix.*') { return @() }
    try { $deps = @((Get-Content -LiteralPath $file -Raw -ErrorAction Stop | ConvertFrom-Json).Dependencies) }
    catch { return @() }
    return @($deps | Where-Object { $_ -and $_ -ne $Common -and $_ -notlike 'yuta.ffxremix.*' })
}

# ---- 1. switched-off mods -------------------------------------------------------------------
if (-not (Test-Path -LiteralPath (Join-Path (Join-Path $ModsDir $Common) "$Common.dll"))) {
    Fail "the shared library mods\$Common is missing. Unpack the whole zip into the Fahrenheit folder (it holds mods\$Common and mods\$ModId)."
}
$loadorder = Join-Path $ModsDir 'loadorder'
$lines = @()
$hasLoadOrder = Test-Path -LiteralPath $loadorder
if ($hasLoadOrder) {
    $lines = @(Get-Content -LiteralPath $loadorder | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
}
$off = @(Read-Ids $OffFile | Where-Object { Switchable $_ })
$offBefore = $off -join "`n"
$toDisable = @(Split-Ids $Disable)
$toEnable = @(Split-Ids $Enable)
foreach ($id in $toDisable + $toEnable) {
    if ($id -eq $Common) { Fail "$Common is the shared library every mod of the family needs: it is not switched on or off." }
    if (-not (Switchable $id)) { Fail "$id is not switched on or off by install.cmd$(if ($id -like 'yuta.ffxremix.*') { " (ffx-remix's launcher installs it)" })." }
}
foreach ($id in $toDisable) {
    if (-not (Has-Mod $id)) { Fail "-Disable: there is no mod $id in $ModsDir (no folder mods\$id with its manifest)." }
}
foreach ($id in $toEnable) {
    if ($toDisable -contains $id) { Fail "-Disable and -Enable both name $id." }
    if (-not (Has-Mod $id) -and $off -notcontains $id) { Fail "-Enable: there is no mod $id in $ModsDir (no folder mods\$id with its manifest)." }
}
if ($hasLoadOrder) {
    foreach ($id in @(Removed-By-Hand @(Read-Ids $RecordFile) $lines { param($i) Has-Mod $i })) {
        if ($off -contains $id -or $toEnable -contains $id) { continue }
        $off += $id
        Write-Host "$id is no longer in mods\loadorder: it stays off (it is now in mods\$OffName; install.cmd -Enable $($id -replace '^yuta\.', '') switches it on again)." -ForegroundColor Yellow
    }
}
foreach ($id in $toDisable) {
    if ($off -notcontains $id) { $off += $id }
    Write-Host "Switched off: $id (it stays in its folder; install.cmd -Enable $($id -replace '^yuta\.', '') switches it on again)." -ForegroundColor Yellow
}
foreach ($id in $toEnable) {
    if ($off -contains $id) {
        $off = @($off | Where-Object { $_ -ne $id })
        Write-Host "Switched on: $id." -ForegroundColor Green
    }
    else { Write-Host "$id was not switched off." }
}
if (($off -join "`n") -ne $offBefore) {
    if ($off.Count -gt 0) { [IO.File]::WriteAllLines($OffFile, [string[]](Off-Text $off)) }
    elseif (Test-Path -LiteralPath $OffFile) { Remove-Item -LiteralPath $OffFile }
}

# ---- 2. what to install ---------------------------------------------------------------------
# The bundles in the mods folder.
$Bundles = @(Get-ChildItem -LiteralPath $ModsDir -Directory -Filter 'yuta.*' -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -ne 'yuta.demaster' -and (Bundled (Join-Path $_.FullName "$($_.Name).manifest.json")).Count -gt 0 } | ForEach-Object { $_.Name })
# In the shared library's or a bundle's folder: every mod of the family.
$All = $ModId -eq $Common -or $Bundles -contains $ModId
$SharedExe = Join-Path (Join-Path $ModsDir $Common) 'demaster.exe'
$exe = Join-Path $ModDir 'demaster.exe'
if (-not (Test-Path -LiteralPath $exe) -and (Test-Path -LiteralPath $SharedExe)) { $exe = $SharedExe }
$hasExe = Test-Path -LiteralPath $exe
$Targets = @()
if ($All) {
    if (-not $hasExe) { Fail "there is no demaster.exe in $(Split-Path -Parent $SharedExe). Unpack the whole zip into the Fahrenheit folder." }
    # The mods in the converter's order (demaster mods), those that are in the mods folder.
    $present = @()
    foreach ($line in (& $exe mods)) {
        if ($line -match '^(yuta\.[a-z0-9-]+) \(' -and (Has-Mod $Matches[1])) { $present += $Matches[1] }
    }
    if ($present.Count -eq 0) { Fail "no mod of the family is in $ModsDir. Unpack the whole zip into the Fahrenheit folder." }
    foreach ($b in $Bundles) {
        $missing = @(Bundled (Join-Path (Join-Path $ModsDir $b) "$b.manifest.json") | Where-Object { $present -notcontains $_ })
        if ($missing) { Write-Host "$b`: of its mods these have no folder in $ModsDir and stay off: $($missing -join ', ')" -ForegroundColor Yellow }
    }
    $Targets = @($present + $Bundles | Where-Object { $off -notcontains $_ })
    $isOff = @($present + $Bundles | Where-Object { $off -contains $_ })
    if ($isOff) { Write-Host "Switched off (mods\$OffName): $($isOff -join ', '). install.cmd -Enable <mod> switches one on again." -ForegroundColor Yellow }
    if ($Targets.Count -gt 0) { Write-Host "Installing $($Targets.Count) mods into $ModsDir`: $($Targets -join ', ')" }
    else { Write-Host "Every mod is switched off: none to install." -ForegroundColor Yellow }
}
elseif ($off -contains $ModId) {
    Write-Host "$ModId is switched off (mods\$OffName): it is not installed. install.cmd -Enable $Name switches it on again." -ForegroundColor Yellow
}
else {
    $Targets = @($ModId)
    Write-Host "Installing $ModId into $ModsDir"
}
foreach ($id in $toEnable) {
    if ($Targets -notcontains $id -and $off -notcontains $id) {
        Write-Host "To install $id now, run mods\$id\install.cmd." -ForegroundColor Yellow
    }
}

# ---- 3. the load order ----------------------------------------------------------------------
$before = $lines -join "`n"
$new = @(New-LoadOrder $lines $Targets $Bundles $off $All)
if ($All -and $lines -contains 'yuta.demaster') {
    Write-Host "mods\loadorder: removed yuta.demaster (the old all-in-one FFX Demaster; these mods replace it). Its folder mods\yuta.demaster can be deleted." -ForegroundColor Yellow
}
if (($new -join "`n") -ne $before) {
    [System.IO.File]::WriteAllLines($loadorder, [string[]]$new)
    Write-Host "mods\loadorder: $($new -join ', ')"
}
elseif ($Targets.Count -gt 0) {
    Write-Host "mods\loadorder already lists $(if ($All) { 'the mods' } else { 'the mod' })."
}
# What install.cmd put into mods\loadorder, to notice a line deleted by hand next time.
$record = @($new | Where-Object { Switchable $_ })
[IO.File]::WriteAllLines($RecordFile, [string[]](@('# Written by install.cmd: the mods of the family it found in mods\loadorder. A mod listed here that is no longer in mods\loadorder was switched off by hand.') + $record))
if (-not $All -and $Targets.Count -eq 0) { Finish 0 }

# ---- 4. the settings ------------------------------------------------------------------------
foreach ($t in $Targets) {
    $did = Update-Settings (Join-Path $ModsDir $t)
    if ($did) { Write-Host "mods\$t`: $did" }
}
if ($Targets.Count -eq 0) {
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Finish 0
}

$settings = Join-Path $ModDir 'settings.toml'
$codeOnly = -not $hasExe
if ($hasExe -and -not $All) { $codeOnly = [bool](& $exe mods | Where-Object { $_ -like "$ModId (*code only*" }) }
if ($All) {
    $builders = @(& $exe mods | Where-Object { $_ -notlike '*code only*' } | ForEach-Object { if ($_ -match '^(yuta\.[a-z0-9-]+) \(') { $Matches[1] } } | Where-Object { $Targets -contains $_ })
    $codeOnly = $builders.Count -eq 0
}
if ($codeOnly) {
    Write-Host ''
    if ($All) { Write-Host 'None of these mods has PS2 files to build: they are installed.' -ForegroundColor Green }
    elseif (Test-Path -LiteralPath (Join-Path $ModDir 'settings.default.toml')) { Write-Host "$ModId has no PS2 files to build: it is installed. Its settings: $settings" -ForegroundColor Green }
    else { Write-Host "$ModId has no PS2 files to build: it is installed." -ForegroundColor Green }
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Finish 0
}

# ---- 2. the disc ----------------------------------------------------------------------------
# All mods: without the disc, only what needs none is built (PS2 lighting's shaders).
$optional = $All -or [bool](& $exe mods | Where-Object { $_ -like "$ModId (*optional*" })

# A native program's output as one line (Windows PowerShell wraps its stderr lines in error records).
function Text($lines) {
    return (@($lines) | ForEach-Object { "$_".Trim() } | Where-Object { $_ -and $_ -ne 'System.Management.Automation.RemoteException' }) -join ' '
}

function Test-Disc([string]$path) {
    & $exe disc-check $path 2>$null | Out-Null
    return $LASTEXITCODE -eq 0
}

# A value saved under [source] in a settings.toml (iso, game), or $null.
function Saved([string]$file, [string]$key) {
    if (-not (Test-Path -LiteralPath $file)) { return $null }
    $m = [regex]::Match((Get-Content -LiteralPath $file -Raw), "(?m)^\s*$key\s*=\s*(?:'([^']*)'|""([^""]*)"")")
    if (-not $m.Success) { return $null }
    $v = $m.Groups[1].Value + $m.Groups[2].Value
    if ($v.Trim() -eq '') { return $null }
    return $v
}

function Pick-Image {
    try {
        Add-Type -AssemblyName System.Windows.Forms
        $dialog = New-Object System.Windows.Forms.OpenFileDialog
        $dialog.Title = 'Your Final Fantasy X PS2 disc image'
        $dialog.Filter = 'Disc images (*.iso;*.bin;*.img)|*.iso;*.bin;*.img|All files (*.*)|*.*'
        if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { return $dialog.FileName }
        return $null
    }
    catch {
        return (Read-Host 'The path of your disc image').Trim('"', ' ')
    }
}

# This mod's settings.toml first, then the other mods' of the family (FFX Demaster's demaster.toml too).
$family = @($settings) + @(Get-ChildItem -LiteralPath $ModsDir -Directory -Filter 'yuta.*' -ErrorAction SilentlyContinue |
    ForEach-Object { Join-Path $_.FullName 'settings.toml'; Join-Path $_.FullName 'demaster.toml' } | Where-Object { Test-Path -LiteralPath $_ })

$source = $null
if ($Drive) {
    $source = ($Drive.TrimEnd(':', '\', '/')) + ':'
}
elseif ($Iso) {
    $resolved = Resolve-Path -LiteralPath $Iso -ErrorAction SilentlyContinue
    if (-not $resolved) { Fail "the disc image $Iso does not exist" }
    $source = $resolved.ProviderPath
}
elseif (-not $NoDisc) {
    # Choices: drives with the disc in them, an image used before (this mod or another of the family), a file.
    $choices = @()
    try {
        foreach ($d in Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=5') {
            if (Test-Disc $d.DeviceID) { $choices += , @("the disc in drive $($d.DeviceID)", $d.DeviceID) }
        }
    }
    catch { }
    $seen = @{}
    foreach ($f in $family) {
        $v = Saved $f 'iso'
        if ($v -and -not $seen.ContainsKey($v) -and ($v -match '^[A-Za-z]:\\?$' -or (Test-Path -LiteralPath $v))) {
            $seen[$v] = 1
            $choices += , @("$v (used before)", $v)
        }
    }
    $choices += , @('a disc image file (.iso): choose it', '?file')
    if ($optional) { $choices += , @('no disc: build only what needs none', '?none') }
    Write-Host ''
    Write-Host 'Where is your Final Fantasy X PS2 disc?'
    for ($i = 0; $i -lt $choices.Count; $i++) { Write-Host ("  [{0}] {1}" -f ($i + 1), $choices[$i][0]) }
    if (-not ($choices | Where-Object { $_[1] -match '^[A-Za-z]:$' })) {
        Write-Host '  (a disc drive is listed when the PS2 disc is in it)'
    }
    $answer = Read-Host "Choose 1-$($choices.Count)"
    $n = 0
    if (-not [int]::TryParse($answer, [ref]$n) -or $n -lt 1 -or $n -gt $choices.Count) { Fail "no choice made" }
    $pick = $choices[$n - 1][1]
    if ($pick -eq '?file') {
        $source = Pick-Image
        if (-not $source) { Fail 'no disc image chosen' }
    }
    elseif ($pick -eq '?none') { $NoDisc = $true }
    else { $source = $pick }
}

if ($NoDisc -and -not $optional) { Fail "$ModId needs the PS2 disc." }
if ($NoDisc -and $All) { Write-Host 'No disc: the mods that need it are not built (run install.cmd again with the disc).' -ForegroundColor Yellow }
if ($source) {
    $check = & $exe disc-check $source 2>&1
    if ($LASTEXITCODE -ne 0) { Fail "$source can't be used: $(Text $check)" }
    Write-Host "Disc: $source"
}

# ---- 3. the PC game -------------------------------------------------------------------------
$gameArgs = @()
if ($Game) { $gameArgs = @('--game', $Game) }
$found = & $exe find-game --config $settings @gameArgs 2>&1
if ($LASTEXITCODE -ne 0 -and -not $Game) {
    # a game folder another mod of the family was installed with
    foreach ($f in $family) {
        $v = Saved $f 'game'
        if (-not $v) { continue }
        $try = & $exe find-game --config $settings --game $v 2>&1
        if ($LASTEXITCODE -eq 0) { $found = $try; $gameArgs = @('--game', $v); break }
    }
}
if ($LASTEXITCODE -ne 0) {
    Write-Host ''
    Write-Host "The game was not found ($(Text $found))."
    $folder = $null
    try {
        Add-Type -AssemblyName System.Windows.Forms
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
        $dialog.Description = 'The FINAL FANTASY X/X-2 HD Remaster folder (the one with FFX.exe)'
        if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { $folder = $dialog.SelectedPath }
    }
    catch {
        $folder = (Read-Host 'The game folder (the one with FFX.exe)').Trim('"', ' ')
    }
    if (-not $folder) { Fail 'no game folder chosen' }
    $gameArgs = @('--game', $folder)
    $found = & $exe find-game --config $settings @gameArgs 2>&1
    if ($LASTEXITCODE -ne 0) { Fail (Text $found) }
}
Write-Host "Game: $(Text $found)"

# ---- 4. the build ---------------------------------------------------------------------------
if ($All) { $buildArgs = @('build', '--all', '--mods-dir', $ModsDir) + $gameArgs }
else { $buildArgs = @('build', '--mod', $Name, '--mod-dir', $ModDir, '--config', $settings, '--save-source') + $gameArgs }
if ($source) { $buildArgs += @('--iso', $source) }
if ($NoDisc) { $buildArgs += '--no-disc' }
if ($Force) { $buildArgs += '--force' }
Write-Host ''
& $exe @buildArgs
if ($LASTEXITCODE -ne 0) { Fail "the build did not finish (see above). The game shows the HD version of what is missing; run install.cmd again to retry." }
Write-Host ''
if ($All) {
    Write-Host "Installed: $($Targets -join ', ')." -ForegroundColor Green
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Write-Host "Each mod has its settings in mods\yuta.<mod>\settings.toml. After changing one, run $(Join-Path $ModDir 'install.cmd') again (or that mod's own install.cmd)."
    Write-Host "To switch a mod off: $(Join-Path $ModDir 'install.cmd') -Disable <mod> (on again: -Enable <mod>)."
}
else {
    Write-Host "$ModId is installed." -ForegroundColor Green
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Write-Host "After changing $settings, run install.cmd again."
}
Finish 0
