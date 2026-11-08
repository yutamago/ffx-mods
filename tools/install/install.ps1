# SPDX-License-Identifier: MIT
#
# Installs one mod of ffx-mods (the folder this script is in, mods\yuta.<name>):
#  1. adds it and the shared library yuta.ffx-mods to Fahrenheit's mods\loadorder;
#  2. for a mod with PS2 files (demaster.exe next to this script), builds them from your
#     PS2 disc (an image file, or the disc in a drive) and your PC game into this folder.
# After that the mod needs neither the disc nor the image. Running it again is safe: the
# build only redoes what changed (after a change in settings.toml, run it again).
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
Write-Host "Installing $ModId into $ModsDir"

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
if (-not $out.Contains($ModId)) { $out.Add($ModId) }
$lines = $out.ToArray()
if (($lines -join "`n") -ne $before) {
    [System.IO.File]::WriteAllLines($loadorder, [string[]]$lines)
    Write-Host "mods\loadorder: $($lines -join ', ')"
}
else {
    Write-Host 'mods\loadorder already lists the mod.'
}

$exe = Join-Path $ModDir 'demaster.exe'
$settings = Join-Path $ModDir 'settings.toml'
if (-not (Test-Path -LiteralPath $exe)) {
    Write-Host ''
    Write-Host "$ModId has no PS2 files to build: it is installed. Its settings: $settings" -ForegroundColor Green
    Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
    Finish 0
}

# ---- 2. the disc ----------------------------------------------------------------------------
$optional = [bool](& $exe mods | Where-Object { $_ -like "$ModId (*optional*" })

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
if ($source) {
    $check = & $exe disc-check $source 2>&1
    if ($LASTEXITCODE -ne 0) { Fail "$source is not a Final Fantasy X PS2 disc: $check" }
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
    Write-Host "The game was not found ($found)."
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
    if ($LASTEXITCODE -ne 0) { Fail "$found" }
}
Write-Host "Game: $found"

# ---- 4. the build ---------------------------------------------------------------------------
$buildArgs = @('build', '--mod', $Name, '--mod-dir', $ModDir, '--config', $settings, '--save-source') + $gameArgs
if ($source) { $buildArgs += @('--iso', $source) }
if ($NoDisc) { $buildArgs += '--no-disc' }
if ($Force) { $buildArgs += '--force' }
Write-Host ''
& $exe @buildArgs
if ($LASTEXITCODE -ne 0) { Fail "the build did not finish (see above). The game shows the HD version of what is missing; run install.cmd again to retry." }
Write-Host ''
Write-Host "$ModId is installed." -ForegroundColor Green
Write-Host 'Start the game through Fahrenheit; F8 in the game shows the mods and their state.'
Write-Host "After changing $settings, run install.cmd again."
Finish 0
