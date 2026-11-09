@echo off
rem Installs this mod of ffx-mods (install.ps1 next to this file says what it does).
rem Double-click it, or run: install.cmd [-Iso <disc image> | -Drive <letter> | -NoDisc] [-Game <game folder>] [-Force]
rem   [-Disable <mod>[,<mod>...]] [-Enable <mod>[,<mod>...]] (switch mods off and on again)
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1" %*
exit /b %ERRORLEVEL%
