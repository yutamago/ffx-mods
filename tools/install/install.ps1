# SPDX-License-Identifier: MIT
#
# Installs one mod of ffx-mods (the folder this script is in, mods\yuta.<name>):
#  1. adds it and the shared library yuta.ffx-mods to Fahrenheit's mods\loadorder;
#  2. for a mod with PS2 files, builds them from your PS2 disc (an image file, or the disc
#     in a drive) and your PC game into this folder, with demaster.exe (next to this
#     script, or in mods\yuta.ffx-mods as in the PS2 bundle).
# In a bundle's folder (mods\yuta.ps2-bundle: a mod whose manifest depends on other mods of
# the family) and in mods\yuta.ffx-mods it installs every mod of the family in the mods
# folder: all of them into mods\loadorder (the bundles after the mods), then one build of
# all their PS2 files from one pass over the disc (demaster.exe build --all), each with its
# own settings.toml.
# After that the mods need neither the disc nor the image. Running it again is safe: the
# build only redoes what changed (after a change in a settings.toml, run it again).
#
# Double-click install.cmd, or from a terminal:
#   install.cmd                         asks where the disc is
#   install.cmd -Iso 'D:\PS2\FFX.iso'   the disc image
#   install.cmd -Drive E                the disc in drive E:
#   install.cmd -Game 'D:\Steam\...'    the game folder, if it is not found by itself
#   install.cmd -NoDisc                 PS2 lighting only: without the disc
#   install.cmd -Force                  build everything again
# Windows PowerShell 5.1 (part of Windows) runs it.

param(
    [string]$Iso,
    [string]$Drive,
    [string]$Game,
    [switch]$NoDisc,
    [switch]$Force,
    [switch]$NoPause
)

# Not 'Stop': Windows PowerShell turns a native program's stderr into errors, and the
# converter reports progress there. Failures are checked by exit code.
$ErrorActionPreference = 'Continue'
$Common = 'yuta.ffx-mods'

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

# A bundle: a mod whose manifest depends on mods of the family besides the shared library.
# ffx-remix's mods (yuta.ffxremix.*: they use the shared library and its F8 window, and depend
# on their own yuta.ffxremix.core) are not bundles: ffx-remix's launcher installs them, and they
# never count as the mods of a bundle either.
function Bundled([string]$file) {
    if ((Split-Path -Leaf (Split-Path -Parent $file)) -like 'yuta.ffxremix.*') { return @() }
    try { $deps = @((Get-Content -LiteralPath $file -Raw | ConvertFrom-Json).Dependencies) }
    catch { return @() }
    return @($deps | Where-Object { $_ -and $_ -ne $Common -and $_ -notlike 'yuta.ffxremix.*' })
}
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
    foreach ($line in (& $exe mods)) {
        if ($line -match '^(yuta\.[a-z0-9-]+) \(') {
            $id = $Matches[1]
            if (Test-Path -LiteralPath (Join-Path (Join-Path $ModsDir $id) "$id.manifest.json")) { $Targets += $id }
        }
    }
    if ($Targets.Count -eq 0) { Fail "no mod of the family is in $ModsDir. Unpack the whole zip into the Fahrenheit folder." }
    foreach ($b in $Bundles) {
        $missing = @(Bundled (Join-Path (Join-Path $ModsDir $b) "$b.manifest.json") | Where-Object { $Targets -notcontains $_ })
        if ($missing) { Write-Host "$b`: of its mods these have no folder in $ModsDir and stay off: $($missing -join ', ')" -ForegroundColor Yellow }
    }
    $Targets += $Bundles
    Write-Host "Installing $($Targets.Count) mods into $ModsDir`: $($Targets -join ', ')"
}
else {
    $Targets = @($ModId)
    Write-Host "Installing $ModId into $ModsDir"
}

# ---- 1. the load order ----------------------------------------------------------------------
if (-not (Test-Path -LiteralPath (Join-Path (Join-Path $ModsDir $Common) "$Common.dll"))) {
    Fail "the shared library mods\$Common is missing. Unpack the whole zip into the Fahrenheit folder (it holds mods\$Common and mods\$ModId)."
}
$loadorder = Join-Path $ModsDir 'loadorder'
$lines = @()
if (Test-Path -LiteralPath $loadorder) {
    $lines = @(Get-Content -LiteralPath $loadorder | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
}
$before = $lines -join "`n"
# The shared library before the mods that need it, then this mod (if not listed yet).
$out = New-Object System.Collections.Generic.List[string]
$placed = $false
foreach ($l in $lines) {
    if ($l -eq $Common) { continue }
    if (-not $placed -and $l -like 'yuta.*') { $out.Add($Common); $placed = $true }
    $out.Add($l)
}
if (-not $placed) { $out.Add($Common) }
foreach ($t in $Targets) { if ($t -ne $Common -and -not $out.Contains($t)) { $out.Add($t) } }
# A bundle after the mods it lists (it only shows them in F8; Fahrenheit needs no order).
if ($All) {
    foreach ($b in $Bundles) { if ($out.Remove($b)) { $out.Add($b) } }
}
# FFX Demaster's old all-in-one mod writes the same files as these mods: take it out.
if ($All -and $out.Contains('yuta.demaster')) {
    [void]$out.Remove('yuta.demaster')
    Write-Host "mods\loadorder: removed yuta.demaster (the old all-in-one FFX Demaster; these mods replace it). Its folder mods\yuta.demaster can be deleted." -ForegroundColor Yellow
}
$lines = $out.ToArray()
if (($lines -join "`n") -ne $before) {
    [System.IO.File]::WriteAllLines($loadorder, [string[]]$lines)
    Write-Host "mods\loadorder: $($lines -join ', ')"
}
else {
    Write-Host "mods\loadorder already lists $(if ($All) { 'the mods' } else { 'the mod' })."
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
    else { Write-Host "$ModId has no PS2 files to build: it is installed. Its settings: $settings" -ForegroundColor Green }
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
}
else {
    Write-Host "$ModId is installed." -ForegroundColor Green
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Write-Host "After changing $settings, run install.cmd again."
}
Finish 0
