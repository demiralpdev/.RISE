@echo off
REM RISE auto-seal daemon installer (Windows)
REM Registers a SYSTEM scheduled task that survives any login state.
mkdir "%USERPROFILE%\.rise\bin" 2>nul
mkdir "%USERPROFILE%\.rise\keys" 2>nul
copy /y "%~dp0auto-seal.ps1" "%USERPROFILE%\.rise\auto-seal.ps1" >nul
"%USERPROFILE%\.rise\bin\rise-core.exe" keygen --out "%USERPROFILE%\.rise\keys\auto" 2>nul
schtasks /delete /tn RISE-AutoSeal /f 2>nul
schtasks /create /tn RISE-AutoSeal /tr "powershell -ExecutionPolicy Bypass -WindowStyle Hidden -File %USERPROFILE%\.rise\auto-seal.ps1" /sc onlogon /ru SYSTEM /f
schtasks /run /tn RISE-AutoSeal
echo RISE auto-seal daemon installed (SYSTEM task, starts at any logon).
