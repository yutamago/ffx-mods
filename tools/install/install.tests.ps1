# SPDX-License-Identifier: MIT
#
# Tests of install.ps1, without Pester (Windows PowerShell 5.1, which runs the installer for
# players, and PowerShell 7):
#  - its functions: the settings merge, the switched-off list, the load order;
#  - the merge against every mods/<name>/settings.default.toml;
#  - the installer itself in a made-up Fahrenheit folder (code-only mods, no demaster.exe):
#    a fresh install, an update over it, -Disable / -Enable, a line deleted by hand;
#  - with -BundleZip <yuta.ps2-bundle-*.zip> (CI), the bundle's installer as a player runs it,
#    with every mod that builds PS2 files switched off (so it needs no disc and no game).
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\install\install.tests.ps1 [-BundleZip <zip>]
#   pwsh tools/install/install.tests.ps1

param([string]$BundleZip)

$here = $PSScriptRoot
$repo = Split-Path -Parent (Split-Path -Parent $here)
$installer = Join-Path $here 'install.ps1'
$env:FFXMODS_INSTALL_FUNCTIONS_ONLY = '1'
. $installer
$env:FFXMODS_INSTALL_FUNCTIONS_ONLY = $null
$ErrorActionPreference = 'Stop'

$script:failed = 0
$script:passed = 0
function Check([bool]$ok, [string]$what) {
    if ($ok) { $script:passed++ }
    else { $script:failed++; Write-Host "FAIL: $what" -ForegroundColor Red }
}
function Same($a, $b, [string]$what) {
    $x = @($a) -join '|'; $y = @($b) -join '|'
    Check ($x -ceq $y) "$what`n  got:  $x`n  want: $y"
}
function Keys([string]$text) { return @((Scan-Toml $text) | Where-Object { $_.Kind -eq 'key' } | ForEach-Object { $_.Key }) }
# key -> the value as written
function Values([string]$text) {
    $v = @{}
    foreach ($it in (Scan-Toml $text)) { if ($it.Kind -eq 'key') { $v[$it.Key] = ($it.Line -replace '^[^=]*=\s*', '').Trim() } }
    return $v
}

# ---- ids, lists, load order -----------------------------------------------------------------
Same (Mod-Id 'ps2-movies') 'yuta.ps2-movies' 'Mod-Id name'
Same (Mod-Id 'YUTA.PS2-Movies') 'yuta.ps2-movies' 'Mod-Id case'
Same (Mod-Id 'mods\yuta.ps2-maps\') 'yuta.ps2-maps' 'Mod-Id folder'
Same (Split-Ids 'ps2-movies, yuta.ps2-maps;ps2-movies') @('yuta.ps2-movies', 'yuta.ps2-maps') 'Split-Ids'
Same (Split-Ids '') @() 'Split-Ids empty'

$tmp = Join-Path ([IO.Path]::GetTempPath()) "ffxmods-install-tests-$PID"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
[void](New-Item -ItemType Directory $tmp)
$list = Join-Path $tmp 'list.txt'
[IO.File]::WriteAllLines($list, [string[]](Off-Text @('yuta.ps2-movies', 'yuta.ps2-maps')))
Same (Read-Ids $list) @('yuta.ps2-movies', 'yuta.ps2-maps') 'Read-Ids reads Off-Text'
[IO.File]::WriteAllText($list, "# comment`r`n  ps2-movies  # trailing`r`n`r`nyuta.ps2-movies`r`n")
Same (Read-Ids $list) @('yuta.ps2-movies') 'Read-Ids comments, blanks, duplicates'
Same (Read-Ids (Join-Path $tmp 'none.txt')) @() 'Read-Ids without file'

Same (New-LoadOrder @() @('yuta.ps2-lighting') @() @() $false) @('yuta.ffx-mods', 'yuta.ps2-lighting') 'loadorder fresh'
Same (New-LoadOrder @('other', 'yuta.ps2-lighting', 'yuta.ffx-mods') @('yuta.ps2-lighting', 'yuta.bug-fixes') @() @() $false) `
    @('other', 'yuta.ffx-mods', 'yuta.ps2-lighting', 'yuta.bug-fixes') 'loadorder: shared library first, new mod added'
Same (New-LoadOrder @('yuta.ffx-mods', 'yuta.ps2-bundle', 'yuta.ps2-movies', 'yuta.demaster') @('yuta.ps2-movies', 'yuta.ps2-maps', 'yuta.ps2-bundle') @('yuta.ps2-bundle') @() $true) `
    @('yuta.ffx-mods', 'yuta.ps2-movies', 'yuta.ps2-maps', 'yuta.ps2-bundle') 'loadorder: bundle last, yuta.demaster out'
Same (New-LoadOrder @('yuta.ffx-mods', 'yuta.ps2-movies', 'yuta.ps2-maps', 'yuta.ps2-bundle') @('yuta.ps2-maps', 'yuta.ps2-bundle') @('yuta.ps2-bundle') @('yuta.ps2-movies') $true) `
    @('yuta.ffx-mods', 'yuta.ps2-maps', 'yuta.ps2-bundle') 'loadorder: a switched-off mod is taken out'
Same (New-LoadOrder @('yuta.ffx-mods', 'yuta.ps2-maps') @('yuta.ps2-maps', 'yuta.ps2-bundle') @('yuta.ps2-bundle') @('yuta.ps2-bundle') $true) `
    @('yuta.ffx-mods', 'yuta.ps2-maps') 'loadorder: a switched-off bundle is not added'

Same (Removed-By-Hand @('yuta.ps2-movies', 'yuta.ps2-maps', 'yuta.gone') @('yuta.ffx-mods', 'yuta.ps2-maps') { param($i) $i -ne 'yuta.gone' }) @('yuta.ps2-movies') 'Removed-By-Hand'

# ---- the settings merge ---------------------------------------------------------------------
$defaults = @'
# Title.
#
# A note.

[source]
# the disc
iso = ''
game = ''

[ui]
# Menus.
menus = "ps2"
# Font,
# two lines.
font = "ps2"
# New: the boot logo.
boot_logo = "hd"

[extra]
# A new table.
thing = true
'@ -replace "`r`n", "`n"

$m = Merge-Settings $defaults $defaults
Check ($m.Added.Count -eq 0 -and $m.Text -ceq $defaults -and -not $m.Unsafe) 'merge: the defaults over themselves change nothing'

$old = @'
# Title.

[source]
iso = 'D:\PS2\FFX.iso'
game = ''

[ui]
# Menus.
menus = "hd"

'@ -replace "`r`n", "`n"
$m = Merge-Settings $old $defaults
Same $m.Added @('ui.font', 'ui.boot_logo', 'extra.thing') 'merge: added keys'
$want = @'
# Title.

[source]
iso = 'D:\PS2\FFX.iso'
game = ''

[ui]
# Menus.
menus = "hd"
# Font,
# two lines.
font = "ps2"
# New: the boot logo.
boot_logo = "hd"

[extra]
# A new table.
thing = true
'@ -replace "`r`n", "`n"
Check ($m.Text -ceq ($want + "`n")) "merge: text`n$($m.Text)"
$v = Values $m.Text
Check ($v['ui.menus'] -eq '"hd"' -and $v['source.iso'] -eq "'D:\PS2\FFX.iso'") 'merge: the player''s values stay'
Same (Merge-Settings $m.Text $defaults).Added @() 'merge: a second run adds nothing'

# CRLF stays CRLF; a table in the middle
$crlf = "[source]`r`niso = ''`r`ngame = ''`r`n`r`n[ui]`r`nfont = ""hd""`r`n`r`n[extra]`r`nthing = false`r`n"
$m = Merge-Settings $crlf $defaults
Same $m.Added @('ui.menus', 'ui.boot_logo') 'merge CRLF: added'
Check ($m.Text -notmatch "[^`r]`n") 'merge CRLF: every line ends in CRLF'
Check ($m.Text -match "font = ""hd""`r`n# Menus.`r`nmenus = ""ps2""`r`n# New: the boot logo.`r`nboot_logo = ""hd""`r`n`r`n\[extra\]") "merge CRLF: at the end of [ui]`n$($m.Text)"
Check ((Values $m.Text)['extra.thing'] -eq 'false') 'merge CRLF: extra.thing stays false'

# a key outside any table goes before the first table
$rootDefaults = "# Head.`n`n# Movies.`nmovies = ""ps2""`n`n[source]`niso = ''`n"
$m = Merge-Settings "# Head.`n`n[source]`niso = 'E:'`n" $rootDefaults
Check ($m.Text -ceq "# Head.`n# Movies.`nmovies = ""ps2""`n`n[source]`niso = 'E:'`n") "merge: root key before the first table`n$($m.Text)"
$m = Merge-Settings "" $rootDefaults
Same (Keys $m.Text) @('movies', 'source.iso') 'merge: an empty file gets every setting'

# lines this reader does not know: the file is left alone
foreach ($bad in @("ui.font = ""hd""`n", "[ui]`nfont = { a = 1 }`n", "[ui]`nfont = '''`nx'''`n", "[[ui]]`n", "[ui]`n""font"" = ""hd""`n")) {
    $m = Merge-Settings $bad $defaults
    Check ($m.Unsafe -and $m.Text -ceq $bad -and $m.Added.Count -eq 0) "merge leaves alone: $bad"
}

# every mod's settings.default.toml: any one setting missing comes back, the rest stays
foreach ($f in Get-ChildItem (Join-Path $repo 'mods') -Recurse -Filter 'settings.default.toml') {
    $text = [IO.File]::ReadAllText($f.FullName)
    $all = Keys $text
    Check ($all.Count -gt 0) "$($f.FullName): keys"
    Check (-not (Merge-Settings $text $text).Unsafe) "$($f.FullName): the reader knows every line"
    $items = Scan-Toml $text
    foreach ($k in $all) {
        $lines = @($items | Where-Object { -not ($_.Kind -eq 'key' -and $_.Key -eq $k) } | ForEach-Object { $_.Line })
        $user = ($lines -join "`n") + "`n"
        $m = Merge-Settings $user $text
        Same $m.Added @($k) "$($f.Directory.Name): $k added"
        Same ((Keys $m.Text) | Sort-Object) ($all | Sort-Object) "$($f.Directory.Name): keys after adding $k"
    }
}

# ---- the installer in a made-up Fahrenheit folder -------------------------------------------
$ps = (Get-Process -Id $PID).Path
function Run-Install([string]$modDir, [string[]]$extra) {
    # Windows PowerShell turns a native program's stderr into errors: not 'Stop' here.
    $ErrorActionPreference = 'Continue'
    $out = & $ps -NoProfile -ExecutionPolicy Bypass -File (Join-Path $modDir 'install.ps1') -NoPause @extra 2>&1 | Out-String
    return @{ Code = $LASTEXITCODE; Out = $out }
}
function LoadOrder([string]$mods) { return @(Get-Content (Join-Path $mods 'loadorder') | Where-Object { $_ }) }
function Add-Mod([string]$mods, [string]$name, [string]$defaultsText) {
    $d = Join-Path $mods "yuta.$name"
    [void](New-Item -ItemType Directory -Force $d)
    Set-Content -LiteralPath (Join-Path $d "yuta.$name.manifest.json") -Value ('{"Id":"yuta.' + $name + '","Dependencies":["yuta.ffx-mods"]}')
    Copy-Item $installer (Join-Path $d 'install.ps1')
    if ($defaultsText) { [IO.File]::WriteAllText((Join-Path $d 'settings.default.toml'), $defaultsText) }
    return $d
}

$fh = Join-Path $tmp 'fh'
$mods = Join-Path $fh 'mods'
[void](New-Item -ItemType Directory -Force (Join-Path $mods 'yuta.ffx-mods'))
Set-Content -LiteralPath (Join-Path $mods 'yuta.ffx-mods/yuta.ffx-mods.dll') -Value ''
$bugDefaults = [IO.File]::ReadAllText((Join-Path $repo 'mods/bug-fixes/settings.default.toml'))
$feelDefaults = [IO.File]::ReadAllText((Join-Path $repo 'mods/ps2-game-feel/settings.default.toml'))
$bug = Add-Mod $mods 'bug-fixes' $bugDefaults
$feel = Add-Mod $mods 'ps2-game-feel' $feelDefaults
Set-Content -LiteralPath (Join-Path $mods 'loadorder') -Value 'someone.else'

# (a) fresh install: settings.toml from the defaults
$r = Run-Install $bug @()
Check ($r.Code -eq 0) "fresh install exits 0`n$($r.Out)"
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.bug-fixes') 'fresh install: loadorder'
Check ([IO.File]::ReadAllText((Join-Path $bug 'settings.toml')) -ceq $bugDefaults) 'fresh install: settings.toml is the defaults'
$r = Run-Install $feel @()
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.bug-fixes', 'yuta.ps2-game-feel') 'second mod: loadorder'

# (b) a changed setting survives a newer version (which brings a new setting)
$settingsFile = Join-Path $bug 'settings.toml'
[IO.File]::WriteAllText($settingsFile, ([IO.File]::ReadAllText($settingsFile) -replace 'lulu_fury = true', 'lulu_fury = false'))
$newer = $bugDefaults -replace '(\r?\n)(npc_mouths = true)', ('$1$2$1# A newer fix.$1brand_new = true')
Check ($newer -match 'brand_new') 'the newer defaults have a new setting'
[IO.File]::WriteAllText((Join-Path $bug 'settings.default.toml'), $newer)
$r = Run-Install $bug @()
Check ($r.Code -eq 0 -and $r.Out -match 'added the new settings fixes.brand_new') "update: new setting reported`n$($r.Out)"
$v = Values ([IO.File]::ReadAllText($settingsFile))
Check ($v['fixes.lulu_fury'] -eq 'false') 'update: the changed value stays'
Check ($v['fixes.brand_new'] -eq 'true') 'update: the new setting is there with its default'
Check ([IO.File]::ReadAllText($settingsFile) -match "# A newer fix.\r?\nbrand_new = true") 'update: with its comment'

# (c) switch off, stays off, on again
$r = Run-Install $feel @('-Disable', 'bug-fixes')
Check ($r.Code -eq 0) "-Disable exits 0`n$($r.Out)"
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.ps2-game-feel') '-Disable: out of loadorder'
Same (Read-Ids (Join-Path $mods 'ffx-mods-off.txt')) @('yuta.bug-fixes') '-Disable: in the off list'
$r = Run-Install $bug @()
Check ($r.Code -eq 0 -and $r.Out -match 'switched off') "a switched-off mod's own install.cmd says so`n$($r.Out)"
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.ps2-game-feel') 'its own install.cmd leaves it off'
# deleting a line by hand: that mod stays off too
Set-Content -LiteralPath (Join-Path $mods 'loadorder') -Value @('someone.else', 'yuta.ffx-mods')
$r = Run-Install $bug @('-Enable', 'yuta.bug-fixes')
Check ($r.Code -eq 0) "-Enable exits 0`n$($r.Out)"
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.bug-fixes') '-Enable: back in loadorder; the hand-deleted one stays out'
Same (Read-Ids (Join-Path $mods 'ffx-mods-off.txt')) @('yuta.ps2-game-feel') 'the hand-deleted mod is in the off list'
$r = Run-Install $feel @('-Enable', 'ps2-game-feel')
Same (LoadOrder $mods) @('someone.else', 'yuta.ffx-mods', 'yuta.bug-fixes', 'yuta.ps2-game-feel') 'on again'
Check (-not (Test-Path (Join-Path $mods 'ffx-mods-off.txt'))) 'the empty off list is deleted'
$r = Run-Install $feel @('-Disable', 'yuta.ffx-mods')
Check ($r.Code -ne 0) 'the shared library cannot be switched off'
$r = Run-Install $feel @('-Disable', 'no-such-mod')
Check ($r.Code -ne 0) 'an unknown mod cannot be switched off'

# ---- the PS2 bundle's installer, from its zip -----------------------------------------------
if ($BundleZip) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $fh2 = Join-Path $tmp 'fh2'
    [IO.Compression.ZipFile]::ExtractToDirectory((Resolve-Path $BundleZip).ProviderPath, $fh2)
    $mods2 = Join-Path $fh2 'mods'
    $bundle = Join-Path $mods2 'yuta.ps2-bundle'
    $exe = Join-Path $mods2 'yuta.ffx-mods/demaster.exe'
    $builders = @(& $exe mods | Where-Object { $_ -match '^yuta\.' -and $_ -notlike '*code only*' } | ForEach-Object { ($_ -split ' ')[0] } |
        Where-Object { Test-Path (Join-Path $mods2 $_) })
    $codeOnly = @(Get-ChildItem $mods2 -Directory | ForEach-Object { $_.Name } | Where-Object { $_ -ne 'yuta.ffx-mods' -and $_ -ne 'yuta.ps2-bundle' -and $builders -notcontains $_ })
    Check ($builders.Count -gt 0 -and $codeOnly.Count -gt 0) "bundle: mods that build ($($builders -join ',')) and code-only ones ($($codeOnly -join ','))"
    Check (@(Get-ChildItem $mods2 -Recurse -Filter 'settings.toml').Count -eq 0) 'bundle zip: no settings.toml'
    $r = Run-Install $bundle @('-Disable', ($builders -join ','))
    Check ($r.Code -eq 0) "bundle install with the PS2-file mods off`n$($r.Out)"
    $lo = LoadOrder $mods2
    Check ($lo[0] -eq 'yuta.ffx-mods' -and $lo[-1] -eq 'yuta.ps2-bundle') "bundle: shared library first, bundle last: $($lo -join ',')"
    foreach ($id in $codeOnly) {
        Check ($lo -contains $id) "bundle: $id in loadorder"
        Check (Test-Path (Join-Path $mods2 "$id/settings.toml")) "bundle: $id has settings.toml"
    }
    foreach ($id in $builders) { Check ($lo -notcontains $id) "bundle: switched-off $id not in loadorder" }
    # a hand-deleted line stays deleted
    $gone = $codeOnly[0]
    Set-Content -LiteralPath (Join-Path $mods2 'loadorder') -Value @($lo | Where-Object { $_ -ne $gone })
    $r = Run-Install $bundle @()
    Check ($r.Code -eq 0) "bundle reinstall`n$($r.Out)"
    Check ((LoadOrder $mods2) -notcontains $gone) "bundle: hand-deleted $gone stays out"
    Check ((Read-Ids (Join-Path $mods2 'ffx-mods-off.txt')) -contains $gone) "bundle: $gone in the off list"
}

Remove-Item -Recurse -Force $tmp
Write-Host "$script:passed passed, $script:failed failed"
if ($script:failed -gt 0) { exit 1 }
exit 0
